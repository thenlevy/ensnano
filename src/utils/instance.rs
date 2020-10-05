use ultraviolet::{Mat4, Rotor3, Vec3, Vec4};
#[derive(Debug, Copy, Clone)]
/// The instantiation of an object
pub struct Instance {
    /// The position in space
    pub position: Vec3,
    /// The rotation of the instance
    pub rotor: Rotor3,
    pub color: Vec4,
    pub id: u32,
    pub scale: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InstanceRaw {
    /// The model matrix of the instance
    pub model: Mat4,
    pub color: Vec4,
    pub id: Vec4,
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        let scale = Mat4::from_nonuniform_scale(Vec3::new(self.scale, 1., 1.));
        InstanceRaw {
            model: Mat4::from_translation(self.position)
                * self.rotor.into_matrix().into_homogeneous()
                * scale,
            color: self.color,
            id: Self::id_from_u32(self.id),
        }
    }

    pub fn color_from_u32(color: u32) -> Vec4 {
        let red = (color & 0xFF0000) >> 16;
        let green = (color & 0x00FF00) >> 8;
        let blue = color & 0x0000FF;
        Vec4::new(
            red as f32 / 255.,
            green as f32 / 255.,
            blue as f32 / 255.,
            1.,
        )
    }

    pub fn color_from_au32(color: u32) -> Vec4 {
        let red = (color & 0xFF0000) >> 16;
        let green = (color & 0x00FF00) >> 8;
        let blue = color & 0x0000FF;
        let alpha = (color & 0xFF000000) >> 24;
        Vec4::new(
            red as f32 / 255.,
            green as f32 / 255.,
            blue as f32 / 255.,
            alpha as f32 / 255.,
        )
    }

    pub fn id_from_u32(id: u32) -> Vec4 {
        let a = (id & 0xFF000000) >> 24;
        let r = (id & 0x00FF0000) >> 16;
        let g = (id & 0x0000FF00) >> 8;
        let b = id & 0x000000FF;
        Vec4::new(r as f32 / 255., g as f32 / 255., b as f32 / 255., a as f32)
    }
}
