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

use super::AddressPointer;
use ensnano_design::Design;
use std::cell::{Ref, RefCell};

mod presenter;
use presenter::Presenter;
mod controller;
use controller::Controller;

/// The `DesignInteractor` handles all read/write operations on the design. It is a stateful struct
/// so it is meant to be unexpansive to clone.
#[derive(Clone, Default)]
pub struct DesignInteractor {
    /// The current design
    design: AddressPointer<Design>,
    /// The structure that handles "read" operations. The graphic components of EnsNano access the
    /// presenter via a trait that defines each components needs.
    presenter: RefCell<AddressPointer<Presenter>>,
    /// The structure that handles "write" operations.
    controller: AddressPointer<Controller>,
}

impl DesignInteractor {
    pub(super) fn get_design_reader<'a>(&'a self) -> DesignReader<'a> {
        self.presenter.replace_with(|presenter| {
            AddressPointer::new(presenter.clone_inner().update(self.design.clone()))
        });
        DesignReader {
            presenter: self.presenter.borrow(),
        }
    }
}

/// A reference to a Presenter that is guaranted to always have up to date internal data
/// structures.
pub(super) struct DesignReader<'a> {
    presenter: Ref<'a, AddressPointer<Presenter>>,
}
