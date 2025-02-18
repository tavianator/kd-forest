//! [Dynamization](https://en.wikipedia.org/wiki/Dynamization) for nearest neighbor search.

use acap::distance::Proximity;
use acap::kd::FlatKdTree;
use acap::knn::{NearestNeighbors, Neighborhood};
use acap::vp::FlatVpTree;

use std::iter;

/// A trait for objects that can be soft-deleted.
pub trait SoftDelete {
    /// Check whether this item is deleted.
    fn is_deleted(&self) -> bool;
}

/// Blanket [SoftDelete] implementation for references.
impl<'a, T: SoftDelete> SoftDelete for &'a T {
    fn is_deleted(&self) -> bool {
        (*self).is_deleted()
    }
}

/// The number of bits dedicated to the flat buffer.
const BUFFER_BITS: usize = 6;
/// The maximum size of the buffer.
const BUFFER_SIZE: usize = 1 << BUFFER_BITS;

/// A dynamic wrapper for a static nearest neighbor search data structure.
///
/// This type applies [dynamization](https://en.wikipedia.org/wiki/Dynamization) to an arbitrary
/// nearest neighbor search structure `T`, allowing new items to be added dynamically.  It also
/// implements [soft deletion](https://en.wiktionary.org/wiki/soft_deletion) for dynamic removal.
#[derive(Debug)]
pub struct Forest<T: IntoIterator> {
    /// A flat buffer used for the first few items, to avoid repeatedly rebuilding small trees.
    buffer: Vec<T::Item>,
    /// The trees of the forest, with sizes in geometric progression.
    trees: Vec<Option<T>>,
}

impl<T, U> Forest<U>
where
    T: SoftDelete,
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

    /// Remove deleted items from the buffer.
    fn filter_buffer(&mut self) {
        self.buffer.retain(|e| !e.is_deleted());
    }

    /// Drain all items out of the trees and into the buffer.
    fn deforest(&mut self) {
        self.buffer.extend(
            self.trees
                .drain(..)
                .flatten()
                .flatten()
                .filter(|e| !e.is_deleted())
        );
    }

    /// Move excess items from the buffer to the trees.
    fn reforest(&mut self) {
        let mut len = self.buffer.len();

        for i in 0.. {
            let bit = 1 << (i + BUFFER_BITS);
            if bit > len {
                break;
            }

            if i >= self.trees.len() {
                self.trees.push(None);
            }

            let tree = self.trees[i].take();
            self.trees[i] = match (tree, len & bit > 0) {
                (Some(tree), true) => {
                    len += bit;
                    self.buffer.extend(tree.into_iter().filter(|e| !e.is_deleted()));
                    None
                }
                (None, true) => {
                    let offset = self.buffer.len().saturating_sub(bit);
                    Some(self.buffer.drain(offset..).collect())
                }
                (tree, _) => tree,
            }
        }

        debug_assert!(self.buffer.len() < BUFFER_SIZE);
    }

    /// Rebuild this index, discarding deleted items.
    pub fn rebuild(&mut self) {
        self.filter_buffer();
        self.deforest();
        self.reforest();
    }
}

impl<T, U> Default for Forest<U>
where
    T: SoftDelete,
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, U> Extend<T> for Forest<U>
where
    T: SoftDelete,
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, items: I) {
        self.buffer.extend(items);

        if self.buffer.len() >= BUFFER_SIZE {
            self.filter_buffer();
            self.reforest();
        }
    }
}

impl<T, U> FromIterator<T> for Forest<U>
where
    T: SoftDelete,
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        let mut forest = Self::new();
        forest.extend(items);
        forest
    }
}

impl<T, U> IntoIterator for Forest<U>
where
    T: SoftDelete,
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(mut self) -> Self::IntoIter {
        self.filter_buffer();
        self.deforest();
        self.buffer.into_iter()
    }
}

/// [Neighborhood] wrapper that ignores soft-deleted items.
#[derive(Debug)]
struct SoftNeighborhood<N>(N);

impl<K, V, N> Neighborhood<K, V> for SoftNeighborhood<N>
where
    V: SoftDelete,
    K: Proximity<V>,
    N: Neighborhood<K, V>,
{
    fn target(&self) -> K {
        self.0.target()
    }

    fn contains<D>(&self, distance: D) -> bool
    where
        D: PartialOrd<K::Distance>
    {
        self.0.contains(distance)
    }

    fn consider(&mut self, item: V) -> K::Distance {
        if item.is_deleted() {
            self.target().distance(&item)
        } else {
            self.0.consider(item)
        }
    }
}

impl<K, V, T> NearestNeighbors<K, V> for Forest<T>
where
    K: Proximity<V>,
    V: SoftDelete,
    T: NearestNeighbors<K, V>,
    T: IntoIterator<Item = V>,
{
    fn search<'k, 'v, N>(&'v self, mut neighborhood: N) -> N
    where
        K: 'k,
        V: 'v,
        N: Neighborhood<&'k K, &'v V>
    {
        for item in &self.buffer {
            if !item.is_deleted() {
                neighborhood.consider(item);
            }
        }

        let neighborhood = SoftNeighborhood(neighborhood);

        self.trees
            .iter()
            .flatten()
            .fold(neighborhood, |n, t| t.search(n))
            .0
    }
}

/// A forest of k-d trees.
pub type KdForest<T> = Forest<FlatKdTree<T>>;

/// A forest of vantage-point trees.
pub type VpForest<T> = Forest<FlatVpTree<T>>;

#[cfg(test)]
mod tests {
    use super::*;

    use acap::coords::Coordinates;
    use acap::euclid::{euclidean_distance, Euclidean, EuclideanDistance};
    use acap::exhaustive::ExhaustiveSearch;
    use acap::knn::{NearestNeighbors, Neighbor};

    use rand::random;

    type Point = Euclidean<[f32; 3]>;

    #[derive(Clone, Debug, PartialEq)]
    struct SoftPoint {
        point: [f32; 3],
        deleted: bool,
    }

    impl SoftPoint {
        fn new(x: f32, y: f32, z: f32) -> Self {
            Self {
                point: [x, y, z],
                deleted: false,
            }
        }

        fn deleted(x: f32, y: f32, z: f32) -> Self {
            Self {
                point: [x, y, z],
                deleted: true,
            }
        }
    }

    impl SoftDelete for SoftPoint {
        fn is_deleted(&self) -> bool {
            self.deleted
        }
    }

    impl Proximity for SoftPoint {
        type Distance = EuclideanDistance<f32>;

        fn distance(&self, other: &Self) -> Self::Distance {
            euclidean_distance(&self.point, &other.point)
        }
    }

    impl Coordinates for SoftPoint {
        type Value = <Point as Coordinates>::Value;

        fn dims(&self) -> usize {
            self.point.dims()
        }

        fn coord(&self, i: usize) -> Self::Value {
            self.point.coord(i)
        }
    }

    impl Proximity<SoftPoint> for Point {
        type Distance = EuclideanDistance<f32>;

        fn distance(&self, other: &SoftPoint) -> Self::Distance {
            euclidean_distance(&self, &other.point)
        }
    }

    fn test_empty<T, F>(from_iter: &F)
    where
        T: NearestNeighbors<Point, SoftPoint>,
        F: Fn(Vec<SoftPoint>) -> T,
    {
        let points = Vec::new();
        let index = from_iter(points);
        let target = Euclidean([0.0, 0.0, 0.0]);
        assert_eq!(index.nearest(&target), None);
        assert_eq!(index.nearest_within(&target, 1.0), None);
        assert!(index.k_nearest(&target, 0).is_empty());
        assert!(index.k_nearest(&target, 3).is_empty());
        assert!(index.k_nearest_within(&target, 0, 1.0).is_empty());
        assert!(index.k_nearest_within(&target, 3, 1.0).is_empty());
    }

    fn test_pythagorean<T, F>(from_iter: &F)
    where
        T: NearestNeighbors<Point, SoftPoint>,
        F: Fn(Vec<SoftPoint>) -> T,
    {
        let points = vec![
            SoftPoint::deleted(0.0, 0.0, 0.0),
            SoftPoint::new(3.0, 4.0, 0.0),
            SoftPoint::new(5.0, 0.0, 12.0),
            SoftPoint::new(0.0, 8.0, 15.0),
            SoftPoint::new(1.0, 2.0, 2.0),
            SoftPoint::new(2.0, 3.0, 6.0),
            SoftPoint::new(4.0, 4.0, 7.0),
        ];
        let index = from_iter(points);
        let target = Euclidean([0.0, 0.0, 0.0]);

        assert_eq!(
            index.nearest(&target).expect("No nearest neighbor found"),
            Neighbor::new(&SoftPoint::new(1.0, 2.0, 2.0), 3.0)
        );

        assert_eq!(index.nearest_within(&target, 2.0), None);
        assert_eq!(
            index.nearest_within(&target, 4.0).expect("No nearest neighbor found within 4.0"),
            Neighbor::new(&SoftPoint::new(1.0, 2.0, 2.0), 3.0)
        );

        assert!(index.k_nearest(&target, 0).is_empty());
        assert_eq!(
            index.k_nearest(&target, 3),
            vec![
                Neighbor::new(&SoftPoint::new(1.0, 2.0, 2.0), 3.0),
                Neighbor::new(&SoftPoint::new(3.0, 4.0, 0.0), 5.0),
                Neighbor::new(&SoftPoint::new(2.0, 3.0, 6.0), 7.0),
            ]
        );

        assert!(index.k_nearest(&target, 0).is_empty());
        assert_eq!(
            index.k_nearest_within(&target, 3, 6.0),
            vec![
                Neighbor::new(&SoftPoint::new(1.0, 2.0, 2.0), 3.0),
                Neighbor::new(&SoftPoint::new(3.0, 4.0, 0.0), 5.0),
            ]
        );
        assert_eq!(
            index.k_nearest_within(&target, 3, 8.0),
            vec![
                Neighbor::new(&SoftPoint::new(1.0, 2.0, 2.0), 3.0),
                Neighbor::new(&SoftPoint::new(3.0, 4.0, 0.0), 5.0),
                Neighbor::new(&SoftPoint::new(2.0, 3.0, 6.0), 7.0),
            ]
        );
    }

    fn test_random_points<T, F>(from_iter: &F)
    where
        T: NearestNeighbors<Point, SoftPoint>,
        F: Fn(Vec<SoftPoint>) -> T,
    {
        let mut points = Vec::new();
        for _ in 0..255 {
            points.push(SoftPoint::new(random(), random(), random()));
            points.push(SoftPoint::deleted(random(), random(), random()));
        }
        let target = Euclidean([random(), random(), random()]);

        let eindex: ExhaustiveSearch<_> = points
            .iter()
            .filter(|p| !p.is_deleted())
            .cloned()
            .collect();

        let index = from_iter(points);

        assert_eq!(index.k_nearest(&target, 3), eindex.k_nearest(&target, 3));
    }

    /// Test a [NearestNeighbors] impl.
    fn test_nearest_neighbors<T, F>(from_iter: F)
    where
        T: NearestNeighbors<Point, SoftPoint>,
        F: Fn(Vec<SoftPoint>) -> T,
    {
        test_empty(&from_iter);
        test_pythagorean(&from_iter);
        test_random_points(&from_iter);
    }

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
