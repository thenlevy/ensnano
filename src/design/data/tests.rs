use super::*;

impl Data {
    pub(super) fn test_named_junction(&self, fail_msg: &'static str) {
        let mut xover_cpy = self.xover_ids.clone();
        for s in self.design.strands.values() {
            let mut expected_prime5: Option<Nucl> = None;
            let mut expected_prime5_domain: Option<usize> = None;
            for (i, d) in s.domains.iter().enumerate() {
                if let Some(prime3) = d.prime5_end() {
                    if let Some(prime5) = expected_prime5 {
                        if prime5.prime3() == prime3 {
                            // Expect adjacent
                            if s.junctions[expected_prime5_domain.unwrap()]
                                != DomainJunction::Adjacent
                            {
                                panic!(
                                    "In test{} \n
                                    Expected junction {:?}, got {:?}",
                                    fail_msg,
                                    DomainJunction::Adjacent,
                                    s.junctions[expected_prime5_domain.unwrap()]
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
                                    Expected junction {:?}, got {:?}",
                                        fail_msg,
                                        DomainJunction::IdentifiedXover(id),
                                        s.junctions[expected_prime5_domain.unwrap()]
                                    );
                                }
                            } else {
                                panic!(
                                    "In test{} \n
                                    Could not find xover in xover_ids {:?}",
                                    fail_msg,
                                    (prime5, prime3)
                                );
                            }
                        }
                    }
                }
                if let Some(nucl) = d.prime3_end() {
                    expected_prime5 = Some(nucl);
                    expected_prime5_domain = Some(i);
                }
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
