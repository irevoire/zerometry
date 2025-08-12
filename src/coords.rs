use core::fmt;

use bytemuck::cast_slice;

use crate::{COORD_SIZE_IN_BYTES, COORD_SIZE_IN_FLOATS, Coord};

#[repr(transparent)]
pub struct Coords {
    data: [f64],
}

impl<'a> Coords {
    pub fn from_bytes(data: &'a [u8]) -> &'a Self {
        debug_assert!(
            data.len() % COORD_SIZE_IN_BYTES == 0,
            "Not an even number of scalars"
        );
        debug_assert!(
            data.as_ptr() as usize % std::mem::align_of::<f64>() == 0,
            "data is not aligned"
        );
        let slice: &[f64] = cast_slice(data);
        unsafe { std::mem::transmute(slice) }
    }

    pub fn from_slice(data: &[f64]) -> &Self {
        debug_assert!(data.len() % COORD_SIZE_IN_FLOATS == 0);
        unsafe { std::mem::transmute(data) }
    }

    pub fn from_slice_mut(data: &mut [f64]) -> &mut Self {
        debug_assert!(data.len() % COORD_SIZE_IN_FLOATS == 0);
        unsafe { std::mem::transmute(data) }
    }

    pub fn len(&self) -> usize {
        self.data.len() / COORD_SIZE_IN_FLOATS
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &Coord> {
        self.data
            .chunks(COORD_SIZE_IN_FLOATS)
            .map(Coord::from_slice)
    }

    pub fn consecutive_pairs(&self) -> impl Iterator<Item = &[f64]> {
        self.data
            .windows(COORD_SIZE_IN_FLOATS * 2)
            .step_by(COORD_SIZE_IN_FLOATS)
    }
}

impl fmt::Debug for Coords {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl std::ops::Index<usize> for Coords {
    type Output = Coord;
    fn index(&self, index: usize) -> &Self::Output {
        Coord::from_slice(
            &self.data[index * COORD_SIZE_IN_FLOATS..(index + 1) * COORD_SIZE_IN_FLOATS],
        )
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;

    use super::*;

    // ====== TEST ON BYTES ======

    #[test]
    fn test_basic_create_coords_from_bytes() {
        let data = [1.0, 2.0, 3.0, 4.0];
        let coords = Coords::from_bytes(cast_slice(&data));
        // len works
        assert_eq!(coords.len(), 2);
        // index works
        assert_eq!(coords[0].lng(), 1.0);
        assert_eq!(coords[0].lat(), 2.0);
        assert_eq!(coords[1].lng(), 3.0);
        assert_eq!(coords[1].lat(), 4.0);
        // iter works
        assert_eq!(
            coords
                .iter()
                .map(|c| (c.lng(), c.lat()))
                .collect::<Vec<_>>(),
            vec![(1.0, 2.0), (3.0, 4.0)]
        );
        // Debug+iter works
        insta::assert_snapshot!(format!("{:?}", coords), @"[Coord { x: 1.0, y: 2.0 }, Coord { x: 3.0, y: 4.0 }]");
    }

    #[test]
    #[should_panic]
    fn test_coords_panic_on_too_short_bytes() {
        let data = [1.0];
        Coords::from_bytes(cast_slice(&data));
    }

    #[test]
    #[should_panic]
    fn test_coord_panic_on_bad_number_of_floats_from_bytes() {
        let data = [1.0, 2.0, 3.0];
        Coords::from_bytes(cast_slice(&data));
    }

    #[test]
    #[should_panic]
    fn test_coord_panic_on_unaligned_bytes() {
        let data = [1.0, 2.0, 3.0];
        Coords::from_bytes(&cast_slice(&data)[1..]);
    }

    // ====== TEST ON SLICES ======

    #[test]
    fn test_basic_create_coords_from_slice() {
        let data = [1.0, 2.0, 3.0, 4.0];
        let coords = Coords::from_slice(&data);
        // len works
        assert_eq!(coords.len(), 2);
        // index works
        assert_eq!(coords[0].lng(), 1.0);
        assert_eq!(coords[0].lat(), 2.0);
        assert_eq!(coords[1].lng(), 3.0);
        assert_eq!(coords[1].lat(), 4.0);
        // iter works
        assert_eq!(
            coords
                .iter()
                .map(|c| (c.lng(), c.lat()))
                .collect::<Vec<_>>(),
            vec![(1.0, 2.0), (3.0, 4.0)]
        );
        // Debug+iter works
        insta::assert_snapshot!(format!("{:?}", coords), @"[Coord { x: 1.0, y: 2.0 }, Coord { x: 3.0, y: 4.0 }]");
    }

    #[test]
    #[should_panic]
    fn test_coord_panic_on_too_short_slice() {
        let data = [1.0];
        Coords::from_slice(&data);
    }

    #[test]
    #[should_panic]
    fn test_coord_panic_on_bad_number_of_floats_from_slice() {
        let data = [1.0, 2.0, 3.0];
        Coords::from_slice(&data);
    }
}
