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

const DEFAULT_BEZIER_TENGENT_NORM: f32 = 1. / 3.;

/// An object capable of instantiating PieceWiseBezier curves.
pub(crate) trait PieceWiseBezierInstantiator {
    fn nb_vertices(&self) -> usize;
    fn position(&self, i: usize) -> Option<Vec3>;
    fn vector_in(&self, i: usize) -> Option<Vec3>;
    fn vector_out(&self, i: usize) -> Option<Vec3>;
    fn cyclic(&self) -> bool;

    fn instantiate(&self) -> Option<InstanciatedPiecewiseBezier> {
        let descriptor = if self.nb_vertices() > 2 {
            let n = self.nb_vertices();
            let idx_iterator: Box<dyn Iterator<Item = ((usize, usize), usize)>> = if self.cyclic() {
                Box::new(
                    (0..n)
                        .cycle()
                        .skip(n - 1)
                        .zip((0..n).cycle().take(n + 1))
                        .zip((0..n).cycle().skip(1)),
                )
            } else {
                // iterate from 0 to n-1 and add manually the first and last vertices
                // afterwards
                Box::new((0..n).zip((0..n).skip(1)).zip((0..n).skip(2)))
            };
            let mut bezier_points: Vec<_> = idx_iterator
                .filter_map(|((idx_from, idx), idx_to)| {
                    let pos_from = self.position(idx_from)?;
                    let pos = self.position(idx)?;
                    let pos_to = self.position(idx_to)?;
                    let vector_in = self
                        .vector_in(idx)
                        .unwrap_or((pos_to - pos_from) * DEFAULT_BEZIER_TENGENT_NORM);
                    let vector_out = self
                        .vector_out(idx)
                        .unwrap_or((pos_to - pos_from) * DEFAULT_BEZIER_TENGENT_NORM);

                    Some(BezierEndCoordinates {
                        position: pos,
                        vector_in,
                        vector_out,
                    })
                })
                .collect();
            if !self.cyclic() {
                // Add manually the first and last vertices
                let first_point = {
                    let second_point = bezier_points.get(0)?;
                    let pos = self.position(0)?;
                    let control = second_point.position - second_point.vector_in;

                    let vector_out = self.vector_out(0).unwrap_or((control - pos) / 2.);

                    let vector_in = self.vector_in(0).unwrap_or((control - pos) / 2.);

                    BezierEndCoordinates {
                        position: pos,
                        vector_out,
                        vector_in,
                    }
                };
                bezier_points.insert(0, first_point);
                let last_point = {
                    let second_to_last_point = bezier_points.last()?;
                    // Ok to unwrap because vertices has length > 2
                    let pos = self.position(self.nb_vertices() - 1)?;
                    let control = second_to_last_point.position + second_to_last_point.vector_out;
                    let vector_out = self
                        .vector_out(self.nb_vertices() - 1)
                        .unwrap_or((pos - control) / 2.);

                    let vector_in = self
                        .vector_in(self.nb_vertices() - 1)
                        .unwrap_or((pos - control) / 2.);
                    BezierEndCoordinates {
                        position: pos,
                        vector_out,
                        vector_in,
                    }
                };
                bezier_points.push(last_point);
            } else {
                bezier_points.pop();
            }
            Some(bezier_points)
        } else if self.nb_vertices() == 2 {
            let pos_first = self.position(0)?;
            let pos_last = self.position(1)?;
            let vec = (pos_last - pos_first) / 3.;
            Some(vec![
                BezierEndCoordinates {
                    position: pos_first,
                    vector_in: vec,
                    vector_out: vec,
                },
                BezierEndCoordinates {
                    position: pos_last,
                    vector_in: vec,
                    vector_out: vec,
                },
            ])
        } else if self.nb_vertices() == 1 {
            let pos_first = self.position(0)?;
            Some(vec![BezierEndCoordinates {
                position: pos_first,
                vector_in: f32::NAN * Vec3::one(),
                vector_out: f32::NAN * Vec3::one(),
            }])
        } else {
            None
        }?;
        Some(InstanciatedPiecewiseBezier {
            t_min: None,
            t_max: Some(descriptor.len() as f64 - 1.),
            ends: descriptor,
            cyclic: self.cyclic(),
        })
    }
}
