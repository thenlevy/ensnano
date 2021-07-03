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

use crate::app_state::AddressPointer;
use ensnano_design::{
    grid::{GridDescriptor, GridPosition},
    mutate_helix, Design, Domain, DomainJunction, Helix, Nucl, Strand,
};
use ensnano_interactor::operation::Operation;
use ensnano_interactor::{
    DesignOperation, DesignRotation, DesignTranslation, DomainIdentifier, IsometryTarget,
    NeighbourDescriptor, NeighbourDescriptorGiver, Selection, StrandBuilder,
};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::grid_data::GridManager;
use ultraviolet::{Isometry2, Rotor3, Vec2, Vec3};

#[derive(Clone, Default)]
pub(super) struct Controller {
    color_idx: usize,
    state: ControllerState,
}

impl Controller {
    /// Apply an operation to the design. This will either produce a modified copy of the design,
    /// or result in an error that could be shown to the user to explain why the requested
    /// operation could no be applied.
    pub fn apply_operation(
        &self,
        design: &Design,
        operation: DesignOperation,
    ) -> Result<(OkOperation, Self), ErrOperation> {
        if !self.check_compatibilty(&operation) {
            return Err(ErrOperation::IncompatibleState);
        }
        match operation {
            DesignOperation::RecolorStaples => Ok(self.ok_apply(Self::recolor_stapples, design)),
            DesignOperation::SetScaffoldSequence(sequence) => Ok(self.ok_apply(
                |ctrl, design| ctrl.set_scaffold_sequence(design, sequence),
                design,
            )),
            DesignOperation::HelicesToGrid(selection) => {
                self.apply(|c, d| c.turn_selection_into_grid(d, selection), design)
            }
            DesignOperation::AddGrid(descriptor) => {
                Ok(self.ok_apply(|c, d| c.add_grid(d, descriptor), design))
            }
            DesignOperation::ChangeColor { color, strands } => {
                Ok(self.ok_apply(|c, d| c.change_color_strands(d, color, strands), design))
            }
            DesignOperation::SetHelicesPersistance {
                grid_ids,
                persistant,
            } => Ok(self.ok_apply(
                |c, d| c.set_helices_persisance(d, grid_ids, persistant),
                design,
            )),
            DesignOperation::SetSmallSpheres { grid_ids, small } => {
                Ok(self.ok_apply(|c, d| c.set_small_spheres(d, grid_ids, small), design))
            }
            DesignOperation::SnapHelices {
                pivots,
                translation,
            } => Ok(self.ok_apply(|c, d| c.snap_helices(d, pivots, translation), design)),
            DesignOperation::SetIsometry { helix, isometry } => {
                Ok(self.ok_apply(|c, d| c.set_isometry(d, helix, isometry), design))
            }
            DesignOperation::RotateHelices {
                helices,
                center,
                angle,
            } => Ok(self.ok_apply(|c, d| c.rotate_helices(d, helices, center, angle), design)),
            DesignOperation::Translation(translation) => {
                self.apply(|c, d| c.apply_translation(d, translation), design)
            }
            DesignOperation::Rotation(rotation) => {
                self.apply(|c, d| c.apply_rotattion(d, rotation), design)
            }
            DesignOperation::RequestStrandBuilders { nucls } => {
                self.apply(|c, d| c.request_strand_builders(d, nucls), design)
            }
            DesignOperation::MoveBuilders(n) => {
                self.apply(|c, d| c.move_strand_builders(d, n), design)
            }
            DesignOperation::Cut { nucl, .. } => self.apply(|c, d| c.cut(d, nucl), design),
            DesignOperation::Xover { .. } => Err(ErrOperation::NotImplemented),
            DesignOperation::AddGridHelix {
                position,
                length,
                start,
            } => self.apply(|c, d| c.add_grid_helix(d, position, start, length), design),
            _ => Err(ErrOperation::NotImplemented),
        }
    }

    pub fn update_pending_operation(
        &self,
        design: &Design,
        operation: Arc<dyn Operation>,
    ) -> Result<(OkOperation, Self), ErrOperation> {
        let effect = operation.effect();
        let mut ret = self.apply_operation(design, effect)?;
        ret.1.state.update_operation(operation);
        Ok(ret)
    }

    pub fn notify(&self, notification: InteractorNotification) -> Self {
        let mut new_interactor = self.clone();
        match notification {
            InteractorNotification::FinishOperation => new_interactor.state = self.state.finish(),
        }
        new_interactor
    }

    fn check_compatibilty(&self, operation: &DesignOperation) -> bool {
        match self.state {
            ControllerState::Normal => true,
            ControllerState::WithPendingOp(_) => true,
            ControllerState::ChangingColor => {
                if let DesignOperation::ChangeColor { .. } = operation {
                    true
                } else {
                    false
                }
            }
            ControllerState::ApplyingOperation { .. } => true,
            ControllerState::BuildingStrand { initializing, .. } => {
                if let DesignOperation::MoveBuilders(_) = operation {
                    true
                } else {
                    initializing
                }
            }
            _ => false,
        }
    }

    fn update_state_and_design(&mut self, design: &mut Design) {
        if let ControllerState::ApplyingOperation {
            design: design_ptr, ..
        } = &self.state
        {
            *design = design_ptr.clone_inner();
        } else {
            self.state = ControllerState::ApplyingOperation {
                design: AddressPointer::new(design.clone()),
                operation: None,
            };
        }
    }

    fn return_design(&self, design: Design) -> OkOperation {
        match self.state {
            ControllerState::Normal => OkOperation::Push(design),
            ControllerState::WithPendingOp(_) => OkOperation::Push(design),
            _ => OkOperation::Replace(design),
        }
    }

    /// Apply an opperation that cannot fail on the design
    fn ok_apply<F>(&self, design_op: F, design: &Design) -> (OkOperation, Self)
    where
        F: FnOnce(&mut Self, Design) -> Design,
    {
        let mut new_controller = self.clone();
        let returned_design = design_op(&mut new_controller, design.clone());
        (self.return_design(returned_design), new_controller)
    }

    fn apply<F>(&self, design_op: F, design: &Design) -> Result<(OkOperation, Self), ErrOperation>
    where
        F: FnOnce(&mut Self, Design) -> Result<Design, ErrOperation>,
    {
        let mut new_controller = self.clone();
        let returned_design = design_op(&mut new_controller, design.clone())?;
        Ok((self.return_design(returned_design), new_controller))
    }

    fn turn_selection_into_grid(
        &mut self,
        mut design: Design,
        selection: Vec<Selection>,
    ) -> Result<Design, ErrOperation> {
        let mut grid_manager = GridManager::new_from_design(&design);
        let helices =
            ensnano_interactor::list_of_helices(&selection).ok_or(ErrOperation::BadSelection)?;
        grid_manager.make_grid_from_helices(&mut design, &helices.1)?;
        Ok(design)
    }

    fn add_grid(&mut self, mut design: Design, descriptor: GridDescriptor) -> Design {
        let mut new_grids = Vec::clone(design.grids.as_ref());
        new_grids.push(descriptor);
        design.grids = Arc::new(new_grids);
        design
    }

    pub(super) fn is_changing_color(&self) -> bool {
        if let ControllerState::ChangingColor = self.state {
            true
        } else {
            false
        }
    }

    pub(super) fn get_strand_builders(&self) -> &[StrandBuilder] {
        if let ControllerState::BuildingStrand { builders, .. } = &self.state {
            builders.as_slice()
        } else {
            &[]
        }
    }

    fn apply_translation(
        &mut self,
        design: Design,
        translation: DesignTranslation,
    ) -> Result<Design, ErrOperation> {
        match translation.target {
            IsometryTarget::Design => Err(ErrOperation::NotImplemented),
            IsometryTarget::Helices(helices, snap) => {
                Ok(self.translate_helices(design, snap, helices, translation.translation))
            }
            IsometryTarget::Grids(grid_ids) => {
                Ok(self.translate_grids(design, grid_ids, translation.translation))
            }
        }
    }

    fn apply_rotattion(
        &mut self,
        design: Design,
        rotation: DesignRotation,
    ) -> Result<Design, ErrOperation> {
        match rotation.target {
            IsometryTarget::Design => Err(ErrOperation::NotImplemented),
            IsometryTarget::Helices(helices, snap) => Ok(self.rotate_helices_3d(
                design,
                snap,
                helices,
                rotation.rotation,
                rotation.origin,
            )),
            IsometryTarget::Grids(grid_ids) => {
                Ok(self.rotate_grids(design, grid_ids, rotation.rotation, rotation.origin))
            }
        }
    }

    fn translate_helices(
        &mut self,
        mut design: Design,
        snap: bool,
        helices: Vec<usize>,
        translation: Vec3,
    ) -> Design {
        self.update_state_and_design(&mut design);
        let mut new_helices = BTreeMap::clone(design.helices.as_ref());
        for h_id in helices.iter() {
            if let Some(h) = new_helices.get_mut(h_id) {
                mutate_helix(h, |h| h.translate(translation));
            }
        }
        let mut new_design = design.clone();
        new_design.helices = Arc::new(new_helices);
        if snap {
            self.attempt_reattach(design, new_design, &helices)
        } else {
            new_design
        }
    }

    fn rotate_helices_3d(
        &mut self,
        mut design: Design,
        snap: bool,
        helices: Vec<usize>,
        rotation: Rotor3,
        origin: Vec3,
    ) -> Design {
        self.update_state_and_design(&mut design);
        let mut new_helices = BTreeMap::clone(design.helices.as_ref());
        for h_id in helices.iter() {
            if let Some(h) = new_helices.get_mut(h_id) {
                mutate_helix(h, |h| h.rotate_arround(rotation, origin))
            }
        }
        let mut new_design = design.clone();
        new_design.helices = Arc::new(new_helices);
        if snap {
            self.attempt_reattach(design, new_design, &helices)
        } else {
            new_design
        }
    }

    fn attempt_reattach(
        &mut self,
        design: Design,
        mut new_design: Design,
        helices: &[usize],
    ) -> Design {
        let mut grid_manager = GridManager::new_from_design(&new_design);
        let mut successfull_reattach = true;
        for h_id in helices.iter() {
            successfull_reattach &= grid_manager.reattach_helix(*h_id, &mut new_design, true);
        }
        if successfull_reattach {
            new_design
        } else {
            design
        }
    }

    fn translate_grids(
        &mut self,
        mut design: Design,
        grid_ids: Vec<usize>,
        translation: Vec3,
    ) -> Design {
        self.update_state_and_design(&mut design);
        let mut new_grids = Vec::clone(design.grids.as_ref());
        for g_id in grid_ids.into_iter() {
            if let Some(desc) = new_grids.get_mut(g_id) {
                desc.position += translation;
            }
        }
        design.grids = Arc::new(new_grids);
        design
    }

    fn rotate_grids(
        &mut self,
        mut design: Design,
        grid_ids: Vec<usize>,
        rotation: Rotor3,
        origin: Vec3,
    ) -> Design {
        self.update_state_and_design(&mut design);
        let mut new_grids = Vec::clone(design.grids.as_ref());
        for g_id in grid_ids.into_iter() {
            if let Some(desc) = new_grids.get_mut(g_id) {
                desc.position -= origin;
                desc.orientation = rotation * desc.orientation;
                desc.position = rotation * desc.position;
                desc.position += origin;
            }
        }
        design.grids = Arc::new(new_grids);
        design
    }
}

/// An operation has been successfully applied on a design, resulting in a new modified design. The
/// variants of these enums indicate different ways in which the result should be handled
pub enum OkOperation {
    /// Push the current design on the undo stack and replace it by the wrapped value. This variant
    /// is produced when the operation has been peroformed on a non transitory design and can be
    /// undone.
    Push(Design),
    /// Replace the current design by the wrapped value. This variant is produced when the
    /// operation has been peroformed on a transitory design and should not been undone.
    ///
    /// This happens for example for operations that are performed by drag and drop, where each new
    /// mouse mouvement produce a new design. In this case, the successive design should not be
    /// pushed on the undo stack, since an undo is expected to revert back to the state prior to
    /// the whole drag and drop operation.
    Replace(Design),
}

#[derive(Debug)]
pub enum ErrOperation {
    NotImplemented,
    NotEnoughHelices {
        actual: usize,
        required: usize,
    },
    /// The operation cannot be applied on the current selection
    BadSelection,
    /// The controller is in a state incompatible with applying the operation
    IncompatibleState,
    CannotBuildOn(Nucl),
    CutInexistingStrand,
    GridDoesNotExist(usize),
    GridPositionAlreadyUsed,
}

impl Controller {
    fn recolor_stapples(&mut self, mut design: Design) -> Design {
        for (s_id, strand) in design.strands.iter_mut() {
            if Some(*s_id) != design.scaffold_id {
                let color = {
                    let hue = (self.color_idx as f64 * (1. + 5f64.sqrt()) / 2.).fract() * 360.;
                    let saturation =
                        (self.color_idx as f64 * 7. * (1. + 5f64.sqrt() / 2.)).fract() * 0.4 + 0.4;
                    let value =
                        (self.color_idx as f64 * 11. * (1. + 5f64.sqrt() / 2.)).fract() * 0.7 + 0.1;
                    let hsv = color_space::Hsv::new(hue, saturation, value);
                    let rgb = color_space::Rgb::from(hsv);
                    (0xFF << 24) | ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32)
                };
                self.color_idx += 1;
                strand.color = color;
            }
        }
        design
    }

    fn set_scaffold_sequence(&mut self, mut design: Design, sequence: String) -> Design {
        design.scaffold_sequence = Some(sequence);
        design
    }

    fn change_color_strands(
        &mut self,
        mut design: Design,
        color: u32,
        strands: Vec<usize>,
    ) -> Design {
        self.state = ControllerState::ChangingColor;
        for s_id in strands.iter() {
            if let Some(strand) = design.strands.get_mut(s_id) {
                strand.color = color;
            }
        }
        design
    }

    fn set_helices_persisance(
        &mut self,
        mut design: Design,
        grid_ids: Vec<usize>,
        persistant: bool,
    ) -> Design {
        for g_id in grid_ids.into_iter() {
            if persistant {
                design.no_phantoms.remove(&g_id);
            } else {
                design.no_phantoms.insert(g_id);
            }
        }
        design
    }

    fn set_small_spheres(
        &mut self,
        mut design: Design,
        grid_ids: Vec<usize>,
        small: bool,
    ) -> Design {
        for g_id in grid_ids.into_iter() {
            if small {
                design.small_spheres.insert(g_id);
            } else {
                design.small_spheres.remove(&g_id);
            }
        }
        design
    }

    fn snap_helices(&mut self, mut design: Design, pivots: Vec<Nucl>, translation: Vec2) -> Design {
        self.update_state_and_design(&mut design);
        let mut new_helices = BTreeMap::clone(design.helices.as_ref());
        for p in pivots.iter() {
            if let Some(h) = new_helices.get_mut(&p.helix) {
                if let Some(old_pos) = nucl_pos_2d(&design, p) {
                    let position = old_pos + translation;
                    let position = Vec2::new(position.x.round(), position.y.round());
                    mutate_helix(h, |h| {
                        if let Some(isometry) = h.isometry2d.as_mut() {
                            isometry.append_translation(position - old_pos)
                        }
                    })
                }
            }
        }
        design.helices = Arc::new(new_helices);
        design
    }

    fn set_isometry(&mut self, mut design: Design, h_id: usize, isometry: Isometry2) -> Design {
        let mut new_helices = BTreeMap::clone(design.helices.as_ref());
        if let Some(h) = new_helices.get_mut(&h_id) {
            mutate_helix(h, |h| h.isometry2d = Some(isometry));
            design.helices = Arc::new(new_helices);
        }
        design
    }

    fn rotate_helices(
        &mut self,
        mut design: Design,
        helices: Vec<usize>,
        center: Vec2,
        angle: f32,
    ) -> Design {
        self.update_state_and_design(&mut design);
        let angle = {
            let k = (angle / std::f32::consts::FRAC_PI_8).round();
            k * std::f32::consts::FRAC_PI_8
        };
        let mut new_helices = BTreeMap::clone(design.helices.as_ref());
        for h_id in helices.iter() {
            if let Some(h) = new_helices.get_mut(h_id) {
                mutate_helix(h, |h| {
                    if let Some(isometry) = h.isometry2d.as_mut() {
                        isometry.append_translation(-center);
                        isometry.append_rotation(ultraviolet::Rotor2::from_angle(angle));
                        isometry.append_translation(center);
                    }
                })
            }
        }
        design.helices = Arc::new(new_helices);
        design
    }

    fn request_strand_builders(
        &mut self,
        mut design: Design,
        nucls: Vec<Nucl>,
    ) -> Result<Design, ErrOperation> {
        let mut builders = Vec::with_capacity(nucls.len());
        for nucl in nucls.into_iter() {
            builders.push(
                self.request_one_builder(&mut design, nucl)
                    .ok_or(ErrOperation::CannotBuildOn(nucl))?,
            );
        }
        self.state = ControllerState::BuildingStrand {
            builders,
            initializing: true,
            // The initial design is indeed the one AFTER adding the new strands
            initial_design: AddressPointer::new(design.clone()),
        };
        Ok(design)
    }

    fn request_one_builder(&mut self, design: &mut Design, nucl: Nucl) -> Option<StrandBuilder> {
        // if there is a strand that passes through the nucleotide
        if design.get_strand_nucl(&nucl).is_some() {
            self.strand_builder_on_exisiting(design, nucl)
        } else {
            self.new_strand_builder(design, nucl)
        }
    }

    fn strand_builder_on_exisiting(
        &mut self,
        design: &Design,
        nucl: Nucl,
    ) -> Option<StrandBuilder> {
        let left = design.get_neighbour_nucl(nucl.left());
        let right = design.get_neighbour_nucl(nucl.right());
        let axis = design
            .helices
            .get(&nucl.helix)
            .map(|h| h.get_axis(&design.parameters.unwrap_or_default()))?;
        let desc = design.get_neighbour_nucl(nucl)?;
        let strand_id = desc.identifier.strand;
        let filter = |d: &NeighbourDescriptor| d.identifier != desc.identifier;
        let neighbour_desc = left.filter(filter).or(right.filter(filter));
        let stick = neighbour_desc.map(|d| d.identifier.strand) == Some(strand_id);
        if left.filter(filter).and(right.filter(filter)).is_some() {
            // TODO maybe we should do something else ?
            return None;
        }
        match design.strands.get(&strand_id).map(|s| s.length()) {
            Some(n) if n > 1 => Some(StrandBuilder::init_existing(
                desc.identifier,
                nucl,
                axis,
                desc.fixed_end,
                neighbour_desc,
                stick,
            )),
            _ => Some(StrandBuilder::init_empty(
                DomainIdentifier {
                    strand: strand_id,
                    domain: 0,
                },
                nucl,
                axis,
                neighbour_desc,
                false,
            )),
        }
    }

    fn new_strand_builder(&mut self, design: &mut Design, nucl: Nucl) -> Option<StrandBuilder> {
        let left = design.get_neighbour_nucl(nucl.left());
        let right = design.get_neighbour_nucl(nucl.right());
        let axis = design
            .helices
            .get(&nucl.helix)
            .map(|h| h.get_axis(&design.parameters.unwrap_or_default()))?;
        if left.is_some() && right.is_some() {
            return None;
        }
        let new_key = self.init_strand(design, nucl);
        Some(StrandBuilder::init_empty(
            DomainIdentifier {
                strand: new_key,
                domain: 0,
            },
            nucl,
            axis,
            left.or(right),
            true,
        ))
    }

    fn init_strand(&mut self, design: &mut Design, nucl: Nucl) -> usize {
        let s_id = design.strands.keys().max().map(|n| n + 1).unwrap_or(0);
        let color = {
            let hue = (self.color_idx as f64 * (1. + 5f64.sqrt()) / 2.).fract() * 360.;
            let saturation =
                (self.color_idx as f64 * 7. * (1. + 5f64.sqrt() / 2.)).fract() * 0.4 + 0.4;
            let value = (self.color_idx as f64 * 11. * (1. + 5f64.sqrt() / 2.)).fract() * 0.7 + 0.1;
            let hsv = color_space::Hsv::new(hue, saturation, value);
            let rgb = color_space::Rgb::from(hsv);
            (0xFF << 24) | ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32)
        };
        self.color_idx += 1;
        design.strands.insert(
            s_id,
            Strand::init(nucl.helix, nucl.position, nucl.forward, color),
        );
        s_id
    }

    fn add_strand(
        &mut self,
        design: &mut Design,
        helix: usize,
        position: isize,
        forward: bool,
    ) -> usize {
        let new_key = if let Some(k) = design.strands.keys().max() {
            *k + 1
        } else {
            0
        };
        let color = {
            let hue = (self.color_idx as f64 * (1. + 5f64.sqrt()) / 2.).fract() * 360.;
            let saturation =
                (self.color_idx as f64 * 7. * (1. + 5f64.sqrt() / 2.)).fract() * 0.4 + 0.4;
            let value = (self.color_idx as f64 * 11. * (1. + 5f64.sqrt() / 2.)).fract() * 0.7 + 0.1;
            let hsv = color_space::Hsv::new(hue, saturation, value);
            let rgb = color_space::Rgb::from(hsv);
            (0xFF << 24) | ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32)
        };
        self.color_idx += 1;

        design
            .strands
            .insert(new_key, Strand::init(helix, position, forward, color));
        new_key
    }

    fn move_strand_builders(&mut self, _: Design, n: isize) -> Result<Design, ErrOperation> {
        if let ControllerState::BuildingStrand {
            initial_design,
            builders,
            initializing,
        } = &mut self.state
        {
            let mut design = initial_design.clone_inner();
            for builder in builders.iter_mut() {
                builder.move_to(n, &mut design)
            }
            *initializing = false;
            Ok(design)
        } else {
            Err(ErrOperation::IncompatibleState)
        }
    }

    fn cut(&mut self, mut design: Design, nucl: Nucl) -> Result<Design, ErrOperation> {
        let _ = Self::split_strand(&mut design, &nucl, None)?;
        Ok(design)
    }

    /// Split a strand at nucl, and return the id of the newly created strand
    ///
    /// The part of the strand that contains nucl is given the original
    /// strand's id, the other part is given a new id.
    ///
    /// If `force_end` is `Some(true)`, nucl will be on the 3 prime half of the split.
    /// If `force_end` is `Some(false)` nucl will be on the 5 prime half of the split.
    /// If `force_end` is `None`, nucl will be on the 5 prime half of the split unless nucl is the 3
    /// prime extremity of a crossover, in which case nucl will be on the 3 prime half of the
    /// split.
    fn split_strand(
        design: &mut Design,
        nucl: &Nucl,
        force_end: Option<bool>,
    ) -> Result<usize, ErrOperation> {
        let id = design
            .get_strand_nucl(nucl)
            .ok_or(ErrOperation::CutInexistingStrand)?;

        let strand = design.strands.remove(&id).expect("strand");
        if strand.cyclic {
            let new_strand = Self::break_cycle(strand.clone(), *nucl, force_end);
            design.strands.insert(id, new_strand);
            //self.clean_domains_one_strand(id);
            //println!("Cutting cyclic strand");
            return Ok(id);
        }
        if strand.length() <= 1 {
            // return without putting the strand back
            return Err(ErrOperation::CutInexistingStrand);
        }
        let mut i = strand.domains.len();
        let mut prim5_domains = Vec::new();
        let mut len_prim5 = 0;
        let mut domains = None;
        let mut on_3prime = force_end.unwrap_or(false);
        let mut prev_helix = None;
        let mut prime5_junctions: Vec<DomainJunction> = Vec::new();
        let mut prime3_junctions: Vec<DomainJunction> = Vec::new();

        println!("Spliting");
        println!("{:?}", strand.domains);
        println!("{:?}", strand.junctions);

        for (d_id, domain) in strand.domains.iter().enumerate() {
            if domain.prime5_end() == Some(*nucl)
                && prev_helix != domain.helix()
                && force_end != Some(false)
            {
                // nucl is the 5' end of the next domain so it is the on the 3' end of a xover.
                // nucl is not required to be on the 5' half of the split, so we put it on the 3'
                // half
                on_3prime = true;
                i = d_id;
                if let Some(j) = prime5_junctions.last_mut() {
                    *j = DomainJunction::Prime3;
                }
                break;
            } else if domain.prime3_end() == Some(*nucl) && force_end != Some(true) {
                // nucl is the 3' end of the current domain so it is the on the 5' end of a xover.
                // nucl is not required to be on the 3' half of the split, so we put it on the 5'
                // half
                i = d_id + 1;
                prim5_domains.push(domain.clone());
                len_prim5 += domain.length();
                prime5_junctions.push(DomainJunction::Prime3);
                break;
            } else if let Some(n) = domain.has_nucl(nucl) {
                let n = if force_end == Some(true) { n - 1 } else { n };
                i = d_id;
                len_prim5 += n;
                domains = domain.split(n);
                prime5_junctions.push(DomainJunction::Prime3);
                prime3_junctions.push(strand.junctions[d_id].clone());
                break;
            } else {
                len_prim5 += domain.length();
                prim5_domains.push(domain.clone());
                prime5_junctions.push(strand.junctions[d_id].clone());
            }
            prev_helix = domain.helix();
        }

        let mut prim3_domains = Vec::new();
        if let Some(ref domains) = domains {
            prim5_domains.push(domains.0.clone());
            prim3_domains.push(domains.1.clone());
            i += 1;
        }

        for n in i..strand.domains.len() {
            let domain = &strand.domains[n];
            prim3_domains.push(domain.clone());
            prime3_junctions.push(strand.junctions[n].clone());
        }

        let seq_prim5;
        let seq_prim3;
        if let Some(seq) = strand.sequence {
            let seq = seq.into_owned();
            let chars = seq.chars();
            seq_prim5 = Some(Cow::Owned(chars.clone().take(len_prim5).collect()));
            seq_prim3 = Some(Cow::Owned(chars.clone().skip(len_prim5).collect()));
        } else {
            seq_prim3 = None;
            seq_prim5 = None;
        }

        println!("prime5 {:?}", prim5_domains);
        println!("prime5 {:?}", prime5_junctions);

        println!("prime3 {:?}", prim3_domains);
        println!("prime3 {:?}", prime3_junctions);
        let strand_5prime = Strand {
            domains: prim5_domains,
            color: strand.color,
            junctions: prime5_junctions,
            cyclic: false,
            sequence: seq_prim5,
        };

        let strand_3prime = Strand {
            domains: prim3_domains,
            color: strand.color,
            cyclic: false,
            junctions: prime3_junctions,
            sequence: seq_prim3,
        };
        let new_id = (*design.strands.keys().max().unwrap_or(&0)).max(id) + 1;
        println!("new id {}, ; id {}", new_id, id);
        let (id_5prime, id_3prime) = if !on_3prime {
            (id, new_id)
        } else {
            (new_id, id)
        };
        if strand_5prime.domains.len() > 0 {
            design.strands.insert(id_5prime, strand_5prime);
        }
        if strand_3prime.domains.len() > 0 {
            design.strands.insert(id_3prime, strand_3prime);
        }
        //self.make_hash_maps();

        /*
        if crate::MUST_TEST {
            self.test_named_junction("TEST AFTER SPLIT STRAND");
        }*/
        Ok(new_id)
    }

    /// Split a cyclic strand at nucl
    ///
    /// If `force_end` is `Some(true)`, nucl will be the new 5' end of the strand.
    /// If `force_end` is `Some(false)` nucl will be the new 3' end of the strand.
    /// If `force_end` is `None`, nucl will be the new 3' end of the strand unless nucl is the 3'
    /// prime extremity of a crossover, in which case nucl will be the new 5' end of the strand
    fn break_cycle(mut strand: Strand, nucl: Nucl, force_end: Option<bool>) -> Strand {
        let mut last_dom = None;
        let mut replace_last_dom = None;
        let mut prev_helix = None;

        let mut junctions: Vec<DomainJunction> = Vec::with_capacity(strand.domains.len());

        for (i, domain) in strand.domains.iter().enumerate() {
            if domain.prime5_end() == Some(nucl)
                && prev_helix != domain.helix()
                && force_end != Some(false)
            {
                last_dom = if i != 0 {
                    Some(i - 1)
                } else {
                    Some(strand.domains.len() - 1)
                };

                break;
            } else if domain.prime3_end() == Some(nucl) && force_end != Some(true) {
                last_dom = Some(i);
                break;
            } else if let Some(n) = domain.has_nucl(&nucl) {
                let n = if force_end == Some(true) { n - 1 } else { n };
                last_dom = Some(i);
                replace_last_dom = domain.split(n);
            }
            prev_helix = domain.helix();
        }
        let last_dom = last_dom.expect("Could not find nucl in strand");
        let mut new_domains = Vec::new();
        if let Some((_, ref d2)) = replace_last_dom {
            new_domains.push(d2.clone());
            junctions.push(strand.junctions[last_dom].clone());
        }
        for (i, d) in strand.domains.iter().enumerate().skip(last_dom + 1) {
            new_domains.push(d.clone());
            junctions.push(strand.junctions[i].clone());
        }
        for (i, d) in strand.domains.iter().enumerate().take(last_dom) {
            new_domains.push(d.clone());
            junctions.push(strand.junctions[i].clone());
        }

        if let Some((ref d1, _)) = replace_last_dom {
            new_domains.push(d1.clone())
        } else {
            new_domains.push(strand.domains[last_dom].clone())
        }
        junctions.push(DomainJunction::Prime3);

        strand.domains = new_domains;
        strand.cyclic = false;
        strand.junctions = junctions;
        strand
    }

    fn add_grid_helix(
        &mut self,
        mut design: Design,
        position: GridPosition,
        start: isize,
        length: usize,
    ) -> Result<Design, ErrOperation> {
        let grid_manager = GridManager::new_from_design(&design);
        if grid_manager
            .pos_to_helix(position.grid, position.x, position.y)
            .is_some()
        {
            return Err(ErrOperation::GridPositionAlreadyUsed);
        }
        let grid = grid_manager
            .grids
            .get(position.grid)
            .ok_or(ErrOperation::GridDoesNotExist(position.grid))?;
        let mut new_helices = BTreeMap::clone(design.helices.as_ref());
        let helix = Helix::new_on_grid(grid, position.x, position.y, position.grid);
        let helix_id = new_helices.keys().last().unwrap_or(&0) + 1;
        new_helices.insert(helix_id, Arc::new(helix));
        if length > 0 {
            for b in [false, true].iter() {
                let new_key = self.add_strand(&mut design, helix_id, start, *b);
                if let Domain::HelixDomain(ref mut dom) =
                    design.strands.get_mut(&new_key).unwrap().domains[0]
                {
                    dom.end = dom.start + length as isize;
                }
            }
        }
        design.helices = Arc::new(new_helices);
        Ok(design)
    }
}

fn nucl_pos_2d(design: &Design, nucl: &Nucl) -> Option<Vec2> {
    let local_position = nucl.position as f32 * Vec2::unit_x()
        + if nucl.forward {
            Vec2::zero()
        } else {
            Vec2::unit_y()
        };
    let isometry = design.helices.get(&nucl.helix).and_then(|h| h.isometry2d);

    isometry.map(|i| i.into_homogeneous_matrix().transform_point2(local_position))
}

#[derive(Clone)]
enum ControllerState {
    Normal,
    MakingHyperboloid,
    BuildingStrand {
        builders: Vec<StrandBuilder>,
        initial_design: AddressPointer<Design>,
        initializing: bool,
    },
    ChangingColor,
    WithPendingOp(Arc<dyn Operation>),
    ApplyingOperation {
        design: AddressPointer<Design>,
        operation: Option<Arc<dyn Operation>>,
    },
}

impl Default for ControllerState {
    fn default() -> Self {
        Self::Normal
    }
}

impl ControllerState {
    fn update_operation(&mut self, op: Arc<dyn Operation>) {
        match self {
            Self::ApplyingOperation { operation, .. } => *operation = Some(op),
            Self::WithPendingOp(old_op) => *old_op = op,
            _ => (),
        }
    }

    fn get_operation(&self) -> Option<Arc<dyn Operation>> {
        match self {
            Self::ApplyingOperation { operation, .. } => operation.clone(),
            Self::WithPendingOp(op) => Some(op.clone()),
            _ => None,
        }
    }

    fn finish(&self) -> Self {
        if let Some(op) = self.get_operation() {
            Self::WithPendingOp(op)
        } else {
            Self::Normal
        }
    }
}

pub enum InteractorNotification {
    FinishOperation,
}
