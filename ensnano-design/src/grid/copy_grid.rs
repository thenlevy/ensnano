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

use crate::*;

impl Design {
    pub fn copy_grid(
        &mut self,
        grid_id: usize,
        position: Vec3,
        orientation: Rotor3,
    ) -> Result<(), GridCopyError> {
        let source_grid = self
            .grids
            .get(grid_id)
            .ok_or(GridCopyError::GridDoesNotExist(grid_id))?;

        let mut new_grids = Vec::clone(self.grids.as_ref());

        let new_grid = GridDescriptor {
            position,
            orientation,
            grid_type: source_grid.grid_type,
            invisible: false,
        };

        let new_grid_id = new_grids.len();
        new_grids.push(new_grid);

        let mut new_helices = BTreeMap::clone(self.helices.as_ref());
        let mut new_strands = self.strands.clone();
        let new_helix_map = self.make_grid_copy_helix_map(grid_id);
        let source_strand_ids = self.get_id_of_strands_on_grid(grid_id);

        let mut new_strand_id = new_strands.keys().max().map(|n| *n + 1).unwrap_or(0);

        for s_id in source_strand_ids.into_iter() {
            let strand = self.copy_strand_on_new_grid(s_id, &new_helix_map)?;
            new_strands.insert(new_strand_id, strand);
            new_strand_id += 1;
        }

        for (old_h_id, new_h_id) in new_helix_map.iter() {
            let old_helix = new_helices
                .get(old_h_id)
                .ok_or(GridCopyError::HelixDoesNotExist(*old_h_id))?;
            let grid_position = old_helix.grid_position.map(|gp| GridPosition {
                grid: new_grid_id,
                ..gp.clone()
            });
            if grid_position.is_none() {
                return Err(GridCopyError::HelixNotOnGrid(*old_h_id));
            }
            let new_helix = Arc::new(Helix {
                position: Vec3::zero(),
                orientation: Rotor3::identity(),
                roll: old_helix.roll,
                visible: true,
                locked_for_simulations: false,
                grid_position,
                isometry2d: None,
            });
            new_helices.insert(*new_h_id, new_helix);
        }

        self.strands = new_strands;
        self.helices = Arc::new(new_helices);
        self.grids = Arc::new(new_grids);
        Ok(())
    }

    fn make_grid_copy_helix_map(&self, grid_id: usize) -> HashMap<usize, usize> {
        let mut new_helix_id = self.helices.keys().max().map(|n| n + 1).unwrap_or(0);

        let mut ret = HashMap::new();

        for (h_id, h) in self.helices.iter() {
            if matches!(h.grid_position, Some(gp) if gp.grid == grid_id) {
                ret.insert(*h_id, new_helix_id);
                new_helix_id += 1;
            }
        }
        ret
    }

    fn get_id_of_strands_on_grid(&self, grid_id: usize) -> Vec<usize> {
        let mut ret = Vec::new();
        for (s_id, s) in self.strands.iter() {
            let mut insert = true;
            for d in s.domains.iter() {
                if matches!(d, Domain::HelixDomain(HelixInterval { helix, .. }) if !self.helix_is_on_grid(*helix, grid_id))
                {
                    insert = false;
                    break;
                }
            }
            if insert {
                ret.push(*s_id)
            }
        }
        ret
    }

    fn helix_is_on_grid(&self, h_id: usize, grid_id: usize) -> bool {
        self.helices
            .get(&h_id)
            .and_then(|h| h.grid_position)
            .filter(|gp| gp.grid == grid_id)
            .is_some()
    }

    fn copy_strand_on_new_grid(
        &self,
        s_id: usize,
        new_helices_map: &HashMap<usize, usize>,
    ) -> Result<Strand, GridCopyError> {
        let mut new_strand_domains = Vec::new();
        let source_strand = self
            .strands
            .get(&s_id)
            .ok_or(GridCopyError::StrandDoesNotExist(s_id))?;
        for d in source_strand.domains.iter() {
            match d {
                Domain::Insertion(n) => new_strand_domains.push(Domain::Insertion(*n)),
                Domain::HelixDomain(HelixInterval {
                    helix,
                    start,
                    sequence,
                    end,
                    forward,
                }) => {
                    let new_domain_helix = new_helices_map
                        .get(helix)
                        .cloned()
                        .ok_or(GridCopyError::HelixIdNotInNewHelixMap(*helix))?;
                    let new_domain = Domain::HelixDomain(HelixInterval {
                        helix: new_domain_helix,
                        start: *start,
                        sequence: sequence.clone(),
                        end: *end,
                        forward: *forward,
                    });
                    new_strand_domains.push(new_domain);
                }
            }
        }

        let mut new_junctions = Vec::new();
        for j in source_strand.junctions.iter() {
            match j {
                DomainJunction::Prime3 => new_junctions.push(DomainJunction::Prime3),
                DomainJunction::IdentifiedXover(_) | DomainJunction::UnindentifiedXover => {
                    new_junctions.push(DomainJunction::UnindentifiedXover)
                }
                DomainJunction::Adjacent => new_junctions.push(DomainJunction::Adjacent),
            }
        }

        Ok(Strand {
            domains: new_strand_domains,
            sequence: source_strand.sequence.clone(),
            color: source_strand.color,
            junctions: new_junctions,
            cyclic: source_strand.cyclic,
            name: source_strand
                .name
                .as_ref()
                .map(|n| Cow::from(format!("{}_copy", n))),
        })
    }
}

#[derive(Debug)]
pub enum GridCopyError {
    GridDoesNotExist(usize),
    StrandDoesNotExist(usize),
    HelixIdNotInNewHelixMap(usize),
    HelixDoesNotExist(usize),
    HelixNotOnGrid(usize),
}
