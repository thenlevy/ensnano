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

pub enum ParameterField {
    Choice(Vec<String>),
    Value,
}

pub struct Parameter {
    pub field: ParameterField,
    pub name: String,
}

use std::sync::Arc;

pub trait Operation: std::fmt::Debug + Sync + Send {
    /// The set of parameters that can be modified via a GUI component
    fn parameters(&self) -> Vec<Parameter>;
    /// The values associated to the parameters.
    fn values(&self) -> Vec<String>;
    /// The effect of self that must be sent as a notifications to the targeted designs
    fn effect(&self) -> super::DesignOperation;
    /// A description of self of display in the GUI
    fn description(&self) -> String;
    /// Produce an new opperation by setting the value of the `n`-th parameter to `val`.
    fn with_new_value(&self, n: usize, val: String) -> Option<Arc<dyn Operation>>;

    fn must_reverse(&self) -> bool {
        true
    }

    fn drop_undo(&self) -> bool {
        false
    }

    fn redoable(&self) -> bool {
        true
    }
}
