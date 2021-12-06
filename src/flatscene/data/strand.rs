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
use super::super::view::InsertionInstance;
use super::helix::{Helix, Shift};
use super::{CameraPtr, FlatNucl};
use lyon::math::Point;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::{StrokeVertex, StrokeVertexConstructor};
use ultraviolet::Vec2;

type Vertices = lyon::tessellation::VertexBuffers<StrandVertex, u16>;

/// The factor by which the width of hilighted strands is multiplied
const HIGHLIGHT_FACTOR: f32 = 1.7;

pub struct Strand {
    pub color: u32,
    pub points: Vec<FlatNucl>,
    pub insertions: Vec<FlatNucl>,
    pub id: usize,
    pub highlight: bool,
}

impl Strand {
    pub fn new(
        color: u32,
        points: Vec<FlatNucl>,
        insertions: Vec<FlatNucl>,
        id: usize,
        highlight: bool,
    ) -> Self {
        Self {
            color,
            points,
            id,
            insertions,
            highlight,
        }
    }

    pub fn to_vertices(
        &self,
        helices: &[Helix],
        free_end: &Option<FreeEnd>,
        my_cam: &CameraPtr,
        other_cam: &CameraPtr,
    ) -> (Vertices, Vertices) {
        let mut vertices = Vertices::new();
        let mut cross_split_vertices = Vertices::new();
        if self.points.len() == 0 {
            return (vertices, cross_split_vertices);
        }
        let color = if self.highlight {
            crate::utils::instance::Instance::color_from_au32(self.color)
        } else {
            crate::utils::instance::Instance::color_from_u32(self.color)
        };
        let color = [color.x, color.y, color.z, color.w];
        let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

        let mut builder = Path::builder_with_attributes(2);
        let mut cross_split_builder = Path::builder_with_attributes(2);
        let mut last_nucl: Option<FlatNucl> = None;
        let mut last_point = match free_end {
            Some(FreeEnd {
                point,
                strand_id,
                prime3,
                ..
            }) if *strand_id == self.id && !prime3 => {
                alternative_position(*point, my_cam, other_cam).or(Some(*point))
            }
            _ => None,
        };

        let mut last_depth = None;
        let mut sign = 1.;
        let mut nb_point_helix = 0;

        for (i, nucl) in self.points.iter().enumerate() {
            let position = helices[nucl.helix].get_nucl_position(&nucl, Shift::Prime5);
            let depth = helices[nucl.helix].get_depth();
            let point = Point::new(position.x, position.y);
            let xover = if last_point.is_some() {
                if Some(nucl.helix) == last_nucl.map(|n| n.helix) {
                    nb_point_helix += 1;
                    nb_point_helix % 2 == 0
                } else {
                    nb_point_helix = 0;
                    true
                }
            } else {
                false
            };
            if i == 0 && last_point.is_none() {
                builder.begin(point, &[depth, sign]);
            //} else if last_point.is_some() && Some(nucl.helix) != last_nucl.map(|n| n.helix) {
            } else if xover {
                let depth = depth.min(last_depth.unwrap_or(depth));
                let mut cut = false;
                if let Some(nucl) = last_nucl {
                    // We are drawing a xover
                    let point = helices[nucl.helix].get_nucl_position(&nucl, Shift::Prime3);
                    last_point = Some(point);
                    builder.line_to(Point::new(point.x, point.y), &[depth, sign]);
                } else {
                    // We are drawing the free end
                    let position = last_point.unwrap();
                    builder.begin(Point::new(position.x, position.y), &[depth, sign]);
                }
                let last_pos = last_point.unwrap();
                let alternate = must_use_alternate(last_pos, position, my_cam, other_cam);
                let must_draw = my_cam.borrow().can_see_world_point(last_pos)
                    || my_cam.borrow().can_see_world_point(position);
                let xover_origin = if alternate {
                    if let Some(alt) = alternative_position(last_pos, my_cam, other_cam) {
                        cut = true;
                        alt
                    } else {
                        last_pos
                    }
                } else {
                    last_pos
                };

                let xover_target = if alternate {
                    if let Some(alt) = alternative_position(position, my_cam, other_cam) {
                        cut = true;
                        alt
                    } else {
                        position
                    }
                } else {
                    position
                };

                let normal = {
                    let diff = (xover_target - xover_origin).normalized();
                    Vec2::new(diff.y, diff.x)
                };
                let control = (xover_origin + xover_target) / 2. + normal / 3.;
                if cut {
                    cross_split_builder
                        .begin(Point::new(xover_origin.x, xover_origin.y), &[depth, 5.]);
                    cross_split_builder.line_to(
                        Point::new(xover_origin.x + 0.01, xover_origin.y + 0.01),
                        &[depth, 5.],
                    );
                    cross_split_builder
                        .line_to(Point::new(xover_target.x, xover_target.y), &[depth, 5.]);
                    cross_split_builder.end(false);
                } else {
                    sign *= -1.;
                    if must_draw {
                        builder.quadratic_bezier_to(
                            Point::new(control.x, control.y),
                            Point::new(xover_target.x, xover_target.y),
                            &[depth, sign],
                        );
                    } else {
                        builder.end(false);
                        builder.begin(Point::new(xover_target.x, xover_target.y), &[depth, sign]);
                    }
                }
                if cut {
                    builder.end(false);
                    builder.begin(point, &[depth, sign]);
                }
            } else {
                builder.line_to(point, &[depth, sign]);
            }
            last_point = Some(position);
            last_nucl = Some(*nucl);
            last_depth = Some(depth);
        }
        if let Some(nucl) = last_nucl {
            let point = helices[nucl.helix].get_nucl_position(&nucl, Shift::Prime3);
            last_point = Some(point);
            builder.line_to(Point::new(point.x, point.y), &[last_depth.unwrap(), sign]);
        }
        match free_end {
            Some(FreeEnd {
                strand_id,
                point: position,
                prime3,
                ..
            }) if *strand_id == self.id && *prime3 => {
                let depth = 1e-4;
                let last_pos = last_point.unwrap();
                let mut cut = false;
                let alternate = must_use_alternate(last_pos, *position, my_cam, other_cam);
                let xover_origin = if alternate {
                    if let Some(alt) = alternative_position(last_pos, my_cam, other_cam) {
                        cut = true;
                        alt
                    } else {
                        last_pos
                    }
                } else {
                    last_pos
                };
                let xover_target = if alternate {
                    if let Some(alt) = alternative_position(*position, my_cam, other_cam) {
                        cut = true;
                        alt
                    } else {
                        *position
                    }
                } else {
                    *position
                };

                let normal = {
                    let diff = (xover_target - xover_origin).normalized();
                    Vec2::new(diff.y, diff.x)
                };
                let control = (xover_origin + xover_target) / 2. + normal / 3.;
                if cut {
                    cross_split_builder
                        .begin(Point::new(xover_origin.x, xover_origin.y), &[depth, 5.]);
                    cross_split_builder.line_to(
                        Point::new(xover_origin.x + 0.01, xover_origin.y + 0.01),
                        &[depth, 5.],
                    );
                    cross_split_builder
                        .line_to(Point::new(xover_target.x, xover_target.y), &[depth, 5.]);
                    cross_split_builder.end(false);
                } else {
                    sign *= -1.;
                    builder.quadratic_bezier_to(
                        Point::new(control.x, control.y),
                        Point::new(xover_target.x, xover_target.y),
                        &[depth, sign],
                    );
                }
            }
            _ => {
                // Draw the tick of the 3' end if the strand is not empty
                if let Some(nucl) = last_nucl {
                    let position = helices[nucl.helix].get_arrow_end(&nucl);
                    let point = Point::new(position.x, position.y);
                    builder.line_to(point, &[last_depth.unwrap(), sign]);
                }
            }
        }
        builder.end(false);
        let path = builder.build();
        let cross_split_path = cross_split_builder.build();
        stroke_tess
            .tessellate_path(
                &path,
                &tessellation::StrokeOptions::tolerance(0.01)
                    .with_line_cap(tessellation::LineCap::Round)
                    .with_end_cap(tessellation::LineCap::Round)
                    .with_start_cap(tessellation::LineCap::Round)
                    .with_line_join(tessellation::LineJoin::Round),
                &mut tessellation::BuffersBuilder::new(
                    &mut vertices,
                    WithAttributes {
                        color,
                        highlight: self.highlight,
                    },
                ),
            )
            .expect("Error durring tessellation");
        stroke_tess
            .tessellate_path(
                &cross_split_path,
                &tessellation::StrokeOptions::tolerance(0.01)
                    .with_line_cap(tessellation::LineCap::Round)
                    .with_end_cap(tessellation::LineCap::Round)
                    .with_start_cap(tessellation::LineCap::Round)
                    .with_line_join(tessellation::LineJoin::Round),
                &mut tessellation::BuffersBuilder::new(
                    &mut cross_split_vertices,
                    WithAttributes {
                        color,
                        highlight: self.highlight,
                    },
                ),
            )
            .expect("Error durring tessellation");
        (vertices, cross_split_vertices)
    }

    pub fn get_insertions(&self, helices: &[Helix]) -> Vec<InsertionInstance> {
        let mut ret = Vec::with_capacity(self.insertions.len());
        for i in self.insertions.iter() {
            ret.push(helices[i.helix].insertion_instance(i, self.color));
        }
        ret
    }

    pub fn indication(nucl1: FlatNucl, nucl2: FlatNucl, helices: &[Helix]) -> Vertices {
        let mut vertices = Vertices::new();
        let mut builder = Path::builder_with_attributes(2);
        let color = [0.823, 0.525, 0.058, 0.75];
        let start = helices[nucl1.helix].get_nucl_position(&nucl1, Shift::No);
        let end = helices[nucl2.helix].get_nucl_position(&nucl2, Shift::No);

        builder.begin(Point::new(start.x, start.y), &[1e-4, 1.]);
        builder.line_to(Point::new(end.x, end.y), &[1e-4, 1.]);
        let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

        builder.end(false);
        let path = builder.build();
        stroke_tess
            .tessellate_path(
                &path,
                &tessellation::StrokeOptions::tolerance(0.01)
                    .with_line_cap(tessellation::LineCap::Round)
                    .with_end_cap(tessellation::LineCap::Round)
                    .with_start_cap(tessellation::LineCap::Round)
                    .with_line_join(tessellation::LineJoin::Round),
                &mut tessellation::BuffersBuilder::new(
                    &mut vertices,
                    WithAttributes {
                        color,
                        highlight: false,
                    },
                ),
            )
            .expect("Error durring tessellation");
        vertices
    }

    pub fn highlighted(&self, color: u32) -> Self {
        Self {
            color,
            highlight: true,
            points: self.points.clone(),
            insertions: self.insertions.clone(),
            ..*self.clone()
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StrandVertex {
    position: [f32; 2],
    normal: [f32; 2],
    color: [f32; 4],
    depth: f32,
    width: f32,
}

pub struct WithAttributes {
    color: [f32; 4],
    highlight: bool,
}

impl StrokeVertexConstructor<StrandVertex> for WithAttributes {
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> StrandVertex {
        let mut width = vertex.interpolated_attributes()[1].min(1.).powi(2).max(0.3);
        if self.highlight {
            width *= HIGHLIGHT_FACTOR;
        }
        let color = self.color;

        let mut depth = if vertex.interpolated_attributes()[1] > 1.00001 {
            1e-7
        } else {
            vertex.interpolated_attributes()[0]
        };
        if self.highlight {
            depth *= 0.99;
        }

        StrandVertex {
            position: vertex.position_on_path().to_array(),
            normal: vertex.normal().to_array(),
            color,
            depth,
            width,
        }
    }
}

#[derive(Debug)]
pub struct FreeEnd {
    pub strand_id: usize,
    pub point: Vec2,
    pub prime3: bool,
    pub candidates: Vec<FlatNucl>,
}

/// If nucl is visible on cam2, and not on cam 1, convert the position of the nucl in cam2
/// screen coordinate then back to cam1 world coordinate
fn alternative_position(position: Vec2, cam1: &CameraPtr, cam2: &CameraPtr) -> Option<Vec2> {
    if cam1.borrow().bottom == cam2.borrow().bottom {
        None
    } else {
        if !cam1.borrow().can_see_world_point(position)
            && cam2.borrow().can_see_world_point(position)
        {
            let cam2_screen = cam2.borrow().world_to_norm_screen(position.x, position.y);
            let alternative = if cam1.borrow().bottom {
                cam1.borrow()
                    .norm_screen_to_world(cam2_screen.0, cam2_screen.1 - 1.)
            } else {
                cam1.borrow()
                    .norm_screen_to_world(cam2_screen.0, cam2_screen.1 + 1.)
            };
            Some(Vec2::new(alternative.0, alternative.1))
        } else {
            None
        }
    }
}

fn must_use_alternate(a: Vec2, b: Vec2, my_cam: &CameraPtr, other_cam: &CameraPtr) -> bool {
    if my_cam.borrow().can_see_world_point(a) && !other_cam.borrow().can_see_world_point(a) {
        !my_cam.borrow().can_see_world_point(b) && other_cam.borrow().can_see_world_point(b)
    } else if !my_cam.borrow().can_see_world_point(a) && other_cam.borrow().can_see_world_point(a) {
        my_cam.borrow().can_see_world_point(b) && !other_cam.borrow().can_see_world_point(b)
    } else {
        false
    }
}
