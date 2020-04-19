//! [k-d trees](https://en.wikipedia.org/wiki/K-d_tree).

use super::{Metric, NearestNeighbors, Neighborhood};

use ordered_float::OrderedFloat;

use std::iter::FromIterator;

/// A point in Cartesian space.
pub trait Cartesian {
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

/// Standard cartesian space.
impl Cartesian for [f64] {
    fn dimensions(&self) -> usize {
        self.len()
    }

    fn coordinate(&self, i: usize) -> f64 {
        self[i]
    }
}

/// A node in a k-d tree.
#[derive(Debug)]
struct KdNode<T> {
    /// The value stored in this node.
    item: T,
    /// The left subtree, if any.
    left: Option<Box<Self>>,
    /// The right subtree, if any.
    right: Option<Box<Self>>,
}

trait KdSearch<'a, T, U, N> {
    /// Recursively search for nearest neighbors.
    fn search(&'a self, i: usize, neighborhood: &mut N);

    /// Search the left subtree.
    fn search_left(&'a self, i: usize, distance: f64, neighborhood: &mut N);

    /// Search the right subtree.
    fn search_right(&'a self, i: usize, distance: f64, neighborhood: &mut N);
}

impl<'a, T, U, N> KdSearch<'a, T, U, N> for KdNode<T>
where
    T: 'a + Cartesian,
    U: Cartesian + Metric<&'a T>,
    N: Neighborhood<&'a T, U>,
{
    fn search(&'a self, i: usize, neighborhood: &mut N) {
        neighborhood.consider(&self.item);

        let distance = neighborhood.target().coordinate(i) - self.item.coordinate(i);
        let j = (i + 1) % self.item.dimensions();
        if distance <= 0.0 {
            self.search_left(j, distance, neighborhood);
            self.search_right(j, -distance, neighborhood);
        } else {
            self.search_right(j, -distance, neighborhood);
            self.search_left(j, distance, neighborhood);
        }
    }

    fn search_left(&'a self, i: usize, distance: f64, neighborhood: &mut N) {
        if let Some(left) = &self.left {
            if neighborhood.contains(distance) {
                left.search(i, neighborhood);
            }
        }
    }

    fn search_right(&'a self, i: usize, distance: f64, neighborhood: &mut N) {
        if let Some(right) = &self.right {
            if neighborhood.contains(distance) {
                right.search(i, neighborhood);
            }
        }
    }
}

impl<T: Cartesian> KdNode<T> {
    /// Create a new KdNode.
    fn new(i: usize, mut items: Vec<T>) -> Option<Box<Self>> {
        if items.is_empty() {
            return None;
        }

        items.sort_unstable_by_key(|x| OrderedFloat::from(x.coordinate(i)));

        let mid = items.len() / 2;
        let right: Vec<T> = items.drain((mid + 1)..).collect();
        let item = items.pop().unwrap();
        let j = (i + 1) % item.dimensions();
        Some(Box::new(Self {
            item,
            left: Self::new(j, items),
            right: Self::new(j, right),
        }))
    }
}

/// A [k-d tree](https://en.wikipedia.org/wiki/K-d_tree).
#[derive(Debug)]
pub struct KdTree<T> {
    root: Option<Box<KdNode<T>>>,
}

impl<T: Cartesian> FromIterator<T> for KdTree<T> {
    /// Create a new k-d tree from a set of points.
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        Self {
            root: KdNode::new(0, items.into_iter().collect()),
        }
    }
}

impl<T, U> NearestNeighbors<T, U> for KdTree<T>
where
    T: Cartesian,
    U: Cartesian + Metric<T>,
{
    fn search<'a, 'b, N>(&'a self, mut neighborhood: N) -> N
    where
        T: 'a,
        U: 'b,
        N: Neighborhood<&'a T, &'b U>,
    {
        if let Some(root) = &self.root {
            root.search(0, &mut neighborhood);
        }
        neighborhood
    }
}

/// An iterator that the moves values out of a k-d tree.
#[derive(Debug)]
pub struct IntoIter<T> {
    stack: Vec<Box<KdNode<T>>>,
}

impl<T> IntoIter<T> {
    fn new(node: Option<Box<KdNode<T>>>) -> Self {
        Self {
            stack: node.into_iter().collect(),
        }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.stack.pop().map(|node| {
            self.stack.extend(node.left);
            self.stack.extend(node.right);
            node.item
        })
    }
}

impl<T> IntoIterator for KdTree<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self.root)
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
