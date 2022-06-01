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

use ensnano_design::HelixCollection;

use super::*;

pub fn get_parity(design: &Design, max_helix_idx: usize) -> Result<Vec<bool>, CadnanoError> {
    let mut father = make_group(design, max_helix_idx);
    let graph = make_graph(design, max_helix_idx, &mut father)?;

    let color_first_helix = get_color_first_helix(design, &father);

    color_graph(&graph, max_helix_idx, &mut father, color_first_helix)
}

fn make_graph(
    design: &Design,
    max_helix_idx: usize,
    father: &mut Vec<usize>,
) -> Result<Vec<Vec<bool>>, CadnanoError> {
    let mut ret = vec![vec![false; max_helix_idx + 1]; max_helix_idx + 1];
    for s in design.strands.values() {
        let mut group_sens: Vec<usize> = Vec::new();
        let mut group_anti: Vec<usize> = Vec::new();

        for d in &s.domains {
            if let Domain::HelixDomain(d) = d {
                if d.forward {
                    group_sens.push(d.helix as usize);
                } else {
                    group_anti.push(d.helix as usize);
                }
            }
        }

        for i in group_sens.iter() {
            for j in group_anti.iter() {
                let repr_i = find(*i, father);
                let repr_j = find(*j, father);
                if repr_i == repr_j {
                    return Err(CadnanoError::Not2Colorable);
                }
                ret[repr_i][repr_j] = true;
                ret[repr_j][repr_i] = true;
            }
        }
    }
    Ok(ret)
}

fn get_color_first_helix(design: &Design, father: &[usize]) -> bool {
    for h_id in 0..father.len() {
        if father[h_id] == h_id {
            if let Some(grid_pos) = design
                .helices
                .get(&h_id)
                .and_then(|h| h.grid_position.as_ref())
            {
                return (grid_pos.x + grid_pos.y) % 2 == 0;
            }
        }
    }
    false
}

fn color_graph(
    graph: &Vec<Vec<bool>>,
    max_helix_idx: usize,
    father: &mut Vec<usize>,
    color_first_helix: bool,
) -> Result<Vec<bool>, CadnanoError> {
    let mut color = vec![color_first_helix; max_helix_idx + 1];
    let mut seen: Vec<bool> = (0..(max_helix_idx + 1)).map(|i| i != father[i]).collect();

    for i in 0..(max_helix_idx + 1) {
        if !seen[i] {
            seen[i] = true;
            let mut to_do: Vec<usize> = vec![i];
            while to_do.len() > 0 {
                let i = to_do.pop().unwrap();
                let i = find(i, father);
                for j in 0..(max_helix_idx + 1) {
                    let j = find(j, father);
                    if graph[i][j] && !seen[j] {
                        if seen[j] && color[j] == color[i] {
                            return Err(CadnanoError::Not2Colorable);
                        }
                        seen[j] = true;
                        color[j] = !color[i];
                        to_do.push(j);
                    }
                }
            }
        }
    }

    for i in 0..(max_helix_idx + 1) {
        let repr_i = find(i, father);
        color[i] = color[repr_i];
    }
    Ok(color)
}

fn make_group(design: &Design, max_helix_idx: usize) -> Vec<usize> {
    let mut father: Vec<usize> = (0..(max_helix_idx + 1)).map(|i| i).collect();
    let mut rank: Vec<usize> = vec![0; max_helix_idx + 1];
    for s in design.strands.values() {
        let mut group_sens: Vec<usize> = Vec::new();
        let mut group_anti: Vec<usize> = Vec::new();

        for d in &s.domains {
            if let Domain::HelixDomain(d) = d {
                if d.forward {
                    group_sens.push(d.helix as usize);
                } else {
                    group_anti.push(d.helix as usize);
                }
            }
        }
        for i in group_sens.iter() {
            for j in group_sens.iter() {
                union(*i, *j, &mut father, &mut rank);
            }
        }
        for i in group_anti.iter() {
            for j in group_anti.iter() {
                union(*i, *j, &mut father, &mut rank);
            }
        }
    }
    father
}

fn union(i: usize, j: usize, father: &mut Vec<usize>, rank: &mut Vec<usize>) {
    let i_root = find(i, father);
    let j_root = find(j, father);
    if i_root != j_root {
        if rank[i_root] < rank[j_root] {
            father[i_root] = j_root;
        } else {
            father[j_root] = i_root;
            if rank[j_root] == rank[i_root] {
                rank[i_root] += 1;
            }
        }
    }
}

fn find(i: usize, father: &mut Vec<usize>) -> usize {
    if father[i] != i {
        father[i] = find(father[i], father);
    }
    father[i]
}
