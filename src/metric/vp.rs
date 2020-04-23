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
    /// The size of the subtree inside the radius.
    inside_len: usize,
}

impl<T: Metric> VpNode<T> {
    /// Create a new VpNode.
    fn new(item: T) -> Self {
        Self {
            item,
            radius: 0.0,
            inside_len: 0,
        }
    }

    /// Build a VP tree recursively.
    fn build(slice: &mut [VpNode<T>]) {
        if let Some((node, children)) = slice.split_first_mut() {
            let item = &node.item;
            children.sort_by_cached_key(|n| item.distance(&n.item));

            let (inside, outside) = children.split_at_mut(children.len() / 2);
            if let Some(last) = inside.last() {
                node.radius = item.distance(&last.item).into();
            }
            node.inside_len = inside.len();

            Self::build(inside);
            Self::build(outside);
        }
    }

    /// Recursively search for nearest neighbors.
    fn recurse<'a, U, N>(slice: &'a [VpNode<T>], neighborhood: &mut N)
    where
        T: 'a,
        U: Metric<&'a T>,
        N: Neighborhood<&'a T, U>,
    {
        let (node, children) = slice.split_first().unwrap();
        let (inside, outside) = children.split_at(node.inside_len);

        let distance = neighborhood.consider(&node.item).into();

        if distance <= node.radius {
            if !inside.is_empty() && neighborhood.contains(distance - node.radius) {
                Self::recurse(inside, neighborhood);
            }
            if !outside.is_empty() && neighborhood.contains(node.radius - distance) {
                Self::recurse(outside, neighborhood);
            }
        } else {
            if !outside.is_empty() && neighborhood.contains(node.radius - distance) {
                Self::recurse(outside, neighborhood);
            }
            if !inside.is_empty() && neighborhood.contains(distance - node.radius) {
                Self::recurse(inside, neighborhood);
            }
        }
    }
}

/// A [vantage-point tree](https://en.wikipedia.org/wiki/Vantage-point_tree).
#[derive(Debug)]
pub struct VpTree<T>(Vec<VpNode<T>>);

impl<T: Metric> FromIterator<T> for VpTree<T> {
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        let mut nodes: Vec<_> = items.into_iter().map(VpNode::new).collect();
        VpNode::build(nodes.as_mut_slice());
        Self(nodes)
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
        if !self.0.is_empty() {
            VpNode::recurse(&self.0, &mut neighborhood);
        }

        neighborhood
    }
}

/// An iterator that moves values out of a VP tree.
#[derive(Debug)]
pub struct IntoIter<T>(std::vec::IntoIter<VpNode<T>>);

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.0.next().map(|n| n.item)
    }
}

impl<T> IntoIterator for VpTree<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
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
