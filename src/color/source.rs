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
    shifts: [usize; 3],
}

impl AllColors {
    /// Create an AllColors source with the given bit depth.
    pub fn new(bits: usize) -> Self {
        // Allocate bits from most to least perceptually important
        let gbits = (bits + 2) / 3;
        let rbits = (bits + 1) / 3;
        let bbits = bits / 3;

        Self {
            dims: [1 << rbits, 1 << gbits, 1 << bbits],
            shifts: [8 - rbits, 8 - gbits, 8 - bbits],
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
