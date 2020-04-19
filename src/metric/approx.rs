//! [Approximate nearest neighbor search](https://en.wikipedia.org/wiki/Nearest_neighbor_search#Approximate_nearest_neighbor).

use super::{Metric, NearestNeighbors, Neighborhood};

/// An approximate [Neighborhood], for approximate nearest neighbor searches.
#[derive(Debug)]
struct ApproximateNeighborhood<N> {
    inner: N,
    ratio: f64,
    limit: usize,
}

impl<N> ApproximateNeighborhood<N> {
    fn new(inner: N, ratio: f64, limit: usize) -> Self {
        Self {
            inner,
            ratio,
            limit,
        }
    }
}

impl<T, U, N> Neighborhood<T, U> for ApproximateNeighborhood<N>
where
    U: Metric<T>,
    N: Neighborhood<T, U>,
{
    fn target(&self) -> U {
        self.inner.target()
    }

    fn contains(&self, distance: f64) -> bool {
        if self.limit > 0 {
            self.inner.contains(self.ratio * distance)
        } else {
            false
        }
    }

    fn contains_distance(&self, distance: U::Distance) -> bool {
        self.contains(self.ratio * distance.into())
    }

    fn consider(&mut self, item: T) -> U::Distance {
        self.limit = self.limit.saturating_sub(1);
        self.inner.consider(item)
    }
}

/// An [approximate nearest neighbor search](https://en.wikipedia.org/wiki/Nearest_neighbor_search#Approximate_nearest_neighbor)
/// index.
///
/// This wrapper converts an exact nearest neighbor search algorithm into an approximate one by
/// modifying the behavior of [Neighborhood::contains].  The approximation is controlled by two
/// parameters:
///
/// * `ratio`: The [nearest neighbor distance ratio](https://en.wikipedia.org/wiki/Nearest_neighbor_search#Nearest_neighbor_distance_ratio),
///   which controls how much closer new candidates must be to be considered.  For example, a ratio
///   of 2.0 means that a neighbor must be less than half of the current threshold away to be
///   considered.  A ratio of 1.0 means an exact search.
///
/// * `limit`: A limit on the number of candidates to consider.  Typical nearest neighbor algorithms
///   find a close match quickly, so setting a limit bounds the worst-case search time while keeping
///   good accuracy.
#[derive(Debug)]
pub struct ApproximateSearch<T> {
    inner: T,
    ratio: f64,
    limit: usize,
}

impl<T> ApproximateSearch<T> {
    /// Create a new ApproximateSearch index.
    ///
    /// * `inner`: The [NearestNeighbors] implementation to wrap.
    /// * `ratio`: The nearest neighbor distance ratio.
    /// * `limit`: The maximum number of results to consider.
    pub fn new(inner: T, ratio: f64, limit: usize) -> Self {
        Self {
            inner,
            ratio,
            limit,
        }
    }
}

impl<T, U, V> NearestNeighbors<T, U> for ApproximateSearch<V>
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
        self.inner
            .search(ApproximateNeighborhood::new(
                neighborhood,
                self.ratio,
                self.limit,
            ))
            .inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::metric::kd::KdTree;
    use crate::metric::tests::test_nearest_neighbors;
    use crate::metric::vp::VpTree;

    use std::iter::FromIterator;

    #[test]
    fn test_approx_kd_tree() {
        test_nearest_neighbors(|iter| {
            ApproximateSearch::new(KdTree::from_iter(iter), 1.0, std::usize::MAX)
        });
    }

    #[test]
    fn test_approx_vp_tree() {
        test_nearest_neighbors(|iter| {
            ApproximateSearch::new(VpTree::from_iter(iter), 1.0, std::usize::MAX)
        });
    }
}
