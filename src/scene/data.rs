//! This modules handles internal informations about the scene, such as the selected objects etc..
//! It also communicates with the desgings to get the position of the objects to draw on the scene.

use super::view::RawDnaInstance;
use super::{GridIntersection, LetterInstance, SceneElement, View, ViewUpdate};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use ultraviolet::{Rotor3, Vec3};

use super::view::Mesh;
use crate::consts::*;
use crate::design::{utils::*, Design, Nucl, ObjectType, Referential, Strand, StrandBuilder};
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
    selected: Vec<SceneElement>,
    /// The set of candidates elements
    candidates: Vec<SceneElement>,
    /// The kind of selection being perfomed on the scene.
    pub selection_mode: SelectionMode,
    /// The kind of action being performed on the scene
    pub action_mode: ActionMode,
    /// A position determined by the current selection. If only one nucleotide is selected, it's
    /// the position of the nucleotide.
    selected_position: Option<Vec3>,
    selection_update: bool,
    candidate_update: bool,
    instance_update: bool,
    matrices_update: bool,
    widget_basis: Option<WidgetBasis>,
}

impl Data {
    pub fn new(view: ViewPtr) -> Self {
        Self {
            view,
            designs: Vec::new(),
            selected: Vec::new(),
            candidates: Vec::new(),
            selection_mode: SelectionMode::default(),
            action_mode: Default::default(),
            selected_position: None,
            selection_update: false,
            candidate_update: false,
            instance_update: false,
            matrices_update: false,
            widget_basis: None,
        }
    }

    /// Add a new design to be drawn
    pub fn add_design(&mut self, design: Arc<Mutex<Design>>) {
        self.designs.push(Design3D::new(design));
        self.notify_instance_update();
        self.notify_matrices_update();
    }

    /// Remove all designs to be drawn
    pub fn clear_designs(&mut self) {
        self.designs = Vec::new();
        self.selected = Vec::new();
        self.candidates = Vec::new();
        self.reset_selection();
        self.reset_candidate();
        self.notify_instance_update();
        self.notify_matrices_update();
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

        if self.matrices_update {
            self.update_matrices();
            self.matrices_update = false;
        }
    }

    /// Return the sets of selected designs
    pub fn get_selected_designs(&self) -> HashSet<u32> {
        self.selected
            .iter()
            .map(|x| self.get_element_design(x))
            .collect()
    }

    fn get_element_design(&self, element: &SceneElement) -> u32 {
        match element {
            SceneElement::DesignElement(d_id, _) => *d_id,
            SceneElement::PhantomElement(phantom_element) => phantom_element.design_id,
            SceneElement::Grid(d_id, _) => *d_id,
            _ => unreachable!(),
        }
    }

    /// Convert `self.selection` into a set of elements according to `self.selection_mode`
    fn expand_selection(&self, object_type: ObjectType) -> Vec<SceneElement> {
        let mut ret = Vec::new();
        for element in &self.selected {
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
                if phantom_element.bound == object_type.is_bound() {
                    ret.push(SceneElement::PhantomElement(*phantom_element));
                }
            }
        }
        ret
    }

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
    }

    /// Return the instances of selected spheres
    pub fn get_selected_spheres(&self) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::with_capacity(self.selected.len());
        for element in self.expand_selection(ObjectType::Nucleotide(0)).iter() {
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
        Rc::new(ret)
    }

    /// Return the instances of selected tubes
    pub fn get_selected_tubes(&self) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::with_capacity(self.selected.len());
        for element in self.expand_selection(ObjectType::Bound(0, 0)).iter() {
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
        Rc::new(ret)
    }

    /// Return the instances of candidate spheres
    pub fn get_candidate_spheres(&self) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::with_capacity(self.selected.len());
        for element in self.expand_candidate(ObjectType::Nucleotide(0)).iter() {
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
        Rc::new(ret)
    }

    /// Return the instances of candidate tubes
    pub fn get_candidate_tubes(&self) -> Rc<Vec<RawDnaInstance>> {
        let mut ret = Vec::with_capacity(self.selected.len());
        for element in self.expand_candidate(ObjectType::Bound(0, 0)).iter() {
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
        Rc::new(ret)
    }

    /// Return the identifier of the first selected group
    pub fn get_selected_group(&self) -> u32 {
        match self.selected.get(0) {
            Some(SceneElement::DesignElement(design_id, element_id)) => {
                self.get_group_identifier(*design_id, *element_id)
            }
            Some(SceneElement::PhantomElement(phantom_element)) => phantom_element.helix_id,
            Some(SceneElement::Grid(_, g_id)) => *g_id as u32,
            _ => unreachable!(),
        }
    }

    /// Return the group to which an element belongs. The group depends on self.selection_mode.
    fn get_group_identifier(&self, design_id: u32, element_id: u32) -> u32 {
        match self.selection_mode {
            SelectionMode::Nucleotide => element_id,
            SelectionMode::Design => design_id,
            SelectionMode::Strand => self.designs[design_id as usize].get_strand(element_id),
            SelectionMode::Helix => self.designs[design_id as usize].get_helix(element_id),
            SelectionMode::Grid => element_id,
        }
    }

    /// Return the group to which a phantom element belongs. The group depends on self.selection_mode.
    fn get_group_identifier_phantom(&self, phantom_element: PhantomElement) -> Option<u32> {
        let nucl = Nucl {
            helix: phantom_element.helix_id as usize,
            forward: phantom_element.forward,
            position: phantom_element.position as isize,
        };

        let design_id = phantom_element.design_id;
        let element_id = self.designs[design_id as usize].get_identifier_nucl(nucl);

        match self.selection_mode {
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
    fn get_group_member(&self, design_id: u32, group_id: u32) -> HashSet<u32> {
        match self.selection_mode {
            SelectionMode::Nucleotide => vec![group_id].into_iter().collect(),
            SelectionMode::Design => self.designs[design_id as usize].get_all_elements(),
            SelectionMode::Strand => self.designs[design_id as usize].get_strand_elements(group_id),
            SelectionMode::Helix => self.designs[design_id as usize].get_helix_elements(group_id),
            SelectionMode::Grid => vec![group_id].into_iter().collect(),
        }
    }

    /// Return the postion of a given element, either in the world pov or in the model pov
    pub fn get_element_position(
        &self,
        element: &SceneElement,
        referential: Referential,
    ) -> Option<Vec3> {
        let design_id = element.get_design()?;
        let design = self.designs.get(design_id as usize)?;
        match self.selection_mode {
            SelectionMode::Helix => design.get_element_axis_position(element, referential),
            SelectionMode::Nucleotide
            | SelectionMode::Strand
            | SelectionMode::Design
            | SelectionMode::Grid => design.get_element_position(element, referential),
        }
    }

    pub fn get_selected_position(&self) -> Option<Vec3> {
        self.selected_position
    }

    /// Update the selection by selecting the group to which a given nucleotide belongs. Return the
    /// selected group
    pub fn set_selection(&mut self, element: Option<SceneElement>) -> Option<Selection> {
        if let Some(SceneElement::WidgetElement(_)) = element {
            return None;
        }
        let future_selection = if let Some(element) = element {
            vec![element]
        } else {
            Vec::new()
        };
        if self.selected == future_selection {
            self.toggle_widget_basis()
        } else {
            self.widget_basis = Some(WidgetBasis::World);
            self.selection_update = true;
        }
        self.selected = future_selection;
        self.update_selected_position();
        let selection = if let Some(element) = element {
            match element {
                SceneElement::DesignElement(design_id, element_id) => {
                    let group_id = self.get_group_identifier(design_id, element_id);
                    match self.selection_mode {
                        SelectionMode::Design => Selection::Design(design_id),
                        SelectionMode::Strand => Selection::Strand(design_id, group_id),
                        SelectionMode::Nucleotide => {
                            let nucl = self.designs[design_id as usize].get_nucl(group_id);
                            let bound = self.designs[design_id as usize].get_bound(group_id);
                            if let Some(nucl) = nucl {
                                Selection::Nucleotide(design_id, nucl)
                            } else if let Some((n1, n2)) = bound {
                                Selection::Bound(design_id, n1, n2)
                            } else {
                                Selection::Nothing
                            }
                        }
                        SelectionMode::Helix => Selection::Helix(design_id, group_id),
                        SelectionMode::Grid => Selection::Grid(design_id, group_id as usize),
                    }
                }
                SceneElement::Grid(d_id, g_id) => Selection::Grid(d_id, g_id),
                SceneElement::PhantomElement(phantom) => {
                    Selection::Nucleotide(phantom.design_id, phantom.to_nucl())
                }
                _ => Selection::Nothing,
            }
        } else {
            Selection::Nothing
        };
        Some(selection)
    }

    /// This function must be called when the current movement ends.
    pub fn end_movement(&mut self) {
        self.update_selected_position()
    }

    pub fn get_selected_nucl_relax(&self) -> Option<Nucl> {
        if let Some(SceneElement::DesignElement(d_id, e_id)) = self.selected.get(0) {
            self.designs[*d_id as usize].get_nucl_relax(*e_id)
        } else {
            None
        }
    }

    pub fn get_strand_raw(&self, s_id: usize, d_id: usize) -> Option<Strand> {
        self.designs[d_id].get_strand_raw(s_id)
    }

    pub fn attempt_xover(&self, target: Option<SceneElement>) -> Option<(Nucl, Nucl, usize)> {
        let mut design_id = 0;
        let source_nucl = self.get_selected_nucl_relax();
        let target_nucl = if let Some(SceneElement::DesignElement(d_id, e_id)) = target {
            design_id = d_id as usize;
            self.designs[design_id].get_nucl_relax(e_id)
        } else {
            None
        };
        source_nucl.zip(target_nucl).map(|(a, b)| (a, b, design_id))
    }

    fn update_selected_position(&mut self) {
        self.selected_position = {
            if let Some(element) = self.selected.get(0) {
                self.get_element_position(element, Referential::World)
            } else {
                None
            }
        };
    }

    /// Clear self.selected
    pub fn reset_selection(&mut self) {
        self.selection_update |= !self.selected.is_empty();
        self.selected_position = None;
        self.selected = Vec::new();
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
        let grid = if let Some(SceneElement::Grid(d_id, g_id)) = self.selected.get(0) {
            Some((*d_id, *g_id))
        } else {
            None
        };
        self.view.borrow_mut().set_selected_grid(grid);
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
            for element in self.selected.iter() {
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
        let ret = self.selection_mode == SelectionMode::Helix
            || (self.action_mode.is_build() && self.selection_mode != SelectionMode::Grid);
        if ret {
            true
        } else {
            for element in self.selected.iter() {
                if let SceneElement::PhantomElement(_) = element {
                    return true;
                }
            }
            false
        }
    }

    /// Set the set of candidates to a given nucleotide
    pub fn set_candidate(&mut self, element: Option<SceneElement>) {
        let future_candidate = if let Some(element) = element {
            vec![element]
        } else {
            Vec::new()
        };
        self.candidate_update |= self.candidates != future_candidate;
        self.candidates = future_candidate;
    }

    /// Clear the set of candidates to a given nucleotide
    pub fn reset_candidate(&mut self) {
        self.candidate_update |= !self.candidates.is_empty();
        self.candidates = Vec::new();
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
        let grid = if let Some(SceneElement::Grid(d_id, g_id)) = self.candidates.get(0) {
            Some((*d_id, *g_id))
        } else {
            None
        };
        self.view.borrow_mut().set_candidate_grid(grid);
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
            for grid in design.get_grid().iter() {
                grids.push(grid.clone());
            }
            for sphere in design.get_suggested_spheres() {
                suggested_spheres.push(sphere)
            }
            for tube in design.get_suggested_tubes() {
                suggested_tubes.push(tube)
            }
        }
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
        self.view.borrow_mut().update(ViewUpdate::Letter(letters));
        self.view
            .borrow_mut()
            .update(ViewUpdate::Grids(Rc::new(grids)));
        self.selection_update = true
    }

    fn update_discs(&mut self) {
        let mut discs = Vec::new();
        let mut letters: Vec<Vec<LetterInstance>> = vec![vec![]; 10];
        let right = self.view.borrow().get_camera().borrow().right_vec();
        let up = self.view.borrow().get_camera().borrow().up_vec();
        for (d_id, design) in self.designs.iter().enumerate() {
            for grid in design.get_grid().iter() {
                for (x, y) in design.get_helices_grid_coord(grid.id) {
                    let element = Some(SceneElement::GridCircle(d_id as u32, grid.id, x, y));
                    if self.selected.get(0) != element.as_ref()
                        && self.candidates.get(0) != element.as_ref()
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
        if let Some(SceneElement::GridCircle(d_id, g_id, x, y)) = self.selected.get(0) {
            let grid = &self.designs[*d_id as usize].get_grid()[*g_id as usize];
            let (d1, d2) = grid.disc(*x, *y, 0xAA_FF_00_00, *d_id as u32);
            discs.push(d1);
            discs.push(d2);
        }
        if let Some(SceneElement::GridCircle(d_id, g_id, x, y)) = self.candidates.get(0) {
            let grid = &self.designs[*d_id as usize].get_grid()[*g_id as usize];
            let (d1, d2) = grid.disc(*x, *y, 0xAA_00_FF_00, *d_id as u32);
            discs.push(d1);
            discs.push(d2);
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
        self.set_selection(self.selected.get(0).cloned());
        self.instance_update = true;
    }

    pub fn get_action_mode(&self) -> ActionMode {
        self.action_mode
    }

    pub fn change_action_mode(&mut self, action_mode: ActionMode) {
        self.action_mode = action_mode;
        self.instance_update = true;
        self.update_matrices();
    }

    pub fn toggle_widget_basis(&mut self) {
        if let Some(w) = self.widget_basis.as_mut() {
            w.toggle()
        }
    }

    pub fn get_widget_basis(&self) -> Rotor3 {
        match self.widget_basis.as_ref().expect("widget basis") {
            //WidgetBasis::World => Rotor3::identity(),
            WidgetBasis::World => self.get_selected_basis().unwrap(),
            WidgetBasis::Object => self.get_selected_basis().unwrap(),
        }
    }

    fn get_selected_basis(&self) -> Option<Rotor3> {
        match self.selected.get(0) {
            Some(SceneElement::DesignElement(d_id, _)) => match self.selection_mode {
                SelectionMode::Nucleotide
                | SelectionMode::Design
                | SelectionMode::Strand
                | SelectionMode::Grid => Some(self.designs[*d_id as usize].get_basis()),
                SelectionMode::Helix => {
                    let h_id = self.get_selected_group();
                    self.designs[*d_id as usize].get_helix_basis(h_id)
                }
            },
            Some(SceneElement::PhantomElement(phantom_element)) => {
                let d_id = phantom_element.design_id;
                match self.selection_mode {
                    SelectionMode::Nucleotide
                    | SelectionMode::Design
                    | SelectionMode::Strand
                    | SelectionMode::Grid => Some(self.designs[d_id as usize].get_basis()),
                    SelectionMode::Helix => {
                        let h_id = phantom_element.helix_id;
                        self.designs[d_id as usize].get_helix_basis(h_id)
                    }
                }
            }
            Some(SceneElement::Grid(d_id, g_id)) => {
                self.designs[*d_id as usize].get_grid_basis(*g_id)
            }
            _ => None,
        }
    }

    pub fn selection_can_rotate_freely(&self) -> bool {
        match self.selected.get(0) {
            Some(SceneElement::DesignElement(d_id, _)) => match self.selection_mode {
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
                match self.selection_mode {
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

    pub fn select_5prime(&mut self) {
        let selected = self.selected.get(0);
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

    pub fn select_3prime(&mut self) {
        let selected = self.selected.get(0);
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

    pub fn get_strand_builder(&mut self) -> Option<StrandBuilder> {
        if let ActionMode::Build(b) = self.action_mode {
            let selected = self.candidates.get(0)?;
            let design = selected.get_design()?;
            self.designs[design as usize].get_builder(selected, b)
        } else {
            None
        }
    }

    pub fn build_helix(&mut self, intersection: &Option<GridIntersection>) -> bool {
        if let Some(GridIntersection {
            grid_id, design_id, ..
        }) = intersection
        {
            if self.action_mode.is_build() && self.selection_mode == SelectionMode::Grid {
                self.set_selection(Some(SceneElement::Grid(*design_id as u32, *grid_id)));
                self.selection_update = true;
                true
            } else {
                false
            }
        } else {
            self.set_selection(None);
            false
        }
    }

    pub fn get_nucl_position(&self, nucl: Nucl, design_id: usize) -> Option<Vec3> {
        let design = self.designs.get(design_id)?;
        design.get_nucl_position(nucl)
    }

    pub fn select_nucl(&mut self, nucl: Nucl, design_id: usize) {
        let e_id = self.designs[design_id].get_identifier_nucl(nucl);
        if let Some(id) = e_id {
            self.set_selection(Some(SceneElement::DesignElement(design_id as u32, id)));
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
    pub fn toggle(&mut self) {
        match self {
            WidgetBasis::World => *self = WidgetBasis::Object,
            WidgetBasis::Object => *self = WidgetBasis::World,
        }
    }
}
