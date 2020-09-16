use ultraviolet::{Vec3, Rotor3, Mat4, Vec4};
#[derive(Debug, Copy, Clone)]
/// The instantiation of an object
pub struct Instance {
    /// The position in space
    pub position: Vec3,
    /// The rotation of the instance
    pub rotor: Rotor3,
    pub color: Vec3,
    pub id: u32
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InstanceRaw {
    /// The model matrix of the instance
    pub model: Mat4,
    pub color: Vec3,
    pub id: Vec4,
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
            id: Self::id_from_u32(self.id),
            _padding: 0,
        }
    }

    pub fn color_from_u32(color: u32) -> Vec3 {
        let red = (color & 0xFF0000) >> 16;
        let green = (color & 0x00FF00) >> 8;
        let blue = color & 0x0000FF;
        Vec3::new(red as f32 / 255., green as f32 / 255., blue as f32 / 255.)
    }

    pub fn id_from_u32(id: u32) -> Vec4 {
        let a = (id & 0xFF000000) >> 24;
        let r = (id & 0x00FF0000) >> 16;
        let g = (id & 0x0000FF00) >> 8;
        let b = id & 0x000000FF;
        Vec4::new(a as f32 / 255., r as f32 / 255., g as f32 / 255., b as f32 / 255.)
    }

    pub fn size_of_raw() -> usize {
        std::mem::size_of::<Mat4>() + std::mem::size_of::<Vec3>() + std::mem::size_of::<u32>()
    }
}
