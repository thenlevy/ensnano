use std::path::Path;
use crate::scene::Scene;

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
