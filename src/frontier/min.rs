//! Minimum selection frontier.

use super::{neighbors, Frontier, RcPixel, Target};

use crate::color::{ColorSpace, Rgb8};
use crate::soft::SoftKdForest;

use acap::knn::NearestNeighbors;

use rand::Rng;

/// A pixel on a min frontier.
#[derive(Debug)]
struct MinPixel<C> {
    pixel: Option<RcPixel<C>>,
    filled: bool,
}

impl<C: ColorSpace> MinPixel<C>
where
    C::Value: PartialOrd<C::Distance>,
{
    fn new() -> Self {
        Self {
            pixel: None,
            filled: false,
        }
    }
}

/// A [Frontier] that places colors on a neighbor of the closest pixel so far.
#[derive(Debug)]
pub struct MinFrontier<C, R> {
    rng: R,
    pixels: Vec<MinPixel<C>>,
    forest: SoftKdForest<RcPixel<C>>,
    width: u32,
    height: u32,
    x0: u32,
    y0: u32,
    len: usize,
    deleted: usize,
}

impl<C: ColorSpace, R: Rng> MinFrontier<C, R>
where
    C::Value: PartialOrd<C::Distance>,
{
    /// Create a MinFrontier with the given dimensions and initial pixel location.
    pub fn new(rng: R, width: u32, height: u32, x0: u32, y0: u32) -> Self {
        let size = (width as usize) * (height as usize);
        let mut pixels = Vec::with_capacity(size);
        for _ in 0..size {
            pixels.push(MinPixel::new());
        }

        Self {
            rng,
            pixels,
            forest: SoftKdForest::new(),
            width,
            height,
            x0,
            y0,
            len: 0,
            deleted: 0,
        }
    }

    fn pixel_index(&self, x: u32, y: u32) -> usize {
        debug_assert!(x < self.width);
        debug_assert!(y < self.height);

        (x + y * self.width) as usize
    }

    fn free_neighbor(&mut self, x: u32, y: u32) -> Option<(u32, u32)> {
        // Pick a pseudo-random neighbor
        let offset: usize = self.rng.gen();

        let neighbors = neighbors(x, y);
        for i in 0..8 {
            let (x, y) = neighbors[(i + offset) % 8];
            if x < self.width && y < self.height {
                let i = self.pixel_index(x, y);
                if !self.pixels[i].filled {
                    return Some((x, y));
                }
            }
        }

        None
    }

    fn fill(&mut self, x: u32, y: u32, color: C) -> Option<(u32, u32)> {
        let i = self.pixel_index(x, y);
        let pixel = &mut self.pixels[i];
        if pixel.filled {
            return None;
        }

        let rc = RcPixel::new(x, y, color);
        pixel.pixel = Some(rc.clone());
        pixel.filled = true;

        if self.free_neighbor(x, y).is_some() {
            self.forest.push(rc);
            self.len += 1;
        }

        for &(x, y) in &neighbors(x, y) {
            if x < self.width && y < self.height && self.free_neighbor(x, y).is_none() {
                let i = self.pixel_index(x, y);
                if let Some(pixel) = self.pixels[i].pixel.take() {
                    pixel.delete();
                    self.deleted += 1;
                }
            }
        }

        if 2 * self.deleted >= self.len {
            self.forest.rebuild();
            self.len -= self.deleted;
            self.deleted = 0;
        }

        Some((x, y))
    }
}

impl<C: ColorSpace, R: Rng> Frontier for MinFrontier<C, R>
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

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn place(&mut self, rgb8: Rgb8) -> Option<(u32, u32)> {
        let color = C::from(rgb8);
        let (x, y) = self
            .forest
            .nearest(&Target(color))
            .map(|n| n.item.pos)
            .map(|(x, y)| self.free_neighbor(x, y).unwrap())
            .unwrap_or((self.x0, self.y0));

        self.fill(x, y, color)
    }
}
