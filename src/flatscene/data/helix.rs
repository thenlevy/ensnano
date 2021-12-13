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
use super::super::view::{CharInstance, CircleInstance, InsertionInstance};
use super::super::{CameraPtr, Flat, FlatHelix, FlatIdx};
use super::{FlatNucl, Helix2d};
use crate::consts::*;
use crate::flatscene::view::EditionInfo;
use crate::utils::instance::Instance;
use ahash::RandomState;
use ensnano_design::Nucl;
use lyon::math::{rect, Point};
use lyon::path::builder::{BorderRadii, PathBuilder};
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::{
    FillVertex, FillVertexConstructor, StrokeVertex, StrokeVertexConstructor,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use ultraviolet::{Isometry2, Mat2, Rotor2, Vec2, Vec4};

type Vertices = lyon::tessellation::VertexBuffers<GpuVertex, u16>;

const CIRCLE_WIDGET_RADIUS: f32 = 1.5;
const ZOOM_THRESHOLD: f32 = 7.0;

#[derive(Debug, Clone)]
pub struct Helix {
    /// The first drawn nucleotide
    left: isize,
    /// The first nucleotide that is not drawn
    right: isize,
    pub isometry: Isometry2,
    scale: f32,
    color: u32,
    z_index: i32,
    stroke_width: f32,
    /// The position of self in the Helix vector of the design
    pub flat_id: FlatHelix,
    pub real_id: usize,
    pub visible: bool,
}

impl Flat for Helix {}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HelixModel {
    color: Vec4,       // padding 0
    position: Vec2,    // padding 2
    rotation: Mat2,    // padding 2
    z_index: i32,      // padding 3
    stroke_width: f32, // padding 0
}

impl Helix {
    pub fn new(
        left: isize,
        right: isize,
        isometry: Isometry2,
        flat_id: FlatHelix,
        real_id: usize,
        visible: bool,
        _basis_map: Arc<HashMap<Nucl, char, RandomState>>,
        _groups: Arc<BTreeMap<usize, bool>>,
    ) -> Self {
        Self {
            left,
            right,
            isometry,
            scale: 1f32,
            color: HELIX_BORDER_COLOR,
            z_index: 500,
            stroke_width: 0.01,
            flat_id,
            real_id,
            visible,
        }
    }

    pub fn update(&mut self, helix2d: &Helix2d, id_map: &HashMap<usize, FlatIdx>) {
        self.left = self.left.min(helix2d.left);
        self.right = self.right.max(helix2d.right);
        self.visible = helix2d.visible;
        self.real_id = helix2d.id;
        if let Some(flat_id) = FlatHelix::from_real(self.real_id, id_map) {
            self.flat_id = flat_id
        } else {
            log::error!("real id does not exist {}", self.real_id);
        }
        self.isometry = helix2d.isometry;
    }

    pub fn background_vertices(&self) -> Vertices {
        let mut vertices = Vertices::new();
        let left = self.left as f32;
        let right = self.right.max(self.left + 1) as f32 + 1.;
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
                        id: self.flat_id.flat.0 as u32,
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
        let right = self.right.max(self.left + 1) as f32 + 1.;
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
                        id: self.flat_id.flat.0 as u32,
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
    pub fn get_nucl_position(&self, nucl: &FlatNucl, shift: Shift) -> Vec2 {
        let local_position = nucl.position as f32 * Vec2::unit_x()
            + match shift {
                Shift::Prime3 => {
                    if nucl.forward {
                        // on the right and below the center
                        Vec2::new(0.7, 0.6)
                    } else {
                        // on the left and above the center
                        Vec2::new(0.3, 1.4)
                    }
                }
                Shift::Prime5 => {
                    if nucl.forward {
                        // on the left and below the center
                        Vec2::new(0.3, 0.6)
                    } else {
                        // on the right and above the center
                        Vec2::new(0.7, 1.4)
                    }
                }
                Shift::Prime5Outsided => {
                    if nucl.forward {
                        // on the left and below the center
                        Vec2::new(0.3, -0.2)
                    } else {
                        // on the right and above the center
                        Vec2::new(0.7, 2.2)
                    }
                }
                Shift::Prime3Outsided => {
                    if nucl.forward {
                        // on the right and below the center
                        Vec2::new(0.7, -0.2)
                    } else {
                        // on the left and above the center
                        Vec2::new(0.3, 2.2)
                    }
                }
                Shift::No => {
                    if nucl.forward {
                        Vec2::new(0.5, 0.5)
                    } else {
                        Vec2::new(0.5, 1.5)
                    }
                }
            };
        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * local_position)
    }

    /// Return the position at which the 3' tick should end
    pub fn get_arrow_end(&self, nucl: &FlatNucl) -> Vec2 {
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

    /*
    fn get_old_pivot_position(&self, nucl: &FlatNucl) -> Vec2 {
        let local_position = nucl.position as f32 * Vec2::unit_x()
            + if nucl.forward {
                Vec2::zero()
            } else {
                Vec2::unit_y()
            };

        self.old_isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * local_position)
    }*/

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

    pub fn move_handle(&mut self, handle: HelixHandle, position: Vec2) -> (isize, isize) {
        let (pos, _) = self.get_click_unbounded(position.x, position.y);
        match handle {
            HelixHandle::Left => self.left = (self.right - 2).min(pos + 1),
            HelixHandle::Right => self.right = (self.left + 2).max(pos - 1),
        }
        (self.left, self.right)
    }

    pub fn reset_handle(&mut self, handle: HelixHandle) -> (isize, isize) {
        match handle {
            HelixHandle::Left => self.left = self.right - 1,
            HelixHandle::Right => self.right = self.left + 1,
        };
        (self.left, self.right)
    }

    pub fn redim_zero(&mut self) -> (isize, isize) {
        let (left, right) = (self.right - 1, self.left + 1);
        self.left = left;
        self.right = right;
        (left, right)
    }

    pub fn click_on_handle(&self, x: f32, y: f32) -> Option<HelixHandle> {
        let click = {
            let ret = Vec2::new(x, y);
            let iso = self.isometry.inversed().into_homogeneous_matrix();
            iso.transform_point2(ret)
        };
        if click.y <= 0. || click.y >= 2. {
            None
        } else {
            let ret = self.get_click_unbounded(x, y);
            if ret.0 == self.left - 1 {
                Some(HelixHandle::Left)
            } else if ret.0 == self.right + 1 {
                Some(HelixHandle::Right)
            } else {
                None
            }
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
        if let Some(center) = self.get_circle(camera, &BTreeMap::new()) {
            (center.center - Vec2::new(x, y)).mag() < center.radius
        } else {
            false
        }
    }

    /*
    pub fn translate(&mut self, translation: Vec2) {
        self.isometry.translation = self.old_isometry.translation + translation
    }
    */

    /*
    /// Translate self so that the pivot ends up on position.
    pub fn snap(&mut self, pivot: FlatNucl, translation: Vec2) {
        let old_pos = self.get_old_pivot_position(&pivot);
        let position = old_pos + translation;
        let position = Vec2::new(position.x.round(), position.y.round());
        self.translate(position - old_pos)
    }*/

    /*
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
    }*/

    pub fn get_pivot(&self, position: isize) -> Vec2 {
        self.isometry * (self.scale * Vec2::new(position as f32, 1.))
    }

    /*
    pub fn end_movement(&mut self) {
        self.old_isometry = self.isometry
    }*/

    pub fn set_color(&mut self, color: u32) {
        self.color = color
    }

    pub fn get_depth(&self) -> f32 {
        self.z_index as f32 + self.flat_id.flat.0 as f32 / 1000.
    }

    pub fn move_forward(&mut self) {
        self.z_index -= 1;
    }

    pub fn move_backward(&mut self) {
        self.z_index += 1;
    }

    fn x_position(&self, x: f32, line: HelixLine) -> Vec2 {
        let local_position = x * Vec2::unit_x() + line.adjustment();

        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * local_position)
    }

    fn num_position_top(&self, x: isize, width: f32, height: f32, show_seq: bool) -> Vec2 {
        let center_nucl = (x as f32 + 0.5) * Vec2::unit_x();

        let center_text = if show_seq {
            center_nucl - 3. * height / 2. * Vec2::unit_y()
        } else {
            center_nucl - height / 2. * Vec2::unit_y()
        };

        let real_center = self
            .isometry
            .into_homogeneous_matrix()
            .transform_point2(center_text);

        let angle_sin = Vec2::unit_y().dot(Vec2::unit_x().rotated_by(self.isometry.rotation));

        real_center + ((angle_sin - width) / 2.) * Vec2::unit_x() - height / 2. * Vec2::unit_y()
    }

    fn char_position_top(&self, x: isize, width: f32, height: f32) -> Vec2 {
        let center_nucl = (x as f32 + 0.5) * Vec2::unit_x();

        let center_text = center_nucl - height / 2. * Vec2::unit_y();

        let real_center = self
            .isometry
            .into_homogeneous_matrix()
            .transform_point2(center_text);

        let angle_sin = Vec2::unit_y().dot(Vec2::unit_x().rotated_by(self.isometry.rotation));

        real_center + ((angle_sin - width) / 2.) * Vec2::unit_x() - height / 2. * Vec2::unit_y()
    }

    fn char_position_bottom(&self, x: isize, width: f32, height: f32) -> Vec2 {
        let center_nucl = (x as f32 + 0.5) * Vec2::unit_x();

        let center_text = center_nucl + (2. + height / 2.) * Vec2::unit_y();

        let real_center = self
            .isometry
            .into_homogeneous_matrix()
            .transform_point2(center_text);

        let angle_sin = Vec2::unit_y().dot(Vec2::unit_x().rotated_by(self.isometry.rotation));

        real_center + ((angle_sin - width) / 2.) * Vec2::unit_x() - height / 2. * Vec2::unit_y()
    }

    pub fn handle_circles(&self) -> Vec<CircleInstance> {
        let top_left_pos = self.get_nucl_position(
            &FlatNucl {
                helix: self.flat_id,
                position: self.left - 1,
                forward: true,
            },
            Shift::No,
        );
        let bottom_left_pos = self.get_nucl_position(
            &FlatNucl {
                helix: self.flat_id,
                position: self.left - 1,
                forward: false,
            },
            Shift::No,
        );
        let top_right_pos = self.get_nucl_position(
            &FlatNucl {
                helix: self.flat_id,
                position: self.right + 1,
                forward: true,
            },
            Shift::No,
        );
        let bottom_right_pos = self.get_nucl_position(
            &FlatNucl {
                helix: self.flat_id,
                position: self.right + 1,
                forward: false,
            },
            Shift::No,
        );
        vec![
            CircleInstance::new(top_left_pos, 0.5, self.flat_id.flat.0 as i32, CIRCLE2D_GREY),
            CircleInstance::new(
                bottom_left_pos,
                0.5,
                self.flat_id.flat.0 as i32,
                CIRCLE2D_GREY,
            ),
            CircleInstance::new(
                top_right_pos,
                0.5,
                self.flat_id.flat.0 as i32,
                CIRCLE2D_GREY,
            ),
            CircleInstance::new(
                bottom_right_pos,
                0.5,
                self.flat_id.flat.0 as i32,
                CIRCLE2D_GREY,
            ),
        ]
    }

    /// Return the center of the helix's circle widget.
    ///
    /// If the helix is invisible return None.
    ///
    /// If the helix is visible, the circle widget is displayed, by order of priority:
    /// * On the left of the helix,
    /// * On the right of the helix,
    /// * On the leftmost visible position of the helix
    pub fn get_circle(
        &self,
        camera: &CameraPtr,
        groups: &BTreeMap<usize, bool>,
    ) -> Option<CircleInstance> {
        let (left, right) = self.screen_intersection(camera)?;
        let center = if self.left as f32 > right || (self.right as f32) < left {
            // the helix is invisible
            None
        } else if self.left as f32 - 1. - 2. * CIRCLE_WIDGET_RADIUS > left {
            // There is room on the left of the helix
            Some(self.x_position(
                self.left as f32 - 1. - CIRCLE_WIDGET_RADIUS,
                HelixLine::Middle,
            ))
        } else if self.right as f32 + 2. + 2. * CIRCLE_WIDGET_RADIUS < right {
            // There is room on the right of the helix
            Some(self.x_position(
                self.right as f32 + 2. + CIRCLE_WIDGET_RADIUS,
                HelixLine::Middle,
            ))
        } else {
            Some(self.x_position(left + CIRCLE_WIDGET_RADIUS, HelixLine::Middle))
        };
        let color = if !self.visible {
            CIRCLE2D_GREY
        } else {
            match groups.get(&self.real_id) {
                None => CIRCLE2D_BLUE,
                Some(true) => CIRCLE2D_RED,
                Some(false) => CIRCLE2D_GREEN,
            }
        };
        let radius = if camera.borrow().get_globals().zoom < ZOOM_THRESHOLD {
            CIRCLE_WIDGET_RADIUS * 2.
        } else {
            CIRCLE_WIDGET_RADIUS
        };
        center.map(|c| CircleInstance::new(c, radius, self.flat_id.flat.0 as i32, color))
    }

    pub fn get_circle_nucl(&self, position: isize, forward: bool, color: u32) -> CircleInstance {
        let center = self.get_nucl_position(
            &FlatNucl {
                helix: self.flat_id,
                position,
                forward,
            },
            Shift::No,
        );
        CircleInstance::new(center, 0.4, self.flat_id.flat.0 as i32, color)
    }

    /// Return the nucl under the center of the helix's circle widget.
    /// See [get_circle](get_circle).
    pub fn get_circle_pivot(&self, camera: &CameraPtr) -> Option<FlatNucl> {
        let (left, right) = self.screen_intersection(camera)?;
        if self.left as f32 > right || (self.right as f32) < left {
            // the helix is invisible
            None
        } else if self.left as f32 - 1. - 2. * CIRCLE_WIDGET_RADIUS > left {
            // There is room on the left of the helix
            Some(FlatNucl {
                position: self.left - 3,
                helix: self.flat_id,
                forward: true,
            })
        } else if self.right as f32 + 2. + 2. * CIRCLE_WIDGET_RADIUS < right {
            Some(FlatNucl {
                position: self.left - 3,
                helix: self.flat_id,
                forward: true,
            })
        } else {
            Some(FlatNucl {
                position: self.left,
                helix: self.flat_id,
                forward: true,
            })
        }
    }

    /// A default nucleotide position for when the helix cannot be seen by the camera
    pub fn default_pivot(&self) -> FlatNucl {
        FlatNucl {
            position: self.left - 3,
            helix: self.flat_id,
            forward: true,
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

    pub fn center(&self) -> Vec2 {
        let left = self.left as f32;
        let right = (self.right + 1) as f32;
        let local_position = (left + right) / 2. * Vec2::unit_x() + Vec2::unit_y();
        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * local_position)
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

    pub fn insertion_instance(&self, nucl: &FlatNucl, color: u32) -> InsertionInstance {
        let position = self.get_nucl_position(nucl, Shift::Prime3);
        let mut orientation = self.isometry.rotation;
        if !nucl.forward {
            orientation = Rotor2::from_angle(std::f32::consts::PI) * orientation;
        }
        InsertionInstance::new(position, self.get_depth(), orientation, color)
    }

    pub fn add_char_instances(
        &self,
        camera: &CameraPtr,
        char_map: &mut HashMap<char, Vec<CharInstance>>,
        char_drawers: &HashMap<char, crate::utils::chars2d::CharDrawer>,
        groups: &BTreeMap<usize, bool>,
        basis_map: &HashMap<Nucl, char, RandomState>,
        show_seq: bool,
        edition_info: &Option<EditionInfo>,
        hovered_nucl: &Option<FlatNucl>,
    ) {
        let candidate_pos: Option<isize> = hovered_nucl
            .filter(|n| n.helix == self.flat_id)
            .map(|n| n.position);
        let show_seq = show_seq && camera.borrow().get_globals().zoom >= ZOOM_THRESHOLD;
        let size_id = 3.;
        let size_pos = 1.4;
        let circle = self.get_circle(camera, groups);
        if let Some(circle) = circle {
            let nb_chars = self.real_id.to_string().len(); // ok to use len because digits are ascii
            let scale = size_id / nb_chars as f32;
            let mut advances =
                crate::utils::chars2d::char_positions_x(&self.real_id.to_string(), char_drawers);
            let mut height = crate::utils::chars2d::height(&self.real_id.to_string(), char_drawers);
            if camera.borrow().get_globals().zoom < ZOOM_THRESHOLD {
                height *= 2.;
                for x in advances.iter_mut() {
                    *x *= 2.;
                }
            }
            let x_shift = -advances[nb_chars] / 2. * scale;
            for (c_idx, c) in self.real_id.to_string().chars().enumerate() {
                let instances = char_map.get_mut(&c).unwrap();
                instances.push(CharInstance {
                    center: circle.center + (x_shift + advances[c_idx] * scale) * Vec2::unit_x()
                        - scale * height / 2. * Vec2::unit_y(),
                    rotation: self.isometry.rotation.into_matrix(),
                    size: scale,
                    z_index: self.flat_id.flat.0 as i32,
                    color: [0., 0., 0., 1.].into(),
                })
            }
        }

        let moving_pos = edition_info
            .as_ref()
            .filter(|info| info.nucl.helix == self.flat_id)
            .map(|info| info.nucl.position);
        let mut print_pos = |pos: isize| {
            let nb_chars = pos.to_string().len(); // ok to use len because digits are ascii
            let scale = size_pos;
            let mut advances =
                crate::utils::chars2d::char_positions_x(&pos.to_string(), char_drawers);
            let mut height = crate::utils::chars2d::height(&pos.to_string(), char_drawers);
            if camera.borrow().get_globals().zoom < ZOOM_THRESHOLD {
                height *= 2.;
                for x in advances.iter_mut() {
                    *x *= 2.;
                }
            }
            let x_shift = if pos >= 0 { 0. } else { -advances[1] / 2. };
            for (c_idx, c) in pos.to_string().chars().enumerate() {
                let instances = char_map.get_mut(&c).unwrap();
                let center = self.num_position_top(
                    pos,
                    advances[nb_chars] * scale,
                    height * scale,
                    show_seq,
                );
                let color = if Some(pos) == moving_pos || candidate_pos == Some(pos) {
                    [1., 0., 0., 1.].into()
                } else {
                    [0., 0., 0., 1.].into()
                };
                instances.push(CharInstance {
                    center: center + (x_shift + advances[c_idx] * scale) * Vec2::unit_x(),
                    rotation: self.isometry.rotation.into_matrix(),
                    size: scale,
                    z_index: self.flat_id.flat.0 as i32,
                    color,
                })
            }
        };

        let mut pos = self.left;
        while pos <= self.right {
            if ((pos >= 0 && pos % 8 == 0) || (pos < 0 && -pos % 8 == 0)) && moving_pos != Some(pos)
                || candidate_pos == Some(pos)
            {
                print_pos(pos);
            }
            pos += 1;
        }
        if let Some(position) = moving_pos {
            print_pos(position);
        }

        let mut print_info = |pos: isize, info: &str| {
            let scale = size_pos;
            let advance_idx = info
                .find('/')
                .map(|n| 2 * n + 1)
                .unwrap_or_else(|| info.len()); // ok to use len because the str contains only ascii chars
            let mut advances = crate::utils::chars2d::char_positions_x(info, char_drawers);
            let mut height = crate::utils::chars2d::height(info, char_drawers);
            let mut pos_y = crate::utils::chars2d::char_positions_y(info, char_drawers);
            if camera.borrow().get_globals().zoom < ZOOM_THRESHOLD {
                height *= 2.;
                for x in advances.iter_mut() {
                    *x *= 2.;
                }
                for y in pos_y.iter_mut() {
                    *y *= 2.;
                }
            }
            for (c_idx, c) in info.chars().enumerate() {
                let instances = char_map.get_mut(&c).unwrap();
                let center =
                    self.num_position_top(pos, advances[advance_idx] * scale, height * scale, true);
                instances.push(CharInstance {
                    center: center
                        + (advances[c_idx] * scale) * Vec2::unit_x()
                        + pos_y[c_idx] * Vec2::unit_y(),
                    rotation: self.isometry.rotation.into_matrix(),
                    size: scale,
                    z_index: self.flat_id.flat.0 as i32,
                    color: [0., 0., 0., 1.].into(),
                })
            }
        };

        if let Some(building) = edition_info {
            if building.nucl.helix == self.flat_id {
                print_info(building.nucl.position, &building.to_string());
            }
        }

        let mut print_basis = |position: isize, forward: bool| {
            let scale = size_pos;
            let nucl = Nucl {
                helix: self.real_id,
                position,
                forward,
            };
            if let Some(c) = basis_map.get(&nucl) {
                let advances =
                    crate::utils::chars2d::char_positions_x(&pos.to_string(), char_drawers);
                let height = crate::utils::chars2d::height(&c.to_string(), char_drawers);
                let center = if forward {
                    self.char_position_top(position, advances[1] * scale, height * scale)
                } else {
                    self.char_position_bottom(position, advances[1] * scale, height * scale)
                };
                let instances = char_map.get_mut(&c).unwrap();
                instances.push(CharInstance {
                    center,
                    rotation: self.isometry.rotation.into_matrix(),
                    size: scale,
                    z_index: self.flat_id.flat.0 as i32,
                    color: [0., 0., 0., 1.].into(),
                })
            }
        };

        if show_seq {
            for pos in self.left..=self.right {
                print_basis(pos, true);
                print_basis(pos, false);
            }
        }
    }

    pub fn get_left(&self) -> isize {
        self.left
    }

    pub fn get_right(&self) -> isize {
        self.right
    }

    pub fn rectangle_has_nucl(
        &self,
        nucl: FlatNucl,
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        camera: &CameraPtr,
    ) -> bool {
        if let Some((x0, x1)) =
            self.screen_rectangle_intersection(camera, left, top, right, bottom, HelixLine::Middle)
        {
            if nucl.position >= x0.floor() as isize && nucl.position < x1.ceil() as isize {
                return true;
            }
        }
        if nucl.forward {
            if let Some((x0, x1)) =
                self.screen_rectangle_intersection(camera, left, top, right, bottom, HelixLine::Top)
            {
                if nucl.position >= x0.floor() as isize && nucl.position < x1.ceil() as isize {
                    return true;
                }
            }
        } else {
            if let Some((x0, x1)) = self.screen_rectangle_intersection(
                camera,
                left,
                top,
                right,
                bottom,
                HelixLine::Bottom,
            ) {
                if nucl.position >= x0.floor() as isize && nucl.position < x1.ceil() as isize {
                    return true;
                }
            }
        }
        false
    }

    /// Return the coordinates at which self's axis intersect the screen bounds.
    fn screen_intersection(&self, camera: &CameraPtr) -> Option<(f32, f32)> {
        self.screen_rectangle_intersection(camera, 0., 0., 1., 1., HelixLine::Middle)
    }

    /// Return the coordinates at which self's axis intersect a rectangle on the screen
    fn screen_rectangle_intersection(
        &self,
        camera: &CameraPtr,
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        line: HelixLine,
    ) -> Option<(f32, f32)> {
        let mut ret = Vec::new();
        let x0_screen = {
            let world = self.x_position(0_f32, line);
            camera.borrow().world_to_norm_screen(world.x, world.y)
        };
        let x1_screen = {
            let world = self.x_position(1_f32, line);
            camera.borrow().world_to_norm_screen(world.x, world.y)
        };
        let on_segment = |(_, t): &(f32, f32)| *t >= 0. && *t <= 1.;
        if let Some((s, _)) = line_intersect(
            x0_screen.into(),
            x1_screen.into(),
            (left, top).into(),
            (left, bottom).into(),
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
            (right, top).into(),
            (right, bottom).into(),
        )
        .filter(on_segment)
        {
            ret.push(s);
        }
        if let Some((s, _)) = line_intersect(
            x0_screen.into(),
            x1_screen.into(),
            (left, top).into(),
            (right, top).into(),
        )
        .filter(on_segment)
        {
            ret.push(s);
        }
        if let Some((s, _)) = line_intersect(
            x0_screen.into(),
            x1_screen.into(),
            (left, bottom).into(),
            (right, bottom).into(),
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
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    position: [f32; 2],
    normal: [f32; 2],
    prim_id: u32,
    background: u32,
}

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

pub(super) fn rectangle_intersect(rect_0: Vec2, rect_1: Vec2, a: Vec2, b: Vec2) -> bool {
    let on_segment = |t: &(f32, f32)| 0. <= t.0 && t.0 <= 1. && 0. <= t.1 && t.1 <= 1.;
    line_intersect(rect_0, Vec2::new(rect_0.x, rect_1.y), a, b)
        .filter(on_segment)
        .is_some()
        || line_intersect(rect_0, Vec2::new(rect_1.x, rect_0.y), a, b)
            .filter(on_segment)
            .is_some()
        || line_intersect(rect_1, Vec2::new(rect_0.x, rect_1.y), a, b)
            .filter(on_segment)
            .is_some()
        || line_intersect(rect_1, Vec2::new(rect_1.x, rect_0.y), a, b)
            .filter(on_segment)
            .is_some()
}

/// Represent a slight shift from the center of the square representing nucleotide
pub enum Shift {
    /// No shift, the returned point will be on the center of the nucleotide
    No,
    /// The returned point will be slightly shifted in the 5' direction
    Prime5,
    /// The returned point will be slightly shifted in the 3' direction
    Prime3,
    /// The returned point will be slightly shifted in the 5' direction outside the helix
    Prime5Outsided,
    /// The returned point will be slightly shifted in the 3' direction outside the helix
    Prime3Outsided,
}

#[derive(Debug, Clone, Copy)]
pub enum HelixLine {
    Top,
    Middle,
    Bottom,
}

impl HelixLine {
    fn adjustment(&self) -> Vec2 {
        match self {
            Self::Top => Vec2::zero(),
            Self::Middle => Vec2::unit_y(),
            Self::Bottom => 2. * Vec2::unit_y(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HelixHandle {
    Left,
    Right,
}
