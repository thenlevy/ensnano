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

use ensnano_design::{grid::GridDescriptor, Design};
use ensnano_interactor::{DesignOperation, Selection};

use super::grid_data::GridManager;

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
            _ => Err(ErrOperation::NotImplemented),
        }
    }

    pub fn notify(&self, notification: InteractorNotification) -> Self {
        let mut new_interactor = self.clone();
        match notification {
            InteractorNotification::FinishChangingColor => {
                new_interactor.state = ControllerState::Normal
            }
        }
        new_interactor
    }

    fn check_compatibilty(&self, operation: &DesignOperation) -> bool {
        match self.state {
            ControllerState::Normal => true,
            ControllerState::ChangingColor => {
                if let DesignOperation::ChangeColor { .. } = operation {
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn return_design(&self, design: Design) -> OkOperation {
        match self.state {
            ControllerState::Normal => OkOperation::Push(design),
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
        design.grids.push(descriptor);
        design
    }

    pub(super) fn is_changing_color(&self) -> bool {
        self.state == ControllerState::ChangingColor
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
}

#[derive(Clone, PartialEq, Eq)]
enum ControllerState {
    Normal,
    MakingHyperboloid,
    BuildingStrand,
    ChangingColor,
}

impl Default for ControllerState {
    fn default() -> Self {
        Self::Normal
    }
}

pub enum InteractorNotification {
    FinishChangingColor,
}
