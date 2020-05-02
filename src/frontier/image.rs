//! Frontier that targets an image.

use super::{Frontier, Pixel};

use crate::color::{ColorSpace, Rgb8};
use crate::metric::soft::SoftKdTree;
use crate::metric::NearestNeighbors;

use image::RgbImage;

/// A [Frontier] that places colors on the closest pixel of a target image.
#[derive(Debug)]
pub struct ImageFrontier<C: ColorSpace> {
    nodes: SoftKdTree<Pixel<C>>,
    width: u32,
    height: u32,
    len: usize,
    deleted: usize,
}

impl<C: ColorSpace> ImageFrontier<C> {
    /// Create an ImageFrontier from an image.
    pub fn new(img: &RgbImage) -> Self {
        let width = img.width();
        let height = img.height();
        let len = (width as usize) * (height as usize);

        Self {
            nodes: img
                .enumerate_pixels()
                .map(|(x, y, p)| Pixel::new(x, y, C::from(*p)))
                .collect(),
            width,
            height,
            len,
            deleted: 0,
        }
    }
}

impl<C: ColorSpace> Frontier for ImageFrontier<C> {
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

        if let Some(node) = self.nodes.nearest(&color).map(|n| n.item) {
            let pos = node.pos;

            node.delete();
            self.deleted += 1;

            if 32 * self.deleted >= self.len {
                self.nodes.rebuild();
                self.len -= self.deleted;
                self.deleted = 0;
            }

            Some(pos)
        } else {
            None
        }
    }
}
