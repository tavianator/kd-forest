//! [Dynamization](https://en.wikipedia.org/wiki/Dynamization) for nearest neighbor search.

use super::kd::KdTree;
use super::vp::VpTree;
use super::{Metric, NearestNeighbors, Neighborhood};

use std::iter::{self, Extend, FromIterator};

/// The number of bits dedicated to the flat buffer.
const BUFFER_BITS: usize = 6;
/// The maximum size of the buffer.
const BUFFER_SIZE: usize = 1 << BUFFER_BITS;

/// A dynamic wrapper for a static nearest neighbor search data structure.
///
/// This type applies [dynamization](https://en.wikipedia.org/wiki/Dynamization) to an arbitrary
/// nearest neighbor search structure `T`, allowing new items to be added dynamically.
#[derive(Debug)]
pub struct Forest<T: IntoIterator> {
    /// A flat buffer used for the first few items, to avoid repeatedly rebuilding small trees.
    buffer: Vec<T::Item>,
    /// The trees of the forest, with sizes in geometric progression.
    trees: Vec<Option<T>>,
}

impl<T, U> Forest<U>
where
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    /// Create a new empty forest.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            trees: Vec::new(),
        }
    }

    /// Add a new item to the forest.
    pub fn push(&mut self, item: T) {
        self.extend(iter::once(item));
    }

    /// Get the number of items in the forest.
    pub fn len(&self) -> usize {
        let mut len = self.buffer.len();
        for (i, slot) in self.trees.iter().enumerate() {
            if slot.is_some() {
                len += 1 << (i + BUFFER_BITS);
            }
        }
        len
    }

    /// Check if this forest is empty.
    pub fn is_empty(&self) -> bool {
        if !self.buffer.is_empty() {
            return false;
        }

        self.trees.iter().flatten().next().is_none()
    }
}

impl<T, U> Default for Forest<U>
where
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, U> Extend<T> for Forest<U>
where
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, items: I) {
        self.buffer.extend(items);
        if self.buffer.len() < BUFFER_SIZE {
            return;
        }

        let len = self.len();

        for i in 0.. {
            let bit = 1 << (i + BUFFER_BITS);

            if bit > len {
                break;
            }

            if i >= self.trees.len() {
                self.trees.push(None);
            }

            if len & bit == 0 {
                if let Some(tree) = self.trees[i].take() {
                    self.buffer.extend(tree);
                }
            } else if self.trees[i].is_none() {
                let offset = self.buffer.len() - bit;
                self.trees[i] = Some(self.buffer.drain(offset..).collect());
            }
        }

        debug_assert!(self.buffer.len() < BUFFER_SIZE);
        debug_assert!(self.len() == len);
    }
}

impl<T, U> FromIterator<T> for Forest<U>
where
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        let mut forest = Self::new();
        forest.extend(items);
        forest
    }
}

impl<T: IntoIterator> IntoIterator for Forest<T> {
    type Item = T::Item;
    type IntoIter = std::vec::IntoIter<T::Item>;

    fn into_iter(mut self) -> Self::IntoIter {
        self.buffer.extend(self.trees.into_iter().flatten().flatten());
        self.buffer.into_iter()
    }
}

impl<T, U, V> NearestNeighbors<T, U> for Forest<V>
where
    U: Metric<T>,
    V: NearestNeighbors<T, U>,
    V: IntoIterator<Item = T>,
{
    fn search<'a, 'b, N>(&'a self, mut neighborhood: N) -> N
    where
        T: 'a,
        U: 'b,
        N: Neighborhood<&'a T, &'b U>,
    {
        for item in &self.buffer {
            neighborhood.consider(item);
        }

        self.trees
            .iter()
            .flatten()
            .fold(neighborhood, |n, t| t.search(n))
    }
}

/// A forest of k-d trees.
pub type KdForest<T> = Forest<KdTree<T>>;

/// A forest of vantage-point trees.
pub type VpForest<T> = Forest<VpTree<T>>;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::metric::tests::test_nearest_neighbors;
    use crate::metric::ExhaustiveSearch;

    #[test]
    fn test_exhaustive_forest() {
        test_nearest_neighbors(Forest::<ExhaustiveSearch<_>>::from_iter);
    }

    #[test]
    fn test_forest_forest() {
        test_nearest_neighbors(Forest::<Forest<ExhaustiveSearch<_>>>::from_iter);
    }

    #[test]
    fn test_kd_forest() {
        test_nearest_neighbors(KdForest::from_iter);
    }

    #[test]
    fn test_vp_forest() {
        test_nearest_neighbors(VpForest::from_iter);
    }
}
