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
}
