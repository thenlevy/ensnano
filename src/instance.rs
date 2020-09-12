use ultraviolet::{Vec3, Rotor3, Mat4};
#[derive(Debug)]
/// The instantiation of an object
pub struct Instance {
    /// The position in space
    pub position: Vec3,
    /// The rotation of the instance
    pub rotor: Rotor3,
    pub color: Vec3,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InstanceRaw {
    /// The model matrix of the instance
    pub model: Mat4,
    pub color: Vec3,
    _padding: u32,
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: Mat4::from_translation(self.position)
                * self.rotor.into_matrix().into_homogeneous(),
            color: self.color,
            _padding: 0,
        }
    }

    pub fn color_from_u32(color: u32) -> Vec3 {
        let red = (color & 0xFF0000) >> 16;
        let green = (color & 0x00FF00) >> 8;
        let blue = color & 0x0000FF;
        Vec3::new(red as f32 / 255., green as f32 / 255., blue as f32 / 255.)
    }
}
