use super::super::view::{CharInstance, CircleInstance};
use super::super::CameraPtr;
use super::{Helix2d, Nucl};
use crate::consts::*;
use crate::utils::instance::Instance;
use lyon::math::{rect, Point};
use lyon::path::builder::{BorderRadii, PathBuilder};
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::{
    FillVertex, FillVertexConstructor, StrokeVertex, StrokeVertexConstructor,
};
use std::collections::HashMap;
use ultraviolet::{Isometry2, Mat2, Rotor2, Vec2, Vec4};

type Vertices = lyon::tessellation::VertexBuffers<GpuVertex, u16>;

#[derive(Clone)]
pub struct Helix {
    /// The first drawn nucleotide
    left: isize,
    /// The first nucleotide that is not drawn
    right: isize,
    isometry: Isometry2,
    old_isometry: Isometry2,
    scale: f32,
    color: u32,
    z_index: i32,
    stroke_width: f32,
    id: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HelixModel {
    color: Vec4,       // padding 0
    position: Vec2,    // padding 2
    rotation: Mat2,    // padding 2
    z_index: i32,      // padding 3
    stroke_width: f32, // padding 0
}

unsafe impl bytemuck::Zeroable for HelixModel {}
unsafe impl bytemuck::Pod for HelixModel {}

impl Helix {
    pub fn new(left: isize, right: isize, position: Vec2, id: u32) -> Self {
        Self {
            left,
            right,
            isometry: Isometry2::new(position, Rotor2::identity()),
            old_isometry: Isometry2::new(position, Rotor2::identity()),
            scale: 1f32,
            color: HELIX_BORDER_COLOR,
            z_index: 500,
            stroke_width: 0.01,
            id,
        }
    }

    pub fn update(&mut self, helix2d: &Helix2d) {
        self.left = self.left.min(helix2d.left);
        self.right = self.right.max(helix2d.right);
    }

    pub fn background_vertices(&self) -> Vertices {
        let mut vertices = Vertices::new();
        let left = self.left as f32;
        let right = self.right as f32 + 1.;
        let top = 0.;
        let bottom = 2.;
        let mut fill_tess = lyon::tessellation::FillTessellator::new();

        let mut builder = Path::builder();
        builder.add_rounded_rectangle(
            &rect(left, top, right - left, bottom - top),
            &BorderRadii::new(0.1),
            lyon::tessellation::path::Winding::Positive,
        );
        let path = builder.build();
        fill_tess
            .tessellate_path(
                &path,
                &tessellation::FillOptions::default(),
                &mut tessellation::BuffersBuilder::new(
                    &mut vertices,
                    WithAttribute(VertexAttribute {
                        id: self.id,
                        background: true,
                    }),
                ),
            )
            .expect("error durring tessellation");
        vertices
    }

    pub fn to_vertices(&self) -> Vertices {
        let mut vertices = Vertices::new();
        let left = self.left as f32;
        let right = self.right as f32 + 1.;
        let top = 0.;
        let bottom = 2.;

        let mut stroke_tess = lyon::tessellation::StrokeTessellator::new();

        let mut builder = Path::builder();

        builder.add_rounded_rectangle(
            &rect(left, top, right - left, bottom - top),
            &BorderRadii::new(0.1),
            lyon::tessellation::path::Winding::Positive,
        );
        for i in (self.left + 1)..=self.right {
            builder.begin(Point::new(i as f32, 0.));
            builder.line_to(Point::new(i as f32, 2.));
            builder.end(false);
        }
        builder.begin(Point::new(left, 1.));
        builder.line_to(Point::new(right, 1.));
        builder.end(false);
        let path = builder.build();
        stroke_tess
            .tessellate_path(
                &path,
                &tessellation::StrokeOptions::default(),
                &mut tessellation::BuffersBuilder::new(
                    &mut vertices,
                    WithAttribute(VertexAttribute {
                        id: self.id,
                        background: false,
                    }),
                ),
            )
            .expect("error durring tessellation");
        vertices
    }

    pub fn model(&self) -> HelixModel {
        HelixModel {
            color: Instance::color_from_u32(self.color),
            position: self.isometry.translation,
            rotation: self.isometry.rotation.into_matrix(),
            z_index: self.z_index,
            stroke_width: self.stroke_width,
        }
    }

    /// Return the position of the nucleotide in the 2d drawing
    pub fn get_nucl_position(&self, nucl: &Nucl, last: bool) -> Vec2 {
        let local_position = nucl.position as f32 * Vec2::unit_x()
            + if last {
                if nucl.forward {
                    // on the right and below the center
                    Vec2::new(0.7, 0.6)
                } else {
                    // on the left and above the center
                    Vec2::new(0.3, 1.4)
                }
            } else {
                if nucl.forward {
                    // on the left and below the center
                    Vec2::new(0.3, 0.6)
                } else {
                    // on the right and above the center
                    Vec2::new(0.7, 1.4)
                }
            };

        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * local_position)
    }

    /// Return the position at which the 3' tick should end
    pub fn get_arrow_end(&self, nucl: &Nucl) -> Vec2 {
        let local_position = nucl.position as f32 * Vec2::unit_x()
            + if nucl.forward {
                Vec2::new(0.2, 0.3)
            } else {
                Vec2::new(0.8, 1.7)
            };

        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * local_position)
    }

    fn get_old_pivot_position(&self, nucl: &Nucl) -> Vec2 {
        let local_position = nucl.position as f32 * Vec2::unit_x()
            + if nucl.forward {
                Vec2::zero()
            } else {
                Vec2::unit_y()
            };

        self.old_isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * local_position)
    }

    /// Return the nucleotide displayed at position (x, y) or None if (x, y) is outside the helix
    pub fn get_click(&self, x: f32, y: f32) -> Option<(isize, bool)> {
        let click = {
            let ret = Vec2::new(x, y);
            let iso = self.isometry.inversed().into_homogeneous_matrix();
            iso.transform_point2(ret)
        };
        let forward = if click.y >= 0. && click.y <= 1. {
            Some(true)
        } else if click.y >= 1. && click.y <= 2. {
            Some(false)
        } else {
            None
        }?;
        let position = click.x.floor() as isize;
        if position >= self.left && position <= self.right {
            Some((position, forward))
        } else {
            None
        }
    }

    pub fn translate(&mut self, translation: Vec2) {
        self.isometry.translation = self.old_isometry.translation + translation
    }

    /// Translate self so that the pivot ends up on position.
    pub fn snap(&mut self, pivot: Nucl, position: Vec2) {
        let position = Vec2::new(position.x.round(), position.y.round());
        let old_pos = self.get_old_pivot_position(&pivot);
        self.translate(position - old_pos)
    }

    pub fn rotate(&mut self, pivot: Vec2, angle: f32) {
        let angle = {
            let k = (angle / std::f32::consts::FRAC_PI_8).round();
            k * std::f32::consts::FRAC_PI_8
        };
        self.isometry = self.old_isometry;
        self.isometry.append_translation(-pivot);
        self.isometry
            .append_rotation(ultraviolet::Rotor2::from_angle(angle));
        self.isometry.append_translation(pivot);
    }

    pub fn get_pivot(&self, position: isize) -> Vec2 {
        self.isometry * (self.scale * Vec2::new(position as f32, 1.))
    }

    pub fn end_movement(&mut self) {
        self.old_isometry = self.isometry
    }

    pub fn set_color(&mut self, color: u32) {
        self.color = color
    }

    pub fn get_depth(&self) -> f32 {
        self.z_index as f32 + self.id as f32 / 1000.
    }

    pub fn move_forward(&mut self) {
        self.z_index -= 1;
    }

    pub fn move_backward(&mut self) {
        self.z_index += 1;
    }

    fn x_position(&self, x: isize) -> Vec2 {
        let local_position = x as f32 * Vec2::unit_x() + Vec2::unit_y();

        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * local_position)
    }

    fn num_position_top(&self, x: isize) -> Vec2 {
        let local_position =
            x as f32 * Vec2::unit_x() + 0.5 * Vec2::unit_x() - 0.1 * Vec2::unit_y();

        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * local_position)
    }

    pub fn get_circle(&self, camera: &CameraPtr) -> Option<CircleInstance> {
        let globals = camera.borrow().get_globals().clone();
        let leftmost_position = self.x_position(self.left);
        let rightmost_position = self.x_position(self.right);
        let to_screen = |vec: Vec2| {
            let temp = vec - Vec2::new(globals.scroll_offset[0], globals.scroll_offset[1]);
            Vec2::new(
                temp.x * 2. * globals.zoom / globals.resolution[0],
                -temp.y * 2. * globals.zoom / globals.resolution[1],
            )
        };
        let in_screen = |vec: Vec2| vec.x <= 1. && vec.y <= 1. && vec.x >= -1. && vec.y >= -1.;

        let left_screen = to_screen(leftmost_position);
        let right_screen = to_screen(rightmost_position);
        let visible = segment_intersect(
            left_screen,
            right_screen,
            Vec2::new(-1., -1.),
            Vec2::new(1., -1.),
        ) || segment_intersect(
            left_screen,
            right_screen,
            Vec2::new(-1., -1.),
            Vec2::new(-1., 1.),
        ) || segment_intersect(
            left_screen,
            right_screen,
            Vec2::new(-1., 1.),
            Vec2::new(1., 1.),
        ) || segment_intersect(
            left_screen,
            right_screen,
            Vec2::new(1., -1.),
            Vec2::new(1., 1.),
        ) || in_screen(to_screen(self.x_position(self.left)));

        if visible {
            let left_candidate = self.x_position(self.left - 2);
            let right_candidate = self.x_position(self.right + 3);

            if in_screen(to_screen(left_candidate)) {
                Some(CircleInstance {
                    center: left_candidate,
                })
            } else if in_screen(to_screen(right_candidate)) {
                Some(CircleInstance {
                    center: right_candidate,
                })
            } else if let Some((_, t)) = line_intersect(
                left_screen,
                right_screen,
                Vec2::new(-1., -1.),
                Vec2::new(-1., 1.),
            )
            .filter(|(s, _)| *s >= 0. && *s <= 1.)
            {
                let candidate =
                    self.x_position(self.left + 1) + t * (rightmost_position - leftmost_position);
                if in_screen(to_screen(candidate)) {
                    Some(CircleInstance { center: candidate })
                } else {
                    Some(CircleInstance {
                        center: self.x_position(self.left - 2)
                            + t * (rightmost_position - leftmost_position),
                    })
                }
            } else if let Some((_, t)) = line_intersect(
                left_screen,
                right_screen,
                Vec2::new(-1., 1.),
                Vec2::new(1., 1.),
            ) {
                let candidate =
                    self.x_position(self.left + 1) + t * (rightmost_position - leftmost_position);
                if in_screen(to_screen(candidate)) {
                    Some(CircleInstance { center: candidate })
                } else {
                    Some(CircleInstance {
                        center: self.x_position(self.left - 2)
                            + t * (rightmost_position - leftmost_position),
                    })
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn add_char_instances(
        &self,
        camera: &CameraPtr,
        char_map: &mut HashMap<char, Vec<CharInstance>>,
        char_drawers: &HashMap<char, crate::utils::chars2d::CharDrawer>,
    ) {
        let size_id = 3.;
        let size_pos = 1.4;
        let circle = self.get_circle(camera);
        if let Some(circle) = circle {
            let nb_chars = self.id.to_string().len(); // ok to use len because digits are ascii
            let scale = size_id / nb_chars as f32;
            let advances = crate::utils::chars2d::char_positions(self.id.to_string(), char_drawers);
            let height = crate::utils::chars2d::height(self.id.to_string(), char_drawers);
            let x_shift = -advances[nb_chars] / 2. * scale;
            for (c_idx, c) in self.id.to_string().chars().enumerate() {
                let instances = char_map.get_mut(&c).unwrap();
                instances.push(CharInstance {
                    center: circle.center + (x_shift + advances[c_idx] * scale) * Vec2::unit_x()
                        - scale * height / 2. * Vec2::unit_y(),
                    rotation: self.isometry.rotation.into_matrix(),
                    size: scale,
                    z_index: -1,
                })
            }
        }

        let mut print_pos = |pos: isize| {
            let nb_chars = pos.to_string().len(); // ok to use len because digits are ascii
            let scale = size_pos;
            let advances = crate::utils::chars2d::char_positions(pos.to_string(), char_drawers);
            let height = crate::utils::chars2d::height(pos.to_string(), char_drawers);
            let x_shift = if pos >= 0 {
                -advances[nb_chars] / 2. * scale
            } else {
                (advances[1] - advances[nb_chars] / 2.) * scale
            };
            for (c_idx, c) in pos.to_string().chars().enumerate() {
                let instances = char_map.get_mut(&c).unwrap();
                instances.push(CharInstance {
                    center: self.num_position_top(pos)
                        + (x_shift + advances[c_idx] * scale) * Vec2::unit_x()
                        - scale * height * Vec2::unit_y(),
                    rotation: self.isometry.rotation.into_matrix(),
                    size: scale,
                    z_index: -1,
                })
            }
        };

        let mut pos = 0;
        while pos <= self.right {
            print_pos(pos);
            pos += 5;
        }
        pos = -5;
        while pos >= self.left {
            print_pos(pos);
            pos -= 5;
        }
    }

    pub fn get_left(&self) -> isize {
        self.left
    }

    pub fn get_right(&self) -> isize {
        self.right
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GpuVertex {
    position: [f32; 2],
    normal: [f32; 2],
    prim_id: u32,
    background: u32,
}
unsafe impl bytemuck::Pod for GpuVertex {}
unsafe impl bytemuck::Zeroable for GpuVertex {}

struct VertexAttribute {
    id: u32,
    background: bool,
}

/// This vertex constructor forwards the positions and normals provided by the
/// tessellators and add a shape id.
struct WithAttribute(VertexAttribute);

impl StrokeVertexConstructor<GpuVertex> for WithAttribute {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position_on_path().to_array(),
            normal: vertex.normal().to_array(),
            prim_id: self.0.id,
            background: self.0.background as u32,
        }
    }
}

impl FillVertexConstructor<GpuVertex> for WithAttribute {
    fn new_vertex(&mut self, vertex: FillVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position().to_array(),
            normal: [0., 0.],
            prim_id: self.0.id,
            background: self.0.background as u32,
        }
    }
}

fn segment_intersect(u0: Vec2, v0: Vec2, u1: Vec2, v1: Vec2) -> bool {
    if let Some((s, t)) = line_intersect(u0, v0, u1, v1) {
        s >= 0. && s <= 1. && t >= 0. && t <= 1.
    } else {
        false
    }
}

fn line_intersect(u0: Vec2, v0: Vec2, u1: Vec2, v1: Vec2) -> Option<(f32, f32)> {
    let v0 = v0 - u0;
    let v1 = v1 - u1;
    let x00 = u0.x;
    let y00 = u0.y;
    let x10 = u1.x;
    let y10 = u1.y;
    let x01 = v0.x;
    let y01 = v0.y;
    let x11 = v1.x;
    let y11 = v1.y;
    let d = x11 * y01 - x01 * y11;
    if d.abs() > 1e-5 {
        let s = (1. / d) * ((x00 - x10) * y01 - (y00 - y10) * x01);
        let t = (1. / d) * -(-(x00 - x10) * y11 + (y00 - y10) * x11);
        Some((s, t))
    } else {
        None
    }
}
