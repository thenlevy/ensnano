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

use super::*;

impl DesignOperation {
    pub fn label(&self) -> std::borrow::Cow<'static, str> {
        match self {
            Self::Rotation(rotation) => {
                format!("Rotation of {}", rotation.target.to_string()).into()
            }
            Self::Translation(translation) => {
                format!("Translation of {}", translation.target.to_string()).into()
            }
            Self::AddGridHelix { .. } => "Helix creation".into(),
            Self::AddTwoPointsBezier { .. } => "Bezier curve creation".into(),
            Self::RmHelices { .. } => "Helix deletion".into(),
            Self::RmXovers { .. } => "Xover deletion".into(),
            Self::Cut { nucl, .. } => format!("Cut on {:?}", nucl).into(),
            Self::GeneralXover { source, target } => {
                format!("Xover between {:?} and {:?}", source, target).into()
            }
            Self::Xover { .. } => "Xover".into(),
            Self::CrossCut { .. } => "Cut and crossover".into(),
            Self::RmStrands { .. } => "Strand deletion".into(),
            Self::AddGrid(_) => "Grid creation".into(),
            Self::RmGrid(_) => "Grid delection".into(),
            Self::RecolorStaples => "Staple recoloring".into(),
            Self::ChangeSequence { .. } => "Sequence update".into(),
            Self::ChangeColor { .. } => "Color modification".into(),
            Self::SetScaffoldId(_) => "Scaffold setting".into(),
            Self::SetScaffoldSequence { .. } => "Scaffold sequence setting".into(),
            Self::HyperboloidOperation(_) => "Nanotube operation".into(),
            Self::CleanDesign => "Clean design".into(),
            Self::HelicesToGrid(_) => "Grid creation from helices".into(),
            Self::SetHelicesPersistance {
                persistant: true, ..
            } => "Show phantom helices".into(),
            Self::SetHelicesPersistance {
                persistant: false, ..
            } => "Hide phantom helices".into(),
            Self::UpdateAttribute { .. } => "Update attribute from organizer".into(),
            Self::SetSmallSpheres { small: true, .. } => "Hide nucleotides".into(),
            Self::SetSmallSpheres { small: false, .. } => "Show nucleotides".into(),
            Self::SnapHelices { .. } => "Move 2D helices".into(),
            Self::RotateHelices { .. } => "Translate 2D helices".into(),
            Self::SetIsometry { .. } => "Set isometry of helices".into(),
            Self::RequestStrandBuilders { nucls } => format!("Build on {:?}", nucls).into(),
            Self::MoveBuilders(_) => "Move builders".into(),
            Self::SetRollHelices { .. } => "Set roll of helix".into(),
            Self::SetVisibilityHelix { visible: true, .. } => "Make helices visible".into(),
            Self::SetVisibilityHelix { visible: false, .. } => "Make helices invisible".into(),
            Self::FlipHelixGroup { .. } => "Change xover group of helices".into(),
            Self::FlipAnchors { .. } => "Set/Unset nucl anchor".into(),
            Self::AttachObject { .. } => "Move grid object".into(),
            Self::SetOrganizerTree(_) => "Update organizer tree".into(),
            Self::SetStrandName { .. } => "Update name of strand".into(),
            Self::SetGroupPivot { .. } => "Set group pivot".into(),
            Self::DeleteCamera(_) => "Delete camera".into(),
            Self::CreateNewCamera { .. } => "Create camera shortcut".into(),
            Self::SetGridPosition { .. } => "Set grid position".into(),
            Self::SetGridOrientation { .. } => "Set grid orientation".into(),
            Self::MakeSeveralXovers { .. } => "Multiple xovers".into(),
            _ => "Unamed operation".into(),
        }
    }
}
