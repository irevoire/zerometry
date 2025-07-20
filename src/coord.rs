use core::fmt;

pub(crate) const COORD_SIZE_IN_BYTES: usize = std::mem::size_of::<f64>() * 2;
pub(crate) const COORD_SIZE_IN_FLOATS: usize = 2;

#[repr(transparent)]
pub struct Coord {
    data: [f64],
}

impl<'a> Coord {
    pub fn from_bytes(data: &'a [u8]) -> &'a Self {
        debug_assert_eq!(
            data.len(),
            COORD_SIZE_IN_BYTES,
            "Bad number of bytes: `{}`, expected `{COORD_SIZE_IN_BYTES}`",
            data.len()
        );
        debug_assert!(
            data.as_ptr() as usize % std::mem::align_of::<f64>() == 0,
            "data is not aligned"
        );
        unsafe { std::mem::transmute(data) }
    }

    pub fn from_slice(data: &[f64]) -> &Self {
        debug_assert_eq!(data.len(), 2);
        unsafe { std::mem::transmute(data) }
    }

    pub fn x(&self) -> f64 {
        self.data[0]
    }

    pub fn y(&self) -> f64 {
        self.data[1]
    }

    pub fn to_geo(&self) -> geo_types::Coord<f64> {
        geo_types::Coord {
            x: self.x(),
            y: self.y(),
        }
    }
}

impl fmt::Debug for Coord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Coord")
            .field("x", &self.x())
            .field("y", &self.y())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;

    use super::*;

    #[test]
    fn test_basic_create_coord_from_bytes() {
        let data = [1.0, 2.0];
        let coord = Coord::from_bytes(cast_slice(&data));
        assert_eq!(coord.x(), 1.0);
        assert_eq!(coord.y(), 2.0);
    }

    #[test]
    #[should_panic]
    fn test_coord_panic_on_too_short_bytes() {
        let data = [1.0];
        Coord::from_bytes(cast_slice(&data));
    }
    #[test]
    #[should_panic]
    fn test_coord_panic_on_too_long_bytes() {
        let data = [1.0, 2.0, 3.0];
        Coord::from_bytes(cast_slice(&data));
    }

    #[test]
    #[should_panic]
    fn test_coord_panic_on_unaligned_bytes() {
        let data = [1.0, 2.0, 3.0];
        Coord::from_bytes(&cast_slice(&data)[1..]);
    }

    #[test]
    #[should_panic]
    fn test_coord_panic_on_too_short_slice() {
        let data = [1.0];
        Coord::from_bytes(cast_slice(&data));
    }

    #[test]
    #[should_panic]
    fn test_coord_panic_on_too_long_slice() {
        let data = [1.0, 2.0, 3.0];
        Coord::from_bytes(cast_slice(&data));
    }

    #[test]
    #[should_panic]
    fn debug_impl_support_precision_settings() {
        let data = [1.123456789, 2.987654321];
        let coord = Coord::from_bytes(cast_slice(&data));
        insta::assert_snapshot!(format!("{:.2?}", coord), @"Coord { x: 1.0, y: 2.0 }");
    }
}
