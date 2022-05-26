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
use ensnano_design::Domain;
use ensnano_exports::oxdna::*;
use std::mem::ManuallyDrop;

impl Presenter {
    fn to_oxdna(&self) -> (OxDnaConfig, OxDnaTopology) {
        let basis_map = (*self.content.basis_map.clone()).clone();
        let parameters = self.current_design.parameters.unwrap_or_default();
        let mut maker = OxDnaMaker::new(basis_map, parameters);

        for (strand_id, s) in self.current_design.strands.values().enumerate() {
            let mut strand_maker = maker.new_strand(strand_id);

            for d in s.domains.iter() {
                if let Domain::HelixDomain(dom) = d {
                    for position in dom.iter() {
                        let ox_nucl = self
                            .current_design
                            .helices
                            .get(&dom.helix)
                            .unwrap()
                            .ox_dna_nucl(position, dom.forward, &parameters);
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

    pub fn oxdna_export(&self, directory: &PathBuf) -> std::io::Result<(PathBuf, PathBuf)> {
        let mut config_name = directory.clone();
        config_name.push("export.oxdna");
        let mut topology_name = directory.clone();
        topology_name.push("export.top");
        let (config, topo) = self.to_oxdna();
        config.write(config_name.clone())?;
        topo.write(topology_name.clone())?;
        Ok((config_name, topology_name))
    }

    pub fn cando_export(
        &self,
        out_path: &PathBuf,
    ) -> Result<(), ensnano_exports::cando::CanDoError> {
        use ensnano_exports::cando;

        let mut exporter = cando::CanDoFormater::new();
        let parameters = self.current_design.parameters.unwrap_or_default();

        for s in self.current_design.strands.values() {
            let mut cando_strand = exporter.add_strand();

            for d in s.domains.iter() {
                if let Domain::HelixDomain(dom) = d {
                    for position in dom.iter() {
                        let ox_nucl = self
                            .current_design
                            .helices
                            .get(&dom.helix)
                            .unwrap()
                            .ox_dna_nucl(position, dom.forward, &parameters);
                        let nucl = Nucl {
                            position,
                            helix: dom.helix,
                            forward: dom.forward,
                        };

                        let base = self.content.basis_map.get(&nucl).cloned();
                        //let base = if dom.forward { 'C' } else { 'G'};
                        let sign = if nucl.forward { 1. } else { -1. };
                        cando_strand.add_nucl(
                            nucl,
                            ox_nucl.position,
                            sign * ox_nucl.normal,
                            base,
                        )?;
                    }
                }
            }
            cando_strand.end(s.cyclic)?;
        }
        exporter
            .write_to(out_path)
            .map_err(|e| cando::CanDoError::IOError(e))
    }

    pub fn pdb_export(&self, out_path: &PathBuf) -> Result<(), ensnano_exports::pdb::PdbError> {
        use ensnano_exports::pdb;
        let mut exporter = pdb::PdbFormatter::new(out_path)?;
        let parameters = self.current_design.parameters.unwrap_or_default();

        for s in self.current_design.strands.values() {
            let mut pdb_strand = exporter.start_strand(s.cyclic);

            for d in s.domains.iter() {
                if let Domain::HelixDomain(dom) = d {
                    for position in dom.iter() {
                        let ox_nucl = self
                            .current_design
                            .helices
                            .get(&dom.helix)
                            .unwrap()
                            .ox_dna_nucl(position, dom.forward, &parameters);
                        let nucl = Nucl {
                            position,
                            helix: dom.helix,
                            forward: dom.forward,
                        };

                        let base = self
                            .content
                            .basis_map
                            .get(&nucl)
                            .cloned()
                            .unwrap_or(if dom.forward { 'A' } else { 'T' });
                        //let base = if dom.forward { 'C' } else { 'G'};
                        pdb_strand.add_nucl(base, ox_nucl.position * 10., ox_nucl.get_basis())?;
                    }
                }
            }
            pdb_strand.write()?;
        }

        Ok(())
    }
}
