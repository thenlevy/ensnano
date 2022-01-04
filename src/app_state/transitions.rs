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

use ensnano_interactor::application::Camera3D;

use super::AppState;
use std::borrow::Cow;

/// Represents an undoable operation.
pub struct AppStateTransition {
    /// The state that the operation was performed on.
    pub state: AppState,
    /// A label describing the operation that was performed. It is meant to be displayed in app.
    pub label: TransitionLabel,
    /// The position of the 3d scene's camera at the moment the operation was performed
    pub camera_3d: Camera3D,
}

/// A label describing an operation.
/// To create a `TransitionLabel`, use its `From<String>` or `From<'static str>` implementation
#[derive(Clone, Debug)]
pub struct TransitionLabel(Cow<'static, str>);

impl<T: Into<Cow<'static, str>>> From<T> for TransitionLabel {
    fn from(x: T) -> Self {
        Self(x.into())
    }
}

impl AsRef<str> for TransitionLabel {
    fn as_ref(&self) -> &str {
        &self.0.as_ref()
    }
}

#[derive(Debug)]
pub enum OkOperation {
    NotUndoable,
    Undoable {
        state: AppState,
        label: TransitionLabel,
    },
}
