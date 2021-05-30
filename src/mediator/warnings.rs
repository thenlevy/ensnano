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

pub enum Warning {
    /// Some staples are not completely paired. Can be emitted when trying to download staples.
    AllStaplesNotPaired {
        first_unpaired: Nucl,
    },
    /// The length of the scaffold and its sequence do not match. Can be emitted when trying to
    /// download staples
    SacaffoldSequenceLengthMissmatch {
        scaffold_length: usize,
        sequence_length: usize,
    },
    LoadedDesignWithInsertions,
}

impl Warning {
    pub fn dialog(&self) -> String {
        match self {
            Self::AllStaplesNotPaired { first_unpaired } => {
                all_staples_not_paired_dialog(first_unpaired)
            }
            Self::SacaffoldSequenceLengthMissmatch {
                scaffold_length,
                sequence_length,
            } => scaffold_sequence_length_missmatch(sequence_length, sequence_length),
            Self::LoadedDesignWithInsertions => loaded_design_with_insertions(),
        }
    }
}

fn all_staples_not_paired_dialog(first_unpaired: &Nucl) -> String {
    format!(
        "All stapples are not paired \n
         first unpaired nucleotide {:?}",
        first_unpaired
    )
}

fn scaffold_sequence_length_missmatch(scaffold_length: &usize, sequence_length: &usize) -> String {
    format!(
        "The scaffod length does not match its sequence\n
         Length of the scaffold {}\n
         Length of the sequence {}",
        scaffold_length, sequence_length
    )
}

fn loaded_design_with_insertions() -> String {
    "Your design contains insertions and/or deletions. These are not very well \
     handled by ENSnano at the moment and the current solution is to replace them by single \
     strands on helices specially created for that puropse."
        .into()
}
