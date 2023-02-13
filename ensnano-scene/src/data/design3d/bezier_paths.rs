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
use crate::AppState;
use ensnano_design::{BezierEndCoordinates, BezierVertexId};
use ensnano_interactor::Selection;

impl<R: DesignReader> Design3D<R> {
    pub fn get_bezier_elements(&self, h_id: usize) -> (Vec<RawDnaInstance>, Vec<RawDnaInstance>) {
        let mut spheres = Vec::new();
        let mut tubes = Vec::new();
        if let Some(constructor) = self.design.get_cubic_bezier_controls(h_id) {
            log::info!("got control");
            for (control_point, position) in constructor.iter() {
                spheres.push(make_bezier_controll(
                    *position,
                    h_id as u32,
                    BezierControlPoint::CubicBezier(control_point),
                ));
            }
            tubes.push(make_bezier_squelton(
                constructor.start,
                constructor.control1,
            ));
            tubes.push(make_bezier_squelton(
                constructor.control1,
                constructor.control2,
            ));
            tubes.push(make_bezier_squelton(constructor.control2, constructor.end));
            (spheres, tubes)
        } else if let Some(controls) = self.design.get_piecewise_bezier_controls(h_id) {
            let mut iter = controls.into_iter().enumerate();
            while let Some(((n1, c1), (n2, c2))) = iter.next().zip(iter.next()) {
                spheres.push(make_bezier_controll(
                    c1,
                    h_id as u32,
                    BezierControlPoint::PiecewiseBezier(n1),
                ));
                spheres.push(make_bezier_controll(
                    c2,
                    h_id as u32,
                    BezierControlPoint::PiecewiseBezier(n2),
                ));
                tubes.push(make_bezier_squelton(c1, c2));
            }
            (spheres, tubes)
        } else {
            (spheres, tubes)
        }
    }

    pub fn get_control_point(&self, helix_id: usize, control: BezierControlPoint) -> Option<Vec3> {
        self.design
            .get_position_of_bezier_control(helix_id, control)
    }

    pub fn get_bezier_control_basis(
        &self,
        h_id: usize,
        bezier_control: BezierControlPoint,
    ) -> Option<Rotor3> {
        log::info!(
            "Getting bezier basis {:?} of helix {}",
            bezier_control,
            h_id
        );
        match bezier_control {
            BezierControlPoint::CubicBezier(_) => None,
            BezierControlPoint::PiecewiseBezier(n) => {
                let descriptor = self.design.get_curve_descriptor(h_id)?;
                if let CurveDescriptor::PiecewiseBezier { points, .. } = descriptor {
                    // There are two control points per bezier grid position
                    let g_id = points.get(n / 2).map(|point| point.position.grid)?;
                    let grid_orientation = self.design.get_grid_basis(g_id)?;
                    Some(grid_orientation)
                } else {
                    None
                }
            }
        }
    }

    pub fn get_bezier_sheets<S: AppState>(
        &self,
        app_state: &S,
    ) -> (Vec<Sheet2D>, Vec<RawDnaInstance>) {
        let mut sheets = Vec::new();
        let mut spheres = Vec::new();
        let axis_position = app_state.get_revolution_axis_position();

        let mut first = true;
        for (plane_id, desc) in self.design.get_bezier_planes().iter() {
            let corners = self.design.get_corners_of_plane(*plane_id);
            let sheet = get_sheet_instance(SheetDescriptor {
                corners,
                plane_descritor: desc,
                plane_id: *plane_id,
                parameters: self.design.get_parameters(),
                axis_position: axis_position.filter(|_| first),
            });
            spheres.extend_from_slice(corners_of_sheet(&sheet).as_slice());
            sheets.push(sheet);
            first = false;
        }
        (sheets, spheres)
    }

    pub fn get_bezier_vertex_position(
        &self,
        path_id: BezierPathId,
        vertex_id: usize,
    ) -> Option<Vec3> {
        self.design
            .get_bezier_paths()
            .and_then(|m| m.get(&path_id))
            .and_then(|p| p.bezier_controls().get(vertex_id))
            .map(|v| v.position)
    }

    pub fn get_bezier_paths_elements<S: AppState>(
        &self,
        app_state: &S,
    ) -> (Vec<RawDnaInstance>, Vec<RawDnaInstance>) {
        let mut spheres = Vec::new();
        let mut tubes = Vec::new();
        let selection = app_state.get_selection();
        if let Some(paths) = self.design.get_bezier_paths() {
            for (path_id, path) in paths.iter() {
                for (vertex_id, coordinates) in path.bezier_controls().iter().enumerate() {
                    add_raw_instances_representing_bezier_vertex(
                        BezierVertex {
                            coordinates: coordinates.clone(),
                            id: BezierVertexId {
                                path_id: *path_id,
                                vertex_id,
                            },
                        },
                        RawDnaInstances {
                            tubes: &mut tubes,
                            spheres: &mut spheres,
                        },
                        selection,
                    )
                }
                for point in path.get_curve_points().iter() {
                    spheres.push(
                        SphereInstance {
                            position: Vec3::new(point.x as f32, point.y as f32, point.z as f32),
                            color: [1., 0., 0., 1.].into(),
                            id: 0,
                            radius: 2.0,
                        }
                        .to_raw_instance(),
                    );
                }
            }
        }
        (spheres, tubes)
    }
}

struct BezierSheetCornerDesc<'a> {
    sheet: &'a Sheet2D,
    corner_id: usize,
    corner_position: Vec2,
}

fn corners_of_sheet(sheet: &Sheet2D) -> Vec<RawDnaInstance> {
    sheet
        .corners()
        .into_iter()
        .enumerate()
        .map(|(corner_id, corner_position)| {
            sheet_corner_instance(BezierSheetCornerDesc {
                sheet,
                corner_id,
                corner_position,
            })
        })
        .collect()
}

struct SheetDescriptor<'a> {
    corners: [Vec2; 4],
    plane_id: BezierPlaneId,
    plane_descritor: &'a BezierPlaneDescriptor,
    parameters: Parameters,
    axis_position: Option<f64>,
}

fn get_sheet_instance(desc: SheetDescriptor<'_>) -> Sheet2D {
    let parameters = &desc.parameters;
    let grad_step = 48.0 * parameters.z_step;
    let delta_corners = grad_step / 5.;
    let corners = &desc.corners;
    let axis_position = desc.axis_position.map(|x| x as f32);
    let mut ret = Sheet2D {
        plane_id: desc.plane_id,
        position: desc.plane_descritor.position,
        orientation: desc.plane_descritor.orientation,
        min_x: ((-3. * grad_step).min(corners[0].x - delta_corners) / grad_step).floor()
            * grad_step,
        max_x: ((3. * grad_step).max(corners[3].x + delta_corners) / grad_step).ceil() * grad_step,
        min_y: ((-3. * grad_step).min(corners[0].y - delta_corners) / grad_step).floor()
            * grad_step,
        max_y: ((3. * grad_step).max(corners[3].y + delta_corners) / grad_step).ceil() * grad_step,
        graduation_unit: 48.0 * parameters.z_step,
        axis_position,
    };

    if let Some(axis_position) = axis_position {
        ret.min_x = ret.min_x.min(axis_position - grad_step);
        ret.max_x = ret.max_x.max(axis_position + grad_step);
    }

    ret
}

/// Returns a sphere representing the corner of a bezier sheet
fn sheet_corner_instance(corner_desc: BezierSheetCornerDesc<'_>) -> RawDnaInstance {
    let position = corner_desc
        .sheet
        .space_position_of_point2d(corner_desc.corner_position);
    SphereInstance {
        color: Instance::color_from_u32(BEZIER_SHEET_CORNER_COLOR),
        id: u32::from_be_bytes([
            0xFD,
            corner_desc.corner_id as u8,
            corner_desc.sheet.plane_id.0.to_be_bytes()[2],
            corner_desc.sheet.plane_id.0.to_be_bytes()[3],
        ]),
        position,
        radius: BEZIER_SHEET_CORNER_RADIUS,
    }
    .to_raw_instance()
}

fn make_bezier_controll(
    position: Vec3,
    helix_id: u32,
    bezier_control: BezierControlPoint,
) -> RawDnaInstance {
    let id = bezier_widget_id(helix_id, bezier_control);
    let color = bezier_control_color(bezier_control);
    SphereInstance {
        position,
        id,
        color: Instance::color_from_au32(color),
        radius: BEZIER_CONTROL_RADIUS,
    }
    .to_raw_instance()
}

fn make_bezier_squelton(source: Vec3, dest: Vec3) -> RawDnaInstance {
    let rotor = Rotor3::from_rotation_between(Vec3::unit_x(), (dest - source).normalized());
    let position = (dest + source) / 2.;
    let length = (dest - source).mag();

    TubeInstance {
        position,
        color: Instance::color_from_u32(0),
        id: 0,
        rotor,
        radius: BEZIER_SQUELETON_RADIUS,
        length,
    }
    .to_raw_instance()
}

struct BezierVertex {
    coordinates: BezierEndCoordinates,
    id: BezierVertexId,
}

struct RawDnaInstances<'a> {
    tubes: &'a mut Vec<RawDnaInstance>,
    spheres: &'a mut Vec<RawDnaInstance>,
}

fn add_raw_instances_representing_bezier_vertex(
    vertex: BezierVertex,
    mut instances: RawDnaInstances<'_>,
    selection: &[Selection],
) {
    let tubes = &mut instances.tubes;
    let spheres = &mut instances.spheres;
    let color = if selection
        .iter()
        .any(|s| *s == Selection::BezierVertex(vertex.id))
    {
        [0., 0., 1., 1.].into()
    } else {
        [1., 0., 0., 1.].into()
    };
    spheres.push(
        SphereInstance {
            position: vertex.coordinates.position,
            color,
            id: crate::element_selector::bezier_vertex_id(vertex.id.path_id, vertex.id.vertex_id),
            radius: 10.0,
        }
        .to_raw_instance(),
    );
    spheres.push(
        SphereInstance {
            position: vertex.coordinates.position + vertex.coordinates.vector_out,
            color: Instance::color_from_u32(BEZIER_CONTROL1_COLOR),
            id: crate::element_selector::bezier_tengent_id(
                vertex.id.path_id,
                vertex.id.vertex_id,
                false,
            ),
            radius: 5.0,
        }
        .to_raw_instance(),
    );
    spheres.push(
        SphereInstance {
            position: vertex.coordinates.position - vertex.coordinates.vector_in,
            color: Instance::color_from_u32(BEZIER_CONTROL1_COLOR),
            id: crate::element_selector::bezier_tengent_id(
                vertex.id.path_id,
                vertex.id.vertex_id,
                true,
            ),
            radius: 5.0,
        }
        .to_raw_instance(),
    );
    tubes.push(
        create_dna_bound(
            vertex.coordinates.position,
            vertex.coordinates.position + vertex.coordinates.vector_out,
            0,
            0,
            false,
        )
        .to_raw_instance(),
    );
    tubes.push(
        create_dna_bound(
            vertex.coordinates.position,
            vertex.coordinates.position - vertex.coordinates.vector_in,
            0,
            0,
            false,
        )
        .to_raw_instance(),
    );
}
