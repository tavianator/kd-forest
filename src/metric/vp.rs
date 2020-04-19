//! [Vantage-point trees](https://en.wikipedia.org/wiki/Vantage-point_tree).

use super::{Metric, NearestNeighbors, Neighborhood};

use std::iter::FromIterator;

/// A node in a VP tree.
#[derive(Debug)]
struct VpNode<T> {
    /// The vantage point itself.
    item: T,
    /// The radius of this node.
    radius: f64,
    /// The subtree inside the radius, if any.
    inside: Option<Box<Self>>,
    /// The subtree outside the radius, if any.
    outside: Option<Box<Self>>,
}

impl<T: Metric> VpNode<T> {
    /// Create a new VpNode.
    fn new(mut items: Vec<T>) -> Option<Box<Self>> {
        if items.is_empty() {
            return None;
        }

        let item = items.pop().unwrap();

        items.sort_by_cached_key(|a| item.distance(a));

        let mid = items.len() / 2;
        let outside: Vec<T> = items.drain(mid..).collect();

        let radius = items.last().map(|l| item.distance(l).into()).unwrap_or(0.0);

        Some(Box::new(Self {
            item,
            radius,
            inside: Self::new(items),
            outside: Self::new(outside),
        }))
    }
}

trait VpSearch<'a, T, U, N> {
    /// Recursively search for nearest neighbors.
    fn search(&'a self, neighborhood: &mut N);

    /// Search the inside subtree.
    fn search_inside(&'a self, distance: f64, neighborhood: &mut N);

    /// Search the outside subtree.
    fn search_outside(&'a self, distance: f64, neighborhood: &mut N);
}

impl<'a, T, U, N> VpSearch<'a, T, U, N> for VpNode<T>
where
    T: 'a,
    U: Metric<&'a T>,
    N: Neighborhood<&'a T, U>,
{
    fn search(&'a self, neighborhood: &mut N) {
        let distance = neighborhood.consider(&self.item).into();

        if distance <= self.radius {
            self.search_inside(distance, neighborhood);
            self.search_outside(distance, neighborhood);
        } else {
            self.search_outside(distance, neighborhood);
            self.search_inside(distance, neighborhood);
        }
    }

    fn search_inside(&'a self, distance: f64, neighborhood: &mut N) {
        if let Some(inside) = &self.inside {
            if neighborhood.contains(distance - self.radius) {
                inside.search(neighborhood);
            }
        }
    }

    fn search_outside(&'a self, distance: f64, neighborhood: &mut N) {
        if let Some(outside) = &self.outside {
            if neighborhood.contains(self.radius - distance) {
                outside.search(neighborhood);
            }
        }
    }
}

/// A [vantage-point tree](https://en.wikipedia.org/wiki/Vantage-point_tree).
#[derive(Debug)]
pub struct VpTree<T> {
    root: Option<Box<VpNode<T>>>,
}

impl<T: Metric> FromIterator<T> for VpTree<T> {
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        Self {
            root: VpNode::new(items.into_iter().collect::<Vec<_>>()),
        }
    }
}

impl<T, U> NearestNeighbors<T, U> for VpTree<T>
where
    T: Metric,
    U: Metric<T>,
{
    fn search<'a, 'b, N>(&'a self, mut neighborhood: N) -> N
    where
        T: 'a,
        U: 'b,
        N: Neighborhood<&'a T, &'b U>,
    {
        if let Some(root) = &self.root {
            root.search(&mut neighborhood);
        }
        neighborhood
    }
}

/// An iterator that moves values out of a VP tree.
#[derive(Debug)]
pub struct IntoIter<T> {
    stack: Vec<Box<VpNode<T>>>,
}

impl<T> IntoIter<T> {
    fn new(node: Option<Box<VpNode<T>>>) -> Self {
        Self {
            stack: node.into_iter().collect(),
        }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.stack.pop().map(|node| {
            self.stack.extend(node.inside);
            self.stack.extend(node.outside);
            node.item
        })
    }
}

impl<T> IntoIterator for VpTree<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::metric::tests::test_nearest_neighbors;

    #[test]
    fn test_vp_tree() {
        test_nearest_neighbors(VpTree::from_iter);
    }
}
