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

use super::SimulationUpdate;
use crate::app_state::AddressPointer;
use ensnano_design::{
    elements::{DnaAttribute, DnaElementKey},
    grid::{Edge, GridDescriptor, GridPosition, Hyperboloid},
    group_attributes::GroupPivot,
    mutate_in_arc, CameraId, Design, Domain, DomainJunction, Helix, Nucl, Strand,
};
use ensnano_interactor::{operation::Operation, HyperboloidOperation, SimulationState};
use ensnano_interactor::{
    DesignOperation, DesignRotation, DesignTranslation, DomainIdentifier, IsometryTarget,
    NeighbourDescriptor, NeighbourDescriptorGiver, Selection, StrandBuilder,
};
use ensnano_organizer::GroupId;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use clipboard::{PastedStrand, StrandClipboard};

use self::simulations::{
    GridSystemInterface, GridsSystemThread, HelixSystemInterface, HelixSystemThread,
    PhysicalSystem, RollInterface,
};

use super::grid_data::GridManager;
use ultraviolet::{Isometry2, Rotor3, Vec2, Vec3};

mod clipboard;
use clipboard::Clipboard;
pub use clipboard::CopyOperation;

mod shift_optimization;
use ahash::AHashMap;
pub use shift_optimization::{ShiftOptimizationResult, ShiftOptimizerReader};

mod simulations;
pub use simulations::{
    GridPresenter, HelixPresenter, RigidHelixState, RollPresenter, ShakeTarget,
    SimulationInterface, SimulationOperation, SimulationReader,
};

#[derive(Clone, Default)]
pub(super) struct Controller {
    color_idx: usize,
    state: ControllerState,
    clipboard: AddressPointer<Clipboard>,
}

impl Controller {
    fn new_color(color_idx: &mut usize) -> u32 {
        let color = {
            let hue = (*color_idx as f64 * (1. + 5f64.sqrt()) / 2.).fract() * 360.;
            let saturation = (*color_idx as f64 * 7. * (1. + 5f64.sqrt() / 2.)).fract() * 0.4 + 0.4;
            let value = (*color_idx as f64 * 11. * (1. + 5f64.sqrt() / 2.)).fract() * 0.7 + 0.1;
            let hsv = color_space::Hsv::new(hue, saturation, value);
            let rgb = color_space::Rgb::from(hsv);
            (0xFF << 24) | ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | (rgb.b as u32)
        };
        *color_idx += 1;
        color
    }

    /// Apply an operation to the design. This will either produce a modified copy of the design,
    /// or result in an error that could be shown to the user to explain why the requested
    /// operation could no be applied.
    pub fn apply_operation(
        &self,
        design: &Design,
        operation: DesignOperation,
    ) -> Result<(OkOperation, Self), ErrOperation> {
        log::debug!("operation {:?}", operation);
        match self.check_compatibilty(&operation) {
            OperationCompatibility::Incompatible => return Err(ErrOperation::IncompatibleState),
            OperationCompatibility::FinishFirst => return Err(ErrOperation::FinishFirst),
            OperationCompatibility::Compatible => (),
        }
        log::debug!("applicable");
        match operation {
            DesignOperation::RecolorStaples => Ok(self.ok_apply(Self::recolor_stapples, design)),
            DesignOperation::SetScaffoldSequence { sequence, shift } => Ok(self.ok_apply(
                |ctrl, design| ctrl.set_scaffold_sequence(design, sequence, shift),
                design,
            )),
            DesignOperation::SetScaffoldShift(shift) => {
                Ok(self.ok_apply(|c, d| c.set_scaffold_shift(d, shift), design))
            }
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
            DesignOperation::AddGridHelix {
                position,
                length,
                start,
            } => self.apply(|c, d| c.add_grid_helix(d, position, start, length), design),
            DesignOperation::CrossCut {
                target_3prime,
                source_id,
                target_id,
                nucl,
            } => self.apply(
                |c, d| c.apply_cross_cut(d, source_id, target_id, nucl, target_3prime),
                design,
            ),
            DesignOperation::Xover {
                prime5_id,
                prime3_id,
            } => self.apply(|c, d| c.apply_merge(d, prime5_id, prime3_id), design),
            DesignOperation::GeneralXover { source, target } => {
                self.apply(|c, d| c.apply_general_cross_over(d, source, target), design)
            }
            DesignOperation::RmStrands { strand_ids } => {
                self.apply(|c, d| c.delete_strands(d, strand_ids), design)
            }
            DesignOperation::RmHelices { h_ids } => {
                self.apply(|c, d| c.delete_helices(d, h_ids), design)
            }
            DesignOperation::RmXovers { xovers } => {
                self.apply(|c, d| c.delete_xovers(d, &xovers), design)
            }
            DesignOperation::SetScaffoldId(s_id) => Ok(self.ok_apply(
                |_, mut d| {
                    d.scaffold_id = s_id;
                    d
                },
                design,
            )),
            DesignOperation::HyperboloidOperation(op) => {
                self.apply(|c, d| c.apply_hyperbolid_operation(d, op), design)
            }
            DesignOperation::SetRollHelices { helices, roll } => {
                self.apply(|c, d| c.set_roll_helices(d, helices, roll), design)
            }
            DesignOperation::SetVisibilityHelix { helix, visible } => {
                self.apply(|c, d| c.set_visiblity_helix(d, helix, visible), design)
            }
            DesignOperation::FlipHelixGroup { helix } => {
                self.apply(|c, d| c.flip_helix_group(d, helix), design)
            }
            DesignOperation::UpdateAttribute {
                attribute,
                elements,
            } => self.apply(|c, d| c.update_attribute(d, attribute, elements), design),
            DesignOperation::FlipAnchors { nucls } => {
                self.apply(|c, d| c.flip_anchors(d, nucls), design)
            }
            DesignOperation::RmGrid(_) => Err(ErrOperation::NotImplemented), // TODO
            DesignOperation::ChangeSequence { .. } => Err(ErrOperation::NotImplemented), // TODO
            DesignOperation::CleanDesign => Err(ErrOperation::NotImplemented), // TODO
            DesignOperation::AttachHelix { helix, grid, x, y } => {
                self.apply(|c, d| c.attach_helix(d, helix, grid, x, y), design)
            }
            DesignOperation::SetOrganizerTree(tree) => Ok(self.ok_apply(
                |_, mut d| {
                    d.organizer_tree = Some(Arc::new(tree));
                    d
                },
                design,
            )),
            DesignOperation::SetStrandName { s_id, name } => {
                self.apply(|c, d| c.change_strand_name(d, s_id, name), design)
            }
            DesignOperation::SetGroupPivot { group_id, pivot } => {
                self.apply(|c, d| c.set_group_pivot(d, group_id, pivot), design)
            }
            DesignOperation::CreateNewCamera {
                position,
                orientation,
            } => Ok(self.ok_apply(|c, d| c.create_camera(d, position, orientation), design)),
            DesignOperation::DeleteCamera(cam_id) => {
                self.apply(|c, d| c.delete_camera(d, cam_id), design)
            }
            DesignOperation::SetFavouriteCamera(cam_id) => {
                self.apply(|c, d| c.set_favourite_camera(d, cam_id), design)
            }
            DesignOperation::UpdateCamera {
                camera_id,
                position,
                orientation,
            } => self.apply(
                |c, d| c.update_camera(d, camera_id, position, orientation),
                design,
            ),
            DesignOperation::SetCameraName { camera_id, name } => {
                self.apply(|c, d| c.set_camera_name(d, camera_id, name), design)
            }
            DesignOperation::SetGridPosition { grid_id, position } => {
                self.apply(|c, d| c.set_grid_position(d, grid_id, position), design)
            }
            DesignOperation::SetGridOrientation {
                grid_id,
                orientation,
            } => self.apply(
                |c, d| c.set_grid_orientation(d, grid_id, orientation),
                design,
            ),
        }
    }

    pub fn update_pending_operation(
        &self,
        design: &Design,
        operation: Arc<dyn Operation>,
    ) -> Result<(OkOperation, Self), ErrOperation> {
        let effect = operation.effect();
        let design = if operation.replace_previous() {
            if let ControllerState::WithPendingOp { design, .. } = &self.state {
                design.as_ref()
            } else {
                design
            }
        } else {
            design
        };
        let mut ret = self.apply_operation(design, effect)?;
        ret.1.state.update_operation(operation);
        Ok(ret)
    }

    pub fn apply_copy_operation(
        &self,
        design: &Design,
        operation: CopyOperation,
    ) -> Result<(OkOperation, Self), ErrOperation> {
        match operation {
            CopyOperation::CopyStrands(strand_ids) => {
                self.apply_no_op(|c, d| c.set_templates(d, strand_ids), design)
            }
            CopyOperation::CopyXovers(xovers) => {
                self.apply_no_op(|c, _d| c.copy_xovers(xovers), design)
            }
            CopyOperation::PositionPastingPoint(nucl) => {
                if self.get_pasting_point() == Some(nucl) {
                    Ok((OkOperation::NoOp, self.clone()))
                } else {
                    let design_pasted_on = if let Some(p) = self.get_design_before_pasting_xovers()
                    {
                        p.as_ref()
                    } else {
                        design
                    };
                    self.apply(|c, d| c.position_copy(d, nucl), design_pasted_on)
                }
            }
            CopyOperation::InitStrandsDuplication(strand_ids) => self.apply_no_op(
                |c, d| {
                    c.set_templates(d, strand_ids)?;
                    let clipboard = c.clipboard.as_ref().get_strand_clipboard()?;
                    c.state = ControllerState::PositioningDuplicationPoint {
                        pasted_strands: vec![],
                        duplication_edge: None,
                        pasting_point: None,
                        clipboard,
                    };
                    Ok(())
                },
                design,
            ),
            CopyOperation::Duplicate => self.apply(|c, d| c.apply_duplication(d), design),
            CopyOperation::Paste => self.make_undoable(self.apply(|c, d| c.apply_paste(d), design)),
            CopyOperation::InitXoverDuplication(xovers) => self.apply_no_op(
                |c, d| {
                    c.copy_xovers(xovers.clone())?;
                    c.state = ControllerState::DoingFirstXoversDuplication {
                        initial_design: AddressPointer::new(d.clone()),
                        duplication_edge: None,
                        pasting_point: None,
                        xovers,
                    };
                    Ok(())
                },
                design,
            ),
        }
    }

    pub(super) fn apply_simulation_operation(
        &self,
        design: Design,
        operation: SimulationOperation,
    ) -> Result<(OkOperation, Self), ErrOperation> {
        let mut ret = self.clone();
        match operation {
            SimulationOperation::StartHelices {
                presenter,
                parameters,
                reader,
            } => {
                if self.is_in_persistant_state().is_transitory() {
                    return Err(ErrOperation::IncompatibleState);
                }
                let interface = HelixSystemThread::start_new(presenter, parameters, reader)?;
                ret.state = ControllerState::Simulating {
                    interface,
                    initial_design: AddressPointer::new(design.clone()),
                };
            }
            SimulationOperation::StartGrids {
                presenter,
                parameters,
                reader,
            } => {
                if self.is_in_persistant_state().is_transitory() {
                    return Err(ErrOperation::IncompatibleState);
                }
                let interface = GridsSystemThread::start_new(presenter, parameters, reader)?;
                ret.state = ControllerState::SimulatingGrids {
                    interface,
                    initial_design: AddressPointer::new(design.clone()),
                }
            }
            SimulationOperation::StartRoll {
                presenter,
                target_helices,
                reader,
            } => {
                if self.is_in_persistant_state().is_transitory() {
                    return Err(ErrOperation::IncompatibleState);
                }
                let interface = PhysicalSystem::start_new(presenter, target_helices, reader);
                ret.state = ControllerState::Rolling {
                    interface,
                    initial_design: AddressPointer::new(design.clone()),
                };
            }
            SimulationOperation::UpdateParameters { new_parameters } => {
                if let ControllerState::Simulating { interface, .. } = &ret.state {
                    interface.lock().unwrap().parameters_update = Some(new_parameters);
                } else if let ControllerState::SimulatingGrids { interface, .. } = &ret.state {
                    interface.lock().unwrap().parameters_update = Some(new_parameters)
                } else {
                    return Err(ErrOperation::IncompatibleState);
                }
            }
            SimulationOperation::Shake(target) => {
                if let ControllerState::Simulating { interface, .. } = &ret.state {
                    interface.lock().unwrap().nucl_shake = Some(target);
                } else {
                    return Err(ErrOperation::IncompatibleState);
                }
            }
            SimulationOperation::Stop => {
                if let ControllerState::Simulating { initial_design, .. } = &ret.state {
                    ret.state = ControllerState::WithPausedSimulation {
                        initial_design: initial_design.clone(),
                    };
                } else if let ControllerState::SimulatingGrids { .. } = &ret.state {
                    ret.state = ControllerState::Normal;
                } else if let ControllerState::Rolling { .. } = &ret.state {
                    ret.state = ControllerState::Normal
                }
            }
            SimulationOperation::Reset => {
                if let ControllerState::WithPausedSimulation { initial_design } = &ret.state {
                    let returned_design = initial_design.clone_inner();
                    ret.state = ControllerState::Normal;
                    return Ok((OkOperation::Push(returned_design), ret));
                }
            }
        }
        Ok((self.return_design(design), ret))
    }

    fn change_strand_name(
        &mut self,
        mut design: Design,
        s_id: usize,
        name: String,
    ) -> Result<Design, ErrOperation> {
        let strand = design
            .strands
            .get_mut(&s_id)
            .ok_or(ErrOperation::StrandDoesNotExist(s_id))?;
        self.state = ControllerState::ChangingStrandName { strand_id: s_id };
        strand.set_name(name);
        Ok(design)
    }

    fn add_hyperboloid_helices(
        &mut self,
        design: &mut Design,
        hyperboloid: &Hyperboloid,
        position: Vec3,
        orientation: Rotor3,
    ) {
        use ensnano_design::grid::GridDivision;
        // the hyperboloid grid is always the last one that was added to the design
        let grid_id = design.grids.len() - 1;
        let parameters = design.parameters.unwrap_or_default();
        let (helices, nb_nucl) = hyperboloid.make_helices(&parameters);
        let nb_nucl = nb_nucl.min(5000);
        let mut key = design.helices.keys().max().map(|m| m + 1).unwrap_or(0);
        let mut new_helices = BTreeMap::clone(design.helices.as_ref());
        for (i, mut h) in helices.into_iter().enumerate() {
            let origin = hyperboloid.origin_helix(&parameters, i as isize, 0);
            let z_vec = Vec3::unit_z().rotated_by(orientation);
            let y_vec = Vec3::unit_y().rotated_by(orientation);
            h.position = position + origin.x * z_vec + origin.y * y_vec;
            h.orientation = orientation * hyperboloid.orientation_helix(&parameters, i as isize, 0);
            h.grid_position = Some(GridPosition {
                grid: grid_id,
                x: i as isize,
                y: 0,
                axis_pos: 0,
                roll: 0.,
            });
            new_helices.insert(key, Arc::new(h));
            for b in [true, false].iter() {
                let new_key = self.add_strand(design, key, -(nb_nucl as isize) / 2, *b);
                if let Domain::HelixDomain(ref mut dom) =
                    design.strands.get_mut(&new_key).unwrap().domains[0]
                {
                    dom.end = dom.start + nb_nucl as isize;
                }
            }
            key += 1;
        }
        design.helices = Arc::new(new_helices);
    }

    fn set_roll_helices(
        &mut self,
        mut design: Design,
        helices: Vec<usize>,
        roll: f32,
    ) -> Result<Design, ErrOperation> {
        let mut new_helices = BTreeMap::clone(design.helices.as_ref());
        for h in helices.iter() {
            if let Some(mut helix) = new_helices.get(h).map(|ptr| Helix::clone(ptr)) {
                helix.roll = roll;
                new_helices.insert(*h, Arc::new(helix));
            } else {
                return Err(ErrOperation::HelixDoesNotExists(*h));
            }
        }
        self.state = ControllerState::SettingRollHelices;
        design.helices = Arc::new(new_helices);
        Ok(design)
    }

    fn set_visiblity_helix(
        &mut self,
        mut design: Design,
        helix: usize,
        visible: bool,
    ) -> Result<Design, ErrOperation> {
        ensnano_design::mutate_one_helix(&mut design, helix, |h| h.visible = visible)
            .ok_or(ErrOperation::HelixDoesNotExists(helix))?;
        Ok(design)
    }

    fn flip_helix_group(
        &mut self,
        mut design: Design,
        helix: usize,
    ) -> Result<Design, ErrOperation> {
        let mut new_groups = BTreeMap::clone(design.groups.as_ref());
        log::info!("setting group {:?}", new_groups.get(&helix));
        match new_groups.remove(&helix) {
            None => {
                new_groups.insert(helix, false);
            }
            Some(false) => {
                new_groups.insert(helix, true);
            }
            Some(true) => (),
        }
        design.groups = Arc::new(new_groups);
        Ok(design)
    }

    fn set_group_pivot(
        &mut self,
        mut design: Design,
        group_id: GroupId,
        pivot: GroupPivot,
    ) -> Result<Design, ErrOperation> {
        let attributes = design.group_attributes.entry(group_id).or_default();
        if attributes.pivot.is_none() {
            attributes.pivot = Some(pivot);
        }
        Ok(design)
    }

    fn update_attribute(
        &mut self,
        mut design: Design,
        attribute: DnaAttribute,
        elements: Vec<DnaElementKey>,
    ) -> Result<Design, ErrOperation> {
        log::info!("updating attribute {:?}, {:?}", attribute, elements);
        for elt in elements.iter() {
            match attribute {
                DnaAttribute::Visible(b) => self.make_element_visible(&mut design, elt, b)?,
                DnaAttribute::XoverGroup(g) => self.set_xover_group_of_elt(&mut design, elt, g)?,
                DnaAttribute::LockedForSimulations(locked) => {
                    self.set_lock_during_simulation(&mut design, elt, locked)?
                }
            }
        }
        Ok(design)
    }

    fn flip_anchors(
        &mut self,
        mut design: Design,
        nucls: Vec<Nucl>,
    ) -> Result<Design, ErrOperation> {
        let new_anchor_status = !nucls.iter().all(|n| design.anchors.contains(n));
        if new_anchor_status {
            for n in nucls.into_iter() {
                design.anchors.insert(n);
            }
        } else {
            for n in nucls.iter() {
                design.anchors.remove(n);
            }
        }
        Ok(design)
    }

    fn make_element_visible(
        &self,
        design: &mut Design,
        element: &DnaElementKey,
        visible: bool,
    ) -> Result<(), ErrOperation> {
        match element {
            DnaElementKey::Helix(helix) => {
                ensnano_design::mutate_one_helix(design, *helix, |h| h.visible = visible)
                    .ok_or(ErrOperation::HelixDoesNotExists(*helix))?;
            }
            DnaElementKey::Grid(g_id) => {
                ensnano_design::mutate_one_grid(design, *g_id, |g| g.invisible = !visible)
                    .ok_or(ErrOperation::GridDoesNotExist(*g_id))?;
            }
            _ => (),
        }
        Ok(())
    }

    fn set_xover_group_of_elt(
        &self,
        design: &mut Design,
        element: &DnaElementKey,
        group: Option<bool>,
    ) -> Result<(), ErrOperation> {
        if let DnaElementKey::Helix(h_id) = element {
            if !design.helices.contains_key(h_id) {
                return Err(ErrOperation::HelixDoesNotExists(*h_id));
            }
            let mut new_groups = BTreeMap::clone(design.groups.as_ref());
            if let Some(group) = group {
                new_groups.insert(*h_id, group);
            } else {
                new_groups.remove(h_id);
            }
            design.groups = Arc::new(new_groups);
        }
        Ok(())
    }

    fn set_lock_during_simulation(
        &self,
        design: &mut Design,
        element: &DnaElementKey,
        locked: bool,
    ) -> Result<(), ErrOperation> {
        if let DnaElementKey::Helix(h_id) = element {
            if !design.helices.contains_key(h_id) {
                return Err(ErrOperation::HelixDoesNotExists(*h_id));
            }
            ensnano_design::mutate_one_helix(design, *h_id, |h| h.locked_for_simulations = locked);
        }
        Ok(())
    }

    fn apply_hyperbolid_operation(
        &mut self,
        mut design: Design,
        operation: HyperboloidOperation,
    ) -> Result<Design, ErrOperation> {
        match operation {
            HyperboloidOperation::New {
                position,
                orientation,
                request,
            } => {
                self.state = ControllerState::MakingHyperboloid {
                    position,
                    orientation,
                    initial_design: AddressPointer::new(design.clone()),
                };
                let hyperboloid = request.to_grid();
                let grid_descriptor =
                    GridDescriptor::hyperboloid(position, orientation, hyperboloid.clone());
                design = self.add_grid(design, grid_descriptor);
                self.add_hyperboloid_helices(&mut design, &hyperboloid, position, orientation);
                Ok(design)
            }
            HyperboloidOperation::Update(request) => {
                if let ControllerState::MakingHyperboloid {
                    position,
                    orientation,
                    initial_design,
                } = &self.state
                {
                    let position = position.clone();
                    let orientation = orientation.clone();
                    design = initial_design.clone_inner();
                    let hyperboloid = request.to_grid();
                    let grid_descriptor =
                        GridDescriptor::hyperboloid(position, orientation, hyperboloid.clone());
                    design = self.add_grid(design, grid_descriptor);
                    self.add_hyperboloid_helices(&mut design, &hyperboloid, position, orientation);
                    Ok(design)
                } else {
                    Err(ErrOperation::IncompatibleState)
                }
            }
            HyperboloidOperation::Cancel => {
                if let ControllerState::MakingHyperboloid { initial_design, .. } = &self.state {
                    let design = initial_design.clone_inner();
                    self.state = ControllerState::Normal;
                    Ok(design)
                } else {
                    Err(ErrOperation::IncompatibleState)
                }
            }
            HyperboloidOperation::Finalize => {
                if let ControllerState::MakingHyperboloid { .. } = &self.state {
                    self.state = ControllerState::Normal;
                    Ok(design)
                } else {
                    Err(ErrOperation::IncompatibleState)
                }
            }
        }
    }

    pub(super) fn is_building_hyperboloid(&self) -> bool {
        if let ControllerState::MakingHyperboloid { .. } = &self.state {
            true
        } else {
            false
        }
    }

    pub fn can_iterate_duplication(&self) -> bool {
        if let ControllerState::WithPendingDuplication { .. } = self.state {
            true
        } else if let ControllerState::WithPendingXoverDuplication { .. } = self.state {
            true
        } else {
            false
        }
    }

    pub(super) fn optimize_shift(
        &self,
        chanel_reader: &mut dyn ShiftOptimizerReader,
        nucl_map: Arc<AHashMap<Nucl, u32>>,
        design: &Design,
    ) -> Result<(OkOperation, Self), ErrOperation> {
        if let OperationCompatibility::Incompatible =
            self.check_compatibilty(&DesignOperation::SetScaffoldShift(0))
        {
            return Err(ErrOperation::IncompatibleState);
        }
        Ok(self.ok_no_op(
            |c, d| c.start_shift_optimization(d, chanel_reader, nucl_map),
            design,
        ))
    }

    fn start_shift_optimization(
        &mut self,
        design: &Design,
        chanel_reader: &mut dyn ShiftOptimizerReader,
        nucl_map: Arc<AHashMap<Nucl, u32>>,
    ) {
        self.state = ControllerState::OptimizingScaffoldPosition;
        shift_optimization::optimize_shift(Arc::new(design.clone()), nucl_map, chanel_reader);
    }

    #[allow(dead_code)]
    pub fn size_of_clipboard(&self) -> usize {
        self.clipboard.size()
    }

    pub fn is_pasting(&self) -> PastingStatus {
        match self.state {
            ControllerState::PositioningPastingPoint { .. } => PastingStatus::Copy,
            ControllerState::PositioningDuplicationPoint { .. } => PastingStatus::Duplication,
            ControllerState::PastingXovers { .. } => PastingStatus::Copy,
            ControllerState::DoingFirstXoversDuplication { .. } => PastingStatus::Duplication,
            _ => PastingStatus::None,
        }
    }

    pub fn notify(&self, notification: InteractorNotification) -> Self {
        let mut new_interactor = self.clone();
        match notification {
            InteractorNotification::FinishOperation => new_interactor.state = self.state.finish(),
            InteractorNotification::NewSelection => {
                new_interactor.state = self.state.acknowledge_new_selection()
            }
        }
        new_interactor
    }

    fn check_compatibilty(&self, operation: &DesignOperation) -> OperationCompatibility {
        match self.state {
            ControllerState::Normal => OperationCompatibility::Compatible,
            ControllerState::MakingHyperboloid { .. } => {
                if let DesignOperation::HyperboloidOperation(op) = operation {
                    if let HyperboloidOperation::New { .. } = op {
                        OperationCompatibility::Incompatible
                    } else {
                        OperationCompatibility::Compatible
                    }
                } else {
                    OperationCompatibility::Incompatible
                }
            }
            ControllerState::WithPendingOp { .. } => OperationCompatibility::Compatible,
            ControllerState::WithPendingDuplication { .. } => OperationCompatibility::Compatible,
            ControllerState::ChangingColor => {
                if let DesignOperation::ChangeColor { .. } = operation {
                    OperationCompatibility::Compatible
                } else {
                    OperationCompatibility::Incompatible
                }
            }
            ControllerState::SettingRollHelices => {
                if let DesignOperation::SetRollHelices { .. } = operation {
                    OperationCompatibility::Compatible
                } else {
                    OperationCompatibility::FinishFirst
                }
            }
            ControllerState::ApplyingOperation { .. } => OperationCompatibility::Compatible,
            ControllerState::BuildingStrand { initializing, .. } => {
                if let DesignOperation::MoveBuilders(_) = operation {
                    OperationCompatibility::Compatible
                } else {
                    if initializing {
                        OperationCompatibility::FinishFirst
                    } else {
                        OperationCompatibility::Incompatible
                    }
                }
            }
            ControllerState::OptimizingScaffoldPosition => {
                if let DesignOperation::SetScaffoldShift(_) = operation {
                    OperationCompatibility::Compatible
                } else {
                    OperationCompatibility::Incompatible
                }
            }
            ControllerState::ChangingStrandName {
                strand_id: current_s_id,
            } => {
                if let DesignOperation::SetStrandName { s_id, .. } = operation {
                    if current_s_id == *s_id {
                        OperationCompatibility::Compatible
                    } else {
                        OperationCompatibility::FinishFirst
                    }
                } else {
                    OperationCompatibility::FinishFirst
                }
            }
            _ => OperationCompatibility::Incompatible,
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
        if self.is_in_persistant_state().is_persistant() {
            OkOperation::Push(design)
        } else {
            OkOperation::Replace(design)
        }
    }

    pub(super) fn get_simulation_state(&self) -> SimulationState {
        match self.state {
            ControllerState::Simulating { .. } => SimulationState::RigidHelices,
            ControllerState::WithPausedSimulation { .. } => SimulationState::Paused,
            ControllerState::SimulatingGrids { .. } => SimulationState::RigidGrid,
            ControllerState::Rolling { .. } => SimulationState::Rolling,
            _ => SimulationState::None,
        }
    }

    pub(super) fn is_in_persistant_state(&self) -> StatePersitance {
        match self.state {
            ControllerState::Normal => StatePersitance::Persistant,
            ControllerState::WithPendingOp { .. } => StatePersitance::Persistant,
            ControllerState::WithPendingDuplication { .. } => StatePersitance::Persistant,
            ControllerState::WithPendingXoverDuplication { .. } => StatePersitance::Persistant,
            ControllerState::WithPausedSimulation { .. } => StatePersitance::Persistant,
            ControllerState::SettingRollHelices { .. } => StatePersitance::NeedFinish,
            ControllerState::ChangingStrandName { .. } => StatePersitance::NeedFinish,
            _ => StatePersitance::Transitory,
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

    /// Apply an operation that modifies the interactor and not the design, and that cannot fail.
    #[allow(dead_code)]
    fn ok_no_op<F>(&self, interactor_op: F, design: &Design) -> (OkOperation, Self)
    where
        F: FnOnce(&mut Self, &Design),
    {
        let mut new_controller = self.clone();
        interactor_op(&mut new_controller, design);
        (OkOperation::NoOp, new_controller)
    }

    fn apply<F>(&self, design_op: F, design: &Design) -> Result<(OkOperation, Self), ErrOperation>
    where
        F: FnOnce(&mut Self, Design) -> Result<Design, ErrOperation>,
    {
        let mut new_controller = self.clone();
        let returned_design = design_op(&mut new_controller, design.clone())?;
        Ok((self.return_design(returned_design), new_controller))
    }

    fn make_undoable(
        &self,
        result: Result<(OkOperation, Self), ErrOperation>,
    ) -> Result<(OkOperation, Self), ErrOperation> {
        if self.state.is_undoable_once() {
            match result {
                Ok((ok_op, interactor)) => Ok((ok_op.into_undoable(), interactor)),
                Err(e) => Err(e),
            }
        } else {
            result
        }
    }

    fn apply_no_op<F>(
        &self,
        interactor_op: F,
        design: &Design,
    ) -> Result<(OkOperation, Self), ErrOperation>
    where
        F: FnOnce(&mut Self, &Design) -> Result<(), ErrOperation>,
    {
        let mut new_controller = self.clone();
        interactor_op(&mut new_controller, design)?;
        Ok((OkOperation::NoOp, new_controller))
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

    fn create_camera(&mut self, mut design: Design, position: Vec3, orientation: Rotor3) -> Design {
        design.add_camera(position, orientation);
        design
    }

    fn delete_camera(&mut self, mut design: Design, id: CameraId) -> Result<Design, ErrOperation> {
        if design.rm_camera(id).is_err() {
            Err(ErrOperation::CameraDoesNotExist(id))
        } else {
            Ok(design)
        }
    }

    fn set_favourite_camera(
        &mut self,
        mut design: Design,
        id: CameraId,
    ) -> Result<Design, ErrOperation> {
        if design.set_favourite_camera(id).is_err() {
            Err(ErrOperation::CameraDoesNotExist(id))
        } else {
            Ok(design)
        }
    }

    fn update_camera(
        &mut self,
        mut design: Design,
        id: CameraId,
        position: Vec3,
        orientation: Rotor3,
    ) -> Result<Design, ErrOperation> {
        if let Some(camera) = design.get_camera_mut(id) {
            camera.position = position;
            camera.orientation = orientation;
            Ok(design)
        } else {
            Err(ErrOperation::CameraDoesNotExist(id))
        }
    }

    fn set_camera_name(
        &mut self,
        mut design: Design,
        id: CameraId,
        name: String,
    ) -> Result<Design, ErrOperation> {
        if let Some(camera) = design.get_camera_mut(id) {
            camera.name = name;
            Ok(design)
        } else {
            Err(ErrOperation::CameraDoesNotExist(id))
        }
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
        let mut design = match translation.target {
            IsometryTarget::Design => Err(ErrOperation::NotImplemented),
            IsometryTarget::Helices(helices, snap) => {
                Ok(self.translate_helices(design, snap, helices, translation.translation))
            }
            IsometryTarget::Grids(grid_ids) => {
                Ok(self.translate_grids(design, grid_ids, translation.translation))
            }
            IsometryTarget::GroupPivot(group_id) => {
                self.translate_group_pivot(design, translation.translation, group_id)
            }
        }?;

        if let Some(group_id) = translation.group_id {
            let pivot = design
                .group_attributes
                .get_mut(&group_id)
                .and_then(|attributes| attributes.pivot.as_mut())
                .ok_or(ErrOperation::GroupHasNoPivot(group_id))?;
            pivot.position += translation.translation;
        }
        Ok(design)
    }

    fn translate_group_pivot(
        &mut self,
        mut design: Design,
        translation: Vec3,
        group_id: GroupId,
    ) -> Result<Design, ErrOperation> {
        self.update_state_and_design(&mut design);
        let pivot = design
            .group_attributes
            .get_mut(&group_id)
            .and_then(|attributes| attributes.pivot.as_mut())
            .ok_or(ErrOperation::GroupHasNoPivot(group_id))?;
        pivot.position += translation;
        Ok(design)
    }

    fn rotate_group_pivot(
        &mut self,
        mut design: Design,
        rotation: Rotor3,
        group_id: GroupId,
    ) -> Result<Design, ErrOperation> {
        self.update_state_and_design(&mut design);
        let pivot = design
            .group_attributes
            .get_mut(&group_id)
            .and_then(|attributes| attributes.pivot.as_mut())
            .ok_or(ErrOperation::GroupHasNoPivot(group_id))?;
        pivot.orientation = rotation * pivot.orientation;
        Ok(design)
    }

    fn attach_helix(
        &mut self,
        mut design: Design,
        helix: usize,
        grid: usize,
        x: isize,
        y: isize,
    ) -> Result<Design, ErrOperation> {
        self.update_state_and_design(&mut design);
        let grid_manager = GridManager::new_from_design(&design);
        if matches!(grid_manager.pos_to_helix(grid, x, y), Some(h_id) if h_id != helix) {
            Err(ErrOperation::GridPositionAlreadyUsed)
        } else {
            let mut new_helices = BTreeMap::clone(design.helices.as_ref());
            let helix_ref = new_helices
                .get_mut(&helix)
                .ok_or(ErrOperation::HelixDoesNotExists(helix))?;
            // take previous axis position if there were one
            let axis_pos = helix_ref
                .grid_position
                .map(|pos| pos.axis_pos)
                .unwrap_or_default();
            let roll = helix_ref
                .grid_position
                .map(|pos| pos.roll)
                .unwrap_or_default();
            let grid_ref = grid_manager
                .grids
                .get(grid)
                .ok_or(ErrOperation::GridDoesNotExist(grid))?;
            let helix = Helix::new_on_grid(grid_ref, x, y, grid);
            mutate_in_arc(helix_ref, |h| {
                h.grid_position = Some(GridPosition {
                    grid,
                    x,
                    y,
                    axis_pos,
                    roll,
                });
                h.position = helix.position;
            });
            design.helices = Arc::new(new_helices);
            Ok(design)
        }
    }

    fn apply_rotattion(
        &mut self,
        design: Design,
        rotation: DesignRotation,
    ) -> Result<Design, ErrOperation> {
        let mut design = match rotation.target {
            IsometryTarget::Design => Err(ErrOperation::NotImplemented),
            IsometryTarget::GroupPivot(g_id) => {
                self.rotate_group_pivot(design, rotation.rotation, g_id)
            }
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
        }?;
        if let Some(group_id) = rotation.group_id {
            let pivot = design
                .group_attributes
                .get_mut(&group_id)
                .and_then(|attributes| attributes.pivot.as_mut())
                .ok_or(ErrOperation::GroupHasNoPivot(group_id))?;
            pivot.orientation = rotation.rotation * pivot.orientation;
        }
        Ok(design)
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
                mutate_in_arc(h, |h| h.translate(translation));
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
                mutate_in_arc(h, |h| h.rotate_arround(rotation, origin))
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
            successfull_reattach &=
                grid_manager.reattach_helix(*h_id, &mut new_design, true, helices);
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
    NoOp,
}

impl OkOperation {
    fn into_undoable(self) -> Self {
        match self {
            Self::Replace(design) => Self::Push(design),
            Self::Push(design) => Self::Push(design),
            Self::NoOp => Self::NoOp,
        }
    }
}

#[derive(Debug)]
pub enum ErrOperation {
    GroupHasNoPivot(GroupId),
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
    StrandDoesNotExist(usize),
    HelixDoesNotExists(usize),
    HelixHasNoGridPosition(usize),
    CouldNotMakeEdge(GridPosition, GridPosition),
    MergingSameStrand,
    XoverOnSameHelix,
    NuclDoesNotExist(Nucl),
    XoverBetweenTwoPrime5,
    XoverBetweenTwoPrime3,
    CouldNotCreateEdges,
    EmptyOrigin,
    EmptyClipboard,
    WrongClipboard,
    CannotPasteHere,
    HelixNotEmpty(usize),
    EmptyScaffoldSequence,
    NoScaffoldSet,
    NoGrids,
    FinishFirst,
    CameraDoesNotExist(CameraId),
}

impl Controller {
    fn recolor_stapples(&mut self, mut design: Design) -> Design {
        for (s_id, strand) in design.strands.iter_mut() {
            if Some(*s_id) != design.scaffold_id {
                let color = crate::utils::new_color(&mut self.color_idx);
                strand.color = color;
            }
        }
        design
    }

    fn set_scaffold_sequence(
        &mut self,
        mut design: Design,
        sequence: String,
        shift: usize,
    ) -> Design {
        design.scaffold_sequence = Some(sequence);
        design.scaffold_shift = Some(shift);
        design
    }

    fn set_scaffold_shift(&mut self, mut design: Design, shift: usize) -> Design {
        if let ControllerState::OptimizingScaffoldPosition = self.state {
            self.state = ControllerState::Normal;
        }
        design.scaffold_shift = Some(shift);
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
                    mutate_in_arc(h, |h| {
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
            mutate_in_arc(h, |h| h.isometry2d = Some(isometry));
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
        let step = std::f32::consts::FRAC_PI_6 / 2.; // = Pi / 12 = 15 degrees
        let angle = {
            let k = (angle / step).round();
            k * step
        };
        let mut new_helices = BTreeMap::clone(design.helices.as_ref());
        for h_id in helices.iter() {
            if let Some(h) = new_helices.get_mut(h_id) {
                mutate_in_arc(h, |h| {
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
        let ignored_domains: Vec<_> = nucls
            .iter()
            .filter_map(|nucl| {
                design
                    .get_neighbour_nucl(*nucl)
                    .map(|neighbour| neighbour.identifier)
            })
            .collect();
        for nucl in nucls.into_iter() {
            builders.push(
                self.request_one_builder(&mut design, nucl, &ignored_domains)
                    .ok_or(ErrOperation::CannotBuildOn(nucl))?,
            );
        }
        self.state = ControllerState::BuildingStrand {
            builders,
            initializing: true,
            // The initial design is indeed the one AFTER adding the new strands
            initial_design: AddressPointer::new(design.clone()),
            ignored_domains,
        };
        Ok(design)
    }

    fn request_one_builder(
        &mut self,
        design: &mut Design,
        nucl: Nucl,
        ignored_domains: &[DomainIdentifier],
    ) -> Option<StrandBuilder> {
        // if there is a strand that passes through the nucleotide
        if design.get_strand_nucl(&nucl).is_some() {
            self.strand_builder_on_exisiting(design, nucl, ignored_domains)
        } else {
            self.new_strand_builder(design, nucl)
        }
    }

    fn strand_builder_on_exisiting(
        &mut self,
        design: &Design,
        nucl: Nucl,
        ignored_domains: &[DomainIdentifier],
    ) -> Option<StrandBuilder> {
        let left = design
            .get_neighbour_nucl(nucl.left())
            .filter(|n| !ignored_domains.contains(&n.identifier));
        let right = design
            .get_neighbour_nucl(nucl.right())
            .filter(|n| !ignored_domains.contains(&n.identifier));
        let axis = design
            .helices
            .get(&nucl.helix)
            .map(|h| h.get_axis(&design.parameters.unwrap_or_default()))?;
        let desc = design.get_neighbour_nucl(nucl)?;
        let strand_id = desc.identifier.strand;
        let filter =
            |d: &NeighbourDescriptor| !(d.identifier.is_same_domain_than(&desc.identifier));
        let neighbour_desc = left.filter(filter).or(right.filter(filter));
        // stick to the neighbour if it is its direct neighbour. This is because we want don't want
        // to create a gap between neighbouring domains
        let stick = neighbour_desc
            .filter(|d| (d.identifier.domain as isize - desc.identifier.domain as isize).abs() < 1)
            .is_some();
        if left.filter(filter).and(right.filter(filter)).is_some() {
            // TODO maybe we should do something else ?
            return None;
        }
        let other_end = desc
            .identifier
            .other_end()
            .filter(|d| !ignored_domains.contains(d))
            .is_some()
            .then(|| desc.fixed_end);
        match design.strands.get(&strand_id).map(|s| s.length()) {
            Some(n) if n > 1 => Some(StrandBuilder::init_existing(
                desc.identifier,
                nucl,
                axis,
                other_end,
                neighbour_desc,
                stick,
            )),
            _ => Some(StrandBuilder::init_empty(
                DomainIdentifier {
                    strand: strand_id,
                    domain: 0,
                    start: None,
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
                start: None,
            },
            nucl,
            axis,
            left.or(right),
            true,
        ))
    }

    fn init_strand(&mut self, design: &mut Design, nucl: Nucl) -> usize {
        let s_id = design.strands.keys().max().map(|n| n + 1).unwrap_or(0);
        let color = crate::utils::new_color(&mut self.color_idx);
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
        let color = crate::utils::new_color(&mut self.color_idx);
        design
            .strands
            .insert(new_key, Strand::init(helix, position, forward, color));
        new_key
    }

    fn move_strand_builders(
        &mut self,
        current_design: Design,
        n: isize,
    ) -> Result<Design, ErrOperation> {
        if let ControllerState::BuildingStrand {
            initial_design,
            builders,
            initializing,
            ignored_domains,
        } = &mut self.state
        {
            let delta = builders
                .get(0)
                .map(|b| n - b.get_moving_end_position())
                .unwrap_or(0);
            let mut design = initial_design.clone_inner();
            if builders.len() > 1 {
                let sign = delta.signum();
                let mut blocked = false;
                if delta != 0 {
                    for i in 0..(sign * delta) {
                        let mut copy_builder = builders.clone();
                        for builder in copy_builder.iter_mut() {
                            if sign > 0 && !builder.try_incr(&current_design, ignored_domains) {
                                blocked = true;
                                break;
                            } else if sign < 0
                                && !builder.try_decr(&current_design, ignored_domains)
                            {
                                blocked = true;
                                break;
                            }
                        }
                        if blocked {
                            if i == 0 {
                                return Ok(current_design);
                            }
                            break;
                        }
                        *builders = copy_builder;
                        for builder in builders.iter_mut() {
                            builder.update(&mut design);
                        }
                    }
                } else {
                    return Ok(current_design);
                }
            } else {
                for builder in builders.iter_mut() {
                    let to = builder.get_moving_end_position() + delta;
                    builder.move_to(to, &mut design, &ignored_domains)
                }
            }
            *initializing = false;
            Ok(design)
        } else {
            Err(ErrOperation::IncompatibleState)
        }
    }

    fn delete_xovers(
        &mut self,
        mut design: Design,
        xovers: &[(Nucl, Nucl)],
    ) -> Result<Design, ErrOperation> {
        for (n1, _) in xovers.iter() {
            let _ = Self::split_strand(&mut design, &n1, None)?;
        }
        Ok(design)
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
        let name = strand.name.clone();
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

        log::info!("Spliting");
        log::info!("{:?}", strand.domains);
        log::info!("{:?}", strand.junctions);

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

        log::info!("prime5 {:?}", prim5_domains);
        log::info!("prime5 {:?}", prime5_junctions);

        log::info!("prime3 {:?}", prim3_domains);
        log::info!("prime3 {:?}", prime3_junctions);
        let strand_5prime = Strand {
            domains: prim5_domains,
            color: strand.color,
            junctions: prime5_junctions,
            cyclic: false,
            sequence: seq_prim5,
            name: name.clone(),
        };

        let strand_3prime = Strand {
            domains: prim3_domains,
            color: strand.color,
            cyclic: false,
            junctions: prime3_junctions,
            sequence: seq_prim3,
            name,
        };
        let new_id = (*design.strands.keys().max().unwrap_or(&0)).max(id) + 1;
        log::info!("new id {}, ; id {}", new_id, id);
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

    /// Merge two strands with identifier prime5 and prime3. The resulting strand will have
    /// identifier prime5.
    fn merge_strands(
        design: &mut Design,
        prime5: usize,
        prime3: usize,
    ) -> Result<(), ErrOperation> {
        // We panic, if we can't find the strand, because this means that the program has a bug
        if prime5 != prime3 {
            let strand5prime = design
                .strands
                .remove(&prime5)
                .ok_or(ErrOperation::StrandDoesNotExist(prime5))?;
            let strand3prime = design
                .strands
                .remove(&prime3)
                .ok_or(ErrOperation::StrandDoesNotExist(prime3))?;
            let name = strand5prime.name.or(strand3prime.name);
            let len = strand5prime.domains.len() + strand3prime.domains.len();
            let mut domains = Vec::with_capacity(len);
            let mut junctions = Vec::with_capacity(len);
            for (i, domain) in strand5prime.domains.iter().enumerate() {
                domains.push(domain.clone());
                junctions.push(strand5prime.junctions[i].clone());
            }
            let skip;
            let last_helix = domains.last().and_then(|d| d.half_helix());
            let next_helix = strand3prime
                .domains
                .iter()
                .next()
                .and_then(|d| d.half_helix());
            if last_helix == next_helix && last_helix.is_some() {
                skip = 1;
                domains
                    .last_mut()
                    .as_mut()
                    .unwrap()
                    .merge(strand3prime.domains.iter().next().unwrap());
                junctions.pop();
            } else {
                skip = 0;
                if let Some(j) = junctions.iter_mut().last() {
                    *j = DomainJunction::UnindentifiedXover
                }
            }
            for domain in strand3prime.domains.iter().skip(skip) {
                domains.push(domain.clone());
            }
            for junction in strand3prime.junctions.iter() {
                junctions.push(junction.clone());
            }
            let sequence = if let Some((seq5, seq3)) = strand5prime
                .sequence
                .clone()
                .zip(strand3prime.sequence.clone())
            {
                let new_seq = seq5.into_owned() + &seq3.into_owned();
                Some(Cow::Owned(new_seq))
            } else if let Some(ref seq5) = strand5prime.sequence {
                Some(seq5.clone())
            } else if let Some(ref seq3) = strand3prime.sequence {
                Some(seq3.clone())
            } else {
                None
            };
            let new_strand = Strand {
                domains,
                color: strand5prime.color,
                sequence,
                junctions,
                cyclic: false,
                name,
            };
            design.strands.insert(prime5, new_strand);
            Ok(())
        } else {
            // To make a cyclic strand use `make_cyclic_strand` instead
            Err(ErrOperation::MergingSameStrand)
        }
    }

    /// Make a strand cyclic by linking the 3' and the 5' end, or undo this operation.
    fn make_cycle(design: &mut Design, strand_id: usize, cyclic: bool) -> Result<(), ErrOperation> {
        design
            .strands
            .get_mut(&strand_id)
            .ok_or(ErrOperation::StrandDoesNotExist(strand_id))?
            .cyclic = cyclic;

        let strand = design
            .strands
            .get_mut(&strand_id)
            .ok_or(ErrOperation::StrandDoesNotExist(strand_id))?;
        if cyclic {
            let first_last_domains = (strand.domains.iter().next(), strand.domains.iter().last());
            let merge_insertions =
                if let (Some(Domain::Insertion(n1)), Some(Domain::Insertion(n2))) =
                    first_last_domains
                {
                    Some(n1 + n2)
                } else {
                    None
                };
            if let Some(n) = merge_insertions {
                // If the strand starts and finishes by an Insertion, merge the insertions.
                // TODO UNITTEST for this specific case
                *strand.domains.last_mut().unwrap() = Domain::Insertion(n);
                // remove the first insertions
                strand.domains.remove(0);
                strand.junctions.remove(0);
            }

            let first_last_domains = (strand.domains.iter().next(), strand.domains.iter().last());
            let skip_last = if let (_, Some(Domain::Insertion(_))) = first_last_domains {
                1
            } else {
                0
            };
            let skip_first = if let (Some(Domain::Insertion(_)), _) = first_last_domains {
                1
            } else {
                0
            };
            let last_first_intervals = (
                strand.domains.iter().rev().skip(skip_last).next(),
                strand.domains.get(skip_first),
            );
            if let (Some(Domain::HelixDomain(i1)), Some(Domain::HelixDomain(i2))) =
                last_first_intervals
            {
                let junction = junction(i1, i2);
                *strand.junctions.last_mut().unwrap() = junction;
            } else {
                panic!("Invariant Violated: SaneDomains")
            }
        } else {
            *strand.junctions.last_mut().unwrap() = DomainJunction::Prime3;
        }
        Ok(())
    }

    fn apply_cross_cut(
        &mut self,
        mut design: Design,
        source_strand: usize,
        target_strand: usize,
        nucl: Nucl,
        target_3prime: bool,
    ) -> Result<Design, ErrOperation> {
        Self::cross_cut(
            &mut design,
            source_strand,
            target_strand,
            nucl,
            target_3prime,
        )?;
        self.state = ControllerState::Normal;
        Ok(design)
    }

    fn apply_merge(
        &mut self,
        mut design: Design,
        prime5_id: usize,
        prime3_id: usize,
    ) -> Result<Design, ErrOperation> {
        if prime5_id != prime3_id {
            Self::merge_strands(&mut design, prime5_id, prime3_id)?;
        } else {
            Self::make_cycle(&mut design, prime5_id, true)?;
        }
        self.state = ControllerState::Normal;
        Ok(design)
    }

    /// Cut the target strand at nucl and the make a cross over from the source strand to the part
    /// that contains nucl
    fn cross_cut(
        design: &mut Design,
        source_strand: usize,
        target_strand: usize,
        nucl: Nucl,
        target_3prime: bool,
    ) -> Result<(), ErrOperation> {
        let new_id = design.strands.keys().max().map(|n| n + 1).unwrap_or(0);
        let was_cyclic = design
            .strands
            .get(&target_strand)
            .ok_or(ErrOperation::StrandDoesNotExist(target_strand))?
            .cyclic;
        //println!("half1 {}, ; half0 {}", new_id, target_strand);
        Self::split_strand(design, &nucl, Some(target_3prime))?;
        //println!("splitted");

        if !was_cyclic && source_strand != target_strand {
            if target_3prime {
                // swap the position of the two half of the target strands so that the merged part is the
                // new id
                let half0 = design
                    .strands
                    .remove(&target_strand)
                    .ok_or(ErrOperation::StrandDoesNotExist(target_strand))?;
                let half1 = design
                    .strands
                    .remove(&new_id)
                    .ok_or(ErrOperation::StrandDoesNotExist(new_id))?;
                design.strands.insert(new_id, half0);
                design.strands.insert(target_strand, half1);
                Self::merge_strands(design, source_strand, new_id)
            } else {
                // if the target strand is the 5' end of the merge, we give the new id to the source
                // strand because it is the one that is lost in the merge.
                let half0 = design
                    .strands
                    .remove(&source_strand)
                    .ok_or(ErrOperation::StrandDoesNotExist(source_strand))?;
                let half1 = design
                    .strands
                    .remove(&new_id)
                    .ok_or(ErrOperation::StrandDoesNotExist(new_id))?;
                design.strands.insert(new_id, half0);
                design.strands.insert(source_strand, half1);
                Self::merge_strands(design, target_strand, new_id)
            }
        } else if source_strand == target_strand {
            Self::make_cycle(design, source_strand, true)
        } else {
            if target_3prime {
                Self::merge_strands(design, source_strand, target_strand)
            } else {
                Self::merge_strands(design, target_strand, source_strand)
            }
        }
    }

    fn apply_general_cross_over(
        &mut self,
        mut design: Design,
        source_nucl: Nucl,
        target_nucl: Nucl,
    ) -> Result<Design, ErrOperation> {
        self.general_cross_over(&mut design, source_nucl, target_nucl)?;
        Ok(design)
    }

    fn general_cross_over(
        &mut self,
        design: &mut Design,
        source_nucl: Nucl,
        target_nucl: Nucl,
    ) -> Result<(), ErrOperation> {
        if source_nucl.helix == target_nucl.helix {
            return Err(ErrOperation::XoverOnSameHelix);
        }
        log::info!("cross over between {:?} and {:?}", source_nucl, target_nucl);
        let source_id = design
            .get_strand_nucl(&source_nucl)
            .ok_or(ErrOperation::NuclDoesNotExist(source_nucl))?;
        let target_id = design
            .get_strand_nucl(&target_nucl)
            .ok_or(ErrOperation::NuclDoesNotExist(target_nucl))?;

        let source = design
            .strands
            .get(&source_id)
            .cloned()
            .ok_or(ErrOperation::StrandDoesNotExist(source_id))?;
        let _ = design
            .strands
            .get(&target_id)
            .cloned()
            .ok_or(ErrOperation::StrandDoesNotExist(target_id))?;

        let source_strand_end = design.is_strand_end(&source_nucl);
        let target_strand_end = design.is_strand_end(&target_nucl);
        log::info!(
            "source strand {:?}, target strand {:?}",
            source_id,
            target_id
        );
        log::info!(
            "source end {:?}, target end {:?}",
            source_strand_end.to_opt(),
            target_strand_end.to_opt()
        );
        match (source_strand_end.to_opt(), target_strand_end.to_opt()) {
            (Some(true), Some(true)) => return Err(ErrOperation::XoverBetweenTwoPrime3),
            (Some(false), Some(false)) => return Err(ErrOperation::XoverBetweenTwoPrime5),
            (Some(true), Some(false)) => {
                // We can xover directly
                if source_id == target_id {
                    Self::make_cycle(design, source_id, true)?
                } else {
                    Self::merge_strands(design, source_id, target_id)?
                }
            }
            (Some(false), Some(true)) => {
                // We can xover directly but we must reverse the xover
                if source_id == target_id {
                    Self::make_cycle(design, target_id, true)?
                } else {
                    Self::merge_strands(design, target_id, source_id)?
                }
            }
            (Some(b), None) => {
                // We can cut cross directly, but only if the target and source's helices are
                // different
                let target_3prime = b;
                if source_nucl.helix != target_nucl.helix {
                    Self::cross_cut(design, source_id, target_id, target_nucl, target_3prime)?
                }
            }
            (None, Some(b)) => {
                // We can cut cross directly but we need to reverse the xover
                let target_3prime = b;
                if source_nucl.helix != target_nucl.helix {
                    Self::cross_cut(design, target_id, source_id, source_nucl, target_3prime)?
                }
            }
            (None, None) => {
                if source_nucl.helix != target_nucl.helix {
                    if source_id != target_id {
                        Self::split_strand(design, &source_nucl, None)?;
                        Self::cross_cut(design, source_id, target_id, target_nucl, true)?;
                    } else if source.cyclic {
                        Self::split_strand(design, &source_nucl, Some(false))?;
                        Self::cross_cut(design, source_id, target_id, target_nucl, true)?;
                    } else {
                        // if the two nucleotides are on the same strand care must be taken
                        // because one of them might be on the newly crated strand after the
                        // split
                        let pos1 = source
                            .find_nucl(&source_nucl)
                            .ok_or(ErrOperation::NuclDoesNotExist(source_nucl))?;
                        let pos2 = source
                            .find_nucl(&target_nucl)
                            .ok_or(ErrOperation::NuclDoesNotExist(target_nucl))?;
                        if pos1 > pos2 {
                            // the source nucl will be on the 5' end of the split and the
                            // target nucl as well
                            Self::split_strand(design, &source_nucl, Some(false))?;
                            Self::cross_cut(design, source_id, target_id, target_nucl, true)?;
                        } else {
                            let new_id = Self::split_strand(design, &source_nucl, Some(false))?;
                            Self::cross_cut(design, source_id, new_id, target_nucl, true)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn delete_strands(
        &mut self,
        mut design: Design,
        strand_ids: Vec<usize>,
    ) -> Result<Design, ErrOperation> {
        for s_id in strand_ids.iter() {
            design.strands.remove(s_id);
        }
        Ok(design)
    }

    fn delete_helices(
        &mut self,
        mut design: Design,
        helices_id: Vec<usize>,
    ) -> Result<Design, ErrOperation> {
        let mut new_helices = BTreeMap::clone(design.helices.as_ref());
        for h_id in helices_id.iter() {
            if design.uses_helix(*h_id) {
                return Err(ErrOperation::HelixNotEmpty(*h_id));
            } else {
                new_helices.remove(h_id);
            }
        }
        design.helices = Arc::new(new_helices);
        Ok(design)
    }

    fn set_grid_position(
        &mut self,
        mut design: Design,
        grid_id: usize,
        position: Vec3,
    ) -> Result<Design, ErrOperation> {
        let mut new_grids = Vec::clone(design.grids.as_ref());
        let grid = new_grids
            .get_mut(grid_id)
            .ok_or(ErrOperation::GridDoesNotExist(grid_id))?;
        grid.position = position;
        design.grids = Arc::new(new_grids);
        Ok(design)
    }

    fn set_grid_orientation(
        &mut self,
        mut design: Design,
        grid_id: usize,
        orientation: Rotor3,
    ) -> Result<Design, ErrOperation> {
        let mut new_grids = Vec::clone(design.grids.as_ref());
        let grid = new_grids
            .get_mut(grid_id)
            .ok_or(ErrOperation::GridDoesNotExist(grid_id))?;
        grid.orientation = orientation;
        design.grids = Arc::new(new_grids);
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
    MakingHyperboloid {
        initial_design: AddressPointer<Design>,
        position: Vec3,
        orientation: Rotor3,
    },
    BuildingStrand {
        builders: Vec<StrandBuilder>,
        initial_design: AddressPointer<Design>,
        initializing: bool,
        ignored_domains: Vec<DomainIdentifier>,
    },
    ChangingColor,
    SettingRollHelices,
    WithPendingOp {
        operation: Arc<dyn Operation>,
        design: AddressPointer<Design>,
    },
    ApplyingOperation {
        design: AddressPointer<Design>,
        operation: Option<Arc<dyn Operation>>,
    },
    PositioningPastingPoint {
        pasting_point: Option<Nucl>,
        pasted_strands: Vec<PastedStrand>,
    },
    PositioningDuplicationPoint {
        pasting_point: Option<Nucl>,
        pasted_strands: Vec<PastedStrand>,
        duplication_edge: Option<(Edge, isize)>,
        clipboard: StrandClipboard,
    },
    WithPendingDuplication {
        last_pasting_point: Nucl,
        duplication_edge: (Edge, isize),
        clipboard: StrandClipboard,
    },
    WithPendingXoverDuplication {
        last_pasting_point: Nucl,
        duplication_edge: (Edge, isize),
        xovers: Vec<(Nucl, Nucl)>,
    },
    PastingXovers {
        initial_design: AddressPointer<Design>,
        pasting_point: Option<Nucl>,
    },
    DoingFirstXoversDuplication {
        initial_design: AddressPointer<Design>,
        duplication_edge: Option<(Edge, isize)>,
        xovers: Vec<(Nucl, Nucl)>,
        pasting_point: Option<Nucl>,
    },
    OptimizingScaffoldPosition,
    Simulating {
        interface: Arc<Mutex<HelixSystemInterface>>,
        initial_design: AddressPointer<Design>,
    },
    SimulatingGrids {
        interface: Arc<Mutex<GridSystemInterface>>,
        initial_design: AddressPointer<Design>,
    },
    WithPausedSimulation {
        initial_design: AddressPointer<Design>,
    },
    Rolling {
        interface: Arc<Mutex<RollInterface>>,
        initial_design: AddressPointer<Design>,
    },
    ChangingStrandName {
        strand_id: usize,
    },
}

impl Default for ControllerState {
    fn default() -> Self {
        Self::Normal
    }
}

impl ControllerState {
    #[allow(dead_code)]
    fn state_name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::MakingHyperboloid { .. } => "MakingHyperboloid",
            Self::BuildingStrand { .. } => "BuildingStrand",
            Self::ChangingColor => "ChangingColor",
            Self::WithPendingOp { .. } => "WithPendingOp",
            Self::ApplyingOperation { .. } => "ApplyingOperation",
            Self::PositioningPastingPoint { .. } => "PositioningPastingPoint",
            Self::PositioningDuplicationPoint { .. } => "PositioningDuplicationPoint",
            Self::WithPendingDuplication { .. } => "WithPendingDuplication",
            Self::WithPendingXoverDuplication { .. } => "WithPendingXoverDuplication",
            Self::PastingXovers { .. } => "PastingXovers",
            Self::DoingFirstXoversDuplication { .. } => "DoingFirstXoversDuplication",
            Self::OptimizingScaffoldPosition => "OptimizingScaffoldPosition",
            Self::Simulating { .. } => "Simulation",
            Self::SimulatingGrids { .. } => "Simulating Grids",
            Self::WithPausedSimulation { .. } => "WithPausedSimulation",
            Self::Rolling { .. } => "Rolling",
            Self::SettingRollHelices => "SettingRollHelices",
            Self::ChangingStrandName { .. } => "ChangingStrandName",
        }
    }
    fn update_pasting_position(
        &mut self,
        point: Option<Nucl>,
        strands: Vec<PastedStrand>,
        duplication_edge: Option<(Edge, isize)>,
    ) -> Result<(), ErrOperation> {
        match self {
            Self::PositioningPastingPoint { .. } | Self::Normal | Self::WithPendingOp { .. } => {
                *self = Self::PositioningPastingPoint {
                    pasting_point: point,
                    pasted_strands: strands,
                };
                Ok(())
            }
            Self::PositioningDuplicationPoint { clipboard, .. } => {
                *self = Self::PositioningDuplicationPoint {
                    pasting_point: point,
                    pasted_strands: strands,
                    duplication_edge,
                    clipboard: clipboard.clone(),
                };
                Ok(())
            }
            _ => Err(ErrOperation::IncompatibleState),
        }
    }

    fn update_xover_pasting_position(
        &mut self,
        point: Option<Nucl>,
        edge: Option<(Edge, isize)>,
        design: &Design,
    ) -> Result<(), ErrOperation> {
        match self {
            Self::PastingXovers { pasting_point, .. } => {
                *pasting_point = point;
                Ok(())
            }
            Self::DoingFirstXoversDuplication {
                pasting_point,
                duplication_edge,
                ..
            } => {
                *pasting_point = point;
                *duplication_edge = edge;
                Ok(())
            }
            Self::Normal | Self::WithPendingOp { .. } | Self::WithPendingDuplication { .. } => {
                *self = Self::PastingXovers {
                    pasting_point: point,
                    initial_design: AddressPointer::new(design.clone()),
                };
                Ok(())
            }
            _ => Err(ErrOperation::IncompatibleState),
        }
    }

    fn update_operation(&mut self, op: Arc<dyn Operation>) {
        match self {
            Self::ApplyingOperation { operation, .. } => *operation = Some(op),
            Self::WithPendingOp { operation, .. } => *operation = op,
            _ => (),
        }
    }

    fn get_operation(&self) -> Option<Arc<dyn Operation>> {
        match self {
            Self::ApplyingOperation { operation, .. } => operation.clone(),
            Self::WithPendingOp { operation, .. } => Some(operation.clone()),
            _ => None,
        }
    }

    fn finish(&self) -> Self {
        match self {
            Self::Normal => Self::Normal,
            Self::MakingHyperboloid { .. } => self.clone(),
            Self::BuildingStrand { .. } => Self::Normal,
            Self::ChangingColor => Self::Normal,
            Self::WithPendingOp { operation, design } => self.clone(),
            Self::ApplyingOperation {
                operation: Some(op),
                design,
            } => Self::WithPendingOp {
                operation: op.clone(),
                design: design.clone(),
            },
            Self::ApplyingOperation { .. } => Self::Normal,
            Self::PositioningPastingPoint { .. } => self.clone(),
            Self::PositioningDuplicationPoint { .. } => self.clone(),
            Self::WithPendingDuplication { .. } => self.clone(),
            Self::WithPendingXoverDuplication { .. } => self.clone(),
            Self::PastingXovers { .. } => self.clone(),
            Self::DoingFirstXoversDuplication { .. } => self.clone(),
            Self::OptimizingScaffoldPosition => self.clone(),
            Self::Simulating { .. } => self.clone(),
            Self::SimulatingGrids { .. } => self.clone(),
            Self::WithPausedSimulation { .. } => self.clone(),
            Self::Rolling { .. } => Self::Normal,
            Self::SettingRollHelices => Self::Normal,
            Self::ChangingStrandName { .. } => Self::Normal,
        }
    }

    fn acknowledge_new_selection(&self) -> Self {
        if let Self::WithPendingDuplication { .. } = self {
            Self::Normal
        } else if let Self::WithPendingXoverDuplication { .. } = self {
            Self::Normal
        } else {
            self.clone()
        }
    }

    /// Return true if the operation is undoable only when going from this state to normal
    fn is_undoable_once(&self) -> bool {
        match self {
            Self::PositioningDuplicationPoint { .. } | Self::PositioningPastingPoint { .. } => true,
            _ => false,
        }
    }
}

pub enum InteractorNotification {
    FinishOperation,
    NewSelection,
}

use ensnano_design::HelixInterval;
/// Return the appropriate junction between two HelixInterval
pub(super) fn junction(prime5: &HelixInterval, prime3: &HelixInterval) -> DomainJunction {
    let prime5_nucl = prime5.prime3();
    let prime3_nucl = prime3.prime5();

    if prime3_nucl == prime5_nucl.prime3() {
        DomainJunction::Adjacent
    } else {
        DomainJunction::UnindentifiedXover
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PastingStatus {
    Copy,
    Duplication,
    None,
}

impl PastingStatus {
    pub fn is_pasting(&self) -> bool {
        match self {
            Self::Copy | Self::Duplication => true,
            Self::None => false,
        }
    }
}

enum OperationCompatibility {
    Compatible,
    Incompatible,
    FinishFirst,
}

pub(super) enum StatePersitance {
    Persistant,
    NeedFinish,
    Transitory,
}

impl StatePersitance {
    pub fn is_persistant(&self) -> bool {
        matches!(self, StatePersitance::Persistant)
    }

    pub fn is_transitory(&self) -> bool {
        matches!(self, StatePersitance::Transitory)
    }
}
