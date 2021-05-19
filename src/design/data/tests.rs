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

impl Data {
    pub(super) fn test_named_junction(&self, fail_msg: &'static str) {
        let mut xover_cpy = self.xover_ids.clone();
        for s in self.design.strands.values() {
            let mut expected_prime5: Option<Nucl> = None;
            let mut expected_prime5_domain: Option<usize> = None;
            let nb_taken = if s.cyclic {
                2 * s.domains.len()
            } else {
                s.domains.len()
            };
            for (i, d) in s.domains.iter().enumerate().cycle().take(nb_taken) {
                if let Some(prime3) = d.prime5_end() {
                    if let Some(prime5) = expected_prime5 {
                        if prime5.prime3() == prime3 {
                            // Expect adjacent
                            if s.junctions[expected_prime5_domain.unwrap()]
                                != DomainJunction::Adjacent
                            {
                                panic!(
                                    "In test{} \n
                                    Expected junction {:?}, got {:?}\n
                                    junctions are {:?}",
                                    fail_msg,
                                    DomainJunction::Adjacent,
                                    s.junctions[expected_prime5_domain.unwrap()],
                                    s.junctions,
                                );
                            }
                        } else {
                            // Expect named xover
                            if let Some(id) = self.xover_ids.get_id(&(prime5, prime3)) {
                                xover_cpy.remove(id);
                                if s.junctions[expected_prime5_domain.unwrap()]
                                    != DomainJunction::IdentifiedXover(id)
                                {
                                    panic!(
                                        "In test{} \n
                                    Expected junction {:?}, got {:?}\n
                                    junctions are {:?}",
                                        fail_msg,
                                        DomainJunction::IdentifiedXover(id),
                                        s.junctions[expected_prime5_domain.unwrap()],
                                        s.junctions,
                                    );
                                }
                            } else {
                                panic!(
                                    "In test{} \n
                                    Could not find xover in xover_ids {:?}
                                    xover_ids: {:?}",
                                    fail_msg,
                                    (prime5, prime3),
                                    self.xover_ids.get_all_elements(),
                                );
                            }
                        }
                        if expected_prime5_domain.unwrap() >= i {
                            break;
                        }
                    }
                }
                if let Some(nucl) = d.prime3_end() {
                    expected_prime5 = Some(nucl);
                }
                expected_prime5_domain = Some(i);
            }
        }
        assert!(
            xover_cpy.is_empty(),
            "In test {}\n
        Remaining xovers {:?}",
            fail_msg,
            xover_cpy.get_all_elements()
        );
    }
}
