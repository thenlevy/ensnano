use crate::instance::Instance;
use ultraviolet::{Rotor3, Vec3};
use std::rc::Rc;

pub struct View {
    spheres: Rc<Vec<Instance>>,
    tubes: Rc<Vec<Instance>>,
    origin: Vec3,
    rotor: Rotor3,
    selected_tubes: Rc<Vec<Instance>>,
    selected_spheres: Rc<Vec<Instance>>,
}

impl View {
    pub fn new() -> Self {
        Self {
            spheres: Rc::new(Vec::new()),
            tubes: Rc::new(Vec::new()),
            origin: Vec3::zero(),
            rotor: Rotor3::identity(),
            selected_spheres: Rc::new(Vec::new()),
            selected_tubes: Rc::new(Vec::new()),
        }
    }

    pub fn update_spheres(&mut self, positions: &Vec<([f32 ; 3], u32, u32)>) {
        self.spheres = Rc::new(positions
            .iter()
            .map(|(v, color, id)| Instance {
                position: Vec3 {
                    x: v[0],
                    y: v[1],
                    z: v[2],
                },
               rotor: Rotor3::identity(),
               color: Instance::color_from_u32(*color),
               id: *id,
            })
            .collect());
    }

    pub fn update_selected_spheres(&mut self, positions: &Vec<[f32 ; 3]>) {
        self.selected_spheres = Rc::new(positions
            .iter()
            .map(|v| Instance {
                position: Vec3 {
                    x: v[0],
                    y: v[1],
                    z: v[2],
                },
               rotor: Rotor3::identity(),
               color: Vec3::zero(),
               id: 0,
            })
            .collect());
        self.selected_tubes = Rc::new(Vec::new());
    }

    pub fn update_tubes(&mut self, pairs: &Vec<([f32 ; 3], [f32; 3], u32, u32)>) {
        self.tubes = Rc::new(pairs
            .iter()
            .map(|(a, b, color, id)| {
                let position_a = Vec3 {
                    x: a[0],
                    y: a[1],
                    z: a[2],
                };
                let position_b = Vec3 {
                    x: b[0],
                    y: b[1],
                    z: b[2],
                };
                create_bound(position_a, position_b, *color, *id)
            })
            .flatten()
            .collect());
    }

    pub fn update_selected_tubes(&mut self, pairs: &Vec<([f32 ; 3], [f32; 3])>) {
        self.selected_tubes = Rc::new(pairs
            .iter()
            .map(|(a, b)| {
                let position_a = Vec3 {
                    x: a[0],
                    y: a[1],
                    z: a[2],
                };
                let position_b = Vec3 {
                    x: b[0],
                    y: b[1],
                    z: b[2],
                };
                create_bound(position_a, position_b, 0, 0)
            })
            .flatten()
            .collect());
        self.selected_spheres = Rc::new(Vec::new());
    }
}

impl View {
    pub fn get_spheres(&self) -> Rc<Vec<Instance>> {
        self.spheres.clone()
    }

    pub fn get_tubes(&self) -> Rc<Vec<Instance>> {
        self.tubes.clone()
    }

    pub fn get_selected_spheres(&self) -> Rc<Vec<Instance>> {
        self.selected_spheres.clone()
    }

    pub fn get_selected_tubes(&self) -> Rc<Vec<Instance>> {
        self.selected_tubes.clone()
    }
}


fn create_bound(
    source: Vec3,
    dest: Vec3,
    color: u32,
    id: u32,
) -> Vec<Instance> {
    let mut ret = Vec::new();
    let color = Instance::color_from_u32(color);
    let rotor = Rotor3::from_rotation_between(Vec3::unit_x(), (dest - source).normalized());

    let obj = (dest - source).mag();
    let mut current_source = source.clone();
    let mut current_length = 0.;
    let one_step_len = crate::consts::BOUND_LENGTH;
    let step_dir = (dest - source).normalized();
    let one_step = step_dir * one_step_len;
    while current_length < obj {
        let position = if current_length + one_step_len > obj {
            current_source + step_dir * (obj - current_length) / 2.
        } else {
            current_source + one_step / 2.
        };
        current_source = position + one_step / 2.;
        current_length = (source - current_source).mag();
        ret.push(Instance {
            position,
            rotor,
            color,
            id,
        });
    }
    ret
}
