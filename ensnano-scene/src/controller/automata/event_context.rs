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

use crate::{element_selector::CornerType, view::GridIntersection};

use super::*;
use ensnano_design::{Axis, BezierPlaneIntersection};

/// The context in which an event took place.
pub struct EventContext<'a, S: AppState> {
    controller: &'a Controller<S>,
    app_state: &'a S,
    pixel_reader: &'a mut ElementSelector,
    pub cursor_position: PhysicalPosition<f64>,
}

impl<'a, S: AppState> EventContext<'a, S> {
    pub fn normalized_cursor_position(&self) -> PhysicalPosition<f64> {
        self.normalize_position(self.cursor_position)
    }

    pub fn normalize_position(&self, position: PhysicalPosition<f64>) -> PhysicalPosition<f64> {
        let normalized_x = position.x / self.controller.area_size.width as f64;
        let normalized_y = position.y / self.controller.area_size.height as f64;
        PhysicalPosition {
            x: normalized_x,
            y: normalized_y,
        }
    }

    pub fn get_projection_on_plane(&self, plane_origin: Vec3) -> Vec3 {
        let normalized_cursor = self.normalized_cursor_position();
        self.controller.camera_controller.get_projection(
            plane_origin,
            normalized_cursor.x,
            normalized_cursor.y,
            self.controller.stereography.as_ref(),
        )
    }

    pub fn get_element_under_cursor(&mut self) -> Option<SceneElement> {
        self.pixel_reader.set_selected_id(self.cursor_position)
    }

    /// If element is a grid, get the grid disc corresponding to the grid position under the
    /// current cursor.
    /// Otherwise, return element
    pub fn convert_grid_to_grid_disc(&self, element: Option<SceneElement>) -> Option<SceneElement> {
        let normalized_position = self.normalized_cursor_position();
        if let Some(SceneElement::Grid(d_id, _)) = element {
            if let Some(intersection) = self
                .controller
                .view
                .borrow()
                .grid_intersection(normalized_position.x as f32, normalized_position.y as f32)
            {
                Some(SceneElement::GridCircle(d_id, intersection.grid_position()))
            } else {
                element
            }
        } else {
            element
        }
    }

    pub fn element_to_nucl(
        &self,
        element: &Option<SceneElement>,
        no_phantom: bool,
    ) -> Option<Nucl> {
        self.controller
            .data
            .borrow()
            .element_to_nucl(&element, true)
            .map(|(n, _)| n)
    }

    pub fn get_nucl_position(&self, nucl: Nucl) -> Option<Vec3> {
        self.controller.data.borrow().get_nucl_position(nucl, 0)
    }

    /// If self is over a possible cross-over origin, return it.
    pub fn get_xover_origin_under_cursor(&mut self) -> Option<XoverOrigin> {
        let element = self.get_element_under_cursor();
        let nucl = self.element_to_nucl(&element, true)?;
        let position = self.get_nucl_position(nucl)?;
        Some(XoverOrigin {
            scene_element: element,
            nucl,
            position,
        })
    }

    pub fn can_start_builder(&self, element: Option<SceneElement>) -> Option<Nucl> {
        self.controller.data.borrow().can_start_builder(element)
    }

    /// Project the current cursor position on an axis
    pub fn get_projection_on_axis(&self, axis: Axis<'_>) -> Option<isize> {
        let normalized_cursor_position = self.normalized_cursor_position();
        self.controller.view.borrow().compute_projection_axis(
            axis,
            normalized_cursor_position.x,
            normalized_cursor_position.y,
            None,
            self.controller.stereography.is_some(),
        )
    }

    /// Get the new strand builder position corresponding to the cursor position.
    pub fn get_new_build_position(&mut self) -> Option<isize> {
        let builder = self.app_state.get_strand_builders().get(0)?;
        let element = self.get_element_under_cursor();

        // We can move the builder to a phantom nucl, so we do not exclue phantom nucls from the
        // search
        let no_phantom = false;

        let nucl_under_cursor = self.element_to_nucl(&element, no_phantom);

        nucl_under_cursor
            .map(|n| n.position)
            .or_else(|| self.get_projection_on_axis(builder.get_axis()))
    }

    /// If source and dest are elements that represents nucleotides between which a xover can be
    /// made, return that pair of nucleotide.
    pub fn attempt_xover(
        &self,
        source: &Option<SceneElement>,
        dest: &Option<SceneElement>,
    ) -> Option<(Nucl, Nucl, usize)> {
        self.controller.data.borrow().attempt_xover(source, dest)
    }

    /// Return a reference to the current ModifiersState
    pub fn get_modifiers(&self) -> &ModifiersState {
        &self.controller.current_modifiers
    }

    pub fn get_id_of_bezier_bath_being_eddited(&self) -> Option<Option<BezierPathId>> {
        if let (ActionMode::EditBezierPath { path_id, .. }, _) = self.app_state.get_action_mode() {
            Some(path_id)
        } else {
            None
        }
    }

    pub fn is_pasting(&self) -> bool {
        self.app_state.is_pasting()
    }

    pub fn get_position_of_opposite_plane_corner(
        &self,
        plane_id: BezierPlaneId,
        corner_type: CornerType,
    ) -> Vec2 {
        self.app_state
            .get_design_reader()
            .get_corners_of_plane(plane_id)[corner_type.opposite().to_usize()]
    }

    /// If there is a bezier plane under the cursor, return it's identifier and the coordinates of
    /// the projection of the curosor on the plane
    pub fn get_plane_under_cursor(&self) -> Option<(BezierPlaneId, BezierPlaneIntersection)> {
        let normalized_position = self.normalized_cursor_position();
        let ray = self
            .controller
            .camera_controller
            .ray(normalized_position.x as f32, normalized_position.y as f32);
        ensnano_design::ray_bezier_plane_intersection(
            self.app_state
                .get_design_reader()
                .get_bezier_planes()
                .iter(),
            ray.0,
            ray.1,
        )
    }

    pub fn get_grid_intersection_with_cursor(&self) -> Option<GridIntersection> {
        let normalized_position = self.normalized_cursor_position();
        self.controller
            .view
            .borrow()
            .grid_intersection(normalized_position.x as f32, normalized_position.y as f32)
    }

    pub fn get_action_mode(&self) -> ActionMode {
        self.app_state.get_action_mode().0
    }

    pub fn get_object_at_grid_pos(&self, position: GridPosition) -> Option<GridObject> {
        self.controller.data.borrow().get_grid_object(position)
    }

    /// Return the SceneElement on which to place the camera rotation pivot
    pub fn get_pivot_element(&mut self) -> Option<SceneElement> {
        match self.pixel_reader.set_selected_id(self.cursor_position) {
            Some(SceneElement::Grid(d_id, g_id)) => {
                // for grids we take the precise grid position on which the user clicked.
                let mouse_x = self.cursor_position.x / self.controller.area_size.width as f64;
                let mouse_y = self.cursor_position.y / self.controller.area_size.height as f64;
                if let Some(intersection) = self
                    .controller
                    .view
                    .borrow()
                    .specific_grid_intersection(mouse_x as f32, mouse_y as f32, g_id)
                {
                    Some(SceneElement::GridCircle(d_id, intersection.grid_position()))
                } else {
                    Some(SceneElement::Grid(d_id, g_id))
                }
            }
            element => element,
        }
    }
}

/// The element that was clicked on and that can be the origin of a crossover.
#[derive(Clone)]
pub(super) struct XoverOrigin {
    pub scene_element: Option<SceneElement>,
    pub position: Vec3,
    pub nucl: Nucl,
}
