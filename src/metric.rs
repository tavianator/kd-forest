//! [Metric spaces](https://en.wikipedia.org/wiki/Metric_space).

pub mod forest;
pub mod kd;
pub mod vp;

use ordered_float::OrderedFloat;

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::iter::FromIterator;

/// An [order embedding](https://en.wikipedia.org/wiki/Order_embedding) for distances.
///
/// Implementations of this trait must satisfy, for all non-negative distances `x` and `y`:
///
/// * `x == Self::from(x).into()`
/// * `x <= y` iff `Self::from(x) <= Self::from(y)`
///
/// This trait exists to optimize the common case where distances can be compared more efficiently
/// than their exact values can be computed.  For example, taking the square root can be avoided
/// when comparing Euclidean distances (see [SquaredDistance]).
pub trait Distance: Copy + From<f64> + Into<f64> + Ord {}

/// A raw numerical distance.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct RawDistance(OrderedFloat<f64>);

impl From<f64> for RawDistance {
    fn from(value: f64) -> Self {
        Self(value.into())
    }
}

impl From<RawDistance> for f64 {
    fn from(value: RawDistance) -> Self {
        value.0.into_inner()
    }
}

impl Distance for RawDistance {}

/// A squared distance, to avoid computing square roots unless absolutely necessary.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct SquaredDistance(OrderedFloat<f64>);

impl SquaredDistance {
    /// Create a SquaredDistance from an already squared value.
    pub fn from_squared(value: f64) -> Self {
        Self(value.into())
    }
}

impl From<f64> for SquaredDistance {
    fn from(value: f64) -> Self {
        Self::from_squared(value * value)
    }
}

impl From<SquaredDistance> for f64 {
    fn from(value: SquaredDistance) -> Self {
        value.0.into_inner().sqrt()
    }
}

impl Distance for SquaredDistance {}

/// A [metric space](https://en.wikipedia.org/wiki/Metric_space).
pub trait Metric<T: ?Sized = Self> {
    /// The type used to represent distances.  Use [RawDistance] to compare the actual values
    /// directly, or another type if comparisons can be implemented more efficiently.
    type Distance: Distance;

    /// Computes the distance between this point and another point.  This function must satisfy
    /// three conditions:
    ///
    /// * `x.distance(y) == 0` iff `x == y` (identity of indiscernibles)
    /// * `x.distance(y) == y.distance(x)` (symmetry)
    /// * `x.distance(z) <= x.distance(y) + y.distance(z)` (triangle inequality)
    fn distance(&self, other: &T) -> Self::Distance;
}

/// Blanket [Metric] implementation for references.
impl<'a, 'b, T, U: Metric<T>> Metric<&'a T> for &'b U {
    type Distance = U::Distance;

    fn distance(&self, other: &&'a T) -> Self::Distance {
        (*self).distance(other)
    }
}

/// The standard [Euclidean distance](https://en.wikipedia.org/wiki/Euclidean_distance) metric.
impl Metric for [f64] {
    type Distance = SquaredDistance;

    fn distance(&self, other: &Self) -> Self::Distance {
        debug_assert!(self.len() == other.len());

        let mut sum = 0.0;
        for i in 0..self.len() {
            let diff = self[i] - other[i];
            sum += diff * diff;
        }

        Self::Distance::from_squared(sum)
    }
}

/// A nearest neighbor to a target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Neighbor<T> {
    /// The found item.
    pub item: T,
    /// The distance from the target.
    pub distance: f64,
}

impl<T> Neighbor<T> {
    /// Create a new Neighbor.
    pub fn new(item: T, distance: f64) -> Self {
        Self { item, distance }
    }
}

/// A candidate nearest neighbor found during a search.
#[derive(Debug)]
struct Candidate<T, D> {
    item: T,
    distance: D,
}

impl<T, D: Distance> Candidate<T, D> {
    fn new<U>(target: U, item: T) -> Self
    where
        U: Metric<T, Distance = D>,
    {
        let distance = target.distance(&item);
        Self { item, distance }
    }

    fn into_neighbor(self) -> Neighbor<T> {
        Neighbor::new(self.item, self.distance.into())
    }
}

impl<T, D: Distance> PartialOrd for Candidate<T, D> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.distance.partial_cmp(&other.distance)
    }
}

impl<T, D: Distance> Ord for Candidate<T, D> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.distance.cmp(&other.distance)
    }
}

impl<T, D: Distance> PartialEq for Candidate<T, D> {
    fn eq(&self, other: &Self) -> bool {
        self.distance.eq(&other.distance)
    }
}

impl<T, D: Distance> Eq for Candidate<T, D> {}

/// Accumulates nearest neighbor search results.
pub trait Neighborhood<T, U: Metric<T>> {
    /// Returns the target of the nearest neighbor search.
    fn target(&self) -> U;

    /// Check whether a distance is within this neighborhood.
    fn contains(&self, distance: f64) -> bool {
        distance < 0.0 || self.contains_distance(distance.into())
    }

    /// Check whether a distance is within this neighborhood.
    fn contains_distance(&self, distance: U::Distance) -> bool;

    /// Consider a new candidate neighbor.
    fn consider(&mut self, item: T) -> U::Distance;
}

/// A [Neighborhood] with at most one result.
#[derive(Debug)]
struct SingletonNeighborhood<T, U: Metric<T>> {
    /// The target of the nearest neighbor search.
    target: U,
    /// The current threshold distance to the farthest result.
    threshold: Option<U::Distance>,
    /// The current nearest neighbor, if any.
    candidate: Option<Candidate<T, U::Distance>>,
}

impl<T, U> SingletonNeighborhood<T, U>
where
    U: Copy + Metric<T>,
{
    /// Create a new single metric result tracker.
    ///
    /// * `target`: The target fo the nearest neighbor search.
    /// * `threshold`: The maximum allowable distance.
    fn new(target: U, threshold: Option<f64>) -> Self {
        Self {
            target,
            threshold: threshold.map(U::Distance::from),
            candidate: None,
        }
    }

    /// Consider a candidate.
    fn push(&mut self, candidate: Candidate<T, U::Distance>) -> U::Distance {
        let distance = candidate.distance;

        if self.contains_distance(distance) {
            self.threshold = Some(distance);
            self.candidate = Some(candidate);
        }

        distance
    }

    /// Convert this result into an optional neighbor.
    fn into_option(self) -> Option<Neighbor<T>> {
        self.candidate.map(Candidate::into_neighbor)
    }
}

impl<T, U> Neighborhood<T, U> for SingletonNeighborhood<T, U>
where
    U: Copy + Metric<T>,
{
    fn target(&self) -> U {
        self.target
    }

    fn contains_distance(&self, distance: U::Distance) -> bool {
        self.threshold.map(|t| distance <= t).unwrap_or(true)
    }

    fn consider(&mut self, item: T) -> U::Distance {
        self.push(Candidate::new(self.target, item))
    }
}

/// A [Neighborhood] of up to `k` results, using a binary heap.
#[derive(Debug)]
struct HeapNeighborhood<T, U: Metric<T>> {
    /// The target of the nearest neighbor search.
    target: U,
    /// The number of nearest neighbors to find.
    k: usize,
    /// The current threshold distance to the farthest result.
    threshold: Option<U::Distance>,
    /// A max-heap of the best candidates found so far.
    heap: BinaryHeap<Candidate<T, U::Distance>>,
}

impl<T, U> HeapNeighborhood<T, U>
where
    U: Copy + Metric<T>,
{
    /// Create a new metric result tracker.
    ///
    /// * `target`: The target fo the nearest neighbor search.
    /// * `k`: The number of nearest neighbors to find.
    /// * `threshold`: The maximum allowable distance.
    fn new(target: U, k: usize, threshold: Option<f64>) -> Self {
        Self {
            target,
            k,
            threshold: threshold.map(U::Distance::from),
            heap: BinaryHeap::with_capacity(k),
        }
    }

    /// Consider a candidate.
    fn push(&mut self, candidate: Candidate<T, U::Distance>) -> U::Distance {
        let distance = candidate.distance;

        if self.contains_distance(distance) {
            let heap = &mut self.heap;

            if heap.len() == self.k {
                heap.pop();
            }

            heap.push(candidate);

            if heap.len() == self.k {
                self.threshold = self.heap.peek().map(|c| c.distance)
            }
        }

        distance
    }

    /// Convert these results into a vector of neighbors.
    fn into_vec(self) -> Vec<Neighbor<T>> {
        self.heap
            .into_sorted_vec()
            .into_iter()
            .map(Candidate::into_neighbor)
            .collect()
    }
}

impl<T, U> Neighborhood<T, U> for HeapNeighborhood<T, U>
where
    U: Copy + Metric<T>,
{
    fn target(&self) -> U {
        self.target
    }

    fn contains_distance(&self, distance: U::Distance) -> bool {
        self.k > 0 && self.threshold.map(|t| distance <= t).unwrap_or(true)
    }

    fn consider(&mut self, item: T) -> U::Distance {
        self.push(Candidate::new(self.target, item))
    }
}

/// A [nearest neighbor search](https://en.wikipedia.org/wiki/Nearest_neighbor_search) index.
///
/// Type parameters:
/// * `T`: The search result type.
/// * `U`: The query type.
pub trait NearestNeighbors<T, U: Metric<T> = T> {
    /// Returns the nearest neighbor to `target` (or `None` if this index is empty).
    fn nearest(&self, target: &U) -> Option<Neighbor<&T>> {
        self.search(SingletonNeighborhood::new(target, None))
            .into_option()
    }

    /// Returns the nearest neighbor to `target` within the distance `threshold`, if one exists.
    fn nearest_within(&self, target: &U, threshold: f64) -> Option<Neighbor<&T>> {
        self.search(SingletonNeighborhood::new(target, Some(threshold)))
            .into_option()
    }

    /// Returns the up to `k` nearest neighbors to `target`.
    fn k_nearest(&self, target: &U, k: usize) -> Vec<Neighbor<&T>> {
        self.search(HeapNeighborhood::new(target, k, None))
            .into_vec()
    }

    /// Returns the up to `k` nearest neighbors to `target` within the distance `threshold`.
    fn k_nearest_within(&self, target: &U, k: usize, threshold: f64) -> Vec<Neighbor<&T>> {
        self.search(HeapNeighborhood::new(target, k, Some(threshold)))
            .into_vec()
    }

    /// Search for nearest neighbors and add them to a neighborhood.
    fn search<'a, 'b, N>(&'a self, neighborhood: N) -> N
    where
        T: 'a,
        U: 'b,
        N: Neighborhood<&'a T, &'b U>;
}

/// A [NearestNeighbors] implementation that does exhaustive search.
#[derive(Debug)]
pub struct ExhaustiveSearch<T>(Vec<T>);

impl<T> ExhaustiveSearch<T> {
    /// Create an empty ExhaustiveSearch index.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Add a new item to the index.
    pub fn push(&mut self, item: T) {
        self.0.push(item);
    }
}

impl<T> FromIterator<T> for ExhaustiveSearch<T> {
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        Self(items.into_iter().collect())
    }
}

impl<T> IntoIterator for ExhaustiveSearch<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> Extend<T> for ExhaustiveSearch<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for value in iter {
            self.push(value);
        }
    }
}

impl<T, U: Metric<T>> NearestNeighbors<T, U> for ExhaustiveSearch<T> {
    fn search<'a, 'b, N>(&'a self, mut neighborhood: N) -> N
    where
        T: 'a,
        U: 'b,
        N: Neighborhood<&'a T, &'b U>,
    {
        for e in &self.0 {
            neighborhood.consider(e);
        }
        neighborhood
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use rand::prelude::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Point(pub [f64; 3]);

    impl Metric for Point {
        type Distance = SquaredDistance;

        fn distance(&self, other: &Self) -> Self::Distance {
            self.0.distance(&other.0)
        }
    }

    /// Test a [NearestNeighbors] impl.
    pub fn test_nearest_neighbors<T, F>(from_iter: F)
    where
        T: NearestNeighbors<Point>,
        F: Fn(Vec<Point>) -> T,
    {
        test_empty(&from_iter);
        test_pythagorean(&from_iter);
        test_random_points(&from_iter);
    }

    fn test_empty<T, F>(from_iter: &F)
    where
        T: NearestNeighbors<Point>,
        F: Fn(Vec<Point>) -> T,
    {
        let points = Vec::new();
        let index = from_iter(points);
        let target = Point([0.0, 0.0, 0.0]);
        assert_eq!(index.nearest(&target), None);
        assert_eq!(index.nearest_within(&target, 1.0), None);
        assert!(index.k_nearest(&target, 0).is_empty());
        assert!(index.k_nearest(&target, 3).is_empty());
        assert!(index.k_nearest_within(&target, 0, 1.0).is_empty());
        assert!(index.k_nearest_within(&target, 3, 1.0).is_empty());
    }

    fn test_pythagorean<T, F>(from_iter: &F)
    where
        T: NearestNeighbors<Point>,
        F: Fn(Vec<Point>) -> T,
    {
        let points = vec![
            Point([3.0, 4.0, 0.0]),
            Point([5.0, 0.0, 12.0]),
            Point([0.0, 8.0, 15.0]),
            Point([1.0, 2.0, 2.0]),
            Point([2.0, 3.0, 6.0]),
            Point([4.0, 4.0, 7.0]),
        ];
        let index = from_iter(points);
        let target = Point([0.0, 0.0, 0.0]);

        assert_eq!(
            index.nearest(&target),
            Some(Neighbor::new(&Point([1.0, 2.0, 2.0]), 3.0))
        );

        assert_eq!(index.nearest_within(&target, 2.0), None);
        assert_eq!(
            index.nearest_within(&target, 4.0),
            Some(Neighbor::new(&Point([1.0, 2.0, 2.0]), 3.0))
        );

        assert!(index.k_nearest(&target, 0).is_empty());
        assert_eq!(
            index.k_nearest(&target, 3),
            vec![
                Neighbor::new(&Point([1.0, 2.0, 2.0]), 3.0),
                Neighbor::new(&Point([3.0, 4.0, 0.0]), 5.0),
                Neighbor::new(&Point([2.0, 3.0, 6.0]), 7.0),
            ]
        );

        assert!(index.k_nearest(&target, 0).is_empty());
        assert_eq!(
            index.k_nearest_within(&target, 3, 6.0),
            vec![
                Neighbor::new(&Point([1.0, 2.0, 2.0]), 3.0),
                Neighbor::new(&Point([3.0, 4.0, 0.0]), 5.0),
            ]
        );
        assert_eq!(
            index.k_nearest_within(&target, 3, 8.0),
            vec![
                Neighbor::new(&Point([1.0, 2.0, 2.0]), 3.0),
                Neighbor::new(&Point([3.0, 4.0, 0.0]), 5.0),
                Neighbor::new(&Point([2.0, 3.0, 6.0]), 7.0),
            ]
        );
    }

    fn test_random_points<T, F>(from_iter: &F)
    where
        T: NearestNeighbors<Point>,
        F: Fn(Vec<Point>) -> T,
    {
        let mut points = Vec::new();
        for _ in 0..255 {
            points.push(Point([random(), random(), random()]));
        }
        let target = Point([random(), random(), random()]);

        let eindex = ExhaustiveSearch::from_iter(points.clone());
        let index = from_iter(points);

        assert_eq!(index.k_nearest(&target, 3), eindex.k_nearest(&target, 3));
    }

    #[test]
    fn test_exhaustive_index() {
        test_nearest_neighbors(ExhaustiveSearch::from_iter);
    }
}
