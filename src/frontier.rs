//! Frontiers on which to place pixels.

pub mod image;

use crate::color::{ColorSpace, Rgb8};
use crate::metric::kd::Cartesian;
use crate::metric::soft::SoftDelete;
use crate::metric::Metric;

use std::cell::Cell;

/// A frontier of pixels.
pub trait Frontier {
    /// The width of the image.
    fn width(&self) -> u32;

    /// The height of the image.
    fn height(&self) -> u32;

    /// The number of pixels currently on the frontier.
    fn len(&self) -> usize;

    /// Place the given color on the frontier, and return its position.
    fn place(&mut self, rgb8: Rgb8) -> Option<(u32, u32)>;
}

/// A pixel on a frontier.
#[derive(Debug)]
struct Pixel<C> {
    pos: (u32, u32),
    color: C,
    deleted: Cell<bool>,
}

impl<C: ColorSpace> Pixel<C> {
    fn new(x: u32, y: u32, color: C) -> Self {
        Self {
            pos: (x, y),
            color,
            deleted: Cell::new(false),
        }
    }

    fn delete(&self) {
        self.deleted.set(true);
    }
}

impl<C: Metric> Metric<Pixel<C>> for C {
    type Distance = C::Distance;

    fn distance(&self, other: &Pixel<C>) -> Self::Distance {
        self.distance(&other.color)
    }
}

impl<C: Metric<[f64]>> Metric<[f64]> for Pixel<C> {
    type Distance = C::Distance;

    fn distance(&self, other: &[f64]) -> Self::Distance {
        self.color.distance(other)
    }
}

impl<C: Metric> Metric for Pixel<C> {
    type Distance = C::Distance;

    fn distance(&self, other: &Pixel<C>) -> Self::Distance {
        self.color.distance(&other.color)
    }
}

impl<C: Cartesian> Cartesian for Pixel<C> {
    fn dimensions(&self) -> usize {
        self.color.dimensions()
    }

    fn coordinate(&self, i: usize) -> f64 {
        self.color.coordinate(i)
    }
}

impl<C> SoftDelete for Pixel<C> {
    fn is_deleted(&self) -> bool {
        self.deleted.get()
    }
}
