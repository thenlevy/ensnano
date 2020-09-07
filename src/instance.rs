#[derive(Debug)]
/// The instantiation of an object
pub struct Instance {
    /// The position in space
    pub position: cgmath::Vector3<f32>,
    /// The rotation of the instance
    pub rotation: cgmath::Quaternion<f32>,
    pub color: cgmath::Vector3<f32>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InstanceRaw {
    /// The model matrix of the instance
    pub model: cgmath::Matrix4<f32>,
    pub color: cgmath::Vector3<f32>,
    _padding: u32,
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation),
            color: self.color,
            _padding: 0,
        }
    }

    pub fn color_from_u32(color: u32) -> cgmath::Vector3<f32> {
        let red = (color & 0xFF0000) >> 16;
        let green = (color & 0x00FF00) >> 8;
        let blue = color & 0x0000FF;
        cgmath::Vector3::new(red as f32 / 255., green as f32 / 255., blue as f32 / 255.)
    }
}
