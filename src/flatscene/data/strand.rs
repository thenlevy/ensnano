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
use lyon::path::path::BuilderWithAttributes;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::{StrokeVertex, StrokeVertexConstructor};
use ultraviolet::Vec2;

type Vertices = lyon::tessellation::VertexBuffers<StrandVertex, u16>;

/// The factor by which the width of hilighted strands is multiplied
const HIGHLIGHT_FACTOR: f32 = 1.7;

macro_rules! point {
    ($point: ident) => {
        Point::new($point.x, $point.y)
    };
}

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

    fn get_path_color(&self) -> [f32; 4] {
        let color = if self.highlight {
            crate::utils::instance::Instance::color_from_au32(self.color)
        } else {
            crate::utils::instance::Instance::color_from_u32(self.color)
        };
        [color.x, color.y, color.z, color.w]
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
        let color = self.get_path_color();
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
        let mut sign = 10.;
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
                if let Some(last_nucl) = last_nucl {
                    // We are drawing a xover
                    let point =
                        helices[last_nucl.helix].get_nucl_position(&last_nucl, Shift::Prime3);
                    last_point = Some(point);
                    builder.line_to(Point::new(point.x, point.y), &[depth, sign]);
                } else {
                    // We are drawing the free end
                    let position = last_point.unwrap();
                    builder.begin(Point::new(position.x, position.y), &[depth, sign]);
                }
                let last_pos = last_point.unwrap();
                let alternate = one_point_one_camera(TwoCameraAndPoints {
                    point_1: last_pos,
                    point_2: position,
                    cam_1: my_cam,
                    cam_2: other_cam,
                });
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
                    Vec2::new(diff.y, -diff.x)
                };
                let dir = (xover_target - xover_origin).normalized();
                let dist = (xover_target - xover_origin).mag();
                let normal_1 = if let Some(last_nucl) = last_nucl {
                    let pos = helices[last_nucl.helix]
                        .get_nucl_position(&last_nucl.prime5(), Shift::Prime3Outsided);
                    (pos - xover_origin).normalized()
                } else {
                    normal
                };
                let normal_2 = {
                    let pos = helices[nucl.helix]
                        .get_nucl_position(&nucl.prime3(), Shift::Prime5Outsided);
                    (pos - xover_target).normalized()
                };
                //let control_1 = xover_origin - (dist / 2.) * dir + normal_1;
                //let control_2 = xover_target + (dist / 2.) * dir + normal_2;
                let control_1 = xover_origin + (dist.sqrt() / 2.) * normal_1;
                let control_2 = xover_target + (dist.sqrt() / 2.) * normal_2;
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
                    builder.cubic_bezier_to(
                        Point::new(control_1.x, control_1.y),
                        Point::new(control_2.x, control_2.y),
                        Point::new(xover_target.x, xover_target.y),
                        &[depth, sign],
                    );
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
                let alternate = one_point_one_camera(TwoCameraAndPoints {
                    point_1: last_pos,
                    point_2: *position,
                    cam_1: my_cam,
                    cam_2: other_cam,
                });
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
        let mut width = (vertex.interpolated_attributes()[1] / 3.)
            .min(1.)
            .max(-1.)
            .abs()
            .powf(2.)
            .max(0.3);
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

struct FilteredFreeEnd {
    pub point: Vec2,
    pub prime3: bool,
    pub candidates: Vec<FlatNucl>,
}

impl FilteredFreeEnd {
    fn read(free_end: &Option<FreeEnd>, strand_id: usize) -> Option<Self> {
        free_end
            .as_ref()
            .filter(|f| f.strand_id == strand_id)
            .map(|free_end| Self {
                point: free_end.point,
                prime3: free_end.prime3,
                candidates: free_end.candidates.clone(),
            })
    }
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

struct TwoCameraAndPoints<'a> {
    point_1: Vec2,
    point_2: Vec2,
    cam_1: &'a CameraPtr,
    cam_2: &'a CameraPtr,
}

/// Return true if `a` and `b` are both visible by exactly one camera, and each camera can see
/// exactly one of the points.
fn one_point_one_camera<'a>(input: TwoCameraAndPoints<'a>) -> bool {
    let a = input.point_1;
    let b = input.point_2;
    let my_cam = input.cam_1;
    let other_cam = input.cam_2;

    if my_cam.borrow().can_see_world_point(a) && !other_cam.borrow().can_see_world_point(a) {
        !my_cam.borrow().can_see_world_point(b) && other_cam.borrow().can_see_world_point(b)
    } else if !my_cam.borrow().can_see_world_point(a) && other_cam.borrow().can_see_world_point(a) {
        my_cam.borrow().can_see_world_point(b) && !other_cam.borrow().can_see_world_point(b)
    } else {
        false
    }
}

/// An object that builds the vertices used to draw a strand
struct StrandVertexBuilder<'a> {
    /// The Builder that builds normal path of the strand
    main_path_builder: BuilderWithAttributes,
    /// The Builder that builds the vertices of the splied cross overs
    splited_cross_over_builder: BuilderWithAttributes,
    /// The current position of the path builders
    last_point: Option<Vec2>,
    /// The depth attribute is used to generate the z coordinate of the vertices
    last_depth: Option<f32>,
    /// The sign attribute is used to handle the width of the path. The sign should be flipped
    /// between each extremity of a stroke that should be thin in the middle.
    sign: f32,
    main_camera: &'a CameraPtr,
    alternative_camera: &'a CameraPtr,
    main_builder_is_drawing: bool,
    depth: f32,
}

struct StrandVertexBuilderInitializer<'a> {
    main_camera: &'a CameraPtr,
    alternative_camera: &'a CameraPtr,
    free_end: &'a Option<FilteredFreeEnd>,
}

// We need to use this macro to appease the borrow checker
macro_rules! attributes {
    ($self: ident) => {
        &[$self.depth, $self.sign]
    };
}

impl<'a> StrandVertexBuilder<'a> {
    /// Initialise the builder.
    pub fn init(initializer: StrandVertexBuilderInitializer<'a>) -> Self {
        let main_path_builder = Path::builder_with_attributes(2);
        let splited_cross_over_builder = Path::builder_with_attributes(2);
        let last_point = Self::read_free_end(&initializer);

        Self {
            main_path_builder,
            splited_cross_over_builder,
            last_point,
            last_depth: None,
            sign: 1.0,
            main_camera: initializer.main_camera,
            alternative_camera: initializer.alternative_camera,
            main_builder_is_drawing: false,
            depth: 0.0,
        }
    }

    fn read_free_end(initializer: &StrandVertexBuilderInitializer) -> Option<Vec2> {
        match initializer.free_end {
            Some(FilteredFreeEnd { point, prime3, .. }) if !prime3 => alternative_position(
                *point,
                initializer.main_camera,
                initializer.alternative_camera,
            )
            .or(Some(*point)),
            _ => None,
        }
    }

    pub fn draw(&mut self, instruction: DrawingInstruction) {
        match instruction {
            DrawingInstruction::StartAt {
                position: to,
                depth,
            } => {
                self.depth = depth;
                if let Some(from) = self.last_point {
                    self.draw_free_end(from, to);
                } else {
                    self.start_drawing_on(to);
                }
                self.last_point = Some(to);
            }
            DrawingInstruction::LineTo { position, depth } => {
                self.start_drawing_on(self.last_point.expect("last point"));
                self.depth = depth;
                self.main_path_builder
                    .line_to(Point::new(position.x, position.y), attributes!(self));
                self.last_point = Some(position);
            }
            DrawingInstruction::XoverTo {
                normal_source,
                normal_target,
                to,
                depth_to,
            } => {
                self.depth = depth_to;
                if let Some((from, to)) =
                    self.alternative_positions(self.last_point.expect("last point"), to)
                {
                    self.stop_drawing();
                    self.splited_cross_over_builder
                        .begin(Point::new(from.x, from.y), attributes!(self));
                    self.splited_cross_over_builder
                        .line_to(Point::new(to.x, to.y), attributes!(self));
                    self.splited_cross_over_builder.end(false);
                } else {
                    self.sign *= -1.0;
                    let xover_target = to;
                    let xover_origin = self.last_point.expect("last point");
                    let dist = (xover_target - xover_origin).mag();
                    let normal_1 = (normal_source - xover_origin).normalized();
                    let normal_2 = (normal_target - xover_target).normalized();
                    let control_1 = xover_origin + (dist.sqrt() / 2.) * normal_1;
                    let control_2 = xover_target + (dist.sqrt() / 2.) * normal_2;
                    self.main_path_builder.cubic_bezier_to(
                        point!(control_1),
                        point!(control_2),
                        point!(to),
                        attributes!(self),
                    );
                }
                self.last_point = Some(to);
            }
            DrawingInstruction::FreeEndPrime3(to) => {
                if let Some(from) = self.last_point.take() {
                    self.draw_free_end(from, to);
                }
            }
        }
    }

    fn draw_free_end(&mut self, from: Vec2, to: Vec2) {
        if let Some((from, to)) = self.alternative_positions(from, to) {
            self.splited_cross_over_builder
                .begin(Point::new(from.x, from.y), attributes!(self));
            self.splited_cross_over_builder
                .line_to(Point::new(to.x, to.y), attributes!(self));
            self.splited_cross_over_builder.end(false);
        } else {
            self.start_drawing_on(from);
            self.main_path_builder
                .line_to(point!(to), attributes!(self));
        }
    }

    fn alternative_positions(&self, from: Vec2, to: Vec2) -> Option<(Vec2, Vec2)> {
        if one_point_one_camera(TwoCameraAndPoints {
            point_1: from,
            point_2: to,
            cam_1: self.main_camera,
            cam_2: self.alternative_camera,
        }) {
            alternative_position(from, self.main_camera, self.alternative_camera)
                .zip(alternative_position(to, self.main_camera, self.main_camera))
        } else {
            None
        }
    }

    fn start_drawing_on(&mut self, pos: Vec2) {
        if !self.main_builder_is_drawing {
            self.main_path_builder.begin(point!(pos), attributes!(self));
        }
        self.main_builder_is_drawing = true;
    }

    fn stop_drawing(&mut self) {
        if self.main_builder_is_drawing {
            self.main_path_builder.end(false);
        }
        self.main_builder_is_drawing = false;
    }

    pub fn finish(&mut self) {
        self.stop_drawing();
    }
}

/// An object that reads nucleotides and decide weither drawing the next nucleotide means drawing a
/// cross-over or a strand's domain.
struct StrandTopologyReader<'a> {
    /// The number of points that have been drawn on the current helix
    nb_point_helix: usize,
    /// The last nucleotide that has been drawn to
    last_nucl: Option<FlatNucl>,
    /// The the helices that can translate nucleotide to points in the plane
    helices: &'a [Helix],
}

impl<'a> StrandTopologyReader<'a> {
    pub fn init(helices: &'a [Helix]) -> Self {
        Self {
            nb_point_helix: 0,
            last_nucl: None,
            helices,
        }
    }

    pub fn read_nucl(&mut self, nucl: FlatNucl) -> DrawingInstruction {
        if let Some(last_nucl) = self.last_nucl.replace(nucl) {
            if last_nucl.helix == nucl.helix {
                self.nb_point_helix += 1;
            } else {
                self.nb_point_helix = 0;
            }
            if self.nb_point_helix % 2 == 0 {
                self.xover_instruction(last_nucl, nucl)
            } else {
                self.domain_instruction(nucl)
            }
        } else {
            let position = self.helices[nucl.helix].get_nucl_position(&nucl, Shift::Prime5);
            DrawingInstruction::StartAt {
                position,
                depth: self.get_depth(nucl),
            }
        }
    }

    fn xover_instruction(&self, last_nucl: FlatNucl, nucl: FlatNucl) -> DrawingInstruction {
        // we start the xover at the 3' end of the source and we go to the 5' end of the target
        let normal_source =
            self.helices[last_nucl.helix].get_nucl_position(&last_nucl, Shift::Prime3Outsided);
        let normal_target =
            self.helices[nucl.helix].get_nucl_position(&nucl, Shift::Prime5Outsided);
        let to = self.helices[nucl.helix].get_nucl_position(&nucl, Shift::Prime5);
        DrawingInstruction::XoverTo {
            normal_source,
            normal_target,
            to,
            depth_to: self.get_depth(nucl),
        }
    }

    fn domain_instruction(&self, nucl: FlatNucl) -> DrawingInstruction {
        // We go the the 3' end of the domain that we are drawing
        let position = self.helices[nucl.helix].get_nucl_position(&nucl, Shift::Prime3);
        DrawingInstruction::LineTo {
            position,
            depth: self.get_depth(nucl),
        }
    }

    fn get_depth(&self, nucl: FlatNucl) -> f32 {
        self.helices[nucl.helix].get_depth()
    }

    fn finish(&mut self, free_end: &Option<FilteredFreeEnd>) -> Option<DrawingInstruction> {
        if let Some(free_end) = free_end.as_ref().filter(|free_end| free_end.prime3) {
            Some(DrawingInstruction::FreeEndPrime3(free_end.point))
        } else {
            self.last_nucl.take().map(|nucl| {
                let position = self.helices[nucl.helix].get_arrow_end(&nucl);
                DrawingInstruction::LineTo {
                    position,
                    depth: self.get_depth(nucl),
                }
            })
        }
    }
}

enum DrawingInstruction {
    StartAt {
        position: Vec2,
        depth: f32,
    },
    LineTo {
        position: Vec2,
        depth: f32,
    },
    XoverTo {
        normal_source: Vec2,
        normal_target: Vec2,
        to: Vec2,
        depth_to: f32,
    },
    /// End the drawing by drawing a free end
    FreeEndPrime3(Vec2),
}
