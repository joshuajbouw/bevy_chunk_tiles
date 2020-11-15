use crate::lib::*;

/// A trait that converts a primitive into a 2 dimensional coordinate.
pub trait ToCoord2 {
    /// The conversion from self into a 2 dimensional Vec2 object.
    fn to_coord2(&self, width: f32, height: f32) -> Vec2;
}

impl ToCoord2 for Vec2 {
    fn to_coord2(&self, _width: f32, _height: f32) -> Vec2 {
        *self
    }
}

impl ToCoord2 for usize {
    fn to_coord2(&self, width: f32, height: f32) -> Vec2 {
        let y = *self as f32 / height;
        let x = *self as f32 % width;
        Vec2::new(x, y)
    }
}

/// A trait that converts a primitive into a 3 dimensional coordinate.
pub trait ToCoord3 {
    /// The conversion from self into a 3 dimensional Vec3 object.
    fn to_coord3(&self, width: f32, height: f32) -> Vec3;
}

impl ToCoord3 for Vec3 {
    fn to_coord3(&self, _width: f32, _height: f32) -> Vec3 {
        *self
    }
}

impl ToCoord3 for usize {
    fn to_coord3(&self, width: f32, height: f32) -> Vec3 {
        let z = *self as f32 / (width * height);
        let idx = *self as f32 - (z * width * height);
        let y = height - 1. - (idx / width);
        let x = idx % width;
        Vec3::new(x, y, z)
    }
}

/// A trait that takes a dimensional coordinate and translates it back into an index.
pub trait ToIndex {
    /// The conversion from self into an index value.
    fn to_index(&self, width: f32, height: f32) -> usize;
}

impl ToIndex for usize {
    fn to_index(&self, _width: f32, _height: f32) -> usize {
        *self
    }
}

impl ToIndex for Vec2 {
    fn to_index(&self, width: f32, _height: f32) -> usize {
        ((self.y() * width) + self.x()) as usize
    }
}

impl ToIndex for Vec3 {
    fn to_index(&self, width: f32, height: f32) -> usize {
        ((self.z() * width * height) + (self.y() * width) + self.x()) as usize
    }
}
