use super::icednano::{Domain, Helix};
use super::{Data, Nucl, Parameters};
use std::io::Write;
use std::path::Path;
use ultraviolet::Vec3;

struct OxDnaNucl {
    position: Vec3,
    backbone_base: Vec3,
    normal: Vec3,
    velocity: Vec3,
    angular_velocity: Vec3,
}

struct OxDnaConfig {
    time: f32,
    boundaries: [f32; 3],
    /// Etot, U and K
    kinetic_energies: [f32; 3],
    nucls: Vec<OxDnaNucl>,
}

impl OxDnaConfig {
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::create(path)?;
        let max = self.boundaries[0].max(self.boundaries[1].max(self.boundaries[2]));
        writeln!(&mut file, "t = {}", self.time)?;
        writeln!(&mut file, "b = {} {} {}", max, max, max)?;
        writeln!(
            &mut file,
            "E = {} {} {}",
            self.kinetic_energies[0], self.kinetic_energies[1], self.kinetic_energies[2]
        )?;
        for n in self.nucls.iter() {
            writeln!(
                &mut file,
                "{} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                n.position.x,
                n.position.y,
                n.position.z,
                n.backbone_base.x,
                n.backbone_base.y,
                n.backbone_base.z,
                n.normal.x,
                n.normal.y,
                n.normal.z,
                n.velocity.x,
                n.velocity.y,
                n.velocity.z,
                n.angular_velocity.x,
                n.angular_velocity.y,
                n.angular_velocity.z,
            )?;
        }
        Ok(())
    }
}

struct OxDnaTopology {
    nb_nucl: usize,
    nb_strand: usize,
    bounds: Vec<OxDnaBound>,
}

impl OxDnaTopology {
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let mut file = std::fs::File::create(path)?;
        writeln!(&mut file, "{} {}", self.nb_nucl, self.nb_strand)?;
        for bound in self.bounds.iter() {
            writeln!(
                &mut file,
                "{} {} {} {}",
                bound.strand_id, bound.base, bound.prime5, bound.prime3
            )?;
        }
        Ok(())
    }
}

struct OxDnaBound {
    strand_id: usize,
    base: char,
    prime5: isize,
    prime3: isize,
}

impl Helix {
    fn ox_dna_nucl(&self, nucl_idx: isize, forward: bool, parameters: &Parameters) -> OxDnaNucl {
        let position = self.space_pos(parameters, nucl_idx, forward);
        let backbone_base = {
            let center = self.axis_position(parameters, nucl_idx);
            (position - center).normalized()
        };
        let normal = if forward {
            (self.axis_position(parameters, 1) - self.axis_position(parameters, 0)).normalized()
        } else {
            -(self.axis_position(parameters, 1) - self.axis_position(parameters, 0)).normalized()
        };
        OxDnaNucl {
            position,
            backbone_base,
            normal,
            velocity: Vec3::zero(),
            angular_velocity: Vec3::zero(),
        }
    }
}

impl Data {
    fn to_oxdna(&self) -> (OxDnaConfig, OxDnaTopology) {
        let mut nucl_id = 0isize;
        let mut boundaries = [0f32, 0f32, 0f32];
        let mut bounds = Vec::new();
        let mut nucls = Vec::new();
        let mut basis_map = self.basis_map.read().unwrap().clone();
        let mut nb_strand = 0;
        let parameters = self.design.parameters.unwrap_or_default();
        for (strand_id, s) in self.design.strands.values().enumerate() {
            nb_strand = strand_id + 1;
            let mut prev_nucl: Option<isize> = None;
            let first_strand_nucl = nucl_id;
            for d in s.domains.iter() {
                if let Domain::HelixDomain(dom) = d {
                    for position in dom.iter() {
                        let ox_nucl = self.design.helices[&dom.helix].ox_dna_nucl(
                            position,
                            dom.forward,
                            &parameters,
                        );
                        boundaries[0] = boundaries[0].max(2. * ox_nucl.position.x.abs());
                        boundaries[1] = boundaries[1].max(2. * ox_nucl.position.y.abs());
                        boundaries[2] = boundaries[2].max(2. * ox_nucl.position.z.abs());
                        nucls.push(ox_nucl);
                        let nucl = Nucl {
                            position,
                            helix: dom.helix,
                            forward: dom.forward,
                        };
                        let base = basis_map.get(&nucl).cloned().unwrap_or_else(|| {
                            basis_map
                                .get(&nucl.compl())
                                .cloned()
                                .unwrap_or_else(rand_base)
                        });
                        basis_map.insert(nucl.compl(), compl(base));
                        let bound = OxDnaBound {
                            base,
                            strand_id,
                            prime3: -1,
                            prime5: prev_nucl.unwrap_or(-1),
                        };
                        bounds.push(bound);
                        if let Some(prev) = prev_nucl {
                            bounds.get_mut(prev as usize).unwrap().prime3 = nucl_id;
                        }
                        prev_nucl = Some(nucl_id);
                        nucl_id += 1;
                    }
                }
            }
            if s.cyclic {
                bounds.iter_mut().last().unwrap().prime3 = first_strand_nucl;
                bounds.get_mut(first_strand_nucl as usize).unwrap().prime5 = nucl_id - 1;
            }
        }
        let topo = OxDnaTopology {
            bounds,
            nb_strand,
            nb_nucl: nucl_id as usize,
        };
        let config = OxDnaConfig {
            time: 0f32,
            kinetic_energies: [0f32, 0f32, 0f32],
            boundaries,
            nucls,
        };
        (config, topo)
    }

    pub fn oxdna_export(&self) {
        let mut config_name = self.file_name.clone();
        config_name.set_extension("oxdna");
        let mut topology_name = self.file_name.clone();
        topology_name.set_extension("top");
        let (config, topo) = self.to_oxdna();
        if config.write(config_name).is_err() {
            println!("Could not write config");
        }
        if topo.write(topology_name).is_err() {
            println!("Could not write topo");
        }
    }
}

fn rand_base() -> char {
    match rand::random::<u8>() % 4 {
        0 => 'A',
        1 => 'T',
        2 => 'G',
        _ => 'C',
    }
}

fn compl(c: char) -> char {
    match c {
        'A' => 'T',
        'G' => 'C',
        'T' => 'A',
        _ => 'G',
    }
}
