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

use std::path::Path as StdPath;

use svg::node::element::path::{Command, Data, Position};
use svg::parser::Event;

use super::*;

const SCALE: Vec2 = Vec2 { x: 0.1, y: 0.1 };
const ORIGIN: Vec2 = Vec2 {
    x: 134.23425,
    y: 13.5557,
};

pub fn read_first_svg_path(file_path: &StdPath) -> Result<BezierPath, SvgImportError> {
    let mut content = String::new();
    let events = svg::open(file_path, &mut content)?;

    for event in events {
        if let Event::Tag(_, _, attributes) = event {
            let data = attributes
                .get("d")
                .ok_or_else(|| SvgImportError::AttributeNotFound(String::from("d")))?;
            let data = Data::parse(data)?;

            let mut ret = PathBuilder::default();
            for command in data.iter() {
                match command {
                    Command::Close => return Ok(ret.close()),
                    Command::Move(Position::Absolute, parameters) => {
                        if parameters.len() != 2 {
                            return Err(SvgImportError::BadParameters);
                        } else {
                            let at = Vec2::new(parameters[0], parameters[1]);
                            ret.start(at)?;
                        }
                    }
                    Command::CubicCurve(Position::Absolute, parameters) => {
                        let arg = MoveToParameter::from_svg_paramter(parameters)?;
                        ret.move_to(arg)?
                    }
                    _ => (),
                }
            }
            return Ok(ret.finish());
        }
    }
    Err(SvgImportError::NoPathFound)
}

#[derive(Default)]
struct PathBuilder {
    vertices: Vec<BezierVertex>,
}

impl PathBuilder {
    fn start(&mut self, at: Vec2) -> Result<(), SvgImportError> {
        if self.vertices.is_empty() {
            self.vertices = vec![BezierVertex {
                plane_id: BezierPlaneId(0),
                position: SCALE * at - ORIGIN,
                position_in: None,
                position_out: None,
                grid_translation: Vec3::zero(),
                angle_with_plane: 0.,
            }]
        } else {
            return Err(SvgImportError::UnexpectedCommand(String::from("Move")));
        }

        Ok(())
    }

    fn move_to(&mut self, parameters: MoveToParameter) -> Result<(), SvgImportError> {
        let prev_vertex = self
            .vertices
            .last_mut()
            .ok_or_else(|| SvgImportError::UnexpectedCommand(String::from("CubicCurve")))?;
        prev_vertex.position_out = Some(SCALE * parameters.control_1 - ORIGIN);

        let new_vertex = BezierVertex {
            plane_id: BezierPlaneId(0),
            position: SCALE * parameters.position - ORIGIN,
            position_out: None,
            position_in: Some(SCALE * parameters.control_2 - ORIGIN),
            grid_translation: Vec3::zero(),
            angle_with_plane: 0.,
        };
        self.vertices.push(new_vertex);

        Ok(())
    }

    fn close(self) -> BezierPath {
        BezierPath {
            vertices: self.vertices,
            cyclic: true,
            grid_type: None,
        }
    }

    fn finish(self) -> BezierPath {
        BezierPath {
            vertices: self.vertices,
            cyclic: false,
            grid_type: None,
        }
    }
}

struct MoveToParameter {
    position: Vec2,
    control_1: Vec2,
    control_2: Vec2,
}

impl MoveToParameter {
    fn from_svg_paramter(parameters: &[f32]) -> Result<Self, SvgImportError> {
        if parameters.len() != 6 {
            Err(SvgImportError::BadParameters)
        } else {
            Ok(Self {
                control_1: Vec2::new(parameters[0], parameters[1]),
                control_2: Vec2::new(parameters[2], parameters[3]),
                position: Vec2::new(parameters[4], parameters[5]),
            })
        }
    }
}

#[derive(Debug)]
pub enum SvgImportError {
    IOError(std::io::Error),
    SvgParserError(svg::parser::Error),
    NoPathFound,
    AttributeNotFound(String),
    UnexpectedCommand(String),
    CouldNotParseData,
    BadParameters,
}

impl From<std::io::Error> for SvgImportError {
    fn from(e: std::io::Error) -> Self {
        Self::IOError(e)
    }
}

impl From<svg::parser::Error> for SvgImportError {
    fn from(e: svg::parser::Error) -> Self {
        Self::SvgParserError(e)
    }
}
