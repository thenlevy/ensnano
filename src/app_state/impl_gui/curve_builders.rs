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

use ensnano_design::CurveDescriptor2D;
use ensnano_gui::{
    CurveDescriptorBuilder, CurveDescriptorParameter, InstanciatedParameter, ParameterKind,
};

pub(super) const ELLIPSE_BUILDER: CurveDescriptorBuilder = CurveDescriptorBuilder {
    nb_parameters: 2,
    curve_name: "Ellipse",
    parameters: &[
        CurveDescriptorParameter {
            name: "Semi major axis",
            kind: ParameterKind::Float,
            default_value: ensnano_gui::InstanciatedParameter::Float(2.0),
        },
        CurveDescriptorParameter {
            name: "Semi minor axis",
            kind: ParameterKind::Float,
            default_value: ensnano_gui::InstanciatedParameter::Float(1.0),
        },
    ],
    build: &build_ellipse,
};

fn build_ellipse(parameters: &[InstanciatedParameter]) -> Option<CurveDescriptor2D> {
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

pub(super) const TWO_SPHERES_BUILDER: CurveDescriptorBuilder = CurveDescriptorBuilder {
    nb_parameters: 2,
    curve_name: "Two spheres",
    parameters: &[
        CurveDescriptorParameter {
            name: "Radius extern",
            kind: ParameterKind::Float,
            default_value: ensnano_gui::InstanciatedParameter::Float(2.5),
        },
        CurveDescriptorParameter {
            name: "Radius intern",
            kind: ParameterKind::Float,
            default_value: ensnano_gui::InstanciatedParameter::Float(1.7),
        },
        CurveDescriptorParameter {
            name: "Radius tube",
            kind: ParameterKind::Float,
            default_value: ensnano_gui::InstanciatedParameter::Float(0.76),
        },
        CurveDescriptorParameter {
            name: "Smooth ceil",
            kind: ParameterKind::Float,
            default_value: ensnano_gui::InstanciatedParameter::Float(0.04),
        },
    ],
    build: &build_two_spheres,
};

fn build_two_spheres(parameters: &[InstanciatedParameter]) -> Option<CurveDescriptor2D> {
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