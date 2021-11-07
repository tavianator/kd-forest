//! Mean selection frontier.

use super::{neighbors, Frontier, RcPixel, Target};

use crate::color::{ColorSpace, Rgb8};
use crate::soft::SoftKdForest;

use acap::knn::NearestNeighbors;

use std::iter;

/// A pixel on a mean frontier.
#[derive(Debug)]
enum MeanPixel<C> {
    Empty,
    Fillable(RcPixel<C>),
    Filled(C),
}

impl<C: ColorSpace> MeanPixel<C>
where
    C::Value: PartialOrd<C::Distance>,
{
    fn filled_color(&self) -> Option<C> {
        match self {
            Self::Filled(color) => Some(*color),
            _ => None,
        }
    }
}

/// A [Frontier] that looks at the average color of each pixel's neighbors.
#[derive(Debug)]
pub struct MeanFrontier<C> {
    pixels: Vec<MeanPixel<C>>,
    forest: SoftKdForest<RcPixel<C>>,
    width: u32,
    height: u32,
    len: usize,
    deleted: usize,
}

impl<C: ColorSpace> MeanFrontier<C>
where
    C::Value: PartialOrd<C::Distance>,
{
    /// Create a MeanFrontier with the given dimensions and initial pixel location.
    pub fn new(width: u32, height: u32, x0: u32, y0: u32) -> Self {
        let size = (width as usize) * (height as usize);
        let mut pixels = Vec::with_capacity(size);
        for _ in 0..size {
            pixels.push(MeanPixel::Empty);
        }

        let pixel0 = RcPixel::new(x0, y0, C::from(Rgb8::from([0, 0, 0])));
        let i = (x0 + y0 * width) as usize;
        pixels[i] = MeanPixel::Fillable(pixel0.clone());

        Self {
            pixels,
            forest: iter::once(pixel0).collect(),
            width,
            height,
            len: 1,
            deleted: 0,
        }
    }

    fn pixel_index(&self, x: u32, y: u32) -> usize {
        debug_assert!(x < self.width);
        debug_assert!(y < self.height);

        (x + y * self.width) as usize
    }

    fn fill(&mut self, x: u32, y: u32, color: C) {
        let i = self.pixel_index(x, y);
        match &self.pixels[i] {
            MeanPixel::Empty => {}
            MeanPixel::Fillable(pixel) => {
                pixel.delete();
                self.deleted += 1;
            }
            _ => unreachable!(),
        }
        self.pixels[i] = MeanPixel::Filled(color);

        let mut pixels = Vec::new();
        for &(x, y) in &neighbors(x, y) {
            if x < self.width && y < self.height {
                let i = self.pixel_index(x, y);
                match &self.pixels[i] {
                    MeanPixel::Empty => {}
                    MeanPixel::Fillable(pixel) => {
                        pixel.delete();
                        self.deleted += 1;
                    }
                    MeanPixel::Filled(_) => continue,
                }
                let color = C::average(
                    neighbors(x, y)
                        .iter()
                        .filter(|(x, y)| *x < self.width && *y < self.height)
                        .map(|(x, y)| self.pixel_index(*x, *y))
                        .map(|i| &self.pixels[i])
                        .map(MeanPixel::filled_color)
                        .flatten(),
                );
                let pixel = RcPixel::new(x, y, color);
                self.pixels[i] = MeanPixel::Fillable(pixel.clone());
                pixels.push(pixel);
            }
        }

        self.len += pixels.len();
        self.forest.extend(pixels);

        if 2 * self.deleted >= self.len {
            self.forest.rebuild();
            self.len -= self.deleted;
            self.deleted = 0;
        }
    }
}

impl<C: ColorSpace> Frontier for MeanFrontier<C>
where
    C::Value: PartialOrd<C::Distance>,
{
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn len(&self) -> usize {
        self.len - self.deleted
    }

    fn place(&mut self, rgb8: Rgb8) -> Option<(u32, u32)> {
        let color = C::from(rgb8);
        let (x, y) = self.forest.nearest(&Target(color)).map(|n| n.item.pos)?;

        self.fill(x, y, color);

        Some((x, y))
    }
}
