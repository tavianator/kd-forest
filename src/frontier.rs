//! Frontiers on which to place pixels.

pub mod image;
pub mod mean;
pub mod min;

use crate::color::Rgb8;
use crate::soft::SoftDelete;

use acap::coords::Coordinates;
use acap::distance::{Proximity, Metric};

use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;

/// A frontier of pixels.
pub trait Frontier {
    /// The width of the image.
    fn width(&self) -> u32;

    /// The height of the image.
    fn height(&self) -> u32;

    /// The number of pixels currently on the frontier.
    fn len(&self) -> usize;

    /// Whether the frontier is empty.
    fn is_empty(&self) -> bool;

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

impl<C> Pixel<C> {
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

/// A reference-counted pixel, to work around the coherence rules.
#[derive(Clone, Debug)]
struct RcPixel<C>(Rc<Pixel<C>>);

impl<C> RcPixel<C> {
    fn new(x: u32, y: u32, color: C) -> Self {
        Self(Rc::new(Pixel::new(x, y, color)))
    }
}

impl<C> Deref for RcPixel<C> {
    type Target = Pixel<C>;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

/// A search target, to work around the coherence rules.
#[derive(Debug)]
struct Target<C>(C);

impl<C: Proximity> Proximity<Pixel<C>> for Target<C> {
    type Distance = C::Distance;

    fn distance(&self, other: &Pixel<C>) -> Self::Distance {
        self.0.distance(&other.color)
    }
}

impl<C: Metric> Metric<Pixel<C>> for Target<C> {}

impl<C: Proximity> Proximity for Pixel<C> {
    type Distance = C::Distance;

    fn distance(&self, other: &Pixel<C>) -> Self::Distance {
        self.color.distance(&other.color)
    }
}

impl<C: Metric> Metric for Pixel<C> {}

impl<C: Coordinates> Coordinates for Pixel<C> {
    type Value = C::Value;

    fn dims(&self) -> usize {
        self.color.dims()
    }

    fn coord(&self, i: usize) -> Self::Value {
        self.color.coord(i)
    }
}

impl<C> SoftDelete for Pixel<C> {
    fn is_deleted(&self) -> bool {
        self.deleted.get()
    }
}

impl<C: Proximity> Proximity<RcPixel<C>> for Target<C> {
    type Distance = C::Distance;

    fn distance(&self, other: &RcPixel<C>) -> Self::Distance {
        self.0.distance(&other.0.color)
    }
}

impl<C: Metric> Metric<RcPixel<C>> for Target<C> {}

impl<C: Coordinates> Coordinates for Target<C> {
    type Value = C::Value;

    fn dims(&self) -> usize {
        self.0.dims()
    }

    fn coord(&self, i: usize) -> Self::Value {
        self.0.coord(i)
    }
}

impl<C: Proximity> Proximity for RcPixel<C> {
    type Distance = C::Distance;

    fn distance(&self, other: &Self) -> Self::Distance {
        (*self.0).distance(&*other.0)
    }
}

impl<C: Metric> Metric for RcPixel<C> {}

impl<C: Coordinates> Coordinates for RcPixel<C> {
    type Value = C::Value;

    fn dims(&self) -> usize {
        (*self.0).dims()
    }

    fn coord(&self, i: usize) -> Self::Value {
        (*self.0).coord(i)
    }
}

impl<C> SoftDelete for RcPixel<C> {
    fn is_deleted(&self) -> bool {
        (*self.0).is_deleted()
    }
}

/// Return all the neighbors of a pixel location.
fn neighbors(x: u32, y: u32) -> [(u32, u32); 8] {
    let xm1 = x.wrapping_sub(1);
    let ym1 = y.wrapping_sub(1);
    let xp1 = x + 1;
    let yp1 = y + 1;

    [
        (xm1, ym1),
        (xm1, y),
        (xm1, yp1),
        (x, ym1),
        (x, yp1),
        (xp1, ym1),
        (xp1, y),
        (xp1, yp1),
    ]
}
