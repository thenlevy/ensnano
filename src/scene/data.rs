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
use super::{LetterInstance, SceneElement, View, ViewUpdate};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use ultraviolet::{Rotor3, Vec3};

use super::view::Mesh;
use crate::consts::*;
use crate::design::{Design, Nucl, ObjectType, Referential, StrandBuilder};
use crate::mediator::{ActionMode, Selection, SelectionMode};
use crate::utils::PhantomElement;

type ViewPtr = Rc<RefCell<View>>;

/// A module that handles the instantiation of designs as 3D geometric objects
mod design3d;
use design3d::Design3D;

pub struct Data {
    view: ViewPtr,
    /// A `Design3D` is associated to each design.
    designs: Vec<Design3D>,
    /// The set of selected elements
    selected_element: Option<SceneElement>,
    /// The set of candidates elements
    candidate_element: Option<SceneElement>,
    selection: Vec<Selection>,
    candidates: Vec<Selection>,
    /// The kind of selection being perfomed on the scene.
    selection_mode: SelectionMode,
    /// The kind of selection being performed if self.selection_mode is SelectionMode::Nucl.
    ///
    /// Can be toggled by selecting the same element several
    /// time
    sub_selection_mode: SelectionMode,
    /// The kind of action being performed on the scene
    pub action_mode: ActionMode,
    /// A position determined by the current selection. If only one nucleotide is selected, it's
    /// the position of the nucleotide.
    selected_position: Option<Vec3>,
    selection_update: bool,
    candidate_update: bool,
    instance_update: bool,
    matrices_update: bool,
    widget_basis: WidgetBasis,
    /// The element arround which the camera rotates
    pivot_element: Option<SceneElement>,
    pivot_update: bool,
    pivot_position: Option<Vec3>,
    free_xover: Option<FreeXover>,
    free_xover_update: bool,
}

impl Data {
    pub fn new(view: ViewPtr) -> Self {
        Self {
            view,
            designs: Vec::new(),
            selected_element: None,
            candidate_element: None,
            selection: Vec::new(),
            candidates: Vec::new(),
            selection_mode: SelectionMode::default(),
            sub_selection_mode: SelectionMode::Nucleotide,
            action_mode: Default::default(),
            selected_position: None,
            selection_update: false,
            candidate_update: false,
            instance_update: false,
            matrices_update: false,
            widget_basis: WidgetBasis::Object,
            pivot_element: None,
            pivot_update: false,
            pivot_position: None,
            free_xover: None,
            free_xover_update: false,
        }
    }

    /// Add a new design to be drawn
    pub fn add_design(&mut self, design: Arc<RwLock<Design>>) {
        self.clear_designs();
        self.designs.push(Design3D::new(design));
        self.notify_instance_update();
        self.notify_matrices_update();
    }

    /// Remove all designs to be drawn
    pub fn clear_designs(&mut self) {
        self.designs = Vec::new();
        self.selected_element = None;
        self.candidate_element = None;
        self.selection = Vec::new();
        self.candidates = Vec::new();
        self.reset_selection();
        self.reset_candidate();
        self.notify_instance_update();
        self.notify_matrices_update();
        self.pivot_element = None;
        self.pivot_position = None;
        self.pivot_update = true;
        self.candidate_update = true;
        self.selection_update = true;
    }

    /// Forwards all needed update to the view
    pub fn update_view(&mut self) {
        if self.instance_update || self.selection_update || self.candidate_update {
            self.update_discs();
        }
        if self.instance_update {
            self.update_instances();
            self.instance_update = false;
        }

        if self.selection_update {
            self.update_selection();
            self.selection_update = false;
        }
        if self.candidate_update {
            self.update_candidate();
            self.candidate_update = false;
        }
        if self.pivot_update {
            self.update_pivot();
            self.pivot_update = false;
        }
        if self.free_xover_update {
            self.update_free_xover();
            self.free_xover_update = false;
        }

        if self.matrices_update {
            self.update_matrices();
            self.matrices_update = false;
        }
    }

    /// Return the sets of selected designs
    #[allow(dead_code)]
    pub fn get_selected_designs(&self) -> HashSet<u32> {
        self.selection
            .iter()
            .filter_map(|s| s.get_design())
            .collect()
    }

    pub fn set_pivot_element(&mut self, element: Option<SceneElement>) {
        self.pivot_update |= self.pivot_element != element;
        self.pivot_element = element;
        self.update_pivot_position();
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
                if let Some(b_id) = self.designs[*d_id as usize].get_identifier_bound(n1, n2) {
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
    /// Convert `self.candidates` into a set of elements according to `self.selection_mode`
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
    pub fn get_selected_spheres(&self) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::new();
        for selection in self.selection.iter() {
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
    pub fn get_selected_tubes(&self) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::new();
        for selection in self.selection.iter() {
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
    pub fn get_candidate_spheres(&self) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::new();
        for candidate in self.candidates.iter() {
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
    pub fn get_candidate_tubes(&self) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::new();
        for candidate in self.candidates.iter() {
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
    pub fn get_selected_group(&self) -> u32 {
        match self.selected_element.as_ref() {
            Some(SceneElement::DesignElement(design_id, element_id)) => {
                let selection_mode = self.get_sub_selection_mode();
                self.get_group_identifier(*design_id, *element_id, selection_mode)
            }
            Some(SceneElement::PhantomElement(phantom_element)) => phantom_element.helix_id,
            Some(SceneElement::Grid(_, g_id)) => *g_id as u32,
            _ => unreachable!(),
        }
    }

    /// Return the group to which an element belongs. The group depends on self.selection_mode.
    fn get_group_identifier(
        &self,
        design_id: u32,
        element_id: u32,
        selection_mode: SelectionMode,
    ) -> u32 {
        match selection_mode {
            SelectionMode::Nucleotide => element_id,
            SelectionMode::Design => design_id,
            SelectionMode::Strand => self.designs[design_id as usize].get_strand(element_id),
            SelectionMode::Helix => self.designs[design_id as usize].get_helix(element_id),
            SelectionMode::Grid => element_id,
        }
    }

    /// Return the group to which a phantom element belongs. The group depends on self.selection_mode.
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
            SelectionMode::Strand => {
                element_id.map(|e| self.designs[design_id as usize].get_strand(e))
            }
            SelectionMode::Helix => Some(phantom_element.helix_id),
            SelectionMode::Grid => None,
        }
    }

    fn get_helix_identifier(&self, design_id: u32, element_id: u32) -> u32 {
        self.designs[design_id as usize].get_helix(element_id)
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
                .get_identifier_bound(n1, n2)
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

    pub fn try_update_pivot_position(&mut self) {
        if self.pivot_element.is_none() {
            self.pivot_element = self.selected_element;
            self.pivot_update = true;
            self.update_pivot_position();
        }
    }

    pub fn get_pivot_position(&self) -> Option<Vec3> {
        self.pivot_position.or(self.selected_position)
    }

    /// Update the selection by selecting the group to which a given nucleotide belongs. Return the
    /// selected group
    pub fn set_selection(&mut self, element: Option<SceneElement>) -> Option<Selection> {
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
        self.update_selected_position();
        println!("selected position: {:?}", self.selected_position);
        let selection_mode = if self.selection_mode == SelectionMode::Nucleotide {
            self.sub_selection_mode
        } else {
            self.selection_mode
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
        self.selection_update |= self.selected_element != element;
        self.selection_update |= self.selection != future_selection;
        self.selection = future_selection;

        Some(selection)
    }

    pub fn to_selection(&self, element: Option<SceneElement>) -> Option<Selection> {
        if let Some(SceneElement::WidgetElement(_)) = element {
            return None;
        }
        let selection = if let Some(element) = element.as_ref() {
            self.element_to_selection(element, self.selection_mode)
        } else {
            Selection::Nothing
        };
        Some(selection).filter(|s| *s != Selection::Nothing)
    }

    pub fn add_to_selection(&mut self, element: Option<SceneElement>) -> Option<Vec<Selection>> {
        if let Some(SceneElement::WidgetElement(_)) = element {
            return None;
        }
        self.sub_selection_mode = SelectionMode::Nucleotide;
        let selection = if let Some(element) = element.as_ref() {
            self.element_to_selection(element, self.selection_mode)
        } else {
            Selection::Nothing
        };
        if let Some(element) = element.clone() {
            self.selected_element = Some(element);
        }
        if selection == Selection::Nothing {
            None
        } else {
            if let Some(pos) = self.selection.iter().position(|x| *x == selection) {
                self.selection.remove(pos);
            } else {
                self.selection.push(selection);
            }
            self.selection_update = true;
            Some(self.selection.clone())
        }
    }

    /// This function must be called when the current movement ends.
    pub fn end_movement(&mut self) {
        self.update_selected_position()
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

    fn update_selected_position(&mut self) {
        let selection_mode = self.get_sub_selection_mode();
        self.selected_position = {
            if let Some(element) = self.selected_element.as_ref() {
                self.get_element_position(element, Referential::World, selection_mode)
            } else {
                None
            }
        };
    }

    fn update_pivot_position(&mut self) {
        self.pivot_position = {
            if let Some(element) = self.pivot_element.as_ref() {
                self.get_element_position(element, Referential::World, self.selection_mode)
            } else {
                None
            }
        };
    }

    /// Clear self.selected
    pub fn reset_selection(&mut self) {
        self.selection_update |= self.selected_element.is_some();
        self.selected_position = None;
        self.selected_element = None;
    }

    /// Notify the view that the selected elements have been modified
    fn update_selection(&mut self) {
        self.view.borrow_mut().update(ViewUpdate::RawDna(
            Mesh::SelectedTube,
            self.get_selected_tubes(),
        ));
        self.view.borrow_mut().update(ViewUpdate::RawDna(
            Mesh::SelectedSphere,
            self.get_selected_spheres(),
        ));
        let (sphere, vec) = self.get_phantom_instances();
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
        for s in self.selection.iter() {
            if let Selection::Grid(d_id, g_id) = s {
                grids.push((*d_id as usize, *g_id));
            }
        }
        self.view.borrow_mut().set_selected_grid(grids);
    }

    /// Return the sets of elements of the phantom helix
    pub fn get_phantom_instances(&self) -> (Rc<Vec<RawDnaInstance>>, Rc<Vec<RawDnaInstance>>) {
        let phantom_map = self.get_phantom_helices_set();
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
    fn get_phantom_helices_set(&self) -> HashMap<u32, HashMap<u32, bool>> {
        let mut ret = HashMap::new();

        for (d_id, design) in self.designs.iter().enumerate() {
            let new_helices = design.get_persistent_phantom_helices();
            let set = ret.entry(d_id as u32).or_insert_with(HashMap::new);
            for h_id in new_helices.iter() {
                set.insert(*h_id, true);
            }
        }
        if self.must_draw_phantom() {
            for element in self.selected_element.iter() {
                match element {
                    SceneElement::DesignElement(d_id, elt_id) => {
                        let set = ret.entry(*d_id).or_insert_with(HashMap::new);
                        set.insert(self.get_helix_identifier(*d_id, *elt_id), false);
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

    fn must_draw_phantom(&self) -> bool {
        let ret = self.selection_mode == SelectionMode::Helix;
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
                let group_id = self.get_group_identifier(*design_id, *element_id, selection_mode);
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
    pub fn set_candidate(&mut self, element: Option<SceneElement>) {
        let future_candidates = if let Some(element) = element.as_ref() {
            let selection = self.element_to_selection(element, self.selection_mode);
            if selection != Selection::Nothing {
                vec![selection]
            } else {
                vec![]
            }
        } else {
            vec![]
        };
        self.candidate_update |= self.candidate_element != element;
        self.candidate_update |= self.candidates != future_candidates;
        self.candidates = future_candidates;
        self.candidate_element = element;
    }

    pub fn get_candidate(&self) -> Vec<Selection> {
        self.candidates.clone()
    }

    pub fn notify_candidate(&mut self, candidate: Vec<Selection>) {
        let future_candidates = candidate
            .iter()
            .filter(|c| c.get_design().is_some())
            .cloned()
            .collect();
        self.candidate_update |= self.candidates != future_candidates;
        self.candidates = future_candidates;
    }

    pub fn notify_selection(&mut self, selection: Vec<Selection>) {
        let future_selection = selection
            .iter()
            .filter(|s| s.get_design().is_some())
            .cloned()
            .collect();
        self.selection_update |= self.selection != future_selection;
        self.selection = future_selection;
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
        self.candidate_update |= !self.candidates.is_empty();
        self.candidates = Vec::new();
        self.candidate_element = None;
    }

    /// Notify the view that the instances of candidates have changed
    fn update_candidate(&mut self) {
        self.view.borrow_mut().update(ViewUpdate::RawDna(
            Mesh::CandidateTube,
            self.get_candidate_tubes(),
        ));
        self.view.borrow_mut().update(ViewUpdate::RawDna(
            Mesh::CandidateSphere,
            self.get_candidate_spheres(),
        ));
        let mut grids =
            if let Some(SceneElement::Grid(d_id, g_id)) = self.candidate_element.as_ref() {
                vec![(*d_id as usize, *g_id)]
            } else {
                vec![]
            };
        for c in self.candidates.iter() {
            if let Selection::Grid(d_id, g_id) = c {
                grids.push((*d_id as usize, *g_id));
            }
        }
        self.view.borrow_mut().set_candidate_grid(grids);
    }

    fn update_pivot(&mut self) {
        let spheres = if let Some(pivot) = self.pivot_position {
            vec![Design3D::pivot_sphere(pivot)]
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
                tubes.push(Design3D::free_xover_tube(pos1, pos2))
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
                Some((position, Some(Design3D::free_xover_sphere(position))))
            }
            FreeXoverEnd::Free(position) => Some((*position, None)),
        }
    }

    /// This function must be called when the designs have been modified
    pub fn notify_instance_update(&mut self) {
        self.candidates = vec![];
        self.instance_update = true;
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
        }
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
        self.selection_update = true;
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
    /// This fuction must be called when the model matrices have been modfied
    pub fn notify_matrices_update(&mut self) {
        self.matrices_update = true;
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

    /// Return a position and rotation of the camera that fits the first design
    pub fn get_fitting_camera(&self, ratio: f32, fovy: f32) -> Option<(Vec3, Rotor3)> {
        let design = self.designs.get(0)?;
        Some(design.get_fitting_camera(ratio, fovy))
            .filter(|(v, _)| !v.x.is_nan() && !v.y.is_nan() && !v.z.is_nan())
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

    #[allow(dead_code)]
    pub fn toggle_selection_mode(&mut self) {
        self.selection_mode = match self.selection_mode {
            SelectionMode::Nucleotide => SelectionMode::Design,
            SelectionMode::Design => SelectionMode::Strand,
            SelectionMode::Strand => SelectionMode::Helix,
            SelectionMode::Helix => SelectionMode::Grid,
            SelectionMode::Grid => SelectionMode::Nucleotide,
        }
    }

    pub fn change_selection_mode(&mut self, selection_mode: SelectionMode) {
        self.selection_mode = selection_mode;
    }

    pub fn get_action_mode(&self) -> ActionMode {
        self.action_mode
    }

    pub fn change_action_mode(&mut self, action_mode: ActionMode) {
        self.action_mode = action_mode;
        self.instance_update = true;
        self.update_matrices();
    }

    pub fn toggle_widget_basis(&mut self, axis_aligned: bool) {
        self.widget_basis.toggle(axis_aligned)
    }

    pub fn get_widget_basis(&self) -> Option<Rotor3> {
        self.get_selected_basis().map(|b| {
            if let WidgetBasis::Object = self.widget_basis {
                b
            } else {
                Rotor3::identity()
            }
        })
    }

    fn get_selected_basis(&self) -> Option<Rotor3> {
        match self.selected_element.as_ref() {
            Some(SceneElement::DesignElement(d_id, _)) => match self.get_sub_selection_mode() {
                SelectionMode::Nucleotide | SelectionMode::Design | SelectionMode::Strand => None,
                SelectionMode::Grid => Some(self.designs[*d_id as usize].get_basis()),
                SelectionMode::Helix => {
                    let h_id = self.get_selected_group();
                    self.designs[*d_id as usize].get_helix_basis(h_id)
                }
            },
            Some(SceneElement::PhantomElement(phantom_element)) => {
                let d_id = phantom_element.design_id;
                match self.get_sub_selection_mode() {
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

    pub fn selection_can_rotate_freely(&self) -> bool {
        match self.selected_element.as_ref() {
            Some(SceneElement::DesignElement(d_id, _)) => match self.get_sub_selection_mode() {
                SelectionMode::Nucleotide
                | SelectionMode::Design
                | SelectionMode::Strand
                | SelectionMode::Grid => true,
                SelectionMode::Helix => {
                    let h_id = self.get_selected_group();
                    !self.designs[*d_id as usize].helix_is_on_grid(h_id)
                }
            },
            Some(SceneElement::PhantomElement(phantom_element)) => {
                let d_id = phantom_element.design_id;
                match self.get_sub_selection_mode() {
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

    #[allow(dead_code)]
    pub fn select_5prime(&mut self) {
        let selected = self.selected_element.as_ref();
        if let Some(SceneElement::DesignElement(d_id, e_id)) = selected {
            let new_selection = self
                .designs
                .get(*d_id as usize)
                .and_then(|d| d.get_element_5prime(*e_id));
            if new_selection.is_some() {
                self.set_selection(new_selection);
            }
        }
    }

    #[allow(dead_code)]
    pub fn select_3prime(&mut self) {
        let selected = self.selected_element.as_ref();
        if let Some(SceneElement::DesignElement(d_id, e_id)) = selected {
            let new_selection = self
                .designs
                .get(*d_id as usize)
                .and_then(|d| d.get_element_3prime(*e_id));
            if new_selection.is_some() {
                self.set_selection(new_selection);
            }
        }
    }

    pub fn get_strand_builder(
        &self,
        element: Option<SceneElement>,
        stick: bool,
    ) -> Option<StrandBuilder> {
        let selected = element.as_ref()?;
        let design = selected.get_design()?;
        self.designs[design as usize].get_builder(selected, stick)
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

    /// Set the selection to a given nucleotide if it exists in the design.
    pub fn select_nucl(&mut self, nucl: Nucl, design_id: usize) {
        let e_id = self.designs[design_id].get_identifier_nucl(&nucl);
        if let Some(id) = e_id {
            self.set_selection(Some(SceneElement::DesignElement(design_id as u32, id)));
        }
    }

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

    fn get_sub_selection_mode(&self) -> SelectionMode {
        if self.selection_mode == SelectionMode::Nucleotide {
            self.sub_selection_mode
        } else {
            self.selection_mode
        }
    }

    pub fn get_selected_element(&self) -> Selection {
        if let Some(selection) = self.selected_element.as_ref() {
            self.element_to_selection(selection, self.get_sub_selection_mode())
        } else {
            Selection::Nothing
        }
    }
}

impl ActionMode {
    pub const ALL: [ActionMode; 5] = [
        ActionMode::Normal,
        ActionMode::Translate,
        ActionMode::Rotate,
        ActionMode::Build(false),
        ActionMode::Cut,
    ];

    pub fn wants_rotation(&self) -> bool {
        match self {
            ActionMode::Rotate => true,
            _ => false,
        }
    }

    pub fn wants_handle(&self) -> bool {
        match self {
            ActionMode::Translate => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy)]
enum WidgetBasis {
    World,
    Object,
}

impl WidgetBasis {
    pub fn toggle(&mut self, axis_aligned: bool) {
        if axis_aligned {
            *self = WidgetBasis::World
        } else {
            *self = WidgetBasis::Object
        };
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
