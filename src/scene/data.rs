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
//! This modules handles internal informations about the scene, such as the selected objects etc..
//! It also communicates with the desgings to get the position of the objects to draw on the scene.

use super::view::RawDnaInstance;
use super::{
    HandleOrientation, HandlesDescriptor, LetterInstance, RotationWidgetDescriptor,
    RotationWidgetOrientation, SceneElement, View, ViewUpdate,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use ultraviolet::{Rotor3, Vec3};

use super::view::Mesh;
use crate::consts::*;
use ensnano_design::Nucl;
use ensnano_interactor::{ActionMode, PhantomElement, Selection, SelectionMode};
use ensnano_interactor::{ObjectType, Referential};

use super::AppState;

type ViewPtr = Rc<RefCell<View>>;

/// A module that handles the instantiation of designs as 3D geometric objects
mod design3d;
use design3d::Design3D;
pub use design3d::DesignReader;

pub struct Data<R: DesignReader> {
    view: ViewPtr,
    /// A `Design3D` is associated to each design.
    designs: Vec<Design3D<R>>,
    /// The set of selected elements
    selected_element: Option<SceneElement>,
    /// The set of candidates elements
    candidate_element: Option<SceneElement>,
    /// The kind of selection being performed if app_state.get_selection_mode() is SelectionMode::Nucl.
    ///
    /// Can be toggled by selecting the same element several
    /// time
    sub_selection_mode: SelectionMode,
    /// A position determined by the current selection. If only one nucleotide is selected, it's
    /// the position of the nucleotide.
    selected_position: Option<Vec3>,
    /// The element arround which the camera rotates
    pivot_element: Option<SceneElement>,
    pivot_update: bool,
    pivot_position: Option<Vec3>,
    free_xover: Option<FreeXover>,
    free_xover_update: bool,
    handle_need_opdate: bool,
    last_candidate_disc: Option<SceneElement>,
}

impl<R: DesignReader> Data<R> {
    pub fn new(reader: R, view: ViewPtr) -> Self {
        Self {
            view,
            designs: vec![Design3D::new(reader, 0)],
            selected_element: None,
            candidate_element: None,
            sub_selection_mode: SelectionMode::Nucleotide,
            selected_position: None,
            pivot_element: None,
            pivot_update: false,
            pivot_position: None,
            free_xover: None,
            free_xover_update: false,
            handle_need_opdate: false,
            last_candidate_disc: None,
        }
    }

    /// Add a new design to be drawn
    pub fn update_design(&mut self, design: R) {
        self.designs[0] = Design3D::new(design, 0);
    }

    /// Remove all designs to be drawn
    pub fn clear_designs(&mut self) {
        self.selected_element = None;
        self.candidate_element = None;
        self.reset_selection();
        self.reset_candidate();
        self.pivot_element = None;
        self.pivot_position = None;
        self.pivot_update = true;
    }
}

impl<R: DesignReader> Data<R> {
    /// Forwards all needed update to the view
    pub fn update_view<S: AppState>(&mut self, app_state: &S, older_app_state: &S) {
        if self.discs_need_update(app_state, older_app_state) {
            self.update_discs();
        }
        if app_state.design_was_modified(older_app_state) {
            self.update_instances();
        }

        if app_state.is_changing_color() {
            self.update_selection(&[], app_state)
        } else if app_state.selection_was_updated(older_app_state)
            || app_state.design_was_modified(older_app_state)
        {
            self.update_selection(app_state.get_selection(), app_state);
        }
        self.handle_need_opdate |= app_state.design_was_modified(older_app_state)
            || app_state.selection_was_updated(older_app_state)
            || app_state.get_action_mode() != older_app_state.get_action_mode();
        if self.handle_need_opdate {
            self.update_handle(app_state);
            self.handle_need_opdate = false;
        }
        if app_state.candidates_set_was_updated(older_app_state) {
            self.update_candidate(app_state.get_candidates());
        }
        if self.pivot_update {
            self.update_pivot();
            self.pivot_update = false;
        }
        if self.free_xover_update {
            self.update_free_xover();
            self.free_xover_update = false;
        }

        if app_state.design_model_matrix_was_updated(older_app_state) {
            self.update_matrices();
        }
    }

    fn discs_need_update<S: AppState>(&mut self, app_state: &S, older_app_state: &S) -> bool {
        let ret = app_state.design_was_modified(older_app_state)
            || app_state.selection_was_updated(older_app_state)
            || app_state.candidates_set_was_updated(older_app_state)
            || self.last_candidate_disc != self.candidate_element;
        self.last_candidate_disc = self.candidate_element.clone();
        ret
    }

    fn update_handle<S: AppState>(&self, app_state: &S) {
        let origin = self.get_selected_position();
        let orientation = self.get_widget_basis(app_state);
        let handle_descr = if app_state.get_action_mode().0.wants_handle() {
            origin
                .clone()
                .zip(orientation.clone())
                .map(|(origin, orientation)| HandlesDescriptor {
                    origin,
                    orientation: HandleOrientation::Rotor(orientation),
                    size: 0.25,
                })
        } else {
            None
        };
        self.view
            .borrow_mut()
            .update(ViewUpdate::Handles(handle_descr));
        let only_right = !self.selection_can_rotate_freely(app_state);
        let rotation_widget_descr = if app_state.get_action_mode().0.wants_rotation() {
            origin
                .clone()
                .zip(orientation.clone())
                .map(|(origin, orientation)| RotationWidgetDescriptor {
                    origin,
                    orientation: RotationWidgetOrientation::Rotor(orientation),
                    size: 0.2,
                    only_right,
                })
        } else {
            None
        };
        self.view
            .borrow_mut()
            .update(ViewUpdate::RotationWidget(rotation_widget_descr));
    }
}

impl<R: DesignReader> Data<R> {
    /// Return the sets of selected designs
    #[allow(dead_code)]
    pub fn get_selected_designs(&self, selection: &[Selection]) -> HashSet<u32> {
        selection.iter().filter_map(|s| s.get_design()).collect()
    }

    pub fn set_pivot_element<S: AppState>(&mut self, element: Option<SceneElement>, app_state: &S) {
        self.pivot_update |= self.pivot_element != element;
        self.pivot_element = element;
        self.update_pivot_position(app_state);
    }

    pub fn set_pivot_position(&mut self, position: Vec3) {
        self.pivot_position = Some(position);
        self.pivot_update = true;
    }

    #[allow(dead_code)]
    fn get_element_design(&self, element: &SceneElement) -> u32 {
        match element {
            SceneElement::DesignElement(d_id, _) => *d_id,
            SceneElement::PhantomElement(phantom_element) => phantom_element.design_id,
            SceneElement::Grid(d_id, _) => *d_id,
            _ => unreachable!(),
        }
    }

    /// Convert a selection into a set of elements
    fn expand_selection(
        &self,
        object_type: ObjectType,
        selection: &Selection,
    ) -> Vec<SceneElement> {
        let d_id = selection.get_design();
        if d_id.is_none() {
            return vec![];
        }
        let d_id = d_id.unwrap() as usize;
        let mut ret = Vec::new();
        if let Selection::Nucleotide(d_id, nucl) = selection {
            if !object_type.is_bound() {
                if let Some(n_id) = self.designs[*d_id as usize].get_identifier_nucl(nucl) {
                    ret.push(SceneElement::DesignElement(*d_id, n_id))
                } else {
                    ret.push(SceneElement::PhantomElement(PhantomElement {
                        design_id: *d_id,
                        helix_id: nucl.helix as u32,
                        position: nucl.position as i32,
                        forward: nucl.forward,
                        bound: false,
                    }));
                }
            }
        } else if let Selection::Bound(d_id, n1, n2) = selection {
            if object_type.is_bound() {
                if let Some(b_id) = self.designs[*d_id as usize].get_identifier_bound(*n1, *n2) {
                    ret.push(SceneElement::DesignElement(*d_id, b_id))
                } else {
                    ret.push(SceneElement::PhantomElement(PhantomElement {
                        design_id: *d_id,
                        helix_id: n1.helix as u32,
                        position: n1.position as i32,
                        forward: n1.forward,
                        bound: true,
                    }));
                }
            }
        } else if let Selection::Xover(d_id, xover_id) = selection {
            if object_type.is_bound() {
                if let Some(b_id) =
                    self.designs[*d_id as usize].get_element_identifier_from_xover_id(*xover_id)
                {
                    ret.push(SceneElement::DesignElement(*d_id, b_id))
                }
            }
        } else {
            let group = self.get_group_member(selection);
            for elt in group.iter() {
                if self.designs[d_id]
                    .get_element_type(*elt)
                    .map(|elt| elt.same_type(object_type))
                    .unwrap_or(false)
                {
                    ret.push(SceneElement::DesignElement(d_id as u32, *elt));
                }
            }
        }
        ret
    }

    /*
    /// Convert `self.candidates` into a set of elements according to `app_state.get_selection_mode()`
    fn expand_candidate(&self, object_type: ObjectType) -> Vec<SceneElement> {
        let mut ret = Vec::new();
        for element in &self.candidates {
            if let SceneElement::DesignElement(d_id, elt_id) = element {
                let group_id = self.get_group_identifier(*d_id, *elt_id);
                let group = self.get_group_member(*d_id, group_id);
                for elt in group.iter() {
                    if self.designs[*d_id as usize]
                        .get_element_type(*elt)
                        .map(|elt| elt.same_type(object_type))
                        .unwrap_or(false)
                    {
                        ret.push(SceneElement::DesignElement(*d_id, *elt));
                    }
                }
            } else if let SceneElement::PhantomElement(phantom_element) = element {
                if let Some(group_id) = self.get_group_identifier_phantom(*phantom_element) {
                    let d_id = phantom_element.design_id;
                    let group = self.get_group_member(d_id, group_id);
                    for elt in group.iter() {
                        if self.designs[d_id as usize]
                            .get_element_type(*elt)
                            .unwrap()
                            .same_type(object_type)
                        {
                            ret.push(SceneElement::DesignElement(d_id, *elt));
                        }
                    }
                }
                if phantom_element.bound == object_type.is_bound() {
                    ret.push(SceneElement::PhantomElement(*phantom_element));
                }
            }
        }
        ret
    }*/

    /// Return the instances of selected spheres
    pub fn get_selected_spheres(&self, selection: &[Selection]) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::new();
        for selection in selection.iter() {
            for element in self
                .expand_selection(ObjectType::Nucleotide(0), selection)
                .iter()
            {
                match element {
                    SceneElement::DesignElement(d_id, id) => {
                        if let Some(instance) = self.designs[*d_id as usize].make_instance(
                            *id,
                            SELECTED_COLOR,
                            SELECT_SCALE_FACTOR,
                        ) {
                            ret.push(instance)
                        }
                    }
                    SceneElement::PhantomElement(phantom_element) => {
                        if let Some(instance) = self
                            .designs
                            .get(phantom_element.design_id as usize)
                            .and_then(|d| {
                                d.make_instance_phantom(
                                    phantom_element,
                                    SELECTED_COLOR,
                                    SELECT_SCALE_FACTOR,
                                )
                            })
                        {
                            ret.push(instance);
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
        Rc::new(ret)
    }

    /// Return the instances of selected tubes
    pub fn get_selected_tubes(&self, selection: &[Selection]) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::new();
        for selection in selection.iter() {
            for element in self
                .expand_selection(ObjectType::Bound(0, 0), selection)
                .iter()
            {
                match element {
                    SceneElement::DesignElement(d_id, id) => {
                        if let Some(instance) = self.designs[*d_id as usize].make_instance(
                            *id,
                            SELECTED_COLOR,
                            SELECT_SCALE_FACTOR,
                        ) {
                            ret.push(instance)
                        }
                    }
                    SceneElement::PhantomElement(phantom_element) => {
                        if let Some(instance) = self
                            .designs
                            .get(phantom_element.design_id as usize)
                            .and_then(|d| {
                                d.make_instance_phantom(
                                    phantom_element,
                                    SELECTED_COLOR,
                                    SELECT_SCALE_FACTOR,
                                )
                            })
                        {
                            ret.push(instance);
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
        Rc::new(ret)
    }

    /// Return the instances of candidate spheres
    pub fn get_candidate_spheres(&self, candidates: &[Selection]) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::new();
        for candidate in candidates.iter() {
            for element in self
                .expand_selection(ObjectType::Nucleotide(0), candidate)
                .iter()
            {
                match element {
                    SceneElement::DesignElement(d_id, id) => {
                        if let Some(instance) = self.designs[*d_id as usize].make_instance(
                            *id,
                            CANDIDATE_COLOR,
                            SELECT_SCALE_FACTOR,
                        ) {
                            ret.push(instance)
                        }
                    }
                    SceneElement::PhantomElement(phantom_element) => {
                        if let Some(instance) = self
                            .designs
                            .get(phantom_element.design_id as usize)
                            .and_then(|d| {
                                d.make_instance_phantom(
                                    phantom_element,
                                    CANDIDATE_COLOR,
                                    SELECT_SCALE_FACTOR,
                                )
                            })
                        {
                            ret.push(instance);
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
        Rc::new(ret)
    }

    /// Return the instances of candidate tubes
    pub fn get_candidate_tubes(&self, candidates: &[Selection]) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::new();
        for candidate in candidates.iter() {
            for element in self
                .expand_selection(ObjectType::Bound(0, 0), candidate)
                .iter()
            {
                match element {
                    SceneElement::DesignElement(d_id, id) => {
                        if let Some(instance) = self.designs[*d_id as usize].make_instance(
                            *id,
                            CANDIDATE_COLOR,
                            SELECT_SCALE_FACTOR,
                        ) {
                            ret.push(instance)
                        }
                    }
                    SceneElement::PhantomElement(phantom_element) => {
                        if let Some(instance) = self
                            .designs
                            .get(phantom_element.design_id as usize)
                            .and_then(|d| {
                                d.make_instance_phantom(
                                    phantom_element,
                                    CANDIDATE_COLOR,
                                    SELECT_SCALE_FACTOR,
                                )
                            })
                        {
                            ret.push(instance);
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
        Rc::new(ret)
    }

    /// Return the identifier of the group of the selected element
    pub fn get_selected_group<S: AppState>(&self, app_state: &S) -> Option<u32> {
        match self.selected_element.as_ref() {
            Some(SceneElement::DesignElement(design_id, element_id)) => {
                let selection_mode = self.get_sub_selection_mode(app_state);
                self.get_group_identifier(*design_id, *element_id, selection_mode)
                    .map(|x| x as u32)
            }
            Some(SceneElement::PhantomElement(phantom_element)) => Some(phantom_element.helix_id),
            Some(SceneElement::Grid(_, g_id)) => Some(*g_id as u32),
            _ => None,
        }
    }

    /// Return the group to which an element belongs. The group depends on app_state.get_selection_mode().
    fn get_group_identifier(
        &self,
        design_id: u32,
        element_id: u32,
        selection_mode: SelectionMode,
    ) -> Option<u32> {
        match selection_mode {
            SelectionMode::Nucleotide => Some(element_id),
            SelectionMode::Design => Some(design_id),
            SelectionMode::Strand => self.designs[design_id as usize]
                .get_strand(element_id)
                .map(|x| x as u32),
            SelectionMode::Helix => self.designs[design_id as usize]
                .get_helix(element_id)
                .map(|x| x as u32),
            SelectionMode::Grid => Some(element_id),
        }
    }

    /// Return the group to which a phantom element belongs. The group depends on app_state.get_selection_mode().
    #[allow(dead_code)]
    fn get_group_identifier_phantom(
        &self,
        phantom_element: PhantomElement,
        selection_mode: SelectionMode,
    ) -> Option<u32> {
        let nucl = Nucl {
            helix: phantom_element.helix_id as usize,
            forward: phantom_element.forward,
            position: phantom_element.position as isize,
        };

        let design_id = phantom_element.design_id;
        let element_id = self.designs[design_id as usize].get_identifier_nucl(&nucl);

        match selection_mode {
            SelectionMode::Nucleotide => element_id,
            SelectionMode::Design => Some(design_id),
            SelectionMode::Strand => element_id.and_then(|e| {
                self.designs[design_id as usize]
                    .get_strand(e)
                    .map(|x| x as u32)
            }),
            SelectionMode::Helix => Some(phantom_element.helix_id),
            SelectionMode::Grid => None,
        }
    }

    fn get_helix_identifier(&self, design_id: u32, element_id: u32) -> Option<u32> {
        self.designs[design_id as usize]
            .get_helix(element_id)
            .map(|x| x as u32)
    }

    /// Return the set of elements in a given group
    fn get_group_member(&self, element: &Selection) -> HashSet<u32> {
        match element {
            Selection::Nucleotide(d_id, nucl) => self.designs[*d_id as usize]
                .get_identifier_nucl(nucl)
                .iter()
                .cloned()
                .collect(),
            Selection::Bound(d_id, n1, n2) => self.designs[*d_id as usize]
                .get_identifier_bound(*n1, *n2)
                .iter()
                .cloned()
                .collect(),
            Selection::Xover(d_id, xover_id) => self.designs[*d_id as usize]
                .get_element_identifier_from_xover_id(*xover_id)
                .iter()
                .cloned()
                .collect(),
            Selection::Helix(d_id, h_id) => self.designs[*d_id as usize].get_helix_elements(*h_id),
            Selection::Strand(d_id, s_id) => {
                self.designs[*d_id as usize].get_strand_elements(*s_id)
            }
            Selection::Grid(_, _) => HashSet::new(), // A grid is not made of atomic elements
            Selection::Phantom(_) => HashSet::new(),
            Selection::Nothing => HashSet::new(),
            Selection::Design(d_id) => self.designs[*d_id as usize].get_all_elements(),
        }
    }

    /// Return the postion of a given element, either in the world pov or in the model pov
    fn get_element_position(
        &self,
        element: &SceneElement,
        referential: Referential,
        selection_mode: SelectionMode,
    ) -> Option<Vec3> {
        let design_id = element.get_design()?;
        let design = self.designs.get(design_id as usize)?;
        match selection_mode {
            SelectionMode::Helix => design
                .get_element_axis_position(element, referential)
                .or(design.get_element_position(element, referential)),
            SelectionMode::Nucleotide
            | SelectionMode::Strand
            | SelectionMode::Design
            | SelectionMode::Grid => design.get_element_position(element, referential),
        }
    }

    pub fn get_selected_position(&self) -> Option<Vec3> {
        self.selected_position
    }

    pub fn try_update_pivot_position<S: AppState>(&mut self, app_state: &S) {
        if self.pivot_element.is_none() {
            self.pivot_element = self.selected_element;
            self.pivot_update = true;
            self.update_pivot_position(app_state);
        }
    }

    pub fn get_pivot_position(&self) -> Option<Vec3> {
        self.pivot_position.or(self.selected_position)
    }

    /// Update the selection by selecting the group to which a given nucleotide belongs. Return the
    /// selected group
    pub fn set_selection<S: AppState>(
        &mut self,
        element: Option<SceneElement>,
        app_state: &S,
    ) -> Option<Selection> {
        self.handle_need_opdate = true;
        if let Some(SceneElement::WidgetElement(_)) = element {
            return None;
        }
        println!("selected {:?}", element);
        let future_selection = element;
        if self.selected_element == future_selection {
            self.sub_selection_mode = toggle_selection(self.sub_selection_mode);
        } else {
            self.sub_selection_mode = SelectionMode::Nucleotide;
        }
        self.selected_element = future_selection;
        self.update_selected_position(app_state);
        println!("selected position: {:?}", self.selected_position);
        let selection_mode = if app_state.get_selection_mode() == SelectionMode::Nucleotide {
            self.sub_selection_mode
        } else {
            app_state.get_selection_mode()
        };
        let selection = if let Some(element) = element.as_ref() {
            self.element_to_selection(element, selection_mode)
        } else {
            Selection::Nothing
        };
        let future_selection = if selection != Selection::Nothing {
            vec![selection]
        } else {
            vec![]
        };
        Some(selection)
    }

    pub fn to_selection<S: AppState>(
        &self,
        element: Option<SceneElement>,
        app_state: &S,
    ) -> Option<Selection> {
        if let Some(SceneElement::WidgetElement(_)) = element {
            return None;
        }
        let selection = if let Some(element) = element.as_ref() {
            self.element_to_selection(element, app_state.get_selection_mode())
        } else {
            Selection::Nothing
        };
        Some(selection).filter(|s| *s != Selection::Nothing)
    }

    pub fn add_to_selection<S: AppState>(
        &mut self,
        element: Option<SceneElement>,
        selection: &[Selection],
        app_state: &S,
    ) -> Option<Vec<Selection>> {
        if let Some(SceneElement::WidgetElement(_)) = element {
            return None;
        }
        self.sub_selection_mode = SelectionMode::Nucleotide;
        let selected = if let Some(element) = element.as_ref() {
            self.element_to_selection(element, app_state.get_selection_mode())
        } else {
            Selection::Nothing
        };
        if let Some(element) = element.clone() {
            self.selected_element = Some(element);
        }
        if selected == Selection::Nothing {
            None
        } else {
            let mut new_selection = selection.to_vec();
            if let Some(pos) = new_selection.iter().position(|x| *x == selected) {
                new_selection.remove(pos);
            } else {
                new_selection.push(selected);
            }
            Some(new_selection)
        }
    }

    /// This function must be called when the current movement ends.
    pub fn end_movement<S: AppState>(&mut self, app_state: &S) {
        self.update_selected_position(app_state)
    }

    /// If source is some nucleotide, target is some nucleotide and both nucleotides are
    /// on the same design, return the pair of nucleotides. Otherwise return None
    pub fn attempt_xover(
        &self,
        source: &Option<SceneElement>,
        target: &Option<SceneElement>,
    ) -> Option<(Nucl, Nucl, usize)> {
        let design_id;
        let source_nucl = if let Some(SceneElement::DesignElement(d_id, e_id)) = source {
            design_id = *d_id;
            self.designs[*d_id as usize].get_nucl_relax(*e_id)
        } else {
            design_id = 0;
            None
        }?;
        let target_nucl = if let Some(SceneElement::DesignElement(d_id, e_id)) = target {
            if design_id != *d_id {
                return None;
            }
            self.designs[design_id as usize].get_nucl_relax(*e_id)
        } else {
            None
        }?;
        Some((source_nucl, target_nucl, design_id as usize))
    }

    fn update_selected_position<S: AppState>(&mut self, app_state: &S) {
        let selection_mode = self.get_sub_selection_mode(app_state);
        self.selected_position = {
            if let Some(element) = self.selected_element.as_ref() {
                self.get_element_position(element, Referential::World, selection_mode)
            } else {
                None
            }
        };
    }

    fn update_pivot_position<S: AppState>(&mut self, app_state: &S) {
        self.pivot_position = {
            if let Some(element) = self.pivot_element.as_ref() {
                self.get_element_position(
                    element,
                    Referential::World,
                    app_state.get_selection_mode(),
                )
            } else {
                None
            }
        };
    }

    /// Clear self.selected
    pub fn reset_selection(&mut self) {
        self.selected_position = None;
        self.selected_element = None;
    }

    /// Notify the view that the selected elements have been modified
    fn update_selection<S: AppState>(&mut self, selection: &[Selection], app_state: &S) {
        let sphere = self.get_selected_spheres(selection);
        let tubes = self.get_selected_tubes(selection);
        let pos: Vec3 = sphere
            .iter()
            .chain(tubes.iter())
            .map(|i| i.model.extract_translation())
            .sum();
        let total_len = sphere.len() + tubes.len();
        if selection.len() > 1 && total_len > 1 {
            self.selected_position = Some(pos / (total_len as f32));
        } else {
            self.update_selected_position(app_state);
        }
        self.view.borrow_mut().update(ViewUpdate::RawDna(
            Mesh::SelectedTube,
            self.get_selected_tubes(selection),
        ));
        self.view.borrow_mut().update(ViewUpdate::RawDna(
            Mesh::SelectedSphere,
            self.get_selected_spheres(selection),
        ));
        let (sphere, vec) = self.get_phantom_instances(app_state);
        self.view
            .borrow_mut()
            .update(ViewUpdate::RawDna(Mesh::PhantomSphere, sphere));
        self.view
            .borrow_mut()
            .update(ViewUpdate::RawDna(Mesh::PhantomTube, vec));
        let mut grids = if let Some(SceneElement::Grid(d_id, g_id)) = self.selected_element.as_ref()
        {
            vec![(*d_id as usize, *g_id)]
        } else {
            vec![]
        };
        for s in selection.iter() {
            if let Selection::Grid(d_id, g_id) = s {
                grids.push((*d_id as usize, *g_id));
            }
        }
        self.view.borrow_mut().set_selected_grid(grids);
    }

    /// Return the sets of elements of the phantom helix
    pub fn get_phantom_instances<S: AppState>(
        &self,
        app_state: &S,
    ) -> (Rc<Vec<RawDnaInstance>>, Rc<Vec<RawDnaInstance>>) {
        let phantom_map = self.get_phantom_helices_set(app_state);
        let mut ret_sphere = Vec::new();
        let mut ret_tube = Vec::new();
        for (d_id, set) in phantom_map.iter() {
            let (spheres, tubes) =
                self.designs[*d_id as usize].make_phantom_helix_instances_raw(set);
            for sphere in spheres.iter().cloned() {
                ret_sphere.push(sphere);
            }
            for tube in tubes.iter().cloned() {
                ret_tube.push(tube);
            }
        }
        (Rc::new(ret_sphere), Rc::new(ret_tube))
    }

    /// Return a hashmap, mapping designs identifier to the set of helices whose phantom must be
    /// drawn.
    fn get_phantom_helices_set<S: AppState>(
        &self,
        app_state: &S,
    ) -> HashMap<u32, HashMap<u32, bool>> {
        let mut ret = HashMap::new();

        for (d_id, design) in self.designs.iter().enumerate() {
            let new_helices = design.get_persistent_phantom_helices();
            let set = ret.entry(d_id as u32).or_insert_with(HashMap::new);
            for h_id in new_helices.iter() {
                set.insert(*h_id, true);
            }
        }
        if self.must_draw_phantom(app_state) {
            for element in self.selected_element.iter() {
                match element {
                    SceneElement::DesignElement(d_id, elt_id) => {
                        let set = ret.entry(*d_id).or_insert_with(HashMap::new);
                        if let Some(h_id) = self.get_helix_identifier(*d_id, *elt_id) {
                            set.insert(h_id, false);
                        }
                    }
                    SceneElement::PhantomElement(phantom_element) => {
                        let set = ret
                            .entry(phantom_element.design_id)
                            .or_insert_with(HashMap::new);
                        set.insert(phantom_element.helix_id, false);
                    }
                    SceneElement::Grid(d_id, g_id) => {
                        let new_helices = self.designs[*d_id as usize]
                            .get_helices_grid(*g_id)
                            .unwrap_or_default();
                        let set = ret.entry(*d_id).or_insert_with(HashMap::new);
                        for h_id in new_helices.iter() {
                            set.insert(*h_id as u32, true);
                        }
                    }
                    SceneElement::GridCircle(d_id, g_id, x, y) => {
                        if let Some(h_id) =
                            self.designs[*d_id as usize].get_helix_grid(*g_id, *x, *y)
                        {
                            let set = ret.entry(*d_id).or_insert_with(HashMap::new);
                            set.insert(h_id, false);
                        }
                    }
                    SceneElement::WidgetElement(_) => unreachable!(),
                }
            }
        }
        ret
    }

    fn must_draw_phantom<S: AppState>(&self, app_state: &S) -> bool {
        let ret = app_state.get_selection_mode() == SelectionMode::Helix;
        if ret {
            true
        } else {
            for element in self.selected_element.iter() {
                if let SceneElement::PhantomElement(_) = element {
                    return true;
                }
            }
            false
        }
    }

    pub fn element_to_selection(
        &self,
        element: &SceneElement,
        selection_mode: SelectionMode,
    ) -> Selection {
        match element {
            SceneElement::DesignElement(design_id, element_id) => {
                if let Some(group_id) =
                    self.get_group_identifier(*design_id, *element_id, selection_mode)
                {
                    match selection_mode {
                        SelectionMode::Design => Selection::Design(*design_id),
                        SelectionMode::Strand => Selection::Strand(*design_id, group_id),
                        SelectionMode::Nucleotide => {
                            let nucl = self.designs[*design_id as usize].get_nucl(group_id);
                            let bound = self.designs[*design_id as usize].get_bound(group_id);
                            let xover_id = bound.as_ref().and_then(|xover| {
                                self.designs[*design_id as usize].get_xover_id(xover)
                            });
                            if let Some(nucl) = nucl {
                                Selection::Nucleotide(*design_id, nucl)
                            } else if let Some(id) = xover_id {
                                Selection::Xover(*design_id, id)
                            } else if let Some((n1, n2)) = bound {
                                Selection::Bound(*design_id, n1, n2)
                            } else {
                                Selection::Nothing
                            }
                        }
                        SelectionMode::Helix => Selection::Helix(*design_id, group_id),
                        SelectionMode::Grid => Selection::Grid(*design_id, group_id as usize),
                    }
                } else {
                    Selection::Nothing
                }
            }
            SceneElement::Grid(d_id, g_id) => Selection::Grid(*d_id, *g_id),
            SceneElement::GridCircle(d_id, g_id, _, _) => Selection::Grid(*d_id, *g_id),
            SceneElement::PhantomElement(phantom) if phantom.bound => Selection::Bound(
                phantom.design_id,
                phantom.to_nucl(),
                phantom.to_nucl().left(),
            ),
            SceneElement::PhantomElement(phantom) => {
                if selection_mode == SelectionMode::Helix {
                    Selection::Helix(phantom.design_id, phantom.to_nucl().helix as u32)
                } else {
                    Selection::Nucleotide(phantom.design_id, phantom.to_nucl())
                }
            }
            _ => Selection::Nothing,
        }
    }

    /// Set the set of candidates to a given nucleotide
    pub fn set_candidate<S: AppState>(
        &mut self,
        element: Option<SceneElement>,
        app_state: &S,
    ) -> Option<Selection> {
        self.candidate_element = element;
        let future_candidates = if let Some(element) = element.as_ref() {
            let selection = self.element_to_selection(element, app_state.get_selection_mode());
            if selection != Selection::Nothing {
                Some(selection)
            } else {
                None
            }
        } else {
            None
        };
        future_candidates
    }

    pub fn notify_selection(&mut self, selection: &[Selection]) {
        if selection.len() == 1 {
            match selection[0] {
                Selection::Nucleotide(d_id, nucl) => {
                    self.selected_position = self.designs[d_id as usize].get_nucl_position(nucl);
                }
                Selection::Bound(d_id, n1, n2) => {
                    let pos1 = self.designs[d_id as usize].get_nucl_position(n1);
                    let pos2 = self.designs[d_id as usize].get_nucl_position(n2);
                    self.selected_position = pos1.zip(pos2).map(|(a, b)| (a + b) / 2.);
                }
                Selection::Xover(d_id, xover_id) => {
                    if let Some((n1, n2)) = self.designs[d_id as usize].get_xover_with_id(xover_id)
                    {
                        let pos1 = self.designs[d_id as usize].get_nucl_position(n1);
                        let pos2 = self.designs[d_id as usize].get_nucl_position(n2);
                        self.selected_position = pos1.zip(pos2).map(|(a, b)| (a + b) / 2.);
                    }
                }
                _ => (),
            }
        }
    }

    /// Clear the set of candidates to a given nucleotide
    pub fn reset_candidate(&mut self) {
        self.candidate_element = None;
    }

    /// Notify the view that the instances of candidates have changed
    fn update_candidate(&mut self, candidates: &[Selection]) {
        self.view.borrow_mut().update(ViewUpdate::RawDna(
            Mesh::CandidateTube,
            self.get_candidate_tubes(candidates),
        ));
        self.view.borrow_mut().update(ViewUpdate::RawDna(
            Mesh::CandidateSphere,
            self.get_candidate_spheres(candidates),
        ));
        let mut grids =
            if let Some(SceneElement::Grid(d_id, g_id)) = self.candidate_element.as_ref() {
                vec![(*d_id as usize, *g_id)]
            } else {
                vec![]
            };
        for c in candidates.iter() {
            if let Selection::Grid(d_id, g_id) = c {
                grids.push((*d_id as usize, *g_id));
            }
        }
        self.view.borrow_mut().set_candidate_grid(grids);
    }

    fn update_pivot(&mut self) {
        let spheres = if let Some(pivot) = self.pivot_position {
            vec![Design3D::<R>::pivot_sphere(pivot)]
        } else {
            vec![]
        };
        self.view
            .borrow_mut()
            .update(ViewUpdate::RawDna(Mesh::PivotSphere, Rc::new(spheres)));
    }

    fn update_free_xover(&mut self) {
        let mut spheres = vec![];
        let mut tubes = vec![];
        let mut pos1 = None;
        let mut pos2 = None;
        if let Some(xover) = self.free_xover.as_ref() {
            if let Some((pos, sphere)) = self.convert_free_end(&xover.source, xover.design_id) {
                pos1 = Some(pos);
                if let Some(s) = sphere {
                    spheres.push(s);
                }
            }
            if let Some((pos, sphere)) = self.convert_free_end(&xover.target, xover.design_id) {
                pos2 = Some(pos);
                if let Some(s) = sphere {
                    spheres.push(s);
                }
            }
            if let Some((pos1, pos2)) = pos1.zip(pos2) {
                tubes.push(Design3D::<R>::free_xover_tube(pos1, pos2))
            }
        }
        self.view
            .borrow_mut()
            .update(ViewUpdate::RawDna(Mesh::XoverSphere, Rc::new(spheres)));
        self.view
            .borrow_mut()
            .update(ViewUpdate::RawDna(Mesh::XoverTube, Rc::new(tubes)));
    }

    fn convert_free_end(
        &self,
        free_end: &FreeXoverEnd,
        design_id: usize,
    ) -> Option<(Vec3, Option<RawDnaInstance>)> {
        match free_end {
            FreeXoverEnd::Nucl(nucl) => {
                let position = self.get_nucl_position(*nucl, design_id)?;
                Some((position, Some(Design3D::<R>::free_xover_sphere(position))))
            }
            FreeXoverEnd::Free(position) => Some((*position, None)),
        }
    }

    /// Notify the view that the set of instances have been modified.
    fn update_instances(&mut self) {
        let mut spheres = Vec::with_capacity(self.get_number_spheres());
        let mut tubes = Vec::with_capacity(self.get_number_tubes());
        let mut suggested_spheres = Vec::with_capacity(1000);
        let mut suggested_tubes = Vec::with_capacity(1000);
        let mut pasted_spheres = Vec::with_capacity(1000);
        let mut pasted_tubes = Vec::with_capacity(1000);

        let mut letters = Vec::new();
        let mut grids = Vec::new();
        let mut cones = Vec::new();
        for design in self.designs.iter() {
            for sphere in design.get_spheres_raw().iter() {
                spheres.push(*sphere);
            }
            for tube in design.get_tubes_raw().iter() {
                tubes.push(*tube);
            }
            letters = design.get_letter_instances();
            for grid in design.get_grid().iter().filter(|g| g.visible) {
                grids.push(grid.clone());
            }
            for sphere in design.get_suggested_spheres() {
                suggested_spheres.push(sphere)
            }
            for tube in design.get_suggested_tubes() {
                suggested_tubes.push(tube)
            }
            let (spheres, tubes) = design.get_pasted_strand();
            for sphere in spheres {
                pasted_spheres.push(sphere);
            }
            for tube in tubes {
                pasted_tubes.push(tube);
            }
            for cone in design.get_all_prime3_cone() {
                cones.push(cone);
            }
        }
        println!("nb sphres {}", spheres.len());
        self.update_free_xover();
        self.view
            .borrow_mut()
            .update(ViewUpdate::RawDna(Mesh::Tube, Rc::new(tubes)));
        self.view
            .borrow_mut()
            .update(ViewUpdate::RawDna(Mesh::Sphere, Rc::new(spheres)));
        self.view.borrow_mut().update(ViewUpdate::RawDna(
            Mesh::SuggestionSphere,
            Rc::new(suggested_spheres),
        ));
        self.view.borrow_mut().update(ViewUpdate::RawDna(
            Mesh::SuggestionTube,
            Rc::new(suggested_tubes),
        ));
        self.view.borrow_mut().update(ViewUpdate::RawDna(
            Mesh::PastedSphere,
            Rc::new(pasted_spheres),
        ));
        self.view
            .borrow_mut()
            .update(ViewUpdate::RawDna(Mesh::PastedTube, Rc::new(pasted_tubes)));
        self.view.borrow_mut().update(ViewUpdate::Letter(letters));
        self.view
            .borrow_mut()
            .update(ViewUpdate::Grids(Rc::new(grids)));
        self.view
            .borrow_mut()
            .update(ViewUpdate::RawDna(Mesh::Prime3Cone, Rc::new(cones)));
    }

    fn update_discs(&mut self) {
        let mut discs = Vec::new();
        let mut letters: Vec<Vec<LetterInstance>> = vec![vec![]; 10];
        let right = self.view.borrow().get_camera().borrow().right_vec();
        let up = self.view.borrow().get_camera().borrow().up_vec();
        for (d_id, design) in self.designs.iter().enumerate() {
            for grid in design.get_grid().iter().filter(|g| g.visible) {
                for (x, y) in design.get_helices_grid_coord(grid.id) {
                    let element = Some(SceneElement::GridCircle(d_id as u32, grid.id, x, y));
                    if self.selected_element.as_ref() != element.as_ref()
                        && self.candidate_element.as_ref() != element.as_ref()
                    {
                        let (d1, d2) = grid.disc(x, y, 0xAA_FF_FF_FF, d_id as u32);
                        discs.push(d1);
                        discs.push(d2);
                    }
                }
                for ((x, y), h_id) in design.get_helices_grid_key_coord(grid.id) {
                    grid.letter_instance(x, y, h_id, &mut letters, right, up);
                }
            }
        }
        if let Some(SceneElement::GridCircle(d_id, g_id, x, y)) = self.selected_element.as_ref() {
            if let Some(grid) = self.designs[*d_id as usize].get_grid().get(*g_id as usize) {
                let (d1, d2) = grid.disc(*x, *y, 0xAA_FF_00_00, *d_id as u32);
                discs.push(d1);
                discs.push(d2);
            }
        }
        if let Some(SceneElement::GridCircle(d_id, g_id, x, y)) = self.candidate_element.as_ref() {
            if let Some(grid) = self.designs[*d_id as usize].get_grid().get(*g_id as usize) {
                let (d1, d2) = grid.disc(*x, *y, 0xAA_00_FF_00, *d_id as u32);
                discs.push(d1);
                discs.push(d2);
            }
        }
        self.view.borrow_mut().update(ViewUpdate::GridDiscs(discs));
        self.view
            .borrow_mut()
            .update(ViewUpdate::GridLetter(letters));
    }

    /// Notify the view of an update of the model matrices
    fn update_matrices(&mut self) {
        let mut matrices = Vec::new();
        for design in self.designs.iter() {
            matrices.push(design.get_model_matrix());
        }
        self.view
            .borrow_mut()
            .update(ViewUpdate::ModelMatrices(matrices));
    }

    pub fn get_fitting_camera_position(&self) -> Option<Vec3> {
        let view = self.view.borrow();
        let basis = view.get_camera().borrow().get_basis();
        let fovy = view.get_projection().borrow().get_fovy();
        let ratio = view.get_projection().borrow().get_ratio();
        self.designs
            .get(0)
            .and_then(|d| d.get_fitting_camera_position(basis, fovy, ratio))
    }

    /// Return the point in the middle of the selected design
    pub fn get_middle_point(&self, design_id: u32) -> Vec3 {
        self.designs[design_id as usize].middle_point()
    }

    fn get_number_spheres(&self) -> usize {
        self.designs.iter().map(|d| d.get_spheres_raw().len()).sum()
    }

    fn get_number_tubes(&self) -> usize {
        self.designs.iter().map(|d| d.get_tubes_raw().len()).sum()
    }

    pub fn get_widget_basis<S: AppState>(&self, app_state: &S) -> Option<Rotor3> {
        self.get_selected_basis(app_state).map(|b| {
            if app_state.get_widget_basis().is_axis_aligned()
                && self.selection_can_rotate_freely(app_state)
            {
                Rotor3::identity()
            } else {
                b
            }
        })
    }

    fn get_selected_basis<S: AppState>(&self, app_state: &S) -> Option<Rotor3> {
        match self.selected_element.as_ref() {
            Some(SceneElement::DesignElement(d_id, _)) => match self
                .get_sub_selection_mode(app_state)
            {
                SelectionMode::Nucleotide | SelectionMode::Design | SelectionMode::Strand => None,
                SelectionMode::Grid => Some(self.designs[*d_id as usize].get_basis()),
                SelectionMode::Helix => {
                    let h_id = self.get_selected_group(app_state)?;
                    if let Some(grid_position) =
                        self.designs[*d_id as usize].get_helix_grid_position(h_id)
                    {
                        self.designs[*d_id as usize].get_grid_basis(grid_position.grid)
                    } else {
                        self.designs[*d_id as usize].get_helix_basis(h_id)
                    }
                }
            },
            Some(SceneElement::PhantomElement(phantom_element)) => {
                let d_id = phantom_element.design_id;
                match self.get_sub_selection_mode(app_state) {
                    SelectionMode::Nucleotide | SelectionMode::Design | SelectionMode::Strand => {
                        None
                    }
                    SelectionMode::Grid => Some(self.designs[d_id as usize].get_basis()),
                    SelectionMode::Helix => {
                        let h_id = phantom_element.helix_id;
                        self.designs[d_id as usize].get_helix_basis(h_id)
                    }
                }
            }
            Some(SceneElement::Grid(d_id, g_id)) => {
                self.designs[*d_id as usize].get_grid_basis(*g_id)
            }
            Some(SceneElement::GridCircle(d_id, g_id, _, _)) => {
                self.designs[*d_id as usize].get_grid_basis(*g_id)
            }
            _ => None,
        }
    }

    pub fn selection_can_rotate_freely<S: AppState>(&self, app_state: &S) -> bool {
        match self.selected_element.as_ref() {
            Some(SceneElement::DesignElement(d_id, _)) => {
                match self.get_sub_selection_mode(app_state) {
                    SelectionMode::Nucleotide
                    | SelectionMode::Design
                    | SelectionMode::Strand
                    | SelectionMode::Grid => true,
                    SelectionMode::Helix => {
                        if let Some(h_id) = self.get_selected_group(app_state) {
                            !self.designs[*d_id as usize].helix_is_on_grid(h_id)
                        } else {
                            true
                        }
                    }
                }
            }
            Some(SceneElement::PhantomElement(phantom_element)) => {
                let d_id = phantom_element.design_id;
                match self.get_sub_selection_mode(app_state) {
                    SelectionMode::Nucleotide
                    | SelectionMode::Design
                    | SelectionMode::Strand
                    | SelectionMode::Grid => true,
                    SelectionMode::Helix => {
                        let h_id = phantom_element.helix_id;
                        !self.designs[d_id as usize].helix_is_on_grid(h_id)
                    }
                }
            }
            Some(SceneElement::Grid(_, _)) => true,
            _ => true,
        }
    }

    pub fn can_start_builder(&self, element: Option<SceneElement>) -> Option<Nucl> {
        let selected = element.as_ref()?;
        let design = selected.get_design()?;
        self.designs[design as usize].can_start_builder(selected)
    }

    pub fn element_to_nucl(
        &self,
        element: &Option<SceneElement>,
        non_phantom: bool,
    ) -> Option<(Nucl, usize)> {
        match element {
            Some(SceneElement::DesignElement(d_id, n_id)) => self.designs[*d_id as usize]
                .get_nucl(*n_id)
                .zip(Some(*d_id as usize)),
            Some(SceneElement::PhantomElement(pe)) => {
                let nucl = pe.to_nucl();
                if non_phantom {
                    Some((nucl, pe.design_id as usize))
                        .filter(|n| self.designs[pe.design_id as usize].has_nucl(&n.0))
                } else {
                    Some((nucl, pe.design_id as usize))
                }
            }
            _ => None,
        }
    }

    pub fn get_nucl_position(&self, nucl: Nucl, design_id: usize) -> Option<Vec3> {
        let design = self.designs.get(design_id)?;
        design.get_nucl_position(nucl)
    }

    /*
    /// Set the selection to a given nucleotide if it exists in the design.
    pub fn select_nucl(&mut self, nucl: Nucl, design_id: usize) {
        let e_id = self.designs[design_id].get_identifier_nucl(&nucl);
        if let Some(id) = e_id {
            self.set_selection(Some(SceneElement::DesignElement(design_id as u32, id)));
        }
    }*/

    #[allow(dead_code)]
    pub fn get_candidate_nucl(&self) -> Option<Nucl> {
        match self.candidate_element.as_ref() {
            None => None,
            Some(SceneElement::DesignElement(d_id, n_id)) => {
                self.designs[*d_id as usize].get_nucl(*n_id)
            }
            Some(SceneElement::PhantomElement(pe)) => Some(pe.to_nucl()),
            _ => None,
        }
    }

    pub fn init_free_xover(&mut self, nucl: Nucl, position: Vec3, design_id: usize) {
        self.free_xover_update = true;
        self.free_xover = Some(FreeXover {
            source: FreeXoverEnd::Nucl(nucl),
            target: FreeXoverEnd::Free(position),
            design_id,
        });
    }

    pub fn update_free_xover_target(&mut self, element: Option<SceneElement>, position: Vec3) {
        self.free_xover_update = true;
        let nucl = self.element_to_nucl(&element, true);
        if let Some(free_xover) = self.free_xover.as_mut() {
            free_xover.target = FreeXoverEnd::Free(position);
            if let FreeXoverEnd::Nucl(origin_nucl) = free_xover.source {
                if let Some((nucl, _)) = nucl.filter(|n| n.1 == free_xover.design_id) {
                    if nucl.helix != origin_nucl.helix
                        && !self.designs[free_xover.design_id].both_prime3(origin_nucl, nucl)
                        && !self.designs[free_xover.design_id].both_prime5(origin_nucl, nucl)
                    {
                        free_xover.target = FreeXoverEnd::Nucl(nucl);
                    }
                }
            }
        }
    }

    pub fn end_free_xover(&mut self) {
        self.free_xover_update = true;
        self.free_xover = None;
    }

    fn get_sub_selection_mode<S: AppState>(&self, app_state: &S) -> SelectionMode {
        if app_state.get_selection_mode() == SelectionMode::Nucleotide {
            self.sub_selection_mode
        } else {
            app_state.get_selection_mode()
        }
    }

    pub fn get_selected_element<S: AppState>(&self, app_state: &S) -> Selection {
        if let Some(selection) = self.selected_element.as_ref() {
            self.element_to_selection(selection, self.get_sub_selection_mode(app_state))
        } else {
            Selection::Nothing
        }
    }
}

pub(super) trait WantWidget: Sized + 'static {
    const ALL: &'static [Self];

    fn wants_rotation(&self) -> bool;
    fn wants_handle(&self) -> bool;
}

impl WantWidget for ActionMode {
    const ALL: &'static [ActionMode] = &[
        ActionMode::Normal,
        ActionMode::Translate,
        ActionMode::Rotate,
        ActionMode::Build(false),
        ActionMode::Cut,
    ];

    fn wants_rotation(&self) -> bool {
        match self {
            ActionMode::Rotate => true,
            _ => false,
        }
    }

    fn wants_handle(&self) -> bool {
        match self {
            ActionMode::Translate => true,
            _ => false,
        }
    }
}

struct FreeXover {
    source: FreeXoverEnd,
    target: FreeXoverEnd,
    design_id: usize,
}

enum FreeXoverEnd {
    Free(Vec3),
    Nucl(Nucl),
}

fn toggle_selection(mode: SelectionMode) -> SelectionMode {
    match mode {
        SelectionMode::Nucleotide => SelectionMode::Strand,
        SelectionMode::Strand => SelectionMode::Helix,
        SelectionMode::Helix => SelectionMode::Nucleotide,
        mode => mode,
    }
}

use super::controller::Data as ControllerData;

impl<R: DesignReader> ControllerData for Data<R> {
    fn element_to_nucl(
        &self,
        element: &Option<SceneElement>,
        non_phantom: bool,
    ) -> Option<(Nucl, usize)> {
        self.element_to_nucl(element, non_phantom)
    }

    fn get_nucl_position(&self, nucl: Nucl, design_id: usize) -> Option<Vec3> {
        self.get_nucl_position(nucl, design_id)
    }

    fn attempt_xover(
        &self,
        source: &Option<SceneElement>,
        target: &Option<SceneElement>,
    ) -> Option<(Nucl, Nucl, usize)> {
        self.attempt_xover(source, target)
    }

    fn can_start_builder(&self, element: Option<SceneElement>) -> Option<Nucl> {
        self.can_start_builder(element)
    }
}
