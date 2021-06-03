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

/// This modules defines the method used to replace insertions by helices with single strands.
use super::*;

impl Data {
    pub fn replace_all_insertions(&mut self) {
        let parameters = self.design.parameters.unwrap_or_default();
        let helices = &mut self.design.helices;
        for s in self.design.strands.values_mut() {
            replace_insertions_one_strand(s, helices, &parameters);
        }
        self.update_status = true;
        self.hash_maps_update = true;
    }
}

fn replace_insertions_one_strand(
    strand: &mut Strand,
    helices: &mut BTreeMap<usize, Helix>,
    parameters: &Parameters,
) {
    let insertion_points = strand.insertion_points();
    let mut insertion_point_iterator = insertion_points.iter();
    let mut must_replace = Vec::new();
    for (d_position, d) in strand.domains.iter_mut().enumerate() {
        if let Domain::Insertion(_) = d {
            let (n1, n2) = insertion_point_iterator
                .next()
                .expect("Not enough insertion points");
            if let Some(nucl) = n1 {
                let new_helix_idx = add_neighbour_helix(helices, nucl, parameters);
                transform_insertion_into_single_strand(d, new_helix_idx, nucl.forward)
            } else if let Some(nucl) = n2 {
                let new_helix_idx = add_neighbour_helix(helices, nucl, parameters);
                transform_insertion_into_single_strand(d, new_helix_idx, nucl.forward)
            } else {
                unreachable!("both insertion points are None")
            }
            must_replace.push(d_position);
        }
    }
    for d_position in must_replace {
        update_juctions_after_replace(strand, d_position);
    }
}

fn transform_insertion_into_single_strand(domain: &mut Domain, helix_idx: usize, forward: bool) {
    if let Domain::Insertion(n) = domain {
        let (start, end) = if forward {
            (0, *n as isize)
        } else {
            (*n as isize * -1 + 1, 1)
        };
        *domain = Domain::HelixDomain(HelixInterval {
            helix: helix_idx,
            forward,
            start,
            end,
            sequence: None,
        })
    }
}

fn add_neighbour_helix(
    helices: &mut BTreeMap<usize, Helix>,
    nucl: &Nucl,
    parameters: &Parameters,
) -> usize {
    let helix = helices
        .get(&nucl.helix)
        .expect("Got nucleotide of unexisting helix");
    let new_helix = helix.ideal_neighbour(nucl.position, nucl.forward, parameters);
    let index_new_helix = helices.keys().last().unwrap() + 1; //cannot be none since there is at least one helix
    helices.insert(index_new_helix, new_helix);
    index_new_helix
}

fn update_juctions_after_replace(strand: &mut Strand, d_position: usize) {
    if d_position == 0 {
        if strand.cyclic {
            panic!("Cyclic strand should not start with an insertion");
        }
        let first_junction = strand
            .junctions
            .first_mut()
            .expect("there should be at least one junction");
        if let DomainJunction::UnindentifiedXover = first_junction {
            *first_junction = DomainJunction::UnindentifiedXover;
        }
    } else {
        let juction_5prime = &mut strand.junctions[d_position - 1];
        if let DomainJunction::Adjacent = juction_5prime {
            *juction_5prime = DomainJunction::UnindentifiedXover;
        }
        let junction_3prime = &mut strand.junctions[d_position];
        if let DomainJunction::Adjacent = junction_3prime {
            *junction_3prime = DomainJunction::UnindentifiedXover;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A design with one strand containing 3 domains: H1: -1 -> 4 ; Insertion 5 ; H1: 5 -> 10 ;
    /// H2: 0 <- 10
    fn design_one_strand_insertion() -> Data {
        let path_str = format!(
            "{}/src/design/data/test_designs/one_strand_with_insertion.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let path = Path::new(path_str.as_str());
        Data::new_with_path(&path.into())
            .ok()
            .expect("Could parse file")
    }

    /// A design with one strand containing 3 domains: H1: -1 -> 4 ; Insertion 5 ; H1: 5 -> 10 ;
    /// Insertion 3 ; H2: 0 <- 10
    fn design_one_strand_insertion_before_xover() -> Data {
        let path_str = format!(
            "{}/src/design/data/test_designs/one_strand_with_insertion_before_xover.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let path = Path::new(path_str.as_str());
        Data::new_with_path(&path.into())
            .ok()
            .expect("Could parse file")
    }

    /// A design with a cyclic strand containing 5 domains: Insertion 2 ; H1: -1 -> 4 ; Insertion 5
    /// ; H1: 5 -> 10 ; H2: 0 <- 10 ; Insertion 6
    fn design_pathological_cyclic_strand() -> Data {
        let path_str = format!(
            "{}/src/design/data/test_designs/cyclic_strand_insertion_both_side.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let path = Path::new(path_str.as_str());
        Data::new_with_path(&path.into())
            .ok()
            .expect("Could parse file")
    }

    #[test]
    fn add_correct_number_of_helices_1() {
        let mut data = design_one_strand_insertion();
        let intial_nb_helices = data.design.helices.len();
        data.replace_all_insertions();
        let final_nb_helices = data.design.helices.len();
        assert_eq!(final_nb_helices - intial_nb_helices, 1);
    }

    #[test]
    fn add_correct_number_of_helices_2() {
        let mut data = design_one_strand_insertion_before_xover();
        let intial_nb_helices = data.design.helices.len();
        data.replace_all_insertions();
        let final_nb_helices = data.design.helices.len();
        assert_eq!(final_nb_helices - intial_nb_helices, 2);
    }

    #[test]
    fn add_correct_number_of_helices_2_cyclic_strand() {
        let mut data = design_pathological_cyclic_strand();
        let intial_nb_helices = data.design.helices.len();
        assert_eq!(data.design.strands.len(), 1);
        data.replace_all_insertions();
        let final_nb_helices = data.design.helices.len();
        assert_eq!(final_nb_helices - intial_nb_helices, 2);
    }

    #[test]
    fn preserve_length_1() {
        let mut data = design_one_strand_insertion();
        let intial_length = data.design.strands.values().next().unwrap().length();
        assert_eq!(data.design.strands.len(), 1);
        data.replace_all_insertions();
        let final_length = data.design.strands.values().next().unwrap().length();
        assert_eq!(intial_length, final_length);
    }

    #[test]
    fn preserve_length_2() {
        let mut data = design_one_strand_insertion_before_xover();
        let intial_length = data.design.strands.values().next().unwrap().length();
        assert_eq!(data.design.strands.len(), 1);
        data.replace_all_insertions();
        let final_length = data.design.strands.values().next().unwrap().length();
        assert_eq!(intial_length, final_length);
    }

    #[test]
    fn preserve_length_cyclic_strand() {
        let mut data = design_pathological_cyclic_strand();
        let intial_length = data.design.strands.values().next().unwrap().length();
        assert_eq!(data.design.strands.len(), 1);
        data.replace_all_insertions();
        let final_length = data.design.strands.values().next().unwrap().length();
        assert_eq!(intial_length, final_length);
    }

    #[test]
    fn replace_junctions_correctly() {
        let mut data = design_one_strand_insertion_before_xover();
        assert_eq!(data.design.strands.len(), 1);
        data.replace_all_insertions();
        let junctions = &data.design.strands.values().next().unwrap().junctions;
        assert_eq!(
            junctions,
            &vec![
                DomainJunction::UnindentifiedXover,
                DomainJunction::UnindentifiedXover,
                DomainJunction::UnindentifiedXover,
                DomainJunction::IdentifiedXover(0),
                DomainJunction::Prime3
            ]
        );
        data.make_hash_maps();
    }

    #[test]
    fn replace_junctions_correctly_cyclic() {
        let mut data = design_pathological_cyclic_strand();
        assert_eq!(data.design.strands.len(), 1);
        let junctions = &data.design.strands.values().next().unwrap().junctions;
        assert_eq!(
            junctions,
            &vec![
                DomainJunction::Adjacent,
                DomainJunction::Adjacent,
                DomainJunction::IdentifiedXover(0),
                DomainJunction::Adjacent,
                DomainJunction::IdentifiedXover(1),
            ]
        );
        println!("going to replace");
        data.replace_all_insertions();
        println!("{:?}", data.design.strands.values().next().unwrap().domains);
        let junctions = &data.design.strands.values().next().unwrap().junctions;
        assert_eq!(
            junctions,
            &vec![
                DomainJunction::UnindentifiedXover,
                DomainJunction::UnindentifiedXover,
                DomainJunction::IdentifiedXover(0),
                DomainJunction::UnindentifiedXover,
                DomainJunction::IdentifiedXover(1),
            ]
        );
        data.make_hash_maps();
    }
}
