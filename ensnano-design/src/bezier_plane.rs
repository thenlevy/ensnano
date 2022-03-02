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
use super::collection::HasMap;
use std::collections::BTreeMap;
use std::sync::Arc;
use ultraviolet::{Rotor3, Vec2, Vec3};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BezierPlaneDescriptor {
    pub position: Vec3,
    pub orientation: Rotor3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BezierPlaneId(usize);

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BezierPlanes(Arc<BTreeMap<BezierPlaneId, Arc<BezierPlaneDescriptor>>>);

impl HasMap for BezierPlanes {
    type Key = BezierPlaneId;
    type Item = BezierPlaneDescriptor;
    fn get_map(&self) -> &BTreeMap<Self::Key, Arc<Self::Item>> {
        &self.0
    }
}

impl BezierPlanes {
    pub fn make_mut<'a>(&'a mut self) -> BezierPlanesMut<'a> {
        let new_map = BTreeMap::clone(&self.0);
        BezierPlanesMut {
            source: self,
            new_map,
        }
    }
}

impl BezierPlaneDescriptor {
    pub fn ray_intersection(
        &self,
        origin: Vec3,
        direction: Vec3,
    ) -> Option<BezierPlaneIntersection> {
        let normal = Vec3::unit_x().rotated_by(self.orientation);
        let denom = direction.dot(normal);
        let depth = if denom.abs() < 1e-3 {
            None
        } else {
            let d = (self.position - origin).dot(normal) / denom;
            Some(d)
        }?;
        let (x, y) = {
            let intersection = origin + depth * direction;
            let vec = intersection - self.position;
            let x_dir = Vec3::unit_z().rotated_by(self.orientation);
            let y_dir = Vec3::unit_y().rotated_by(self.orientation);
            (vec.dot(x_dir), vec.dot(y_dir))
        };
        Some(BezierPlaneIntersection { x, y, depth })
    }
}

pub fn ray_bezier_plane_intersection<'a>(
    planes: impl Iterator<Item = (&'a BezierPlaneId, &'a BezierPlaneDescriptor)>,
    origin: Vec3,
    direction: Vec3,
) -> Option<(BezierPlaneId, BezierPlaneIntersection)> {
    let mut ret: Option<(BezierPlaneId, BezierPlaneIntersection)> = None;
    for (id, plane) in planes {
        if let Some(intersection) = plane.ray_intersection(origin, direction) {
            if let Some((best_id, inter)) = ret.as_mut() {
                if inter.depth > intersection.depth {
                    *best_id = *id;
                    *inter = intersection;
                }
            } else {
                ret = Some((*id, intersection));
            }
        }
    }
    ret
}

pub struct BezierPlaneIntersection {
    pub x: f32,
    pub y: f32,
    pub depth: f32,
}

pub struct BezierPlanesMut<'a> {
    source: &'a mut BezierPlanes,
    new_map: BTreeMap<BezierPlaneId, Arc<BezierPlaneDescriptor>>,
}

impl<'a> BezierPlanesMut<'a> {
    pub fn push(&mut self, desc: BezierPlaneDescriptor) {
        let new_key = self
            .new_map
            .keys()
            .max()
            .map(|m| BezierPlaneId(m.0 + 1))
            .unwrap_or_default();
        self.new_map.insert(new_key, Arc::new(desc));
    }
}

impl<'a> Drop for BezierPlanesMut<'a> {
    fn drop(&mut self) {
        *self.source = BezierPlanes(Arc::new(std::mem::take(&mut self.new_map)))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BezierPathId(usize);

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BezierPaths(Arc<BTreeMap<BezierPathId, Arc<BezierPath>>>);

impl HasMap for BezierPaths {
    type Key = BezierPathId;
    type Item = BezierPath;
    fn get_map(&self) -> &BTreeMap<Self::Key, Arc<Self::Item>> {
        self.0.as_ref()
    }
}

pub struct BezierPathsMut<'a> {
    source: &'a mut BezierPaths,
    new_map: BTreeMap<BezierPathId, Arc<BezierPath>>,
}

impl BezierPaths {
    pub fn make_mut<'a>(&'a mut self) -> BezierPathsMut<'a> {
        BezierPathsMut {
            new_map: BTreeMap::clone(&self.0),
            source: self,
        }
    }
}

impl<'a> BezierPathsMut<'a> {
    pub fn create_path(&mut self, first_vertex: BezierVertex) -> BezierPathId {
        let new_key = self
            .new_map
            .keys()
            .max()
            .map(|m| BezierPathId(m.0 + 1))
            .unwrap_or_default();
        let new_path = BezierPath {
            vertices: vec![first_vertex],
            cyclic: false,
        };
        self.new_map.insert(new_key, Arc::new(new_path));
        new_key
    }

    pub fn get_mut(&mut self, id: &BezierPathId) -> Option<&mut BezierPath> {
        self.new_map.get_mut(id).map(Arc::make_mut)
    }
}

impl<'a> Drop for BezierPathsMut<'a> {
    fn drop(&mut self) {
        *self.source = BezierPaths(Arc::new(std::mem::take(&mut self.new_map)))
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BezierPath {
    vertices: Vec<BezierVertex>,
    cyclic: bool,
}

impl BezierPath {
    pub fn add_vertex(&mut self, vertex: BezierVertex) -> usize {
        self.vertices.push(vertex);
        self.vertices.len() - 1
    }

    pub fn get_vertex_mut(&mut self, vertex_id: usize) -> Option<&mut BezierVertex> {
        self.vertices.get_mut(vertex_id)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct BezierVertex {
    pub plane_id: BezierPlaneId,
    pub position: Vec2,
}
