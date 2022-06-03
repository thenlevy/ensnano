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
use crate::strands::*;
use std::borrow::Cow;

impl Design {
    pub fn copy_grids(
        &mut self,
        grid_ids: &[FreeGridId],
        position: Vec3,
        orientation: Rotor3,
    ) -> Result<(), GridCopyError> {
        let (base_position, base_orientation) = {
            let grid_id_0 = grid_ids.get(0).ok_or(GridCopyError::NoGridToCopy)?;
            let source_grid = self
                .free_grids
                .get(grid_id_0)
                .ok_or(GridCopyError::GridDoesNotExist(*grid_id_0))?;
            (
                source_grid.position - position,
                orientation.reversed() * source_grid.orientation,
            )
        };

        let mut grids_clone = FreeGrids::clone(&self.free_grids);
        let mut new_grids = grids_clone.make_mut();

        let new_grid_ids = self.make_grid_copy_grid_map(
            &grid_ids,
            base_position,
            base_orientation,
            &mut new_grids,
        )?;

        let new_helix_map = self.make_grid_copy_helix_map(&grid_ids);
        let source_strand_ids = self.get_id_of_strands_on_grids(&grid_ids);

        for s_id in source_strand_ids.into_iter() {
            let strand = self.copy_strand_on_new_grids(s_id, &new_helix_map)?;
            self.strands.push(strand);
        }

        for (old_h_id, new_h_id) in new_helix_map.iter() {
            let old_helix = self
                .helices
                .get(old_h_id)
                .ok_or(GridCopyError::HelixDoesNotExist(*old_h_id))?;
            let grid_position = old_helix.grid_position.and_then(|gp| {
                let new_grid_id = if let GridId::FreeGrid(id) = gp.grid {
                    new_grid_ids.get(&FreeGridId(id)).cloned()
                } else {
                    None
                }?;
                Some(HelixGridPosition {
                    grid: new_grid_id,
                    ..gp.clone()
                })
            });
            if grid_position.is_none() {
                return Err(GridCopyError::HelixNotOnGrid(*old_h_id));
            }
            let mut new_helix = Helix::new(Vec3::zero(), Rotor3::identity());
            new_helix.grid_position = grid_position;
            new_helix.symmetry = old_helix.symmetry;
            let mut helices_mut = self.helices.make_mut();
            helices_mut.insert(*new_h_id, new_helix);
        }

        drop(new_grids);
        self.free_grids = grids_clone;
        Ok(())
    }

    fn make_grid_copy_grid_map(
        &self,
        grid_ids: &[FreeGridId],
        base_position: Vec3,
        base_orientation: Rotor3,
        new_grids: &mut FreeGridsMut,
    ) -> Result<HashMap<FreeGridId, GridId>, GridCopyError> {
        let mut ret = HashMap::new();
        for grid_id in grid_ids.iter() {
            let source_grid = self
                .free_grids
                .get(grid_id)
                .ok_or(GridCopyError::GridDoesNotExist(*grid_id))?;

            let new_grid = GridDescriptor {
                position: source_grid.position - base_position,
                orientation: base_orientation.reversed() * source_grid.orientation,
                grid_type: source_grid.grid_type,
                invisible: false,
                bezier_vertex: None,
            };

            let new_grid_id = new_grids.push(new_grid);
            ret.insert(*grid_id, new_grid_id);
        }
        Ok(ret)
    }

    fn make_grid_copy_helix_map(&self, grid_ids: &[FreeGridId]) -> HashMap<usize, usize> {
        let mut new_helix_id = self.helices.keys().max().map(|n| n + 1).unwrap_or(0);

        let mut ret = HashMap::new();

        for (h_id, h) in self.helices.iter() {
            if let Some(grid_position) = h.grid_position {
                if matches!(grid_position.grid, GridId::FreeGrid(g_id) if grid_ids.contains(&FreeGridId(g_id)))
                {
                    ret.insert(*h_id, new_helix_id);
                    new_helix_id += 1;
                }
            }
        }
        ret
    }

    fn get_id_of_strands_on_grids(&self, grid_ids: &[FreeGridId]) -> Vec<usize> {
        let mut ret = Vec::new();
        for (s_id, s) in self.strands.iter() {
            let mut insert = true;
            for d in s.domains.iter() {
                if matches!(d, Domain::HelixDomain(HelixInterval { helix, .. }) if !self.helix_is_on_grids(*helix, grid_ids))
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

    fn helix_is_on_grids(&self, h_id: usize, grid_ids: &[FreeGridId]) -> bool {
        self.helices
            .get(&h_id)
            .and_then(|h| h.grid_position)
            .filter(|gp| matches!(gp.grid, GridId::FreeGrid(g_id) if grid_ids.contains(&FreeGridId(g_id))))
            .is_some()
    }

    fn copy_strand_on_new_grids(
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
                Domain::Insertion {
                    nb_nucl,
                    sequence,
                    attached_to_prime3,
                    ..
                } => new_strand_domains.push(Domain::Insertion {
                    nb_nucl: *nb_nucl,
                    instanciation: None,
                    sequence: sequence.clone(),
                    attached_to_prime3: *attached_to_prime3,
                }),
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
    GridDoesNotExist(FreeGridId),
    StrandDoesNotExist(usize),
    HelixIdNotInNewHelixMap(usize),
    GridNotInMap(usize),
    HelixDoesNotExist(usize),
    HelixNotOnGrid(usize),
    NoGridToCopy,
}
