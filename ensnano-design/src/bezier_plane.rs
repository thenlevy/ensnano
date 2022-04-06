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
use super::curves::{Curve, InstanciatedBeizerEnd, InstanciatedPiecewiseBeizer};
use super::Collection;
use super::Parameters;
use crate::grid::*;
use crate::utils::rotor_to_drotor;
use std::collections::BTreeMap;
use std::sync::Arc;
use ultraviolet::{DMat3, DVec3, Mat3, Rotor3, Vec2, Vec3};

const TENGENT: f32 = 1. / 3.;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BezierPlaneDescriptor {
    pub position: Vec3,
    pub orientation: Rotor3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BezierPlaneId(pub u32);

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
    pub fn make_mut(&mut self) -> BezierPlanesMut {
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

    fn position(&self, vec: Vec2) -> Vec3 {
        self.position
            + Vec3::unit_z().rotated_by(self.orientation) * vec.x
            + Vec3::unit_y().rotated_by(self.orientation) * vec.y
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

impl BezierPlaneIntersection {
    pub fn position(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
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

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default, Hash,
)]
pub struct BezierPathId(pub u32);

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
    pub fn make_mut(&mut self) -> BezierPathsMut {
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
            grid_type: None,
        };
        self.new_map.insert(new_key, Arc::new(new_path));
        new_key
    }

    pub fn get_mut(&mut self, id: &BezierPathId) -> Option<&mut BezierPath> {
        self.new_map.get_mut(id).map(Arc::make_mut)
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut BezierPath> {
        self.new_map.values_mut().map(Arc::make_mut)
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_type: Option<GridTypeDescr>,
}

impl BezierPath {
    pub fn add_vertex(&mut self, vertex: BezierVertex) -> usize {
        self.vertices.push(vertex);
        self.vertices.len() - 1
    }

    pub fn get_vertex_mut(&mut self, vertex_id: usize) -> Option<&mut BezierVertex> {
        self.vertices.get_mut(vertex_id)
    }

    pub fn vertices(&self) -> &[BezierVertex] {
        &self.vertices
    }

    pub fn vertices_mut(&mut self) -> &mut [BezierVertex] {
        self.vertices.as_mut_slice()
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct BezierVertex {
    pub plane_id: BezierPlaneId,
    pub position: Vec2,
    pub position_in: Option<Vec2>,
    pub position_out: Option<Vec2>,
    #[serde(default)]
    grid_translation: Vec3,
}

impl BezierVertex {
    pub fn space_position(&self, planes: &BezierPlanes) -> Option<Vec3> {
        if let Some(plane) = planes.0.get(&self.plane_id) {
            Some(
                plane.position
                    + self.position.x * Vec3::unit_z().rotated_by(plane.orientation)
                    + self.position.y * Vec3::unit_y().rotated_by(plane.orientation),
            )
        } else {
            log::error!("Could not get plane");
            None
        }
    }

    pub fn grid_position(&self, planes: &BezierPlanes) -> Option<Vec3> {
        self.space_position(planes)
            .map(|p| p + self.grid_translation)
    }

    pub fn add_translation(&mut self, translation: Vec3) {
        self.grid_translation += translation
    }

    pub fn new(plane_id: BezierPlaneId, position: Vec2) -> Self {
        Self {
            plane_id,
            position,
            position_out: None,
            position_in: None,
            grid_translation: Vec3::zero(),
        }
    }
}

pub struct InstanciatedPath {
    source_planes: BezierPlanes,
    source_path: Arc<BezierPath>,
    pub(crate) curve_descriptor: Option<Arc<InstanciatedPiecewiseBeizer>>,
    curve_descriptor_2d: Option<Arc<InstanciatedPiecewiseBeizer>>,
    curve_2d: Option<Curve>,
    pub(crate) frames: Option<Vec<(Vec3, Rotor3)>>,
}

fn path_to_curve_descriptor(
    source_planes: BezierPlanes,
    source_path: Arc<BezierPath>,
    path_3d: bool,
) -> Option<InstanciatedPiecewiseBeizer> {
    let position = |vertex: &BezierVertex| {
        if path_3d {
            vertex.grid_position(&source_planes)
        } else {
            vertex.space_position(&source_planes)
        }
    };
    let descriptor = if source_path.vertices.len() > 2 {
        let iterator: Box<dyn Iterator<Item = ((&BezierVertex, &BezierVertex), &BezierVertex)>> =
            if source_path.cyclic {
                let n = source_path.vertices().len();
                Box::new(
                    source_path
                        .vertices()
                        .iter()
                        .cycle()
                        .skip(n - 1)
                        .zip(source_path.vertices.iter().cycle().take(n + 1))
                        .zip(source_path.vertices().iter().cycle().skip(1)),
                )
            } else {
                Box::new(
                    source_path
                        .vertices()
                        .iter()
                        .zip(source_path.vertices.iter().skip(1))
                        .zip(source_path.vertices().iter().skip(2)),
                )
            };
        let mut bezier_points: Vec<_> = iterator
            .filter_map(|((v_from, v), v_to)| {
                let pos_from = position(v_from)?;
                let pos = position(v)?;
                let pos_to = position(v_to)?;
                let vector_in = if let Some(position_in) = v.position_in {
                    let plane = source_planes.get(&v.plane_id)?;
                    pos - plane.position(position_in)
                } else {
                    (pos_to - pos_from) * TENGENT
                };
                let vector_out = if let Some(position_out) = v.position_out {
                    let plane = source_planes.get(&v.plane_id)?;
                    plane.position(position_out) - pos
                } else {
                    (pos_to - pos_from) * TENGENT
                };

                Some(InstanciatedBeizerEnd {
                    position: pos,
                    vector_in,
                    vector_out,
                })
            })
            .collect();
        if !source_path.cyclic {
            let first_point = {
                let second_point = bezier_points.get(0)?;
                let pos = position(&source_path.vertices[0])?;
                let control = second_point.position - second_point.vector_in;
                InstanciatedBeizerEnd {
                    position: pos,
                    vector_out: (control - pos) / 2.,
                    vector_in: (control - pos) / 2.,
                }
            };
            bezier_points.insert(0, first_point);
            let last_point = {
                let second_to_last_point = bezier_points.last()?;
                // Ok to unwrap because vertices has length > 2
                let pos = position(source_path.vertices.last().unwrap())?;
                let control = second_to_last_point.position + second_to_last_point.vector_out;
                InstanciatedBeizerEnd {
                    position: pos,
                    vector_out: (pos - control) / 2.,
                    vector_in: (pos - control) / 2.,
                }
            };
            bezier_points.push(last_point);
        }
        Some(bezier_points)
    } else if source_path.vertices.len() == 2 {
        let pos_first = position(&source_path.vertices[0])?;
        let pos_last = position(&source_path.vertices[1])?;
        let vec = (pos_last - pos_first) / 3.;
        Some(vec![
            InstanciatedBeizerEnd {
                position: pos_first,
                vector_in: vec,
                vector_out: vec,
            },
            InstanciatedBeizerEnd {
                position: pos_last,
                vector_in: vec,
                vector_out: vec,
            },
        ])
    } else {
        None
    }?;
    Some(InstanciatedPiecewiseBeizer {
        t_min: None,
        t_max: Some(descriptor.len() as f64 - 1.),
        ends: descriptor,
    })
}

fn curve_descriptor_to_frame(
    source_planes: BezierPlanes,
    source_path: Arc<BezierPath>,
    desc: &InstanciatedPiecewiseBeizer,
) -> Option<Vec<(Vec3, Rotor3)>> {
    source_path
        .vertices
        .iter()
        .zip(desc.ends.iter())
        .map(|(v_desc, v_instance)| {
            let up = source_planes
                .0
                .get(&v_desc.plane_id)
                .map(|p| Vec3::unit_x().rotated_by(p.orientation).normalized())?;
            let right = -v_instance.vector_out.normalized();
            let front = right.cross(up).normalized();
            let up = front.cross(right).normalized();
            let orientation = Mat3::new(right, up, front).into_rotor3();

            Some((v_instance.position, orientation))
        })
        .collect()
}

impl InstanciatedPath {
    fn new(
        source_planes: BezierPlanes,
        source_path: Arc<BezierPath>,
        parameters: &Parameters,
    ) -> Self {
        let descriptor_2d =
            path_to_curve_descriptor(source_planes.clone(), source_path.clone(), false);
        let descriptor_3d =
            path_to_curve_descriptor(source_planes.clone(), source_path.clone(), true);
        let frames = descriptor_2d.as_ref().and_then(|desc| {
            curve_descriptor_to_frame(source_planes.clone(), source_path.clone(), desc)
        });
        let curve_2d = descriptor_2d
            .clone()
            .map(|desc| Curve::new(desc, parameters));
        Self {
            source_planes,
            source_path,
            curve_2d,
            curve_descriptor_2d: descriptor_2d.map(Arc::new),
            curve_descriptor: descriptor_3d.map(Arc::new),
            frames,
        }
    }

    fn updated(
        &self,
        source_planes: BezierPlanes,
        source_path: Arc<BezierPath>,
        parameters: &Parameters,
    ) -> Option<Self> {
        if self.need_update(&source_planes, &source_path) {
            Some(Self::new(source_planes, source_path, parameters))
        } else {
            None
        }
    }

    fn need_update(&self, source_planes: &BezierPlanes, source_path: &Arc<BezierPath>) -> bool {
        !Arc::ptr_eq(&source_planes.0, &self.source_planes.0)
            || !Arc::ptr_eq(&self.source_path, source_path)
    }

    pub fn bezier_controls(&self) -> &[InstanciatedBeizerEnd] {
        self.curve_descriptor_2d
            .as_ref()
            .map(|c| c.ends.as_slice())
            .unwrap_or(&[])
    }

    pub fn get_curve_points(&self) -> &[DVec3] {
        self.curve_2d
            .as_ref()
            .map(|c| c.positions_forward.as_slice())
            .unwrap_or(&[])
    }

    pub fn initial_frame(&self) -> Option<DMat3> {
        self.frames
            .as_ref()
            .and_then(|fs| fs.get(0))
            .as_ref()
            .map(|f| rotor_to_drotor(f.1).into_matrix())
            .map(|m| DMat3::new(m.cols[2], m.cols[1], m.cols[0]))
    }
}

#[derive(Clone)]
pub struct BezierPathData {
    source_planes: BezierPlanes,
    pub(crate) source_paths: BezierPaths,
    pub instanciated_paths: Arc<BTreeMap<BezierPathId, Arc<InstanciatedPath>>>,
}

impl std::fmt::Debug for BezierPathData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BezierPathData")
            .field(
                "instanciated_paths",
                &format_args!("{:p}", &self.instanciated_paths),
            )
            .finish()
    }
}

impl BezierPathData {
    pub fn new(
        source_planes: BezierPlanes,
        source_paths: BezierPaths,
        parameters: &Parameters,
    ) -> Self {
        let instanciated_paths: BTreeMap<_, _> = source_paths
            .0
            .iter()
            .map(|(id, path)| {
                (
                    *id,
                    Arc::new(InstanciatedPath::new(
                        source_planes.clone(),
                        path.clone(),
                        parameters,
                    )),
                )
            })
            .collect();
        Self {
            instanciated_paths: Arc::new(instanciated_paths),
            source_planes,
            source_paths,
        }
    }

    pub fn need_update(&self, source_planes: &BezierPlanes, source_paths: &BezierPaths) -> bool {
        !Arc::ptr_eq(&source_planes.0, &self.source_planes.0)
            || !Arc::ptr_eq(&self.source_paths.0, &source_paths.0)
    }

    pub fn updated(
        &self,
        source_planes: BezierPlanes,
        source_paths: BezierPaths,
        parameters: &Parameters,
    ) -> Option<Self> {
        if self.need_update(&source_planes, &source_paths) {
            let instanciated_paths: BTreeMap<_, _> = source_paths
                .0
                .iter()
                .map(|(id, source_path)| {
                    let path = if let Some(path) = self.instanciated_paths.get(id) {
                        path.updated(source_planes.clone(), source_path.clone(), parameters)
                            .map(Arc::new)
                            .unwrap_or_else(|| path.clone())
                    } else {
                        Arc::new(InstanciatedPath::new(
                            source_planes.clone(),
                            source_path.clone(),
                            parameters,
                        ))
                    };
                    (*id, path)
                })
                .collect();
            Some(Self {
                instanciated_paths: Arc::new(instanciated_paths),
                source_planes,
                source_paths,
            })
        } else {
            None
        }
    }

    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        Arc::ptr_eq(&a.instanciated_paths, &b.instanciated_paths)
    }

    pub fn position_vertex_2d(&self, vertex_id: BezierVertexId) -> Option<Vec3> {
        let path = self.instanciated_paths.get(&vertex_id.path_id)?;
        path.frames
            .as_ref()
            .and_then(|f| f.get(vertex_id.vertex_id))
            .map(|f| f.0)
    }

    pub fn orientation_vertex(&self, vertex_id: BezierVertexId) -> Option<Rotor3> {
        let path = self.instanciated_paths.get(&vertex_id.path_id)?;
        path.frames
            .as_ref()
            .and_then(|f| f.get(vertex_id.vertex_id))
            .map(|f| f.1)
    }

    pub fn grids(&self) -> Vec<(GridId, GridDescriptor)> {
        self.instanciated_paths
            .iter()
            .flat_map(|(path_id, path)| {
                if let Some(grid_type) = path.source_path.grid_type {
                    path.source_path
                        .vertices
                        .iter()
                        .enumerate()
                        .filter_map(|(vertex_id, v)| {
                            let vertex_id = BezierVertexId {
                                path_id: *path_id,
                                vertex_id,
                            };
                            let desc = GridDescriptor {
                                invisible: false,
                                grid_type,
                                orientation: self.orientation_vertex(vertex_id)?,
                                position: self.position_vertex_2d(vertex_id)? + v.grid_translation,
                                bezier_vertex: Some(vertex_id),
                            };
                            Some((GridId::BezierPathGrid(vertex_id), desc))
                        })
                        .collect()
                } else {
                    vec![]
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
pub struct BezierVertexId {
    pub path_id: BezierPathId,
    pub vertex_id: usize,
}
