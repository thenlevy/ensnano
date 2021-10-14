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

use std::fmt;
use std::fmt::Write;

impl Strand {
    pub fn formated_domains(&self) -> String {
        let mut ret = String::new();
        for d in self.domains.iter() {
            writeln!(&mut ret, "{}", d).unwrap_or_default();
        }
        ret
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Insertion(n) => write!(f, "[@{}]", n),
            Self::HelixDomain(dom) => write!(f, "{}", dom),
        }
    }
}

impl fmt::Debug for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for HelixInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.forward {
            write!(f, "[H{}: {} -> {}]", self.helix, self.start, self.end - 1)
        } else {
            write!(f, "[H{}: {} <- {}]", self.helix, self.start, self.end - 1)
        }
    }
}

impl fmt::Debug for HelixInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}
