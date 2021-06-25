/*
ENSnano, a 3d graphical application for DNA nanostructures.
    Copyright (C) 2021  Nicolas Levy <nicolaspierrelevy@gmail.com> and Nicolas Schabanel <nicolas.schabanel@ens-lyon.fr>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use super::*;
use ensnano_design::{Domain, Helix, Nucl, Parameters};
use std::io::Write;
use std::path::Path;
use ultraviolet::Vec3;

const BACKBONE_TO_CM: f32 = 0.34;

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

trait OxDnaHelix {
    fn ox_dna_nucl(&self, nucl_idx: isize, forward: bool, parameters: &Parameters) -> OxDnaNucl;
}

impl OxDnaHelix for Helix {
    fn ox_dna_nucl(&self, nucl_idx: isize, forward: bool, parameters: &Parameters) -> OxDnaNucl {
        let backbone_position = self.space_pos(parameters, nucl_idx, forward);
        let a1 = {
            let other_base = self.space_pos(parameters, nucl_idx, !forward);
            (other_base - backbone_position).normalized()
        };
        let normal = if forward {
            (self.axis_position(parameters, 1) - self.axis_position(parameters, 0)).normalized()
        } else {
            -(self.axis_position(parameters, 1) - self.axis_position(parameters, 0)).normalized()
        };
        let cm_position = backbone_position + a1 * BACKBONE_TO_CM;
        OxDnaNucl {
            position: cm_position,
            backbone_base: a1,
            normal,
            velocity: Vec3::zero(),
            angular_velocity: Vec3::zero(),
        }
    }
}

impl Presenter {
    fn to_oxdna(&self) -> (OxDnaConfig, OxDnaTopology) {
        let mut nucl_id = 0isize;
        let mut boundaries = [0f32, 0f32, 0f32];
        let mut bounds = Vec::new();
        let mut nucls = Vec::new();
        let mut basis_map = (*self.content.basis_map.clone()).clone();
        let mut nb_strand = 0;
        let parameters = self.current_design.parameters.unwrap_or_default();
        for (strand_id, s) in self.current_design.strands.values().enumerate() {
            nb_strand = strand_id + 1;
            let mut prev_nucl: Option<isize> = None;
            let first_strand_nucl = nucl_id;
            for d in s.domains.iter() {
                if let Domain::HelixDomain(dom) = d {
                    for position in dom.iter() {
                        let ox_nucl = self.current_design.helices[&dom.helix].ox_dna_nucl(
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

    pub fn oxdna_export(&self, directory: &PathBuf) -> std::io::Result<(PathBuf, PathBuf)> {
        let mut config_name = directory.clone();
        config_name.push("export.oxdna");
        let mut topology_name = directory.clone();
        topology_name.push("export.top");
        let (config, topo) = self.to_oxdna();
        config.write(config_name.clone())?;
        topo.write(topology_name.clone())?;
        Ok((config_name, topology_name))
        /*
        if success {
            crate::utils::message(
                format!(
                    "Successfully exported to {:?} and {:?}",
                    config_name, topology_name,
                )
                .into(),
                rfd::MessageLevel::Info,
            );
        }*/
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
