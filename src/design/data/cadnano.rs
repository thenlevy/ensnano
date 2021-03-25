use cadnano_format::{Cadnano, VStrand};
use std::collections::{HashSet, HashMap};
use std::path::Path;
use super::icednano::{Design, Strand, Domain, HelixInterval};

const NO_HELIX: usize = std::usize::MAX;

impl Design {
    /// Create a design from a cadnano file
    pub fn from_cadnano<P: AsRef<Path>>(file: P) -> Self {
        let nano = Cadnano::from_file(file).unwrap();
        let vstrands = nano.vstrands;
        let mut seen: HashSet<(usize, usize, bool)> = HashSet::new();
        let mut design = Design::new();
        let mut nb_strand = 0;

        let mut num_to_helix: HashMap<isize, usize> = HashMap::new();

        let honneycomb = vstrands[0].scaf.len() % 21 == 0;
        for (i, v) in vstrands.iter().enumerate() {
            num_to_helix.insert(v.num, i);
            if honneycomb {
                design.add_honneycomb_helix(v.row, v.col);
            } else {
                design.add_adjusted_grid_helix(v.row, v.col);
            }
        }
        num_to_helix.insert(-1, NO_HELIX);

        for scaf in vec![false, true] {
            for i in 0..vstrands.len() {
                let v = &vstrands[i];
                for j in 0..v.stap.len() {
                    let result = if scaf { v.scaf[j] } else { v.stap[j] };
                    if seen.insert((i, j, scaf)) && result != (-1, -1, -1, -1) {
                        println!("{}, {}, {}", scaf, i, j);
                        let end_5 = find_5_end(i, j, &vstrands, &num_to_helix, scaf);
                        println!("end: {:?}", end_5);
                        let strand = make_strand(end_5, &vstrands, &num_to_helix, &mut seen, scaf);
                        design.strands.insert(nb_strand, strand);
                        nb_strand += 1;
                    }
                }
            }
        }
        design
    }
}

fn find_5_end(
    i: usize,
    j: usize,
    vstrands: &Vec<VStrand>,
    num_to_helix: &HashMap<isize, usize>,
    scaf: bool,
) -> (usize, usize, bool) {
    let (mut a, mut b, mut c, mut d) = (i, j, 0, 0);
    let mut cyclic = false;
    while a != NO_HELIX {
        let result = if scaf {
            vstrands[a].scaf[b]
        } else {
            vstrands[a].stap[b]
        };
        c = a;
        d = b;
        a = num_to_helix[&result.0];
        b = result.1 as usize;

        if a == i && b == j {
            cyclic = true;
            a = NO_HELIX;
        }
    }
    (c, d, cyclic)
}

fn make_strand(
    end_5: (usize, usize, bool),
    vstrands: &Vec<VStrand>,
    num_to_helix: &HashMap<isize, usize>,
    seen: &mut HashSet<(usize, usize, bool)>,
    scaf: bool,
) -> Strand {
    println!("making strand {:?}", end_5);
    let cyclic = end_5.2;
    let (mut i, mut j) = (end_5.0, end_5.1);
    let mut ret = Strand {
        domains: Vec::new(),
        sequence: None,
        cyclic,
        color: 0,
    };

    let mut curent_dom = 0;
    while curent_dom == 0 || i != end_5.0 || j != end_5.1 {
        let curent_helix = i;
        let curent_5 = j;
        let mut curent_3 = j;
        let mut once = false;
        while i == curent_helix && (i != end_5.0 || j != end_5.1 || !once) {
            once = true;
            curent_3 = j;
            println!("nucl {}, {}", i, j);
            seen.insert((i, j, scaf));
            let result = if scaf {
                vstrands[i].scaf[j]
            } else {
                vstrands[i].stap[j]
            };
            println!("result {:?}", result);
            i = num_to_helix[&result.2];
            j = result.3 as usize;
        }
        println!("ready to build domain");
        let forward = curent_3 >= curent_5;
        let start = if forward {
            substract_skips(curent_5, curent_helix, vstrands)
        } else {
            substract_skips(curent_3, curent_helix, vstrands)
        };
        let end = if forward {
            substract_skips(curent_3, curent_helix, vstrands)
        } else {
            substract_skips(curent_5, curent_helix, vstrands)
        };

        println!("pushing {} {} {} {}", curent_helix, start, end, forward);
        ret.domains.push(Domain::HelixDomain(HelixInterval {
            helix: curent_helix,
            start,
            end: end + 1,
            forward,
            sequence: None,
        }));
        if i == NO_HELIX {
            break;
        }
        curent_dom += 1;
    }
    if cyclic {
        if let Domain::HelixDomain(dom0) = &ret.domains[0] { 
            if let Domain::HelixDomain(last_dom) = &ret.domains[ret.domains.len() - 1] {
                if last_dom.helix != dom0.helix {
                    let helix = dom0.helix;
                    let start = dom0.start;
                    let end = dom0.start + 1;
                    let forward = dom0.forward;
                    ret.domains.push(Domain::HelixDomain(HelixInterval {
                        helix,
                        start,
                        end,
                        forward,
                        sequence: None,
                    }));
                } else {
                    let len = ret.domains.len();
                    let forward = dom0.forward;
                    let start = dom0.start;
                    let end = dom0.end;
                    if let Domain::HelixDomain(mut last_dom) = &mut ret.domains[len - 1] {
                        if forward {
                            last_dom.end = start + 1;
                        } else {
                            last_dom.start = end - 1;
                        }
                    }
                }
            }
        }
    }
    ret
}

fn substract_skips(nucl: usize, helix: usize, vstrands: &Vec<VStrand>) -> isize {
    let skips: isize = (0..(nucl + 1))
        .map(|n| vstrands[helix].skip[n as usize])
        .sum();
    nucl as isize + skips
}
