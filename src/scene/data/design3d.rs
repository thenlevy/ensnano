use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use ultraviolet::{Mat4, Vec3, Rotor3};
use crate::utils::instance::Instance;
use crate::design::{Design, ObjectType, Referential};


/// An object that handles the 3d graphcial representation of a `Design`
pub struct Design3D {
    design: Arc<Mutex<Design>>,
    id: u32,
}

type Basis = (f32, f32, f32, [f32; 3], u32);

impl Design3D {

    pub fn new(design: Arc<Mutex<Design>>) -> Self {
        let id = design.lock().unwrap().get_id() as u32;
        Self {
            design,
            id,
        }
    }

    /// Convert a list of ids into a list of instances
    pub fn id_to_instances(&self, ids: Vec<u32>) -> Vec<Instance> {
        ids.iter().map(|id| self.make_instance(*id)).collect()
    }

    /// Return the list of sphere instances to be displayed to represent the design
    pub fn get_spheres(&self) -> Rc<Vec<Instance>> {
        let ids = self.design.lock().unwrap().get_all_nucl_ids();
        Rc::new(self.id_to_instances(ids))
    }

    /// Return the list of tube instances to be displayed to represent the design
    pub fn get_tubes(&self) -> Rc<Vec<Instance>> {
        let ids = self.design.lock().unwrap().get_all_bound_ids();
        Rc::new(self.id_to_instances(ids))
    }

    /*
    /// Extract the ids that are identifier of spheres and return the corresponding instances
    pub fn extract_spheres(&self, ids: &Vec<u32>) -> Rc<Vec<Instance>> {
        Rc::new(self.id_to_instances(&mut ids.iter().filter(|id| self.design.lock().unwrap().is_nucl(**id))))
    }

    /// Extract the ids that are identifier of tubes and return the corresponding instances
    pub fn extract_tubes(&self, ids: &Vec<u32>) -> Rc<Vec<Instance>> {
        Rc::new(self.id_to_instances(&mut ids.iter().filter(|id| self.design.lock().unwrap().is_bound(**id))))
    }*/

    pub fn get_model_matrix(&self) -> Mat4 {
        self.design.lock().unwrap().get_model_matrix()
    }

    /// Convert return an instance representing the object with identifier `id`
    /// This function will panic if `id` does not represent an object of the design
    pub fn make_instance(&self, id: u32) -> Instance {
        let kind = self.get_object_type(id).expect("id not in design"); 
        let referential = Referential::Model;
        let instanciable = match kind {
            ObjectType::Bound(id1, id2) => {
                let pos1 = self.get_element_position(id1, referential).unwrap();
                let pos2 = self.get_element_position(id2, referential).unwrap();
                let color = self.get_color(id).unwrap_or(0);
                let id = id | self.id << 24;
                Instantiable::new(ObjectRepr::Tube(pos1, pos2), color, id)
            }
            ObjectType::Nucleotide(id) => {
                let position = self.get_element_position(id, referential).unwrap();
                let color = self.get_color(id).unwrap();
                let id = id | self.id << 24;
                Instantiable::new(ObjectRepr::Sphere(position), color, id)
            }
        };
        instanciable.to_instance()
    }

    pub fn is_nucl(&self, id: u32) -> bool {
        self.design.lock().unwrap().is_nucl(id)
    }

    pub fn is_bound(&self, id: u32) -> bool {
        self.design.lock().unwrap().is_bound(id)
    }

    fn get_nucl_involved(&self, id: u32) -> Option<(u32, u32)> {
        self.design.lock().unwrap().get_nucl_involved(id)
    }

    fn get_object_type(&self, id: u32) -> Option<ObjectType> {
        self.design.lock().unwrap().get_object_type(id)
    }

    pub fn get_element_position(&self, id: u32, referential: Referential) -> Option<Vec3> {
        self.design.lock().unwrap().get_element_position(id, referential)
    }

    fn get_color(&self, id: u32) -> Option<u32> {
        self.design.lock().unwrap().get_color(id)
    }


    /// Return a camera position and orientation so that self fits in the scene.
    pub fn get_fitting_camera(&self, ratio: f32, fovy: f32) -> (Vec3, Rotor3) {
        let mut bases = self.get_bases(ratio);
        let rotation = self.get_fitting_rotor(&bases);
        let direction = rotation.reversed() * -Vec3::unit_z();
        let position = self.get_fitting_position(&mut bases, ratio, fovy, &direction);
        (position, rotation)
    }

    /// Return the middle point of `self` in the world coordinates
    pub fn middle_point(&self) -> Vec3 {
        let boundaries = self.boundaries();
        let middle = Vec3::new(
            (boundaries[0] + boundaries[1]) as f32 / 2.,
            (boundaries[2] + boundaries[3]) as f32 / 2.,
            (boundaries[4] + boundaries[5]) as f32 / 2.,
        );
        self.design.lock().unwrap().get_model_matrix().transform_vec3(middle)
    }

    fn boundaries(&self) -> [f32; 6] {
        let mut min_x = std::f32::INFINITY;
        let mut min_y = std::f32::INFINITY;
        let mut min_z = std::f32::INFINITY;
        let mut max_x = std::f32::NEG_INFINITY;
        let mut max_y = std::f32::NEG_INFINITY;
        let mut max_z = std::f32::NEG_INFINITY;

        let ids = self.design.lock().unwrap().get_all_nucl_ids();
        for id in ids {
            let coord: [f32; 3] = self.design.lock().unwrap().get_element_position(id, Referential::World).unwrap().into();
            if coord[0] < min_x {
                min_x = coord[0];
            }
            if coord[0] > max_x {
                max_x = coord[0];
            }
            if coord[1] < min_y {
                min_y = coord[1];
            }
            if coord[1] > max_y {
                max_y = coord[1];
            }
            if coord[2] < min_z {
                min_z = coord[2];
            }
            if coord[2] > max_z {
                max_z = coord[2];
            }
        }
        [min_x, max_x, min_y, max_y, min_z, max_z]
    }

    /// Return the 3 axis sorted by magnitude of instances coordinates
    fn get_bases(&self, ratio: f32) -> Vec<Basis> {
        let [min_x, max_x, min_y, max_y, min_z, max_z] = self.boundaries();
        let delta_x = ((max_x - min_x) * 1.1) as f32;
        let delta_y = ((max_y - min_y) * 1.1) as f32;
        let delta_z = ((max_z - min_z) * 1.1) as f32;

        let mut bases = vec![
            (delta_x, (max_x + min_x) / 2., max_x, [1., 0., 0.], 0),
            (delta_y, (max_y + min_y) / 2., max_y, [0., 1., 0.], 1),
            (delta_z, (max_z + min_z) / 2., max_z, [0., 0., 1.], 2),
        ];

        bases.sort_by(|a, b| (b.0).partial_cmp(&(a.0)).unwrap());

        if ratio < 1. {
            bases.swap(0, 1);
        }

        bases
    }

    /// Return a rotor that will maps the longest axis to the camera's x axis,
    /// and the second longest axis to the camera's y axis
    fn get_fitting_rotor(&self, bases: &Vec<Basis>) -> Rotor3 {
        let right: Vec3 = bases[0].3.into();
        let up: Vec3 = bases[1].3.into();
        let reverse_direction = right.cross(up);
        // The arguments of Mat3::new are the columns so this is the *inverse* of the rotation
        // matrix
        let inv_rotation_matrix = ultraviolet::Mat3::new(right, up, reverse_direction);
        inv_rotation_matrix.into_rotor3().reversed()
    }

    /// Given the orientation of the camera, computes its position so that it can see everything.
    fn get_fitting_position(
        &self,
        bases: &mut Vec<Basis>,
        ratio: f32,
        fovy: f32,
        direction: &Vec3,
    ) -> Vec3 {
        // We want to fit both the x and the y axis.
        let vertical = (bases[1].0).max(bases[0].0 / ratio) + bases[2].0;

        let x_back = vertical / 2. / (fovy / 2.).tan();

        bases.sort_by_key(|b| b.4);
        let coord = self.middle_point();
        coord - *direction * x_back
    }

}


pub struct Instantiable {
    repr: ObjectRepr,
    color: u32,
    id: u32,
}

impl Instantiable {
    pub fn new(repr: ObjectRepr, color: u32, id: u32) -> Self {
        Self {
            repr,
            color,
            id
        }
    }

    pub fn to_instance(&self) -> Instance {
        match self.repr {
            ObjectRepr::Tube(a, b) => {
                create_bound(a.into(), b.into(), self.color, self.id)
            }
            ObjectRepr::Sphere(x) => {
                Instance {
                    position: x.into(),
                    rotor: Rotor3::identity(),
                    color: Instance::color_from_u32(self.color),
                    id: self.id,
                    scale: 1.
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ObjectRepr {
    /// A sphere given by its center
    Sphere(Vec3),
    /// A tube given by its two coordinates
    Tube(Vec3, Vec3),
}

fn create_bound(source: Vec3, dest: Vec3, color: u32, id: u32) -> Instance {
    let color = Instance::color_from_u32(color);
    let rotor = Rotor3::from_rotation_between(Vec3::unit_x(), (dest - source).normalized());
    let position = (dest + source) / 2.;
    let scale = (dest - source).mag();

    Instance {
        position,
        color,
        rotor,
        id,
        scale
    }
}

