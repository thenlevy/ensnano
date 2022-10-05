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
//! This modules defines operations that can be performed on a design to modify it.
//! The functions that apply thes operations take a mutable reference to the design that they are
//! modifying and may return an `ErrOperation` if the opperation could not be applied.

use super::{bezier_plane::*, grid::*, CurveDescriptor, Design};
use std::sync::Arc;
use ultraviolet::{Rotor3, Vec3};

/// An error that occured when trying to apply an operation.
#[derive(Debug)]
pub enum ErrOperation {
    NotEnoughHelices { actual: usize, needed: usize },
    GridPositionAlreadyUsed,
    HelixDoesNotExists(usize),
    GridDoesNotExist(GridId),
    HelixCollisionDuringTranslation,
    NotEnoughBezierPoints,
    HelixIsNotPiecewiseBezier,
    CouldNotGetPath(BezierPathId),
    CouldNotGetVertex(BezierVertexId),
}

/// The minimum number of helices requiered to infer a grid
pub const MIN_HELICES_TO_MAKE_GRID: usize = 4;

/// Try to create a grid from a set of helices.
pub fn make_grid_from_helices(design: &mut Design, helices: &[usize]) -> Result<(), ErrOperation> {
    super::grid::make_grid_from_helices(design, helices)?;
    Ok(())
}

/// Attach an helix to a grid. The target grid position must be empty
pub fn attach_object_to_grid(
    design: &mut Design,
    object: GridObject,
    grid: GridId,
    x: isize,
    y: isize,
) -> Result<(), ErrOperation> {
    let grid_manager = design.get_updated_grid_data();
    if matches!(grid_manager.pos_to_object(GridPosition{
        grid, x, y
    }), Some(obj) if obj != object)
    {
        Err(ErrOperation::GridPositionAlreadyUsed)
    } else {
        let mut helices_mut = design.helices.make_mut();
        let helix_ref = helices_mut
            .get_mut(&object.helix())
            .ok_or_else(|| ErrOperation::HelixDoesNotExists(object.helix()))?;
        // take previous axis position if there were one
        match object {
            GridObject::Helix(_) => {
                let axis_pos = helix_ref
                    .grid_position
                    .map(|pos| pos.axis_pos)
                    .unwrap_or_default();
                let roll = helix_ref
                    .grid_position
                    .map(|pos| pos.roll)
                    .unwrap_or_default();
                helix_ref.grid_position = Some(HelixGridPosition {
                    grid,
                    x,
                    y,
                    axis_pos,
                    roll,
                });
            }
            GridObject::BezierPoint { n, .. } => {
                let desc: Option<&mut CurveDescriptor> =
                    if let Some(desc) = helix_ref.curve.as_mut() {
                        Some(Arc::make_mut(desc))
                    } else {
                        None
                    };
                if let Some(CurveDescriptor::PiecewiseBezier { points, .. }) = desc {
                    if let Some(point) = points.get_mut(n) {
                        point.position = GridPosition { grid, x, y };
                    } else {
                        return Err(ErrOperation::NotEnoughBezierPoints);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Translate helices by a given translation.
///
/// If snap is true, the helices are mapped to grid position.
/// If this translation would cause helices to compete with other helices for a grid position,
/// an error is returned.
pub fn translate_helices(
    design: &mut Design,
    snap: bool,
    helices: Vec<usize>,
    translation: Vec3,
) -> Result<(), ErrOperation> {
    let mut helices_translator = HelicesTranslator::from_design(design);
    helices_translator.translate_helices(snap, helices, translation)
}

/// Rotate helices by a given rotation
///
/// If snap is true, the helices are mapped to grid position.
/// If this rotation would cause helices to compete with other helices for a grid position,
/// an error is returned.
pub fn rotate_helices_3d(
    design: &mut Design,
    snap: bool,
    helices: Vec<usize>,
    rotation: Rotor3,
    origin: Vec3,
) -> Result<(), ErrOperation> {
    let mut helices_translator = HelicesTranslator::from_design(design);
    helices_translator.rotate_helices_3d(snap, helices, rotation, origin)
}
