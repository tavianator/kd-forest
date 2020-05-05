//! [k-d trees](https://en.wikipedia.org/wiki/K-d_tree).

use super::{Metric, NearestNeighbors, Neighborhood, Ordered};

use std::iter::FromIterator;

/// A point in Cartesian space.
pub trait Cartesian: Metric<[f64]> {
    /// Returns the number of dimensions necessary to describe this point.
    fn dimensions(&self) -> usize;

    /// Returns the value of the `i`th coordinate of this point (`i < self.dimensions()`).
    fn coordinate(&self, i: usize) -> f64;
}

/// Blanket [Cartesian] implementation for references.
impl<'a, T: Cartesian> Cartesian for &'a T {
    fn dimensions(&self) -> usize {
        (*self).dimensions()
    }

    fn coordinate(&self, i: usize) -> f64 {
        (*self).coordinate(i)
    }
}

/// Blanket [Metric<[f64]>](Metric) implementation for [Cartesian] references.
impl<'a, T: Cartesian> Metric<[f64]> for &'a T {
    type Distance = T::Distance;

    fn distance(&self, other: &[f64]) -> Self::Distance {
        (*self).distance(other)
    }
}

/// Standard cartesian space.
impl Cartesian for [f64] {
    fn dimensions(&self) -> usize {
        self.len()
    }

    fn coordinate(&self, i: usize) -> f64 {
        self[i]
    }
}

/// Marker trait for cartesian metric spaces.
pub trait CartesianMetric<T: ?Sized = Self>:
    Cartesian + Metric<T, Distance = <Self as Metric<[f64]>>::Distance>
{
}

/// Blanket [CartesianMetric] implementation for cartesian spaces with compatible metric distance
/// types.
impl<T, U> CartesianMetric<T> for U
where
    T: ?Sized,
    U: ?Sized + Cartesian + Metric<T, Distance = <U as Metric<[f64]>>::Distance>,
{
}

/// A node in a k-d tree.
#[derive(Debug)]
struct KdNode<T> {
    /// The value stored in this node.
    item: T,
    /// The size of the left subtree.
    left_len: usize,
}

impl<T: Cartesian> KdNode<T> {
    /// Create a new KdNode.
    fn new(item: T) -> Self {
        Self { item, left_len: 0 }
    }

    /// Build a k-d tree recursively.
    fn build(slice: &mut [KdNode<T>], i: usize) {
        if slice.is_empty() {
            return;
        }

        slice.sort_unstable_by_key(|n| Ordered(n.item.coordinate(i)));

        let mid = slice.len() / 2;
        slice.swap(0, mid);

        let (node, children) = slice.split_first_mut().unwrap();
        let (left, right) = children.split_at_mut(mid);
        node.left_len = left.len();

        let j = (i + 1) % node.item.dimensions();
        Self::build(left, j);
        Self::build(right, j);
    }

    /// Recursively search for nearest neighbors.
    fn recurse<'a, U, N>(
        slice: &'a [KdNode<T>],
        i: usize,
        closest: &mut [f64],
        neighborhood: &mut N,
    ) where
        T: 'a,
        U: CartesianMetric<&'a T>,
        N: Neighborhood<&'a T, U>,
    {
        let (node, children) = slice.split_first().unwrap();
        neighborhood.consider(&node.item);

        let target = neighborhood.target();
        let ti = target.coordinate(i);
        let ni = node.item.coordinate(i);
        let j = (i + 1) % node.item.dimensions();

        let (left, right) = children.split_at(node.left_len);
        let (near, far) = if ti <= ni {
            (left, right)
        } else {
            (right, left)
        };

        if !near.is_empty() {
            Self::recurse(near, j, closest, neighborhood);
        }

        if !far.is_empty() {
            let saved = closest[i];
            closest[i] = ni;
            if neighborhood.contains_distance(target.distance(closest)) {
                Self::recurse(far, j, closest, neighborhood);
            }
            closest[i] = saved;
        }
    }
}

/// A [k-d tree](https://en.wikipedia.org/wiki/K-d_tree).
#[derive(Debug)]
pub struct KdTree<T>(Vec<KdNode<T>>);

impl<T: Cartesian> FromIterator<T> for KdTree<T> {
    /// Create a new k-d tree from a set of points.
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        let mut nodes: Vec<_> = items.into_iter().map(KdNode::new).collect();
        KdNode::build(nodes.as_mut_slice(), 0);
        Self(nodes)
    }
}

impl<T, U> NearestNeighbors<T, U> for KdTree<T>
where
    T: Cartesian,
    U: CartesianMetric<T>,
{
    fn search<'a, 'b, N>(&'a self, mut neighborhood: N) -> N
    where
        T: 'a,
        U: 'b,
        N: Neighborhood<&'a T, &'b U>,
    {
        if !self.0.is_empty() {
            let target = neighborhood.target();
            let dims = target.dimensions();
            let mut closest: Vec<_> = (0..dims).map(|i| target.coordinate(i)).collect();

            KdNode::recurse(&self.0, 0, &mut closest, &mut neighborhood);
        }

        neighborhood
    }
}

/// An iterator that the moves values out of a k-d tree.
#[derive(Debug)]
pub struct IntoIter<T>(std::vec::IntoIter<KdNode<T>>);

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.0.next().map(|n| n.item)
    }
}

impl<T> IntoIterator for KdTree<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::metric::tests::{test_nearest_neighbors, Point};
    use crate::metric::SquaredDistance;

    impl Metric<[f64]> for Point {
        type Distance = SquaredDistance;

        fn distance(&self, other: &[f64]) -> Self::Distance {
            self.0.distance(other)
        }
    }

    impl Cartesian for Point {
        fn dimensions(&self) -> usize {
            self.0.dimensions()
        }

        fn coordinate(&self, i: usize) -> f64 {
            self.0.coordinate(i)
        }
    }

    #[test]
    fn test_kd_tree() {
        test_nearest_neighbors(KdTree::from_iter);
    }
}
