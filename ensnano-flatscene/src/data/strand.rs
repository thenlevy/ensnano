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
use ensnano_design::ultraviolet::Vec2;
use lyon::math::Point;
use lyon::path::path::BuilderWithAttributes;
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::{StrokeVertex, StrokeVertexConstructor};

type Vertices = lyon::tessellation::VertexBuffers<StrandVertex, u16>;

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
    pub highlight: Option<f32>,
}

impl Strand {
    pub fn new(
        color: u32,
        points: Vec<FlatNucl>,
        insertions: Vec<FlatNucl>,
        id: usize,
        highlight: Option<f32>,
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
        let color = if self.highlight.is_some() {
            ensnano_utils::instance::Instance::color_from_au32(self.color)
        } else {
            ensnano_utils::instance::Instance::color_from_u32(self.color)
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
        if self.points.is_empty() {
            return (vertices, cross_split_vertices);
        }
        let color = self.get_path_color();
        let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

        let filtered_free_end = FilteredFreeEnd::read(free_end, self.id);
        let mut strand_vertex_builder = StrandVertexBuilder::init(StrandVertexBuilderInitializer {
            main_camera: my_cam,
            alternative_camera: other_cam,
            free_end: &filtered_free_end,
        });
        let mut strand_topology_reader = StrandTopologyReader::init(helices);

        for nucl in self.points.iter() {
            let instruction = strand_topology_reader.read_nucl(*nucl);
            strand_vertex_builder.draw(instruction);
        }
        if let Some(instruction) = strand_topology_reader.finish(&filtered_free_end) {
            strand_vertex_builder.draw(instruction);
        }

        let (path, cross_split_path) = strand_vertex_builder.build();
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
                        highlight: None,
                    },
                ),
            )
            .expect("Error durring tessellation");
        vertices
    }

    pub fn highlighted(&self, color: u32, highlight_thickness: f32) -> Self {
        Self {
            color,
            highlight: Some(highlight_thickness),
            points: self.points.clone(),
            insertions: self.insertions.clone(),
            ..*self
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
    highlight: Option<f32>,
}

const THINNING_POWER: f32 = 1.3;
const MINIMUM_THICKNESS: f32 = 0.7;

impl StrokeVertexConstructor<StrandVertex> for WithAttributes {
    fn new_vertex(&mut self, mut vertex: StrokeVertex) -> StrandVertex {
        let mut width = (vertex.interpolated_attributes()[1] * 3.)
            .min(1.)
            .max(-1.)
            .abs()
            .powf(THINNING_POWER)
            .max(MINIMUM_THICKNESS);
        if let Some(thickness) = self.highlight {
            width *= thickness;
        }
        let color = self.color;

        let mut depth = if vertex.interpolated_attributes()[1] > 1.00001 {
            1e-7
        } else {
            vertex.interpolated_attributes()[0]
        };
        if let Some(thickness) = self.highlight {
            depth *= 0.99 + (thickness / 1000.)
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
}

impl FilteredFreeEnd {
    fn read(free_end: &Option<FreeEnd>, strand_id: usize) -> Option<Self> {
        free_end
            .as_ref()
            .filter(|f| f.strand_id == strand_id)
            .map(|free_end| Self {
                point: free_end.point,
                prime3: free_end.prime3,
            })
    }
}

/// If nucl is visible on cam2, and not on cam 1, convert the position of the nucl in cam2
/// screen coordinate then back to cam1 world coordinate
fn alternative_position(position: Vec2, cam1: &CameraPtr, cam2: &CameraPtr) -> Option<Vec2> {
    if cam1.borrow().bottom == cam2.borrow().bottom {
        None
    } else if !cam1.borrow().can_see_world_point(position)
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

struct TwoCameraAndPoints<'a> {
    point_1: Vec2,
    point_2: Vec2,
    cam_1: &'a CameraPtr,
    cam_2: &'a CameraPtr,
}

/// Return true if `a` and `b` are both visible by exactly one camera, and each camera can see
/// exactly one of the points.
#[allow(clippy::needless_lifetimes)]
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
    /// The sign attribute is used to handle the width of the path. The sign should be flipped
    /// between each extremity of a stroke that should be thin in the middle.
    sign: f32,
    main_camera: &'a CameraPtr,
    alternative_camera: &'a CameraPtr,
    main_builder_is_drawing: bool,
    /// The depth attribute is used to generate the z coordinate of the vertices
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

#[derive(Debug)]
struct MainXoverDescriptor {
    origin: Vec2,
    target: Vec2,
    normal_source: Vec2,
    normal_target: Vec2,
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
                self.depth = depth;
                self.start_drawing_on(self.last_point.expect("last point"));
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
                // We use the smallest depth between the two extremities to be above both helices
                self.depth = self.depth.min(depth_to);
                if let Some((from, to)) =
                    self.alternative_positions(self.last_point.expect("last point"), to)
                {
                    self.stop_drawing();
                    self.splited_cross_over_builder
                        .begin(Point::new(from.x, from.y), &[self.depth, 5.0]);
                    self.splited_cross_over_builder
                        .line_to(Point::new(to.x, to.y), &[self.depth, 5.0]);
                    self.splited_cross_over_builder.end(false);
                } else {
                    let origin = self.last_point.expect("last point");
                    if self.can_see(to) || self.can_see(origin) {
                        self.start_drawing_on(origin);
                        self.draw_xover_with_main_builder(MainXoverDescriptor {
                            target: to,
                            origin,
                            normal_source,
                            normal_target,
                        })
                    } else {
                        // We do not draw cross overs whose extremities are both out of sight
                        self.stop_drawing()
                    }
                }
                self.depth = depth_to;
                self.last_point = Some(to);
            }
            DrawingInstruction::FreeEndPrime3(to) => {
                if let Some(from) = self.last_point.take() {
                    self.draw_free_end(from, to);
                }
            }
        }
    }

    fn draw_xover_with_main_builder(&mut self, xover: MainXoverDescriptor) {
        // We flip the sign so that the curve will be thin in its middle
        self.sign *= -1.0;

        let dist = (xover.target - xover.origin).mag();
        let normal_1 = (xover.normal_source - xover.origin).normalized();
        let normal_2 = (xover.normal_target - xover.target).normalized();
        let control_1 = xover.origin + (dist.sqrt() / 2.) * normal_1;
        let control_2 = xover.target + (dist.sqrt() / 2.) * normal_2;
        let target = xover.target;
        self.main_path_builder.cubic_bezier_to(
            point!(control_1),
            point!(control_2),
            point!(target),
            attributes!(self),
        );
    }

    fn can_see(&self, point: Vec2) -> bool {
        self.main_camera.borrow().can_see_world_point(point)
            || self.alternative_camera.borrow().can_see_world_point(point)
    }

    fn draw_free_end(&mut self, from: Vec2, to: Vec2) {
        if let Some((from, to)) = self.alternative_positions(from, to) {
            self.splited_cross_over_builder
                .begin(Point::new(from.x, from.y), attributes!(self));
            self.splited_cross_over_builder
                .line_to(Point::new(to.x, to.y), attributes!(self));
            self.splited_cross_over_builder.end(false);
        } else {
            self.depth = 1e-4;
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
            Some((
                self.alternative_position_one_point(from),
                self.alternative_position_one_point(to),
            ))
        } else {
            None
        }
    }

    fn alternative_position_one_point(&self, point: Vec2) -> Vec2 {
        alternative_position(point, self.main_camera, self.alternative_camera).unwrap_or(point)
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

    pub fn build(mut self) -> (Path, Path) {
        self.stop_drawing();
        (
            self.main_path_builder.build(),
            self.splited_cross_over_builder.build(),
        )
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
                // we are drawing two consecutives xovers on the same helix, link them with a
                // crossover
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
        let normal_source = self.helices[last_nucl.helix]
            .get_nucl_position(&last_nucl.prime5(), Shift::Prime3Outsided);
        let normal_target =
            self.helices[nucl.helix].get_nucl_position(&nucl.prime3(), Shift::Prime5Outsided);
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
