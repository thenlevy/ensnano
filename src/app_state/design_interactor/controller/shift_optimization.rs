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

macro_rules! log_err {
    ($x:expr) => {
        if $x.is_err() {
            log::error!("Unexpected error")
        }
    };
}

use crate::app_state::design_interactor::presenter::NuclCollection;

use super::*;
use std::sync::mpsc;

fn read_scaffold_seq(
    design: &Design,
    nucl_collection: &dyn NuclCollection,
    shift: usize,
) -> Result<BTreeMap<Nucl, char>, ErrOperation> {
    let nb_skip = if let Some(sequence) = design.scaffold_sequence.as_ref() {
        if sequence.len() == 0 {
            return Err(ErrOperation::EmptyScaffoldSequence);
        }
        sequence.len() - (shift % sequence.len())
    } else {
        return Err(ErrOperation::EmptyScaffoldSequence);
    };
    if let Some(mut sequence) = design
        .scaffold_sequence
        .as_ref()
        .map(|s| s.chars().cycle().skip(nb_skip))
    {
        let mut basis_map = BTreeMap::new();
        let s_id = design.scaffold_id.ok_or(ErrOperation::NoScaffoldSet)?;
        let strand = design
            .strands
            .get(&s_id)
            .ok_or(ErrOperation::StrandDoesNotExist(s_id))?;
        for domain in &strand.domains {
            if let Domain::HelixDomain(dom) = domain {
                for nucl_position in dom.iter() {
                    let nucl = Nucl {
                        helix: dom.helix,
                        position: nucl_position,
                        forward: dom.forward,
                    };
                    let basis = sequence.next();
                    let basis_compl = compl(basis);
                    if let Some(virtual_compl) =
                        Nucl::map_to_virtual_nucl(nucl.compl(), &design.helices)
                    {
                        if let Some((basis, basis_compl)) = basis.zip(basis_compl) {
                            basis_map.insert(nucl, basis);
                            if let Some(real_compl) =
                                nucl_collection.virtual_to_real(&virtual_compl)
                            {
                                basis_map.insert(*real_compl, basis_compl);
                            }
                        }
                    } else {
                        log::error!("Could not get virtual mapping of {:?}", nucl.compl());
                    }
                }
            } else if let Domain::Insertion { nb_nucl, .. } = domain {
                for _ in 0..*nb_nucl {
                    sequence.next();
                }
            }
        }
        Ok(basis_map)
    } else {
        Err(ErrOperation::EmptyScaffoldSequence)
    }
}

/// Shift the scaffold at an optimized poisition and return the corresponding score
pub fn optimize_shift<Nc: NuclCollection>(
    design: Arc<Design>,
    nucl_collection: Arc<Nc>,
    chanel_reader: &mut dyn ShiftOptimizerReader,
) {
    let (progress_snd, progress_rcv) = std::sync::mpsc::channel();
    let (result_snd, result_rcv) = std::sync::mpsc::channel();
    chanel_reader.attach_result_chanel(result_rcv);
    chanel_reader.attach_progress_chanel(progress_rcv);
    std::thread::spawn(move || {
        let result =
            get_shift_optimization_result(design.as_ref(), progress_snd, nucl_collection.as_ref());
        log_err!(result_snd.send(result));
    });
}

fn get_shift_optimization_result(
    design: &Design,
    progress_channel: std::sync::mpsc::Sender<f32>,
    nucl_collection: &dyn NuclCollection,
) -> ShiftOptimizationResult {
    let mut best_score = usize::MAX;
    let mut best_shfit = 0;
    let mut best_result = String::new();
    let len = design
        .scaffold_sequence
        .as_ref()
        .map(|s| s.len())
        .ok_or(ErrOperation::NoScaffoldSet)?;
    for shift in 0..len {
        if shift % 100 == 0 {
            log_err!(progress_channel.send(shift as f32 / len as f32))
        }
        let char_map = read_scaffold_seq(design, nucl_collection, shift)?;
        let (score, result) = evaluate_shift(design, &char_map);
        if score < best_score {
            println!("shift {} score {}", shift, score);
            best_score = score;
            best_shfit = shift;
            best_result = result;
        }
        if score == 0 {
            break;
        }
    }
    Ok(ShiftOptimizationOk {
        position: best_shfit,
        score: best_result,
    })
}
/// Evaluate a scaffold position. The score of the position is given by
/// score = nb((A|T)^7) + 10 nb(G^4 | C ^4) + 100 nb (G^5 | C^5) + 1000 nb (G^6 | C^6)
fn evaluate_shift(design: &Design, basis_map: &BTreeMap<Nucl, char>) -> (usize, String) {
    use std::fmt::Write;
    let mut ret = 0;
    let mut shown = false;
    let bad = regex::Regex::new(r"[AT]{7,}?").unwrap();
    let verybad = regex::Regex::new(r"G{4,}?|C{4,}?").unwrap();
    let ultimatelybad = regex::Regex::new(r"G{5,}|C{5,}").unwrap();
    let ultimatelybad2 = regex::Regex::new(r"G{6,}|C{6,}").unwrap();
    for (s_id, strand) in design.strands.iter() {
        if strand.length() == 0 || design.scaffold_id == Some(*s_id) {
            continue;
        }
        let mut sequence = String::with_capacity(10000);
        for domain in &strand.domains {
            if let Domain::HelixDomain(dom) = domain {
                for position in dom.iter() {
                    let nucl = Nucl {
                        position,
                        forward: dom.forward,
                        helix: dom.helix,
                    };
                    sequence.push(*basis_map.get(&nucl).unwrap_or(&'?'));
                }
            }
        }
        let mut matches = bad.find_iter(&sequence);
        while matches.next().is_some() {
            if !shown {
                shown = true;
            }
            ret += 1;
        }
        let mut matches = verybad.find_iter(&sequence);
        while matches.next().is_some() {
            if !shown {
                shown = true;
            }
            ret += 100;
        }
        let mut matches = ultimatelybad.find_iter(&sequence);
        while matches.next().is_some() {
            if !shown {
                shown = true;
            }
            ret += 10_000;
        }
        let mut matches = ultimatelybad2.find_iter(&sequence);
        while matches.next().is_some() {
            if !shown {
                shown = true;
            }
            ret += 1_000_000;
        }
    }
    let result = if ret == 0 {
        "No bad pattern".to_owned()
    } else {
        let mut result = String::new();
        if ret >= 1_000_000 {
            writeln!(&mut result, "{} times G^6 or C^6", ret / 1_000_000).unwrap();
        }
        if (ret % 1_000_000) >= 10_000 {
            writeln!(
                &mut result,
                "{} times G^5 or C^5",
                (ret % 1_000_000) / 10_000
            )
            .unwrap();
        }
        if (ret % 10_000) >= 100 {
            writeln!(&mut result, "{} times G^4 or C^4", (ret % 10_000) / 100).unwrap();
        }
        if ret % 100 > 0 {
            writeln!(&mut result, "{} times (A or T)^7", (ret % 100)).unwrap();
        }
        result
    };
    log::debug!("ret {}, {}", ret, result);
    (ret, result)
}

fn compl(c: Option<char>) -> Option<char> {
    match c {
        Some('T') => Some('A'),
        Some('A') => Some('T'),
        Some('G') => Some('C'),
        Some('C') => Some('G'),
        _ => None,
    }
}

pub struct ShiftOptimizationOk {
    pub position: usize,
    pub score: String,
}

pub type ShiftOptimizationResult = Result<ShiftOptimizationOk, ErrOperation>;

pub trait ShiftOptimizerReader: Send {
    fn attach_progress_chanel(&mut self, chanel: mpsc::Receiver<f32>);
    fn attach_result_chanel(&mut self, chanel: mpsc::Receiver<ShiftOptimizationResult>);
}
