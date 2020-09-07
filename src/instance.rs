#[derive(Debug)]
/// The instantiation of an object
pub struct Instance {
    /// The position in space
    pub position: cgmath::Vector3<f32>,
    /// The rotation of the instance
    pub rotation: cgmath::Quaternion<f32>,
}

#[repr(C)]
#[derive(Copy, Clone)]
/// A wraper arround the model matrix of an Instance
pub struct InstanceRaw {
    /// The model matrix of the instance
    pub model: cgmath::Matrix4<f32>,
}

unsafe impl bytemuck::Pod for InstanceRaw {}
unsafe impl bytemuck::Zeroable for InstanceRaw {}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation),
        }
    }
}
