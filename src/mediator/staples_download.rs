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

//! This modules defines the `download_staples` method as well as the error type returned
//! by this function

use super::*;

impl Mediator {
    #[must_use]
    pub fn download_stapples(&self) -> Result<DownloadStappleOk, DownloadStappleError> {
        let mut warnings = Vec::new();
        let d_id = if let Some(d_id) = self.selected_design() {
            d_id as usize
        } else {
            if self.designs.len() > 1 {
                return Err(DownloadStappleError::SeveralDesignNoneSelected);
            }
            0
        };
        if !self.designs[d_id].read().unwrap().scaffold_is_set() {
            return Err(DownloadStappleError::NoScaffoldSet);
        }
        if !self.designs[d_id].read().unwrap().scaffold_sequence_set() {
            return Err(DownloadStappleError::ScaffoldSequenceNotSet);
        }
        if let Some(nucl) = self.designs[d_id].read().unwrap().get_stapple_mismatch() {
            warnings.push(Warning::AllStaplesNotPaired {
                first_unpaired: nucl,
            });
            //message(msg.into(), rfd::MessageLevel::Warning);
        }

        let scaffold_length = self.designs[d_id]
            .read()
            .unwrap()
            .get_scaffold_len()
            .unwrap();
        let sequence_length = self.designs[d_id]
            .read()
            .unwrap()
            .get_scaffold_sequence_len()
            .unwrap();
        if scaffold_length != sequence_length {
            warnings.push(Warning::SacaffoldSequenceLengthMissmatch {
                scaffold_length,
                sequence_length,
            });
        }
        Ok(DownloadStappleOk {
            design_id: d_id,
            warnings,
        })
    }
}

pub enum DownloadStappleError {
    /// There are several designs and none is selected.
    SeveralDesignNoneSelected,
    /// No strand is set as the scaffold
    NoScaffoldSet,
    /// There is no sequence set for the scaffold
    ScaffoldSequenceNotSet,
}

pub struct DownloadStappleOk {
    pub design_id: usize,
    pub warnings: Vec<Warning>,
}

impl Mediator {
    #[must_use]
    /// Set the sequence of the scaffold strand. Return true if a scaffold strand is set, in which
    /// case the user should be proposed to optimize the scaffold starting position
    pub fn set_scaffold_sequence(
        &mut self,
        sequence: String,
    ) -> Result<SetScaffoldSequenceOk, SetScaffoldSequenceError> {
        let d_id = if let Some(d_id) = self.selected_design() {
            d_id as usize
        } else {
            if self.designs.len() > 1 {
                /*
                message(
                    "No design selected, setting sequence for design 0".into(),
                    rfd::MessageLevel::Warning,
                );
                */
                return Err(SetScaffoldSequenceError::SeveralDesignNoneSelected);
            }
            0
        };
        self.designs[d_id]
            .write()
            .unwrap()
            .set_scaffold_sequence(sequence);
        let default_shift = self.designs[d_id]
            .read()
            .unwrap()
            .get_scaffold_info()
            .and_then(|info| info.shift);
        /*
        let message = format!("Optimize the scaffold position ?\n
        If you chose \"Yes\", ENSnano will position the scaffold in a way that minimizes the number of anti-patern (G^4, C^4 (A|T)^7) in the stapples sequence. If you chose \"No\", the scaffold sequence will begin at position {}", shift);
        yes_no_dialog(
            message.into(),
            requests.clone(),
            KeepProceed::OptimizeShift(d_id as usize),
            None,
        )*/
        Ok(SetScaffoldSequenceOk {
            design_id: d_id,
            default_shift,
        })
    }
}

pub struct SetScaffoldSequenceOk {
    pub default_shift: Option<usize>,
    pub design_id: usize,
}

#[derive(Debug)]
pub enum SetScaffoldSequenceError {
    SeveralDesignNoneSelected,
}
