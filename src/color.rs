//! Colors and color spaces.

pub mod order;
pub mod source;

use acap::coords::{Coordinates, CoordinateMetric, CoordinateProximity};
use acap::distance::{Metric, Proximity};
use acap::euclid::{EuclideanDistance, euclidean_distance};

use image::Rgb;

use std::ops::Index;

/// An 8-bit RGB color.
pub type Rgb8 = Rgb<u8>;

/// A [color space](https://en.wikipedia.org/wiki/Color_space).
pub trait ColorSpace: Copy + From<Rgb8>
    + Coordinates
    + Metric
    + CoordinateMetric<<Self as Coordinates>::Value, Distance = <Self as Proximity>::Distance>
{
    /// Compute the average of the given colors.
    fn average<I: IntoIterator<Item = Self>>(colors: I) -> Self;
}

/// [sRGB](https://en.wikipedia.org/wiki/SRGB) space.
#[derive(Clone, Copy, Debug)]
pub struct RgbSpace([f64; 3]);

impl Index<usize> for RgbSpace {
    type Output = f64;

    fn index(&self, i: usize) -> &f64 {
        &self.0[i]
    }
}

impl From<Rgb8> for RgbSpace {
    fn from(rgb8: Rgb8) -> Self {
        Self([
            (rgb8[0] as f64) / 255.0,
            (rgb8[1] as f64) / 255.0,
            (rgb8[2] as f64) / 255.0,
        ])
    }
}

impl Coordinates for RgbSpace {
    type Value = f64;

    fn dims(&self) -> usize {
        self.0.dims()
    }

    fn coord(&self, i: usize) -> f64 {
        self.0.coord(i)
    }
}

impl Proximity for RgbSpace {
    type Distance = EuclideanDistance<f64>;

    fn distance(&self, other: &Self) -> Self::Distance {
        euclidean_distance(&self.0, &other.0)
    }
}

impl Metric for RgbSpace {}

impl CoordinateProximity<f64> for RgbSpace {
    type Distance = EuclideanDistance<f64>;

    fn distance_to_coords(&self, other: &[f64]) -> Self::Distance {
        euclidean_distance(&self.0, other)
    }
}

impl CoordinateMetric<f64> for RgbSpace {}

impl ColorSpace for RgbSpace {
    fn average<I: IntoIterator<Item = Self>>(colors: I) -> Self {
        let mut sum = [0.0, 0.0, 0.0];
        let mut len: usize = 0;
        for color in colors.into_iter() {
            for i in 0..3 {
                sum[i] += color[i];
            }
            len += 1;
        }
        for s in &mut sum {
            *s /= len as f64;
        }
        Self(sum)
    }
}

/// [CIE XYZ](https://en.wikipedia.org/wiki/CIE_1931_color_space) space.
#[derive(Clone, Copy, Debug)]
struct XyzSpace([f64; 3]);

impl Index<usize> for XyzSpace {
    type Output = f64;

    fn index(&self, i: usize) -> &f64 {
        &self.0[i]
    }
}

/// The inverse of the sRGB gamma function.
fn srgb_inv_gamma(t: f64) -> f64 {
    if t <= 0.040449936 {
        t / 12.92
    } else {
        ((t + 0.055) / 1.055).powf(2.4)
    }
}

impl From<Rgb8> for XyzSpace {
    fn from(rgb8: Rgb8) -> Self {
        let rgb = RgbSpace::from(rgb8);

        let r = srgb_inv_gamma(rgb[0]);
        let g = srgb_inv_gamma(rgb[1]);
        let b = srgb_inv_gamma(rgb[2]);

        Self([
            0.4123808838268995 * r + 0.3575728355732478 * g + 0.1804522977447919 * b,
            0.2126198631048975 * r + 0.7151387878413206 * g + 0.0721499433963131 * b,
            0.0193434956789248 * r + 0.1192121694056356 * g + 0.9505065664127130 * b,
        ])
    }
}

/// CIE D50 [white point](https://en.wikipedia.org/wiki/Standard_illuminant).
const WHITE: XyzSpace = XyzSpace([0.9504060171449392, 0.9999085943425312, 1.089062231497274]);

/// CIE L\*a\*b\* (and L\*u\*v\*) gamma
fn lab_gamma(t: f64) -> f64 {
    if t > 216.0 / 24389.0 {
        t.cbrt()
    } else {
        841.0 * t / 108.0 + 4.0 / 29.0
    }
}

/// [CIE L\*a\*b\*](https://en.wikipedia.org/wiki/CIELAB_color_space) space.
#[derive(Clone, Copy, Debug)]
pub struct LabSpace([f64; 3]);

impl Index<usize> for LabSpace {
    type Output = f64;

    fn index(&self, i: usize) -> &f64 {
        &self.0[i]
    }
}

impl From<Rgb8> for LabSpace {
    fn from(rgb8: Rgb8) -> Self {
        let xyz = XyzSpace::from(rgb8);

        let x = lab_gamma(xyz[0] / WHITE[0]);
        let y = lab_gamma(xyz[1] / WHITE[1]);
        let z = lab_gamma(xyz[2] / WHITE[2]);

        let l = 116.0 * y - 16.0;
        let a = 500.0 * (x - y);
        let b = 200.0 * (y - z);

        Self([l, a, b])
    }
}

impl Coordinates for LabSpace {
    type Value = f64;

    fn dims(&self) -> usize {
        self.0.dims()
    }

    fn coord(&self, i: usize) -> f64 {
        self.0.coord(i)
    }
}

impl Proximity for LabSpace {
    type Distance = EuclideanDistance<f64>;

    fn distance(&self, other: &Self) -> Self::Distance {
        euclidean_distance(self.0, other.0)
    }
}

impl Metric for LabSpace {}

impl CoordinateProximity<f64> for LabSpace {
    type Distance = EuclideanDistance<f64>;

    fn distance_to_coords(&self, other: &[f64]) -> Self::Distance {
        euclidean_distance(&self.0, other)
    }
}

impl CoordinateMetric<f64> for LabSpace {}

impl ColorSpace for LabSpace {
    fn average<I: IntoIterator<Item = Self>>(colors: I) -> Self {
        let mut sum = [0.0, 0.0, 0.0];
        let mut len: usize = 0;
        for color in colors.into_iter() {
            for i in 0..3 {
                sum[i] += color[i];
            }
            len += 1;
        }
        for s in &mut sum {
            *s /= len as f64;
        }
        Self(sum)
    }
}

/// [CIE L\*u\*v\*](https://en.wikipedia.org/wiki/CIELUV) space.
#[derive(Clone, Copy, Debug)]
pub struct LuvSpace([f64; 3]);

impl Index<usize> for LuvSpace {
    type Output = f64;

    fn index(&self, i: usize) -> &f64 {
        &self.0[i]
    }
}

/// Computes the u' and v' values for L\*u\*v\*.
fn uv_prime(xyz: &XyzSpace) -> (f64, f64) {
    let denom = xyz[0] + 15.0 * xyz[1] + 3.0 * xyz[2];
    if denom == 0.0 {
        (0.0, 0.0)
    } else {
        (4.0 * xyz[0] / denom, 9.0 * xyz[1] / denom)
    }
}

impl From<Rgb8> for LuvSpace {
    fn from(rgb8: Rgb8) -> Self {
        let xyz = XyzSpace::from(rgb8);

        let (uprime, vprime) = uv_prime(&xyz);
        let (unprime, vnprime) = uv_prime(&WHITE);

        let l = 116.0 * lab_gamma(xyz[1] / WHITE[1]) - 16.0;
        let u = 13.0 * l * (uprime - unprime);
        let v = 13.0 * l * (vprime - vnprime);

        Self([l, u, v])
    }
}

impl Coordinates for LuvSpace {
    type Value = f64;

    fn dims(&self) -> usize {
        self.0.dims()
    }

    fn coord(&self, i: usize) -> f64 {
        self.0.coord(i)
    }
}

impl Proximity for LuvSpace {
    type Distance = EuclideanDistance<f64>;

    fn distance(&self, other: &Self) -> Self::Distance {
        euclidean_distance(&self.0, &other.0)
    }
}

impl Metric for LuvSpace {}

impl CoordinateProximity<f64> for LuvSpace {
    type Distance = EuclideanDistance<f64>;

    fn distance_to_coords(&self, other: &[f64]) -> Self::Distance {
        euclidean_distance(&self.0, other)
    }
}

impl CoordinateMetric<f64> for LuvSpace {}

impl ColorSpace for LuvSpace {
    fn average<I: IntoIterator<Item = Self>>(colors: I) -> Self {
        let mut sum = [0.0, 0.0, 0.0];
        let mut len: usize = 0;
        for color in colors.into_iter() {
            for i in 0..3 {
                sum[i] += color[i];
            }
            len += 1;
        }
        for s in &mut sum {
            *s /= len as f64;
        }
        Self(sum)
    }
}
