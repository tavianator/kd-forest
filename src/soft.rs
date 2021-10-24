//! [Soft deletion](https://en.wiktionary.org/wiki/soft_deletion) for nearest neighbor search.

use super::forest::{KdForest, VpForest};

use acap::distance::Proximity;
use acap::kd::FlatKdTree;
use acap::knn::{NearestNeighbors, Neighborhood};
use acap::vp::FlatVpTree;

use std::iter;
use std::iter::FromIterator;
use std::mem;

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

/// A [NearestNeighbors] implementation that supports [soft deletes](https://en.wiktionary.org/wiki/soft_deletion).
#[derive(Debug)]
pub struct SoftSearch<T>(T);

impl<T, U> SoftSearch<U>
where
    T: SoftDelete,
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    /// Create a new empty soft index.
    pub fn new() -> Self {
        Self(iter::empty().collect())
    }

    /// Push a new item into this index.
    pub fn push(&mut self, item: T)
    where
        U: Extend<T>,
    {
        self.0.extend(iter::once(item));
    }

    /// Rebuild this index, discarding deleted items.
    pub fn rebuild(&mut self) {
        let items = mem::replace(&mut self.0, iter::empty().collect());
        self.0 = items.into_iter().filter(|e| !e.is_deleted()).collect();
    }
}

impl<T, U> Default for SoftSearch<U>
where
    T: SoftDelete,
    U: FromIterator<T> + IntoIterator<Item = T>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, U: Extend<T>> Extend<T> for SoftSearch<U> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<T, U: FromIterator<T>> FromIterator<T> for SoftSearch<U> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(U::from_iter(iter))
    }
}

impl<T: IntoIterator> IntoIterator for SoftSearch<T> {
    type Item = T::Item;
    type IntoIter = T::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<K, V, T> NearestNeighbors<K, V> for SoftSearch<T>
where
    K: Proximity<V>,
    V: SoftDelete,
    T: NearestNeighbors<K, V>,
{
    fn search<'k, 'v, N>(&'v self, neighborhood: N) -> N
    where
        K: 'k,
        V: 'v,
        N: Neighborhood<&'k K, &'v V>
    {
        self.0.search(SoftNeighborhood(neighborhood)).0
    }
}

/// A k-d forest that supports soft deletes.
pub type SoftKdForest<T> = SoftSearch<KdForest<T>>;

/// A k-d tree that supports soft deletes.
pub type SoftKdTree<T> = SoftSearch<FlatKdTree<T>>;

/// A VP forest that supports soft deletes.
pub type SoftVpForest<T> = SoftSearch<VpForest<T>>;

/// A VP tree that supports soft deletes.
pub type SoftVpTree<T> = SoftSearch<FlatVpTree<T>>;

#[cfg(test)]
mod tests {
    use super::*;

    use acap::coords::Coordinates;
    use acap::euclid::{euclidean_distance, Euclidean, EuclideanDistance};
    use acap::knn::Neighbor;

    type Point = Euclidean<[f32; 3]>;

    #[derive(Debug, PartialEq)]
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

    fn test_index<T>(index: &T)
    where
        T: NearestNeighbors<Point, SoftPoint>,
    {
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

        assert_eq!(
            index.k_nearest(&target, 3),
            vec![
                Neighbor::new(&SoftPoint::new(1.0, 2.0, 2.0), 3.0),
                Neighbor::new(&SoftPoint::new(3.0, 4.0, 0.0), 5.0),
                Neighbor::new(&SoftPoint::new(2.0, 3.0, 6.0), 7.0),
            ]
        );

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

    fn test_soft_index<T>(index: &mut SoftSearch<T>)
    where
        T: Extend<SoftPoint>,
        T: FromIterator<SoftPoint>,
        T: IntoIterator<Item = SoftPoint>,
        T: NearestNeighbors<Point, SoftPoint>,
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

        for point in points {
            index.push(point);
        }
        test_index(index);

        index.rebuild();
        test_index(index);
    }

    #[test]
    fn test_soft_kd_forest() {
        test_soft_index(&mut SoftKdForest::new());
    }

    #[test]
    fn test_soft_vp_forest() {
        test_soft_index(&mut SoftVpForest::new());
    }
}
