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
use ensnano_design::{Domain, Helix, HelixCollection, Nucl, Parameters};
use std::io::Write;
use std::mem::ManuallyDrop;
use std::path::Path;
use ultraviolet::{Mat3, Rotor3, Vec3};

pub const OXDNA_LEN_FACTOR: f32 = 1. / 0.8518;
pub const BACKBONE_TO_CM: f32 = 0.34 * OXDNA_LEN_FACTOR;

pub struct OxDnaNucl {
    pub position: Vec3,
    backbone_base: Vec3,
    pub normal: Vec3,
    velocity: Vec3,
    angular_velocity: Vec3,
}

impl OxDnaNucl {
    pub fn get_basis(&self) -> Rotor3 {
        let a1 = self.backbone_base.normalized();
        let a3 = -self.normal.normalized();
        let a2 = a3.cross(a1).normalized();
        let a3 = a1.cross(a2).normalized();

        Mat3::new(a1, a2, a3).into_rotor3()
    }
}

pub struct OxDnaConfig {
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

pub struct OxDnaTopology {
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

pub trait OxDnaHelix {
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
            (self.normal_at_pos(nucl_idx, forward)).normalized()
        } else {
            -(self.normal_at_pos(nucl_idx, forward)).normalized()
        };
        let cm_position = backbone_position * OXDNA_LEN_FACTOR + a1 * BACKBONE_TO_CM;
        OxDnaNucl {
            position: cm_position,
            backbone_base: a1,
            normal,
            velocity: Vec3::zero(),
            angular_velocity: Vec3::zero(),
        }
    }
}

pub fn free_oxdna_nucl(
    pos: Vec3,
    previous_position: Option<Vec3>,
    free_idx: usize,
    parameters: &Parameters,
) -> OxDnaNucl {
    let backbone_position = pos;
    let normal = (pos - previous_position.unwrap_or_else(Vec3::zero)).normalized();
    let a1 = {
        let tangent = normal.cross(Vec3::new(-normal.z, normal.x, normal.y));
        let bitangent = normal.cross(tangent);
        let angle = std::f32::consts::TAU / parameters.bases_per_turn * -(free_idx as f32);
        tangent * angle.sin() + bitangent * angle.cos()
    };
    let cm_position = backbone_position * OXDNA_LEN_FACTOR + a1 * BACKBONE_TO_CM;
    OxDnaNucl {
        position: cm_position,
        backbone_base: a1,
        normal,
        velocity: Vec3::zero(),
        angular_velocity: Vec3::zero(),
    }
}

pub(super) struct OxDnaMaker<'a> {
    nucl_id: isize,
    boundaries: [f32; 3],
    bounds: Vec<OxDnaBound>,
    nucls: Vec<OxDnaNucl>,
    basis_map: BasisMapper<'a>,
    nb_strand: usize,
    parameters: Parameters,
}

impl<'a> OxDnaMaker<'a> {
    pub fn new(basis_map: BasisMapper<'a>, parameters: Parameters) -> Self {
        Self {
            nucl_id: 0,
            boundaries: Default::default(),
            bounds: Vec::new(),
            nucls: Vec::new(),
            basis_map,
            parameters,
            nb_strand: 0,
        }
    }

    pub fn new_strand<'b>(&'b mut self, strand_id: usize) -> ManuallyDrop<StrandMaker<'b, 'a>> {
        self.nb_strand += 1;
        let first_strand_nucl = self.nucl_id;
        ManuallyDrop::new(StrandMaker {
            context: self,
            strand_id,
            prev_nucl: None,
            first_strand_nucl,
            previous_position: None,
        })
    }

    pub fn end(self) -> (OxDnaConfig, OxDnaTopology) {
        let topo = OxDnaTopology {
            bounds: self.bounds,
            nb_strand: self.nb_strand,
            nb_nucl: self.nucl_id as usize,
        };
        let config = OxDnaConfig {
            time: 0f32,
            kinetic_energies: [0f32, 0f32, 0f32],
            boundaries: self.boundaries,
            nucls: self.nucls,
        };
        (config, topo)
    }
}

pub struct StrandMaker<'a, 'b> {
    context: &'a mut OxDnaMaker<'b>,
    strand_id: usize,
    prev_nucl: Option<isize>,
    first_strand_nucl: isize,
    previous_position: Option<Vec3>,
}

impl StrandMaker<'_, '_> {
    pub fn add_ox_nucl(&mut self, ox_nucl: OxDnaNucl, nucl: Option<Nucl>) {
        self.context.boundaries[0] = self.context.boundaries[0].max(4. * ox_nucl.position.x.abs());
        self.context.boundaries[1] = self.context.boundaries[1].max(4. * ox_nucl.position.y.abs());
        self.context.boundaries[2] = self.context.boundaries[2].max(4. * ox_nucl.position.z.abs());

        self.previous_position = Some(ox_nucl.position);
        self.context.nucls.push(ox_nucl);

        let base = nucl
            .as_ref()
            .map(|nucl| self.context.basis_map.get_basis(&nucl, 'T'));

        let bound = OxDnaBound {
            base: base.unwrap_or(super::rand_base()),
            strand_id: self.strand_id,
            prime3: -1,
            prime5: self.prev_nucl.unwrap_or(-1),
        };
        self.context.bounds.push(bound);

        if let Some(prev) = self.prev_nucl {
            self.context.bounds.get_mut(prev as usize).unwrap().prime3 = self.context.nucl_id;
        }

        self.prev_nucl = Some(self.context.nucl_id);
        self.context.nucl_id += 1;
    }

    pub fn add_free_nucl(&mut self, position: Vec3, free_idx: usize) {
        let ox_nucl = free_oxdna_nucl(
            position,
            self.previous_position,
            free_idx,
            &self.context.parameters,
        );
        self.add_ox_nucl(ox_nucl, None)
    }

    // TODO move the strand maker in a wrapper to force the call to end when droping
    pub fn end(self, cyclic: bool) {
        if cyclic {
            self.context.bounds.iter_mut().last().unwrap().prime3 = self.first_strand_nucl;
            self.context
                .bounds
                .get_mut(self.first_strand_nucl as usize)
                .unwrap()
                .prime5 = self.context.nucl_id - 1;
        }
    }
}

pub(super) fn to_oxdna(design: &Design, basis_map: BasisMapper) -> (OxDnaConfig, OxDnaTopology) {
    let parameters = design.parameters.unwrap_or_default();
    let mut maker = OxDnaMaker::new(basis_map, parameters);

    for (strand_id, s) in design.strands.values().enumerate() {
        let mut strand_maker = maker.new_strand(strand_id);

        for d in s.domains.iter() {
            if let Domain::HelixDomain(dom) = d {
                for position in dom.iter() {
                    let ox_nucl = design.helices.get(&dom.helix).unwrap().ox_dna_nucl(
                        position,
                        dom.forward,
                        &parameters,
                    );
                    let nucl = Nucl {
                        position,
                        helix: dom.helix,
                        forward: dom.forward,
                    };
                    strand_maker.add_ox_nucl(ox_nucl, Some(nucl));
                }
            } else if let Domain::Insertion {
                instanciation: Some(instanciation),
                ..
            } = d
            {
                for (dom_position, space_position) in instanciation.pos().iter().enumerate() {
                    strand_maker.add_free_nucl(*space_position, dom_position);
                }
            }
        }
        // TODO: encapsulate this call to manually drop
        ManuallyDrop::into_inner(strand_maker).end(s.cyclic);
    }

    maker.end()
}
