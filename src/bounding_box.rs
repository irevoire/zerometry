use core::fmt;
use std::mem;

use crate::Coords;

/// Bounding box of a Zerometry.
///
/// The bounding box is a rectangle that contains the Zerometry.
/// It is represented by two coordinates: the bottom-left and top-right corners.
///
/// The coordinates are stored in a `Coords` struct, which is a slice of `f64` values.
/// The first coordinate is the bottom-left corner, and the second coordinate is the top-right corner.
#[repr(transparent)]
pub struct BoundingBox {
    coords: Coords,
}

impl BoundingBox {
    pub fn from_bytes(data: &[u8]) -> &Self {
        Self::from_coords(Coords::from_bytes(data))
    }

    pub fn from_slice(data: &[f64]) -> &Self {
        Self::from_coords(Coords::from_slice(data))
    }

    pub fn from_coords(coords: &Coords) -> &Self {
        unsafe { mem::transmute(coords) }
    }

    pub fn coords(&self) -> &Coords {
        &self.coords
    }
}

impl fmt::Debug for BoundingBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoundingBox")
            .field("bottom_left", &&self.coords[0])
            .field("top_right", &&self.coords[1])
            .finish()
    }
}