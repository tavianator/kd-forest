//! [Soft deletion](https://en.wiktionary.org/wiki/soft_deletion) for nearest neighbor search.

use super::forest::{KdForest, VpForest};
use super::kd::KdTree;
use super::vp::VpTree;
use super::{Metric, NearestNeighbors, Neighborhood};

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

impl<T, U, N> Neighborhood<T, U> for SoftNeighborhood<N>
where
    T: SoftDelete,
    U: Metric<T>,
    N: Neighborhood<T, U>,
{
    fn target(&self) -> U {
        self.0.target()
    }

    fn contains(&self, distance: f64) -> bool {
        self.0.contains(distance)
    }

    fn contains_distance(&self, distance: U::Distance) -> bool {
        self.0.contains_distance(distance)
    }

    fn consider(&mut self, item: T) -> U::Distance {
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

impl<T, U, V> NearestNeighbors<T, U> for SoftSearch<V>
where
    T: SoftDelete,
    U: Metric<T>,
    V: NearestNeighbors<T, U>,
{
    fn search<'a, 'b, N>(&'a self, neighborhood: N) -> N
    where
        T: 'a,
        U: 'b,
        N: Neighborhood<&'a T, &'b U>,
    {
        self.0.search(SoftNeighborhood(neighborhood)).0
    }
}

/// A k-d forest that supports soft deletes.
pub type SoftKdForest<T> = SoftSearch<KdForest<T>>;

/// A k-d tree that supports soft deletes.
pub type SoftKdTree<T> = SoftSearch<KdTree<T>>;

/// A VP forest that supports soft deletes.
pub type SoftVpForest<T> = SoftSearch<VpForest<T>>;

/// A VP tree that supports soft deletes.
pub type SoftVpTree<T> = SoftSearch<VpTree<T>>;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::metric::kd::Cartesian;
    use crate::metric::tests::Point;
    use crate::metric::Neighbor;

    #[derive(Debug, PartialEq)]
    struct SoftPoint {
        point: Point,
        deleted: bool,
    }

    impl SoftPoint {
        fn new(x: f64, y: f64, z: f64) -> Self {
            Self {
                point: Point([x, y, z]),
                deleted: false,
            }
        }

        fn deleted(x: f64, y: f64, z: f64) -> Self {
            Self {
                point: Point([x, y, z]),
                deleted: true,
            }
        }
    }

    impl SoftDelete for SoftPoint {
        fn is_deleted(&self) -> bool {
            self.deleted
        }
    }

    impl Metric for SoftPoint {
        type Distance = <Point as Metric>::Distance;

        fn distance(&self, other: &Self) -> Self::Distance {
            self.point.distance(&other.point)
        }
    }

    impl Metric<[f64]> for SoftPoint {
        type Distance = <Point as Metric>::Distance;

        fn distance(&self, other: &[f64]) -> Self::Distance {
            self.point.distance(other)
        }
    }

    impl Cartesian for SoftPoint {
        fn dimensions(&self) -> usize {
            self.point.dimensions()
        }

        fn coordinate(&self, i: usize) -> f64 {
            self.point.coordinate(i)
        }
    }

    impl Metric<SoftPoint> for Point {
        type Distance = <Point as Metric>::Distance;

        fn distance(&self, other: &SoftPoint) -> Self::Distance {
            self.distance(&other.point)
        }
    }

    fn test_index<T>(index: &T)
    where
        T: NearestNeighbors<SoftPoint, Point>,
    {
        let target = Point([0.0, 0.0, 0.0]);

        assert_eq!(
            index.nearest(&target),
            Some(Neighbor::new(&SoftPoint::new(1.0, 2.0, 2.0), 3.0))
        );

        assert_eq!(index.nearest_within(&target, 2.0), None);
        assert_eq!(
            index.nearest_within(&target, 4.0),
            Some(Neighbor::new(&SoftPoint::new(1.0, 2.0, 2.0), 3.0))
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
        T: NearestNeighbors<SoftPoint, Point>,
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
