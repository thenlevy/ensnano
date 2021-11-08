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

//! A wrapper arround an Arc<T> that uses `Arc::ptr_eq` to test for equality.

use std::sync::Arc;

/// A wrapper arround an Arc<T> that uses `Arc::ptr_eq` to test for equality.
#[derive(Default)]
pub(super) struct AddressPointer<T: Default>(Arc<T>);

impl<T: Default> Clone for AddressPointer<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Default> PartialEq for AddressPointer<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: Default> Eq for AddressPointer<T> {}

impl<T: Default> AsRef<T> for AddressPointer<T> {
    fn as_ref(&self) -> &T {
        self.0.as_ref()
    }
}

impl<T, V: AsRef<[T]> + Default> AsRef<[T]> for AddressPointer<V> {
    fn as_ref(&self) -> &[T] {
        self.0.as_ref().as_ref()
    }
}

impl<T: Default> std::ops::Deref for AddressPointer<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T: Default> AddressPointer<T> {
    pub fn new(content: T) -> Self {
        Self(Arc::new(content))
    }

    pub fn show_address(&self) {
        println!("{:p}", Arc::as_ptr(&self.0))
    }

    pub fn get_ptr(&self) -> *const T {
        Arc::as_ptr(&self.0)
    }
}

use std::ops::Deref;
impl<T: Clone + Default> AddressPointer<T> {
    /// Return a clone of the pointed value.
    pub fn clone_inner(&self) -> T {
        self.0.deref().clone()
    }
}

impl<T: Default + PartialEq> AddressPointer<T> {
    /// Test the content of two pointers for equality
    pub fn content_equal(&self, content: &T) -> bool {
        self.0.as_ref() == content
    }
}

impl<T: Default> From<Arc<T>> for AddressPointer<T> {
    fn from(arc: Arc<T>) -> Self {
        Self(arc)
    }
}
