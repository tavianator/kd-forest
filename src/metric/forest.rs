//! [Dynamization](https://en.wikipedia.org/wiki/Dynamization) for nearest neighbor search.

use super::kd::KdTree;
use super::vp::VpTree;
use super::{Metric, NearestNeighbors, Neighborhood};

use std::iter::{self, Extend, Flatten, FromIterator};

/// A dynamic wrapper for a static nearest neighbor search data structure.
///
/// This type applies [dynamization](https://en.wikipedia.org/wiki/Dynamization) to an arbitrary
/// nearest neighbor search structure `T`, allowing new items to be added dynamically.
#[derive(Debug)]
pub struct Forest<T>(Vec<Option<T>>);

impl<T, U> Forest<U>
where
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    /// Create a new empty forest.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Add a new item to the forest.
    pub fn push(&mut self, item: T) {
        self.extend(iter::once(item));
    }

    /// Get the number of items in the forest.
    pub fn len(&self) -> usize {
        let mut len = 0;
        for (i, slot) in self.0.iter().enumerate() {
            if slot.is_some() {
                len |= 1 << i;
            }
        }
        len
    }
}

impl<T, U> Extend<T> for Forest<U>
where
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, items: I) {
        let mut vec: Vec<_> = items.into_iter().collect();
        let new_len = self.len() + vec.len();

        for i in 0.. {
            let bit = 1 << i;

            if bit > new_len {
                break;
            }

            if i >= self.0.len() {
                self.0.push(None);
            }

            if new_len & bit == 0 {
                if let Some(tree) = self.0[i].take() {
                    vec.extend(tree);
                }
            } else if self.0[i].is_none() {
                let offset = vec.len() - bit;
                self.0[i] = Some(vec.drain(offset..).collect());
            }
        }

        debug_assert!(vec.is_empty());
        debug_assert!(self.len() == new_len);
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

type IntoIterImpl<T> = Flatten<Flatten<std::vec::IntoIter<Option<T>>>>;

/// An iterator that moves items out of a forest.
pub struct IntoIter<T: IntoIterator>(IntoIterImpl<T>);

impl<T: IntoIterator> Iterator for IntoIter<T> {
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<T: IntoIterator> IntoIterator for Forest<T> {
    type Item = T::Item;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter().flatten().flatten())
    }
}

impl<T, U, V> NearestNeighbors<T, U> for Forest<V>
where
    U: Metric<T>,
    V: NearestNeighbors<T, U>,
{
    fn search<'a, 'b, N>(&'a self, neighborhood: N) -> N
    where
        T: 'a,
        U: 'b,
        N: Neighborhood<&'a T, &'b U>,
    {
        self.0
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
