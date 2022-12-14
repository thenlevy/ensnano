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

use ensnano_design::{BezierPathId, CurveDescriptor2D};
use ensnano_gui::{
    CurveDescriptorBuilder, CurveDescriptorParameter, InstanciatedParameter, ParameterKind,
};
use ultraviolet::{Rotor3, Vec3};

pub(super) const ELLIPSE_BUILDER: CurveDescriptorBuilder<super::AppState> =
    CurveDescriptorBuilder {
        nb_parameters: 2,
        curve_name: "Ellipse",
        parameters: &[
            CurveDescriptorParameter {
                name: "Semi major axis",
                kind: ParameterKind::Float,
                default_value: ensnano_gui::InstanciatedParameter::Float(20.0),
            },
            CurveDescriptorParameter {
                name: "Semi minor axis",
                kind: ParameterKind::Float,
                default_value: ensnano_gui::InstanciatedParameter::Float(10.0),
            },
        ],
        build: &build_ellipse,
        bezier_path_id: &no_bezier_path_id,
        frame: &default_frame,
    };

fn build_ellipse(
    parameters: &[InstanciatedParameter],
    _: &super::AppState,
) -> Option<CurveDescriptor2D> {
    let a = parameters
        .get(0)
        .cloned()
        .and_then(InstanciatedParameter::get_float)?;
    let b = parameters
        .get(1)
        .cloned()
        .and_then(InstanciatedParameter::get_float)?;
    Some(CurveDescriptor2D::Ellipse {
        semi_minor_axis: b.into(),
        semi_major_axis: a.into(),
    })
}

pub(super) const TWO_SPHERES_BUILDER: CurveDescriptorBuilder<super::AppState> =
    CurveDescriptorBuilder {
        nb_parameters: 2,
        curve_name: "Two spheres",
        parameters: &[
            CurveDescriptorParameter {
                name: "Radius extern",
                kind: ParameterKind::Float,
                default_value: ensnano_gui::InstanciatedParameter::Float(25.),
            },
            CurveDescriptorParameter {
                name: "Radius intern",
                kind: ParameterKind::Float,
                default_value: ensnano_gui::InstanciatedParameter::Float(17.),
            },
            CurveDescriptorParameter {
                name: "Radius tube",
                kind: ParameterKind::Float,
                default_value: ensnano_gui::InstanciatedParameter::Float(7.6),
            },
            CurveDescriptorParameter {
                name: "Smooth ceil",
                kind: ParameterKind::Float,
                default_value: ensnano_gui::InstanciatedParameter::Float(0.04),
            },
        ],
        build: &build_two_spheres,
        bezier_path_id: &no_bezier_path_id,
        frame: &default_frame,
    };

fn build_two_spheres(
    parameters: &[InstanciatedParameter],
    _: &super::AppState,
) -> Option<CurveDescriptor2D> {
    let radius_extern = parameters
        .get(0)
        .cloned()
        .and_then(InstanciatedParameter::get_float)?
        .into();
    let radius_intern = parameters
        .get(1)
        .cloned()
        .and_then(InstanciatedParameter::get_float)?
        .into();
    let radius_tube = parameters
        .get(2)
        .cloned()
        .and_then(InstanciatedParameter::get_float)?
        .into();
    let smooth_ceil = parameters
        .get(3)
        .cloned()
        .and_then(InstanciatedParameter::get_float)?
        .into();

    Some(CurveDescriptor2D::TwoBalls {
        radius_extern,
        radius_intern,
        radius_tube,
        smooth_ceil,
    })
}

pub(super) const BEZIER_CURVE_BUILDER: CurveDescriptorBuilder<super::AppState> =
    CurveDescriptorBuilder {
        nb_parameters: 1,
        curve_name: "Bezier",
        parameters: &[CurveDescriptorParameter {
            name: "Path n°",
            kind: ParameterKind::Uint,
            default_value: ensnano_gui::InstanciatedParameter::Uint(0),
        }],
        build: &build_bezier,
        bezier_path_id: &get_bezier_path_id,
        frame: &get_bezier_frame,
    };

fn build_bezier(
    parameters: &[InstanciatedParameter],
    app: &super::AppState,
) -> Option<CurveDescriptor2D> {
    let curve_id = parameters
        .get(0)
        .cloned()
        .and_then(InstanciatedParameter::get_uint)?;

    app.0
        .design
        .get_design_reader()
        .get_bezier_path_2d(BezierPathId(curve_id as u32))
        .map(CurveDescriptor2D::Bezier)
}

fn no_bezier_path_id(_: &[InstanciatedParameter]) -> Option<usize> {
    None
}

fn get_bezier_path_id(parameters: &[InstanciatedParameter]) -> Option<usize> {
    parameters
        .get(0)
        .cloned()
        .and_then(InstanciatedParameter::get_uint)
}

fn get_bezier_frame(
    parameters: &[InstanciatedParameter],
    app: &super::AppState,
) -> Option<(Vec3, Rotor3)> {
    let path_id = get_bezier_path_id(parameters)?;
    app.0
        .design
        .get_design_reader()
        .get_first_bezier_plane(BezierPathId(path_id as u32))
        .map(|plane| (plane.position, plane.orientation))
}

fn default_frame(_: &[InstanciatedParameter], app: &super::AppState) -> Option<(Vec3, Rotor3)> {
    app.0
        .design
        .get_design_reader()
        .get_default_bezier()
        .map(|plane| (plane.position, plane.orientation))
}
