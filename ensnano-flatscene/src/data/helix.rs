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
use super::super::view::{CircleInstance, InsertionDescriptor, InsertionInstance};
use super::super::{CameraPtr, Flat, FlatHelix};
use super::{FlatNucl, Helix2d, NuclCollection};
use crate::flattypes::{FlatHelixMaps, FlatPosition, HelixSegment};
use crate::view::EditionInfo;
use abcissa_converter::{AbscissaConverter, AbscissaConverter_};
use ahash::RandomState;
use ensnano_design::ultraviolet;
use ensnano_design::Nucl;
use ensnano_interactor::consts::*;
use ensnano_utils::{
    chars2d::{Line, Sentence, TextDrawer},
    full_isometry::FullIsometry,
    instance::Instance,
};
use lyon::math::{rect, Point};
use lyon::path::builder::{BorderRadii, PathBuilder};
use lyon::path::Path;
use lyon::tessellation;
use lyon::tessellation::{
    FillVertex, FillVertexConstructor, StrokeVertex, StrokeVertexConstructor,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use ultraviolet::{Mat2, Rotor2, Vec2, Vec4};

type Vertices = lyon::tessellation::VertexBuffers<GpuVertex, u16>;

const CIRCLE_WIDGET_RADIUS: f32 = 1.5;
const ZOOM_THRESHOLD: f32 = 7.0;

#[derive(Debug, Clone)]
pub struct Helix {
    /// The first drawn nucleotide
    left: isize,
    /// The first nucleotide that is not drawn
    right: isize,
    pub isometry: FullIsometry,
    scale: f32,
    color: u32,
    z_index: i32,
    stroke_width: f32,
    /// The position of self in the Helix vector of the design
    pub flat_id: FlatHelix,
    pub real_id: usize,
    pub visible: bool,
    abscissa_converter: Arc<AbscissaConverter>,
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
        isometry: FullIsometry,
        flat_id: FlatHelix,
        real_id: usize,
        visible: bool,
        abscissa_converter_: Arc<AbscissaConverter_>,
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
            abscissa_converter: Arc::new(AbscissaConverter {
                converter: abscissa_converter_,
                left: flat_id.segment_left,
            }),
        }
    }

    pub fn update(&mut self, helix2d: &Helix2d, id_map: &FlatHelixMaps) {
        self.left = self.left.min(helix2d.left);
        self.right = self.right.max(helix2d.right);
        self.visible = helix2d.visible;
        self.real_id = helix2d.id;
        let left;
        let segment = HelixSegment {
            helix_idx: helix2d.id,
            segment_idx: helix2d.segment_idx,
        };
        if let Some(flat_id) = FlatHelix::from_real(segment, id_map) {
            left = flat_id.segment_left;
            self.flat_id = flat_id
        } else {
            log::error!("real id does not exist {}", self.real_id);
            left = None;
        }
        self.isometry = helix2d.isometry;
        self.abscissa_converter = Arc::new(AbscissaConverter {
            converter: helix2d.abscissa_converter.clone(),
            left,
        })
    }

    pub fn background_vertices(&self) -> Vertices {
        let mut vertices = Vertices::new();
        let left = self
            .abscissa_converter
            .nucl_to_x_convertion(self.get_flat_left()) as f32;
        let right = self.abscissa_converter.nucl_to_x_convertion(
            self.get_flat_right()
                .right()
                .max(self.get_flat_left().right()),
        ) as f32;
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
        let left = self
            .abscissa_converter
            .nucl_to_x_convertion(self.get_flat_left()) as f32;
        let right = self.abscissa_converter.nucl_to_x_convertion(
            self.get_flat_right()
                .right()
                .max(self.get_flat_left().right()),
        ) as f32;
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
            let x = self
                .abscissa_converter
                .nucl_to_x_convertion(FlatPosition::from_real(i, self.flat_id.segment_left));
            builder.begin(Point::new(x as f32, 0.));
            builder.line_to(Point::new(x as f32, 2.));
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
        let mut rotation = self.isometry.rotation.into_matrix();
        rotation[0] *= self.isometry.symmetry.x;
        rotation[1] *= self.isometry.symmetry.y;
        HelixModel {
            color: Instance::color_from_u32(self.color),
            position: self.isometry.translation,
            rotation,
            z_index: self.z_index,
            stroke_width: self.stroke_width,
        }
    }

    /// Return the position of the nucleotide in the 2d drawing
    pub fn get_nucl_position(&self, nucl: &FlatNucl, shift: Shift) -> Vec2 {
        let mut local_position = nucl.flat_position.0 as f32 * Vec2::unit_x()
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
        let new_x = self
            .abscissa_converter
            .x_conversion(local_position.x as f64);
        local_position.x = new_x as f32;
        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * local_position)
    }

    /// Return the position at which the 3' tick should end
    pub fn get_arrow_end(&self, nucl: &FlatNucl) -> Vec2 {
        let mut local_position = nucl.flat_position.0 as f32 * Vec2::unit_x()
            + if nucl.forward {
                Vec2::new(0.2, 0.3)
            } else {
                Vec2::new(0.8, 1.7)
            };
        let new_x = self
            .abscissa_converter
            .x_conversion(local_position.x as f64);
        local_position.x = new_x as f32;

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
    pub fn get_click(&self, x: f32, y: f32, bounded: bool) -> Option<(FlatPosition, bool)> {
        let click = {
            let ret = Vec2::new(x, y);
            let iso = self.isometry.into_homogeneous_matrix().inversed();
            iso.transform_point2(ret)
        };
        if click.y <= 0.
            || click.y >= 2.
            || (bounded && click.x < self.leftmost_x())
            || (bounded && click.x > self.rightmost_x())
        {
            None
        } else {
            Some(self.get_click_unbounded(x, y))
        }
    }

    pub fn move_handle(&mut self, handle: HelixHandle, position: Vec2) -> (isize, isize) {
        let (pos, _) = self.get_click_unbounded(position.x, position.y);
        match handle {
            HelixHandle::Left => {
                self.left = (self.right - 2).min(pos.right().to_real(self.flat_id.segment_left))
            }
            HelixHandle::Right => {
                self.right = (self.left + 2).max(pos.left().to_real(self.flat_id.segment_left))
            }
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
        if let Some(left) = self.flat_id.segment_left {
            let (left, right) = (left, left + 2);
            self.left = left;
            self.right = right;
            (left, right)
        } else {
            let (left, right) = (self.right - 1, self.left + 1);
            self.left = left;
            self.right = right;
            (left, right)
        }
    }

    pub fn click_on_handle(&self, x: f32, y: f32) -> Option<HelixHandle> {
        let click = {
            let ret = Vec2::new(x, y);
            let iso = self.isometry.into_homogeneous_matrix().inversed();
            iso.transform_point2(ret)
        };
        if click.y <= 0. || click.y >= 2. {
            None
        } else {
            let ret = self
                .get_click_unbounded(x, y)
                .0
                .to_real(self.flat_id.segment_left);
            if ret == self.left - 1 {
                Some(HelixHandle::Left)
            } else if ret == self.right + 1 {
                Some(HelixHandle::Right)
            } else {
                None
            }
        }
    }

    /// Project a click on the helix's axis, and return the corresponding nucleotide
    /// Do not take the left and right bound into account.
    pub fn get_click_unbounded(&self, x: f32, y: f32) -> (FlatPosition, bool) {
        let click = {
            let ret = Vec2::new(x, y);
            let iso = self.isometry.into_homogeneous_matrix().inversed();
            iso.transform_point2(ret)
        };
        let forward = click.y <= 1.;
        let position = self
            .abscissa_converter
            .x_to_nucl_conversion(click.x as f64)
            .floor() as isize;
        (FlatPosition(position), forward)
    }

    /// Return true if (x, y) is on the circle representing self
    pub fn click_on_circle(&self, x: f32, y: f32, camera: &CameraPtr) -> bool {
        if let Some(center) = self.get_circle(camera, &BTreeMap::new()) {
            (center.center - Vec2::new(x, y)).mag() < center.radius
        } else {
            false
        }
    }

    pub fn get_pivot(&self, position: FlatPosition) -> Vec2 {
        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.scale * Vec2::new(self.x_conversion(position.0 as f32), 1.))
    }

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

    fn x_conversion(&self, x: f32) -> f32 {
        self.abscissa_converter.x_conversion(x as f64) as f32
    }

    fn info_position(&self, x: FlatPosition) -> Vec2 {
        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.x_conversion(x.0 as f32 + 0.5) * Vec2::unit_x() - Vec2::unit_y())
    }

    fn char_position_top(&self, x: FlatPosition) -> Vec2 {
        self.isometry
            .into_homogeneous_matrix()
            .transform_point2(self.x_conversion(x.0 as f32 + 0.5) * Vec2::unit_x())
    }

    fn char_position_bottom(&self, x: FlatPosition) -> Vec2 {
        self.isometry.into_homogeneous_matrix().transform_point2(
            self.x_conversion(x.0 as f32 + 0.5) * Vec2::unit_x() + 2. * Vec2::unit_y(),
        )
    }

    pub fn handle_circles(&self) -> Vec<CircleInstance> {
        let top_left_pos = self.get_nucl_position(
            &FlatNucl {
                helix: self.flat_id,
                flat_position: self.get_flat_left().left(),
                forward: true,
            },
            Shift::No,
        );
        let bottom_left_pos = self.get_nucl_position(
            &FlatNucl {
                helix: self.flat_id,
                flat_position: self.get_flat_left().left(),
                forward: false,
            },
            Shift::No,
        );
        let top_right_pos = self.get_nucl_position(
            &FlatNucl {
                helix: self.flat_id,
                flat_position: self.get_flat_right().right(),
                forward: true,
            },
            Shift::No,
        );
        let bottom_right_pos = self.get_nucl_position(
            &FlatNucl {
                helix: self.flat_id,
                flat_position: self.get_flat_right().right(),
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

    fn leftmost_x(&self) -> f32 {
        self.abscissa_converter
            .nucl_to_x_convertion(self.get_flat_left()) as f32
    }

    fn rightmost_x(&self) -> f32 {
        self.abscissa_converter
            .nucl_to_x_convertion(self.get_flat_right().right()) as f32
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
        let center = if self.leftmost_x() as f32 > right || (self.rightmost_x() as f32) < left {
            // the helix is invisible
            None
        } else if self.leftmost_x() as f32 - 1. - 2. * CIRCLE_WIDGET_RADIUS > left {
            // There is room on the left of the helix
            Some(self.x_position(
                self.leftmost_x() as f32 - 1. - CIRCLE_WIDGET_RADIUS,
                HelixLine::Middle,
            ))
        } else if self.rightmost_x() as f32 + 1. + 2. * CIRCLE_WIDGET_RADIUS < right {
            // There is room on the right of the helix
            Some(self.x_position(
                self.rightmost_x() as f32 + 1. + CIRCLE_WIDGET_RADIUS,
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

    pub fn get_circle_nucl(
        &self,
        position: FlatPosition,
        forward: bool,
        color: u32,
    ) -> CircleInstance {
        let center = self.get_nucl_position(
            &FlatNucl {
                helix: self.flat_id,
                flat_position: position,
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
        if self.leftmost_x() > right || (self.rightmost_x() as f32) < left {
            // the helix is invisible
            None
        } else if self.leftmost_x() - 1. - 2. * CIRCLE_WIDGET_RADIUS > left
            || self.rightmost_x() + 2. + 2. * CIRCLE_WIDGET_RADIUS < right
        {
            // There is room on the left of the helix
            Some(FlatNucl {
                flat_position: self.get_flat_left().left().left().left(),
                helix: self.flat_id,
                forward: true,
            })
        } else {
            Some(FlatNucl {
                flat_position: self.get_flat_left(),
                helix: self.flat_id,
                forward: true,
            })
        }
    }

    /// A default nucleotide position for when the helix cannot be seen by the camera
    pub fn default_pivot(&self) -> FlatNucl {
        FlatNucl {
            flat_position: self.get_flat_left().left().left().left(),
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

    pub fn make_visible(&self, position: FlatPosition, camera: CameraPtr) {
        let intersection = self.screen_intersection(&camera);
        let need_center = if let Some((left, right)) = intersection {
            left.floor() as isize > position.0 || (right.ceil() as isize) < position.0
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
        if nucl.forward {
            orientation = Rotor2::from_angle(std::f32::consts::PI) * orientation;
        }

        InsertionInstance::new(InsertionDescriptor {
            position,
            depth: self.get_depth(),
            symmetry: self.isometry.symmetry,
            orientation,
            color,
        })
    }

    fn info_line(&self) -> Line {
        Line {
            origin: self
                .isometry
                .into_homogeneous_matrix()
                .transform_point2(-Vec2::unit_y()),
            direction: self
                .isometry
                .matrix_with_transposed_symetry()
                .transform_vec2(Vec2::unit_x()),
        }
    }

    fn top_line(&self) -> Line {
        Line {
            origin: self
                .isometry
                .into_homogeneous_matrix()
                .transform_point2(Vec2::zero()),
            direction: self
                .isometry
                .matrix_with_transposed_symetry()
                .transform_vec2(Vec2::unit_x()),
        }
    }

    fn bottom_line(&self) -> Line {
        Line {
            origin: self
                .isometry
                .into_homogeneous_matrix()
                .transform_point2(2. * Vec2::unit_y()),
            direction: self
                .isometry
                .matrix_with_transposed_symetry()
                .transform_vec2(-Vec2::unit_x()),
        }
    }
}

pub struct CharCollector<'a> {
    pub camera: &'a CameraPtr,
    pub text_drawer: &'a mut TextDrawer,
    pub groups: &'a BTreeMap<usize, bool>,
    pub basis_map: &'a HashMap<Nucl, char, RandomState>,
    pub show_seq: bool,
    pub edition_info: &'a Option<EditionInfo>,
    pub hovered_nucl: &'a Option<FlatNucl>,
    pub nucl_collection: &'a dyn NuclCollection,
}

impl Helix {
    pub fn add_char_instances(&self, char_collector: CharCollector) {
        let candidate_pos: Option<isize> = char_collector
            .hovered_nucl
            .filter(|n| n.helix == self.flat_id)
            .map(|n| n.to_real().position);
        let show_seq = char_collector.show_seq
            && char_collector.camera.borrow().get_globals().zoom >= ZOOM_THRESHOLD;
        let size_id = 3.;
        let zoom_font = if char_collector.camera.borrow().get_globals().zoom < 7.0 {
            2.
        } else {
            1.
        };
        let camera = char_collector.camera;
        let groups = char_collector.groups;
        let edition_info = char_collector.edition_info;

        let size_pos = 1.4;
        let circle = self.get_circle(camera, groups);
        let rotation = camera.borrow().rotation().reversed();
        let symetry = camera.borrow().get_globals().symetry;
        if let Some(circle) = circle {
            let text = self.real_id.to_string();
            let sentence = Sentence {
                text: &text,
                size: size_id / text.len() as f32 * zoom_font,
                color: [0., 0., 0., 1.].into(),
                z_index: self.flat_id.flat.0 as i32,
                rotation,
                symetry,
            };
            let line = Line {
                origin: circle.center + circle.radius * Vec2::unit_y(),
                direction: Vec2::unit_x(),
            };
            char_collector
                .text_drawer
                .add_sentence(sentence, circle.center, line);
        }

        let moving_pos = edition_info
            .as_ref()
            .filter(|info| info.nucl.helix == self.flat_id)
            .map(|info| info.nucl.flat_position.to_real(self.flat_id.segment_left));
        let mut print_pos = |pos: isize| {
            let color = if Some(pos) == moving_pos || candidate_pos == Some(pos) {
                [1., 0., 0., 1.].into()
            } else {
                [0., 0., 0., 1.].into()
            };
            let text = pos.to_string();
            let flat_pos = FlatPosition::from_real(pos, self.flat_id.segment_left);
            let sentence = Sentence {
                text: &text,
                size: size_pos * zoom_font,
                z_index: self.flat_id.flat.0 as i32,
                color,
                rotation,
                symetry,
            };
            let (position, line) = if show_seq {
                (self.info_position(flat_pos), self.info_line())
            } else {
                (self.char_position_top(flat_pos), self.top_line())
            };
            char_collector
                .text_drawer
                .add_sentence(sentence, position, line);
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

        let mut print_info = |flat_pos: FlatPosition, info: &str| {
            let sentence = Sentence {
                text: info,
                size: size_pos * zoom_font,
                z_index: self.flat_id.flat.0 as i32,
                color: [0., 0., 0., 1.].into(),
                rotation,
                symetry,
            };
            let line = self.info_line();
            char_collector
                .text_drawer
                .add_sentence(sentence, self.info_position(flat_pos), line);
        };

        if let Some(building) = edition_info {
            if building.nucl.helix == self.flat_id {
                print_info(building.nucl.flat_position, &building.to_string());
            }
        }

        let mut print_basis = |flat_position: FlatPosition, forward: bool| {
            let nucl = FlatNucl {
                helix: self.flat_id,
                flat_position,
                forward,
            }
            .to_real();
            if char_collector.nucl_collection.contains(&nucl) {
                let (c, color) = char_collector
                    .basis_map
                    .get(&nucl)
                    .map(|c| (c.to_string(), BLACK_VEC4))
                    .unwrap_or(('?'.to_string(), GREY_UNKNOWN_NUCL_VEC4));
                let sentence = Sentence {
                    text: &c,
                    size: size_pos * zoom_font,
                    z_index: self.flat_id.flat.0 as i32,
                    color,
                    rotation,
                    symetry,
                };
                let (line, position) = if nucl.forward {
                    (self.top_line(), self.char_position_top(flat_position))
                } else {
                    (self.bottom_line(), self.char_position_bottom(flat_position))
                };
                char_collector
                    .text_drawer
                    .add_sentence(sentence, position, line);
            }
        };

        if show_seq {
            for pos in self.left..=self.right {
                let flat_pos = FlatPosition::from_real(pos, self.flat_id.segment_left);
                print_basis(flat_pos, true);
                print_basis(flat_pos, false);
            }
        }
    }

    pub fn get_flat_left(&self) -> FlatPosition {
        FlatPosition::from_real(self.get_left(), self.flat_id.segment_left)
    }

    pub fn get_flat_right(&self) -> FlatPosition {
        FlatPosition::from_real(self.get_right(), self.flat_id.segment_left)
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
            if self.x_conversion(nucl.flat_position.0 as f32) >= x0.floor()
                && self.x_conversion(nucl.flat_position.0 as f32) < x1.ceil()
            {
                return true;
            }
        }
        if nucl.forward {
            if let Some((x0, x1)) =
                self.screen_rectangle_intersection(camera, left, top, right, bottom, HelixLine::Top)
            {
                if self.x_conversion(nucl.flat_position.0 as f32) >= x0.floor()
                    && self.x_conversion(nucl.flat_position.0 as f32) < x1.ceil()
                {
                    return true;
                }
            }
        } else if let Some((x0, x1)) =
            self.screen_rectangle_intersection(camera, left, top, right, bottom, HelixLine::Bottom)
        {
            if self.x_conversion(nucl.flat_position.0 as f32) >= x0.floor()
                && self.x_conversion(nucl.flat_position.0 as f32) < x1.ceil()
            {
                return true;
            }
        }
        false
    }

    /// Return the coordinates at which self's axis intersect the screen bounds.
    fn screen_intersection(&self, camera: &CameraPtr) -> Option<(f32, f32)> {
        self.screen_rectangle_intersection(camera, 0., 0.025, 1., 0.975, HelixLine::Middle)
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

mod abcissa_converter {
    use super::*;
    pub(super) use ensnano_design::AbscissaConverter as AbscissaConverter_;

    #[derive(Debug)]
    pub(super) struct AbscissaConverter {
        pub left: Option<isize>,
        pub converter: Arc<AbscissaConverter_>,
    }

    impl AbscissaConverter {
        pub fn nucl_to_x_convertion(&self, n: FlatPosition) -> f64 {
            let adjust = if let Some(n) = self.left {
                self.converter.nucl_to_x_convertion(n)
            } else {
                0.0
            };

            let real = n.to_real(self.left);
            self.converter.nucl_to_x_convertion(real) - adjust
        }

        pub fn x_conversion(&self, x: f64) -> f64 {
            if let Some(n) = self.left {
                // translate x to the right and back
                let adjust = self.converter.nucl_to_x_convertion(n);
                self.converter.x_conversion(x + n as f64) - adjust
            } else {
                self.converter.x_conversion(x)
            }
        }

        pub fn x_to_nucl_conversion(&self, x: f64) -> f64 {
            if let Some(n) = self.left {
                let shift = self.converter.nucl_to_x_convertion(n);
                self.converter.x_to_nucl_conversion(x + shift) - n as f64
            } else {
                self.converter.x_to_nucl_conversion(x)
            }
        }
    }
}
