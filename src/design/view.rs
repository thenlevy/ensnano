use crate::utils::instance::Instance;
use std::rc::Rc;
use ultraviolet::{Mat4, Rotor3, Vec3};

/// An object that stores the instances to be drawn to represent the desgin.
pub struct View {
    spheres: Rc<Vec<Instance>>,
    tubes: Rc<Vec<Instance>>,
    /// The model matrix of the design
    pub model_matrix: Mat4,
    selected_tubes: Rc<Vec<Instance>>,
    selected_spheres: Rc<Vec<Instance>>,
    /// True if there are new instances to be fetched
    was_updated: bool,
    /// The identifier of the design. Used for fake color drawing
    id: u32,
}

impl View {
    pub fn new(id: u32) -> Self {
        Self {
            spheres: Rc::new(Vec::new()),
            tubes: Rc::new(Vec::new()),
            model_matrix: Mat4::identity(),
            selected_spheres: Rc::new(Vec::new()),
            selected_tubes: Rc::new(Vec::new()),
            was_updated: true,
            id,
        }
    }

    /// Update the instances of spheres, given the list of their center, color and identifier.
    pub fn update_spheres(&mut self, positions: &Vec<([f32; 3], u32, u32)>) {
        self.spheres = Rc::new(
            positions
                .iter()
                .map(|(v, color, id)| {
                    let id = *id | (self.id << 24);
                    Instance {
                        position: Vec3 {
                            x: v[0],
                            y: v[1],
                            z: v[2],
                        },
                        rotor: Rotor3::identity(),
                        color: Instance::color_from_u32(*color),
                        id: id,
                    }
                })
                .collect(),
        );
    }

    /// Update the instances of selected spheres, given the list of their center.
    pub fn update_selected_spheres(&mut self, positions: &Vec<[f32; 3]>) {
        self.selected_spheres = Rc::new(
            positions
                .iter()
                .map(|v| Instance {
                    position: Vec3 {
                        x: v[0],
                        y: v[1],
                        z: v[2],
                    },
                    rotor: Rotor3::identity(),
                    color: Vec3::zero(),
                    id: self.id << 24,
                })
                .collect(),
        );
        self.selected_tubes = Rc::new(Vec::new());
    }

    /// Update the list of tubes given the list of tubes of the form
    /// `(a, b, c, id)` where a and b are the center of the sphere to be connected by the tube, `c`
    /// is the color of the tube and `id` is the identifier of the tube.
    pub fn update_tubes(&mut self, pairs: &Vec<([f32; 3], [f32; 3], u32, u32)>) {
        self.tubes = Rc::new(
            pairs
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
                    let id = *id | (self.id << 24);
                    create_bound(position_a, position_b, *color, id)
                })
                .flatten()
                .collect(),
        );
    }

    /// update the list of selected tubes given the list of tube of the form
    /// `(a, b)` where `a` and `b` are the center of the sphere joined by the tubes
    pub fn update_selected_tubes(&mut self, pairs: &Vec<([f32; 3], [f32; 3])>) {
        self.selected_tubes = Rc::new(
            pairs
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
                    create_bound(position_a, position_b, 0, self.id << 24)
                })
                .flatten()
                .collect(),
        );
        self.selected_spheres = Rc::new(Vec::new());
    }

    /// Return true if the view was updated since the last time this function was called
    pub fn was_updated(&mut self) -> bool {
        let ret = self.was_updated;
        self.was_updated = false;
        ret
    }

    /// Update the model matrix
    pub fn set_matrix(&mut self, matrix: Mat4) {
        self.model_matrix = matrix;
        self.was_updated = true;
    }
}

impl View {
    /// Return the sphere instances
    pub fn get_spheres(&self) -> Rc<Vec<Instance>> {
        self.spheres.clone()
    }

    /// Return the tube instances
    pub fn get_tubes(&self) -> Rc<Vec<Instance>> {
        self.tubes.clone()
    }

    /// Return the instances of selected spheres
    pub fn get_selected_spheres(&self) -> Rc<Vec<Instance>> {
        self.selected_spheres.clone()
    }

    /// Return the instances of selected tubes
    pub fn get_selected_tubes(&self) -> Rc<Vec<Instance>> {
        self.selected_tubes.clone()
    }

    /// Return the model matrix
    pub fn get_model_matrix(&self) -> Mat4 {
        self.model_matrix
    }

}

fn create_bound(source: Vec3, dest: Vec3, color: u32, id: u32) -> Vec<Instance> {
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
