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
//! The modules defines the `StrandBuilder` struct. A `StrandBuilder` is responsible for edditing a
//! strand. It is initialized with a domain that might be an already existing domain or a new
//! strand beign created.
//!
//! The role of the `StrandBuilder` is to move one extremity of the domain being eddited.
//! If the domain is a new one (the `StrandBuilder` was created with `StrandBuilder::init_empty`) then the
//! the the moving end can go in both direction and the fixed end is the nucleotide on whihch the
//! domain was initiated.
//! If the domain is an existing one (the `StrandBuilder` was created with
//! `StrandBuilder::init_existing`), then the moving end in the nucleotide that was selected at the
//! moment of the builder's creation and the fixed end is the other end of the domain. In that case
//! the moving end can never go "on the other side" of the fixed end.
//!
//! The `StrandBuilder` can also modify a second domain, the "neighbour", a neighbour can be a
//! domain that needs to be shortenend to elongate the main domain. Or it can be an existing
//! neighbour of the moving_end at the moment of the builder creation.
//!
//! If the neighbour was already next to the domain at the creation of the builder, it follows the
//! moving end, meaning that the neighbour domain can become larger or smaller. If the neighbour
//! was not next to the domain at the creation of the builder, it can only become smaller than it
//! initially was.
use super::{Axis, Data, Nucl};

