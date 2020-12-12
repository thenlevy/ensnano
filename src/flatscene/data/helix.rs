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
use ultraviolet::{Isometry2, Mat2, Vec2, Vec4};

type Vertices = lyon::tessellation::VertexBuffers<GpuVertex, u16>;

const CIRCLE_WIDGET_RADIUS: f32 = 1.5;

#[derive(Debug, Clone)]
pub struct Helix {
    /// The first drawn nucleotide
    left: isize,
    /// The first nucleotide that is not drawn
    right: isize,
    pub isometry: Isometry2,
    old_isometry: Isometry2,
    scale: f32,
    color: u32,
    z_index: i32,
    stroke_width: f32,
    /// The position of self in the Helix vector of the design
    pub id: u32,
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
    pub fn new(left: isize, right: isize, isometry: Isometry2, id: u32) -> Self {
        Self {
            left,
            right,
            isometry,
            old_isometry: isometry,
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
        if click.y <= 0. || click.y >= 2. {
            None
        } else {
            let ret = self.get_click_unbounded(x, y);
            Some(ret).filter(|(position, _)| *position >= self.left && *position <= self.right)
        }
    }

    /// Project a click on the helix's axis, and return the corresponding nucleotide
    /// Do not take the left and right bound into account.
    pub fn get_click_unbounded(&self, x: f32, y: f32) -> (isize, bool) {
        let click = {
            let ret = Vec2::new(x, y);
            let iso = self.isometry.inversed().into_homogeneous_matrix();
            iso.transform_point2(ret)
        };
        let forward = click.y <= 1.;
        let position = click.x.floor() as isize;
        (position, forward)
    }

    /// Return true if (x, y) is on the circle representing self
    pub fn click_on_circle(&self, x: f32, y: f32, camera: &CameraPtr) -> bool {
        if let Some(center) = self.get_circle(camera) {
            (center.center - Vec2::new(x, y)).mag() < CIRCLE_WIDGET_RADIUS
        } else {
            false
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

    fn x_position(&self, x: f32) -> Vec2 {
        let local_position = x * Vec2::unit_x() + Vec2::unit_y();

        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * local_position)
    }

    fn num_position_top(&self, x: isize, width: f32, height: f32) -> Vec2 {
        let center_nucl = (x as f32 + 0.5) * Vec2::unit_x();

        let center_text = center_nucl - height / 2. * Vec2::unit_y();

        let real_center = self
            .isometry
            .into_homogeneous_matrix()
            .transform_point2(center_text);

        let angle_sin = Vec2::unit_y().dot(Vec2::unit_x().rotated_by(self.isometry.rotation));

        real_center + ((angle_sin - width) / 2.) * Vec2::unit_x() - height / 2. * Vec2::unit_y()
    }

    /// Return the center of the helix's circle widget.
    ///
    /// If the helix is invisible return None.
    ///
    /// If the helix is visible, the circle widget is displayed, by order of priority:
    /// * On the left of the helix,
    /// * On the right of the helix,
    /// * On the leftmost visible position of the helix
    pub fn get_circle(&self, camera: &CameraPtr) -> Option<CircleInstance> {
        let (left, right) = self.screen_intersection(camera)?;
        let center = if self.left as f32 > right || (self.right as f32) < left {
            // the helix is invisible
            None
        } else if self.left as f32 - 1. - 2. * CIRCLE_WIDGET_RADIUS > left {
            // There is room on the left of the helix
            Some(self.x_position(self.left as f32 - 1. - CIRCLE_WIDGET_RADIUS))
        } else if self.right as f32 + 2. + 2. * CIRCLE_WIDGET_RADIUS < right {
            // There is room on the right of the helix
            Some(self.x_position(self.right as f32 + 2. + CIRCLE_WIDGET_RADIUS))
        } else {
            Some(self.x_position(left + CIRCLE_WIDGET_RADIUS))
        };
        center.map(|c| CircleInstance::new(c, CIRCLE_WIDGET_RADIUS))
    }

    /// Return the pivot under the center of the helix's circle widget.
    /// See [get_circle](get_circle).
    pub fn get_circle_pivot(&self, camera: &CameraPtr) -> Option<Nucl> {
        let (left, right) = self.screen_intersection(camera)?;
        if self.left as f32 > right || (self.right as f32) < left {
            // the helix is invisible
            None
        } else if self.left as f32 - 1. - 2. * CIRCLE_WIDGET_RADIUS > left {
            // There is room on the left of the helix
            Some(Nucl {
                position: self.left - 3,
                helix: self.id as usize,
                forward: true,
            })
        } else if self.right as f32 + 2. + 2. * CIRCLE_WIDGET_RADIUS < right {
            Some(Nucl {
                position: self.left - 3,
                helix: self.id as usize,
                forward: true,
            })
        } else {
            Some(Nucl {
                position: self.left,
                helix: self.id as usize,
                forward: true,
            })
        }
    }

    /// Return the center of the visible portion of the helix. Return None if the helix is
    /// invisible (out of screen)
    pub fn visible_center(&self, camera: &CameraPtr) -> Option<Vec2> {
        let (left, right) = self.screen_intersection(camera)?;
        if self.left as f32 > right || (self.right as f32) < left {
            return None;
        }
        let left = left.max(self.left as f32);
        let right = right.min((self.right + 1) as f32);
        let local_position = (left + right) / 2. * Vec2::unit_x() + Vec2::unit_y();

        Some(
            self.isometry
                .into_homogeneous_matrix()
                .transform_point2(self.scale * local_position),
        )
    }

    pub fn make_visible(&self, position: isize, camera: CameraPtr) {
        let intersection = self.screen_intersection(&camera);
        let need_center = if let Some((left, right)) = intersection {
            left.floor() as isize > position || (right.ceil() as isize) < position
        } else {
            true
        };
        if need_center {
            camera.borrow_mut().set_center(self.get_pivot(position))
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
            let mut advances =
                crate::utils::chars2d::char_positions(self.id.to_string(), char_drawers);
            let mut height = crate::utils::chars2d::height(self.id.to_string(), char_drawers);
            if camera.borrow().get_globals().zoom < 7. {
                height *= 2.;
                for x in advances.iter_mut() {
                    *x *= 2.;
                }
            }
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
            let mut advances = crate::utils::chars2d::char_positions(pos.to_string(), char_drawers);
            let mut height = crate::utils::chars2d::height(pos.to_string(), char_drawers);
            if camera.borrow().get_globals().zoom < 7. {
                height *= 2.;
                for x in advances.iter_mut() {
                    *x *= 2.;
                }
            }
            let x_shift = if pos >= 0 { 0. } else { advances[1] };
            for (c_idx, c) in pos.to_string().chars().enumerate() {
                let instances = char_map.get_mut(&c).unwrap();
                let center = self.num_position_top(pos, advances[nb_chars] * scale, height * scale);
                instances.push(CharInstance {
                    center: center + (x_shift + advances[c_idx] * scale) * Vec2::unit_x(),
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

    /// Return the coordinates at which self's axis intersect the screen bounds.
    fn screen_intersection(&self, camera: &CameraPtr) -> Option<(f32, f32)> {
        let mut ret = Vec::new();
        let x0_screen = {
            let world = self.x_position(0_f32);
            camera.borrow().world_to_norm_screen(world.x, world.y)
        };
        let x1_screen = {
            let world = self.x_position(1_f32);
            camera.borrow().world_to_norm_screen(world.x, world.y)
        };
        let on_segment = |(_, t): &(f32, f32)| *t >= 0. && *t <= 1.;
        if let Some((s, _)) = line_intersect(
            x0_screen.into(),
            x1_screen.into(),
            (0., 0.).into(),
            (0., 1.).into(),
        )
        .filter(on_segment)
        {
            ret.push(s);
        }
        // By computing the intersection in this order we avoid any issues that we might have with
        // the diagonals and anti-diagonals
        if let Some((s, _)) = line_intersect(
            x0_screen.into(),
            x1_screen.into(),
            (1., 0.).into(),
            (1., 1.).into(),
        )
        .filter(on_segment)
        {
            ret.push(s);
        }
        if let Some((s, _)) = line_intersect(
            x0_screen.into(),
            x1_screen.into(),
            (0., 0.).into(),
            (1., 0.).into(),
        )
        .filter(on_segment)
        {
            ret.push(s);
        }
        if let Some((s, _)) = line_intersect(
            x0_screen.into(),
            x1_screen.into(),
            (0., 1.).into(),
            (1., 1.).into(),
        )
        .filter(on_segment)
        {
            ret.push(s);
        }
        if ret.len() < 2 {
            None
        } else {
            Some((ret[0].min(ret[1]), ret[0].max(ret[1])))
        }
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

/// Return (s, t) so that u0 + s(v0 - u0) = u1 + t(v1 - u1).
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
        Some((t, s))
    } else {
        None
    }
}
