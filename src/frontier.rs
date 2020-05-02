//! Frontiers on which to place pixels.

use crate::color::Rgb8;

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
