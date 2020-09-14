use crate::instance::Instance;
use ultraviolet::{Rotor3, Vec3};

pub struct View {
    spheres: Vec<Instance>,
    tubes: Vec<Instance>,
    origin: Vec3,
    rotor: Rotor3,
}

impl View {
    pub fn new() -> Self {
        Self {
            spheres: Vec::new(),
            tubes: Vec::new(),
            origin: Vec3::zero(),
            rotor: Rotor3::identity(),
        }
    }

    pub fn update_spheres(&mut self, spheres: &Vec<([f32 ; 3], u32, u32)>) {

    }

    pub fn update_tubes(&mut self, tubes: &Vec<([f32 ; 3], [f32; 3], u32, u32)>) {

    }
}
