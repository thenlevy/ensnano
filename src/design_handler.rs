use crate::scene::Scene;
use cgmath::prelude::*;
use cgmath::{Matrix3, Quaternion, Vector3};
use std::path::{Path, PathBuf};

type Basis = (f32, f64, f64, [f32; 3], u32);

pub struct DesignHandler {
    design: codenano::Design<(), ()>,
}

impl DesignHandler {
    pub fn new() -> Self {
        let design = codenano::Design::<(), ()>::new();
        Self { design }
    }

    pub fn new_with_path(json_path: &Path) -> Self {
        let json_str =
            std::fs::read_to_string(json_path).expect(&format!("File not found {:?}", json_path));
        let design = serde_json::from_str(&json_str).expect("Error in .json file");
        Self { design }
    }

    pub fn get_design(&mut self, file: &PathBuf) {
        let json_str = std::fs::read_to_string(file);
        if let Ok(json_str) = json_str {
            let design = serde_json::from_str(&json_str);
            if let Ok(design) = design {
                self.design = design
            } else {
                println!("could not read the new json file");
            }
        }
    }

    pub fn update_scene(&self, scene: &mut Scene, fit: bool) {
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
                        domain.forward,
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
        if fit {
            self.fit_design(scene);
        }
    }
}

impl DesignHandler {
    pub fn fit_design(&self, scene: &mut Scene) {
        let mut bases = self.get_bases(scene);
        let rotation = self.get_fitting_quaternion(&bases);
        let direction = rotation.rotate_vector([0., 0., -1.].into());
        let position = self.get_fitting_position(&mut bases, scene, &direction);
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

    /// Return the 3 axis sorted by magnitude of instances coordinates
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

        if ratio < 1. {
            bases.swap(0, 1);
        }

        bases
    }

    fn get_fitting_quaternion(&self, bases: &Vec<Basis>) -> Quaternion<f32> {
        let right: Vector3<f32> = bases[0].3.into();
        let up: Vector3<f32> = bases[1].3.into();
        let reverse_direction = right.cross(up);
        let rotation_matrix = Matrix3::from_cols(right, up, reverse_direction);
        rotation_matrix.into()
    }

    /// Given the orientation of the camera, computes its position so that it can see everything.
    fn get_fitting_position(
        &self,
        bases: &mut Vec<Basis>,
        scene: &Scene,
        direction: &Vector3<f32>,
    ) -> Vector3<f32> {
        let ratio = scene.get_ratio();
        // We want to fit both the x and the y axis.
        let vertical = (bases[1].0).max(bases[0].0 / ratio) + bases[2].0;

        let fovy = scene.get_fovy();
        let x_back = vertical / 2. / (fovy / 2.).tan();

        bases.sort_by_key(|b| b.4);
        let coord = Vector3::from([bases[0].1 as f32, bases[1].1 as f32, bases[2].1 as f32]);
        coord - direction * x_back
    }
}
