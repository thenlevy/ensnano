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
use crate::controller::{DownloadStappleError, DownloadStappleOk, StaplesDownloader};
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;

impl StaplesDownloader for DesignReader {
    fn download_staples(&self) -> Result<DownloadStappleOk, DownloadStappleError> {
        let mut warnings = Vec::new();
        if self.presenter.current_design.scaffold_id.is_none() {
            return Err(DownloadStappleError::NoScaffoldSet);
        }
        if self.presenter.current_design.scaffold_sequence.is_none() {
            return Err(DownloadStappleError::ScaffoldSequenceNotSet);
        }

        if let Some(nucl) = self
            .presenter
            .content
            .get_stapple_mismatch(self.presenter.current_design.as_ref())
        {
            warnings.push(warn_all_staples_not_paired(nucl));
        }

        let scaffold_length = self
            .presenter
            .current_design
            .scaffold_id
            .as_ref()
            .and_then(|s_id| {
                self.presenter
                    .current_design
                    .strands
                    .get(s_id)
                    .map(|s| s.length())
            })
            .unwrap();
        let sequence_length = self
            .presenter
            .current_design
            .scaffold_sequence
            .as_ref()
            .map(|s| s.len())
            .unwrap();
        if scaffold_length != sequence_length {
            warnings.push(warn_scaffold_seq_mismatch(scaffold_length, sequence_length));
        }
        Ok(DownloadStappleOk { warnings })
    }

    fn write_staples_xlsx(&self, xlsx_path: &PathBuf) {
        use simple_excel_writer::{row, Row, Workbook};
        let stapples = self
            .presenter
            .content
            .get_staples(&self.presenter.current_design, &self.presenter);
        let mut wb = Workbook::create(xlsx_path.to_str().unwrap());
        let mut sheets = BTreeMap::new();

        let interval_strs: Vec<_> = stapples
            .iter()
            .map(|stapple| {
                if let Ok(s) = serde_json::to_string(&stapple.intervals.intervals) {
                    s
                } else {
                    String::from("error getting domains")
                }
            })
            .collect();
        for (i, stapple) in stapples.iter().enumerate() {
            let sheet = sheets.entry(stapple.plate).or_insert_with(|| {
                vec![vec![
                    "Well Position",
                    "Name",
                    "Sequence",
                    "Domains",
                    "Length",
                    "Domain Length",
                    "Groups",
                    "Color",
                ]]
            });
            sheet.push(vec![
                &stapple.well,
                &stapple.name,
                &stapple.sequence,
                &interval_strs[i],
                &stapple.length_str,
                &stapple.domain_decomposition,
                &stapple.groups_name_str,
                &stapple.color_str,
            ])
        }

        for (sheet_id, rows) in sheets.iter() {
            let mut sheet = wb.create_sheet(&format!("Plate {}", sheet_id));
            wb.write_sheet(&mut sheet, |sw| {
                for row in rows {
                    if let Ok(length) = row[3].parse::<f64>() {
                        sw.append_row(row![
                            row[0], row[1], row[2], length, row[4], row[5], row[6]
                        ])?;
                    } else {
                        sw.append_row(row![
                            row[0], row[1], row[2], row[3], row[4], row[5], row[6]
                        ])?;
                    }
                }
                Ok(())
            })
            .expect("write excel error!");
        }
        wb.close().expect("close excel error!");
    }

    fn write_intervals(&self, origami_path: &PathBuf) {
        let stapples = self
            .presenter
            .content
            .get_staples(&self.presenter.current_design, &self.presenter);
        let origami = Origami {
            scaffold_sequence: self
                .presenter
                .current_design
                .scaffold_sequence
                .clone()
                .unwrap_or("NO SEQUENCE".to_string()),
            intervals: stapples
                .iter()
                .map(|s| (s.intervals.staple_id, s.intervals.intervals.clone()))
                .collect(),
        };
        let mut origamis = Origamis(BTreeMap::new());
        origamis.0.insert(1, origami);
        if let Ok(json_content) = serde_json::to_string_pretty(&origamis) {
            if let Ok(mut f) = std::fs::File::create(origami_path) {
                if let Err(e) = f.write_all(json_content.as_bytes()) {
                    log::error!("Could not write to file {}", e);
                }
            } else {
                log::error!("Could not open file");
            }
        } else {
            log::error!("Serialization error");
        }
    }

    fn default_shift(&self) -> Option<usize> {
        self.presenter.current_design.scaffold_shift
    }
}

fn warn_all_staples_not_paired(first_unpaired: Nucl) -> String {
    format!(
        "All staptes are not paired. First unpaired nucleotide: {}",
        first_unpaired
    )
}

fn warn_scaffold_seq_mismatch(scaffold_length: usize, sequence_length: usize) -> String {
    format!(
        "The lengh of the scaffold is not equal to the length of the sequence.\n
        length of the scaffold: {}\n
        length of the sequence: {}",
        scaffold_length, sequence_length
    )
}

use ensnano_design::grid::HelixGridPosition;
use ensnano_interactor::DesignReader as MainReader;

impl MainReader for DesignReader {
    fn get_xover_id(&self, pair: &(Nucl, Nucl)) -> Option<usize> {
        self.presenter.junctions_ids.get_id(pair)
    }

    fn get_xover_with_id(&self, id: usize) -> Option<(Nucl, Nucl)> {
        self.presenter.junctions_ids.get_element(id)
    }

    fn get_grid_position_of_helix(&self, h_id: usize) -> Option<HelixGridPosition> {
        self.presenter
            .current_design
            .helices
            .get(&h_id)
            .and_then(|h| h.grid_position)
    }

    fn get_strand_with_id(&self, id: usize) -> Option<&ensnano_design::Strand> {
        self.presenter.current_design.strands.get(&id)
    }

    fn get_helix_grid(&self, h_id: usize) -> Option<GridId> {
        self.presenter
            .current_design
            .helices
            .get(&h_id)
            .and_then(|h| h.grid_position.map(|pos| pos.grid))
    }

    fn get_domain_ends(&self, s_id: usize) -> Option<Vec<Nucl>> {
        self.presenter
            .current_design
            .strands
            .get(&s_id)
            .map(|s| s.domain_ends())
    }
}

use std::collections::BTreeMap;
#[derive(Serialize)]
struct Origamis(BTreeMap<usize, Origami>);

#[derive(Serialize)]
struct Origami {
    scaffold_sequence: String,
    intervals: BTreeMap<usize, Vec<(isize, isize)>>,
}
