//! This modules defines the method that return the torsions in a design

use super::roller::cross_over_force;
use super::*;

type Xover = (Nucl, Nucl);
impl Data {
    pub fn get_torsions(&self) -> HashMap<Xover, Torsion> {
        let mut torsions: HashMap<Xover, Torsion> = HashMap::new();
        let helices: BTreeMap<usize, Helix> = self.design.helices.clone();
        let xovers = self.design.get_xovers();
        let parameters = self.design.parameters.unwrap_or_default();
        for xover in xovers.into_iter() {
            let torsion = xover_torsion(&helices, xover.0, xover.1, &parameters);
            let mut insert = true;
            for (candidate, torsion_friend) in torsions.iter_mut() {
                if torsion_friend.friend.is_some() {
                    continue;
                }
                if let Some(b) = are_friends(xover, *candidate) {
                    insert = false;
                    if b {
                        torsion_friend.strength_0 += torsion.strength_0;
                        torsion_friend.strength_1 += torsion.strength_1;
                        torsion_friend.friend = Some(xover);
                    } else {
                        torsion_friend.strength_0 += torsion.strength_1;
                        torsion_friend.strength_1 += torsion.strength_0;
                        torsion_friend.friend = Some((xover.1, xover.0));
                    }
                    break;
                }
            }
            if insert {
                torsions.insert(xover, torsion);
            }
        }
        torsions
    }
}

pub struct Torsion {
    pub strength_0: f32,
    pub strength_1: f32,
    pub friend: Option<Xover>,
}

fn xover_torsion(
    helices: &BTreeMap<usize, Helix>,
    source: Nucl,
    target: Nucl,
    parameters: &Parameters,
) -> Torsion {
    let strength = cross_over_force(
        &helices[&source.helix],
        &helices[&target.helix],
        parameters,
        source.position,
        source.forward,
        target.position,
        target.forward,
    );
    Torsion {
        strength_0: strength.0,
        strength_1: strength.1,
        friend: None,
    }
}

fn are_friends(xover1: Xover, xover2: Xover) -> Option<bool> {
    if xover1.0.is_neighbour(&xover2.0) && xover1.1.is_neighbour(&xover2.1) {
        Some(true)
    } else if xover1.1.is_neighbour(&xover2.0) && xover1.0.is_neighbour(&xover2.1) {
        Some(false)
    } else {
        None
    }
}
