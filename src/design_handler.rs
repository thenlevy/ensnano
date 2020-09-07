use std::path::Path;
use crate::scene::Scene;
use cgmath::{ Quaternion, Vector3, Rad };
use cgmath::prelude::*;
use std::f32::consts::PI;
use std::f32::consts::FRAC_PI_2;

type Basis = (f32, f64, f64, [f32; 3], u32);

pub struct DesignHandler {
    design: codenano::Design<(), ()>
}

impl DesignHandler {
    pub fn new(json_path: &Path) -> Self {
        let json_str = std::fs::read_to_string(json_path).expect(&format!("File not found {:?}", json_path));
        let design = serde_json::from_str(&json_str).expect("Error in .json file");
        Self {
            design
        }
    }

    pub fn update_scene(&self, scene: &mut Scene) {
        let mut nucleotide = Vec::new();
        let mut covalent_bound = Vec::new();
        let mut old_position = None;
        for strand in &self.design.strands {
            let color = if let Some(ref color) = strand.color {
                color.as_int()
            } else {
                strand.default_color().as_int()
            };
            for domain in &strand.domains {
                for nucl in domain.iter() {
                    let position = self.design.helices[domain.helix as usize].space_pos(
                        self.design.parameters.as_ref().unwrap(),
                        nucl,
                        domain.forward
                    );
                    let position = [position[0] as f32, position[1] as f32, position[2] as f32];
                    if let Some(old_position) = old_position.take() {
                        covalent_bound.push((old_position, position, color));
                    }
                    old_position = Some(position);
                    nucleotide.push((position.clone(), color));
                }
            }
            old_position = None;
        }
        scene.update_spheres(&nucleotide);
        scene.update_tubes(&covalent_bound);
    }
}

impl DesignHandler {

    pub fn fit_design(&self, scene: &mut Scene) {

        let rotation = self.get_fitting_quaternion(scene);
        let position = self.get_fitting_position(scene);
        scene.fit(position, rotation);
    }

    fn boundaries(&self) -> [f64; 6] {
        let mut min_x = std::f64::INFINITY;
        let mut min_y = std::f64::INFINITY;
        let mut min_z = std::f64::INFINITY;
        let mut max_x = std::f64::NEG_INFINITY;
        let mut max_y = std::f64::NEG_INFINITY;
        let mut max_z = std::f64::NEG_INFINITY;

        let param = &self.design.parameters.unwrap();
        for s in &self.design.strands {
            for d in &s.domains {
                let helix = &self.design.helices[d.helix as usize];
                for coord in vec![
                    helix.space_pos(param, d.start, d.forward),
                    helix.space_pos(param, d.end, d.forward),
                ] {
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
            }
        }
        [min_x, max_x, min_y, max_y, min_z, max_z]
    }

    fn get_bases(&self, scene: &Scene) -> Vec<Basis> {
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

        let ratio = scene.get_ratio();

        if bases[0].0 / ratio > bases[1].0 {
            bases.swap(0, 1);
        }

        bases
    }

    fn get_fitting_quaternion(&self, scene: &Scene) -> Quaternion<f32> {

        let bases = self.get_bases(scene);


        let rotation = if bases[2].4 == 1 {
            if bases[1].4 == 0 {
                Quaternion::from_axis_angle(Vector3::from([1., 0., 0.]), Rad(-FRAC_PI_2))
                    * Quaternion::from_axis_angle(Vector3::from([0., 1., 0.]), Rad(-FRAC_PI_2))
            } else {
                Quaternion::from_axis_angle(Vector3::from([1., 0., 0.]), Rad(FRAC_PI_2))
            }
        } else if bases[2].4 == 0 {
            if bases[1].4 == 1 {
                Quaternion::from_axis_angle(Vector3::from([0., 1., 0.]), Rad(-FRAC_PI_2))
                    * Quaternion::from_axis_angle(Vector3::from([1., 0., 0.]), Rad(-FRAC_PI_2))
            } else {
                Quaternion::from_axis_angle(Vector3::from([0., 1., 0.]), Rad(FRAC_PI_2))
            }
        } else {
            Quaternion::from([1., 0., 0., 0.])
       };
       rotation
    }

    fn get_fitting_position(&self, scene: &Scene) -> Vector3<f32> {
        let mut bases = self.get_bases(scene);
        let vertical = bases[1].0;

        let fovy = scene.get_fovy();
        let z_back = vertical / 2. / fovy.tan();

        bases.sort_by_key(|b| b.4);
        let coord = Vector3::from([bases[0].1 as f32, bases[1].1 as f32, bases[2].1 as f32]);
        coord - scene.get_camera_direction() * z_back
    }

}
