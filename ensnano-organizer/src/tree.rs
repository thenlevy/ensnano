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

use serde::Deserialize;
#[derive(Clone, Debug, Serialize)]
pub enum OrganizerTree<K> {
    Leaf(K),
    Node {
        name: String,
        childrens: Vec<OrganizerTree<K>>,
        expanded: bool,
        #[serde(default)]
        id: Option<GroupId>,
    },
}

// For compatibility reasons, we need to implement Deserialize ourselved for OrganizerTree.
// We want to be able to accept both the old format (pre 0.3.0) and the current format.

#[derive(Clone, Debug, Serialize, Deserialize)]
enum OldOrganizerTree<K> {
    Leaf(K),
    Node(String, Vec<OrganizerTree<K>>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum NewOrganizerTree<K> {
    Leaf(K),
    Node {
        name: String,
        childrens: Vec<OrganizerTree<K>>,
        expanded: bool,
        #[serde(default)]
        id: Option<GroupId>,
    },
}

impl<K> OldOrganizerTree<K> {
    fn to_new(self) -> OrganizerTree<K> {
        match self {
            Self::Leaf(k) => OrganizerTree::Leaf(k),
            Self::Node(name, childrens) => OrganizerTree::Node {
                name,
                childrens,
                expanded: false,
                id: None,
            },
        }
    }
}

impl<K> NewOrganizerTree<K> {
    fn to_real(self) -> OrganizerTree<K> {
        match self {
            Self::Leaf(k) => OrganizerTree::Leaf(k),
            Self::Node {
                name,
                childrens,
                expanded,
                id,
            } => OrganizerTree::Node {
                name,
                childrens,
                expanded,
                id,
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NewOrOld<K> {
    New(NewOrganizerTree<K>),
    Old(OldOrganizerTree<K>),
}

impl<'de, K: Deserialize<'de>> Deserialize<'de> for OrganizerTree<K> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match NewOrOld::deserialize(deserializer) {
            Ok(NewOrOld::New(new_tree)) => Ok(new_tree.to_real()),
            Ok(NewOrOld::Old(old_tree)) => Ok(old_tree.to_new()),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// The identifier of a group.
///
/// Used to map groups to group attributes.
pub struct GroupId(u64);

use rand::distributions::{Distribution, Standard};
use rand::Rng;

impl Distribution<GroupId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GroupId {
        let id: u64 = rng.gen();
        GroupId(id)
    }
}
