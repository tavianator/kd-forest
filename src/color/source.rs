//! Sources of colors.

use super::Rgb8;

use image::RgbImage;

/// A source of colors in multidimensional space.
pub trait ColorSource {
    /// Get the size of each dimension in this space.
    fn dimensions(&self) -> &[usize];

    /// Get the color at some particular coordinates.
    fn get_color(&self, coords: &[usize]) -> Rgb8;
}

/// The entire RGB space.
#[derive(Debug)]
pub struct AllColors {
    dims: [usize; 3],
    shifts: [u32; 3],
}

impl AllColors {
    /// Create an AllColors source with the given bit depths.
    pub fn new(r: u32, g: u32, b: u32) -> Self {
        Self {
            dims: [1 << r, 1 << g, 1 << b],
            shifts: [8 - r, 8 - g, 8 - b],
        }
    }
}

impl ColorSource for AllColors {
    fn dimensions(&self) -> &[usize] {
        &self.dims
    }

    fn get_color(&self, coords: &[usize]) -> Rgb8 {
        Rgb8::from([
            (coords[0] << self.shifts[0]) as u8,
            (coords[1] << self.shifts[1]) as u8,
            (coords[2] << self.shifts[2]) as u8,
        ])
    }
}

/// Colors extracted from an image.
#[derive(Debug)]
pub struct ImageColors {
    dims: [usize; 2],
    image: RgbImage,
}

impl From<RgbImage> for ImageColors {
    fn from(image: RgbImage) -> Self {
        Self {
            dims: [image.width() as usize, image.height() as usize],
            image: image,
        }
    }
}

impl ColorSource for ImageColors {
    fn dimensions(&self) -> &[usize] {
        &self.dims
    }

    fn get_color(&self, coords: &[usize]) -> Rgb8 {
        *self.image.get_pixel(coords[0] as u32, coords[1] as u32)
    }
}
