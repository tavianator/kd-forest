//! Linear orders for colors.

use super::source::ColorSource;
use super::Rgb8;

use crate::hilbert::hilbert_point;

use rand::seq::SliceRandom;
use rand::Rng;

use std::cmp::Ordering;

/// An iterator over all colors from a source.
#[derive(Debug)]
struct ColorSourceIter<S> {
    source: S,
    coords: Vec<usize>,
}

impl<S: ColorSource> From<S> for ColorSourceIter<S> {
    fn from(source: S) -> Self {
        let coords = vec![0; source.dimensions().len()];

        Self { source, coords }
    }
}

impl<S: ColorSource> Iterator for ColorSourceIter<S> {
    type Item = Rgb8;

    fn next(&mut self) -> Option<Rgb8> {
        if self.coords.is_empty() {
            return None;
        }

        let color = self.source.get_color(&self.coords);

        let dims = self.source.dimensions();
        for i in 0..dims.len() {
            self.coords[i] += 1;
            if self.coords[i] < dims[i] {
                break;
            } else if i == dims.len() - 1 {
                self.coords.clear();
            } else {
                self.coords[i] = 0;
            }
        }

        Some(color)
    }
}

/// Wrapper for sorting colors by hue.
#[derive(Debug, Eq, PartialEq)]
struct Hue {
    /// The quadrant of the hue angle.
    quad: i32,
    /// The numerator of the hue calculation.
    num: i32,
    /// The denominator of the hue calculation.
    denom: i32,
}

impl From<Rgb8> for Hue {
    fn from(rgb8: Rgb8) -> Self {
        // The hue angle is atan2(sqrt(3) * (G - B), 2 * R - G - B).  We avoid actually computing
        // the atan2() as an optimization.
        let r = rgb8[0] as i32;
        let g = rgb8[1] as i32;
        let b = rgb8[2] as i32;

        let num = g - b;
        let mut denom = 2 * r - g - b;
        if num == 0 && denom == 0 {
            denom = 1;
        }

        let quad = match (num >= 0, denom >= 0) {
            (true, true) => 0,
            (true, false) => 1,
            (false, false) => 2,
            (false, true) => 3,
        };

        Self { quad, num, denom }
    }
}

impl Ord for Hue {
    fn cmp(&self, other: &Self) -> Ordering {
        // Within the same quadrant,
        //
        //     atan2(n1, d1) < atan2(n2, d2)  iff
        //           n1 / d1 < n2 / d2        iff
        //           n1 * d2 < n2 * d1
        self.quad
            .cmp(&other.quad)
            .then_with(|| (self.num * other.denom).cmp(&(other.num * self.denom)))
    }
}

impl PartialOrd for Hue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Iterate over colors sorted by their hue.
pub fn hue_sorted<S: ColorSource>(source: S) -> Vec<Rgb8> {
    let mut colors: Vec<_> = ColorSourceIter::from(source).collect();
    colors.sort_by_key(|c| Hue::from(*c));
    colors
}

/// Iterate over colors in random order.
pub fn shuffled<S: ColorSource, R: Rng>(source: S, rng: &mut R) -> Vec<Rgb8> {
    let mut colors: Vec<_> = ColorSourceIter::from(source).collect();
    colors.shuffle(rng);
    colors
}

/// ceil(log_2(n)). for rounding up to powers of 2.
fn log2(n: usize) -> u32 {
    let nbits = 8 * std::mem::size_of::<usize>() as u32;
    nbits - (n - 1).leading_zeros()
}

/// Iterate over colors in Morton order (Z-order).
pub fn morton<S: ColorSource>(source: S) -> Vec<Rgb8> {
    let mut colors = Vec::new();

    let dims = source.dimensions();
    let ndims = dims.len();

    let nbits = ndims * dims.iter().map(|n| log2(*n) as usize).max().unwrap();

    let size = 1usize << nbits;
    let mut coords = vec![0; ndims];
    for i in 0..size {
        for x in &mut coords {
            *x = 0;
        }
        for j in 0..nbits {
            let bit = (i >> j) & 1;
            coords[j % ndims] |= bit << (j / ndims);
        }
        if coords.iter().zip(dims.iter()).all(|(x, n)| x < n) {
            colors.push(source.get_color(&coords));
        }
    }

    colors
}

/// Iterate over colors in Hilbert curve order.
pub fn hilbert<S: ColorSource>(source: S) -> Vec<Rgb8> {
    let mut colors = Vec::new();

    let dims = source.dimensions();
    let ndims = dims.len();

    let bits: Vec<_> = dims.iter().map(|n| log2(*n)).collect();
    let nbits: u32 = bits.iter().sum();
    let size = 1usize << nbits;

    let mut coords = vec![0; ndims];

    for i in 0..size {
        hilbert_point(i, &bits, &mut coords);
        if coords.iter().zip(dims.iter()).all(|(x, n)| x < n) {
            colors.push(source.get_color(&coords));
        }
    }

    colors
}

/// Stripe an ordered list of colors, to reduce artifacts in the generated image.
///
/// The striped ordering gives every other item first, then every other item from the remaining
/// items, etc. For example, the striped form of `0..16` is
/// `[0, 2, 4, 6, 8, 10, 12, 14, 1, 5, 9, 13, 3, 11, 7, 15]`.
pub fn striped(colors: Vec<Rgb8>) -> Vec<Rgb8> {
    let len = colors.len();
    let mut result = Vec::with_capacity(len);
    let mut stripe = 1;
    while stripe <= len {
        for i in ((stripe - 1)..len).step_by(2 * stripe) {
            result.push(colors[i]);
        }
        stripe *= 2;
    }

    result
}
