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
use iced_native::widget::text_input;
use iced_native::widget::text_input::TextInput;

#[derive(Debug, Clone, Copy)]
pub enum ParameterKind {
    Float,
}

pub enum InstanciatedParameters {
    Float(f64),
}

#[derive(Debug, Clone)]
pub struct CurveDescriptorParameter {
    name: &'static str,
    kind: ParameterKind,
}

/*
pub trait CurveDescriptorBuilder {
    const NB_PARAMETERS: usize;
    const CURVE_NAME: &'static str;
    const PARAMETERS: &'static[CurveDescriptorParameter];
    fn build(parameters: &[InstanciatedParameters]) -> CurveDescriptor2D;
}
*/

pub struct CurveDescriptorBuilder {
    nb_parameters: usize,
    curve_name: &'static str,
    parameters: &'static [CurveDescriptorParameter],
    build: &'static dyn Fn(&[InstanciatedParameters]) -> CurveDescriptor2D,
}

enum ParameterWidget {
    Float {
        current_text: String,
        state: text_input::State,
    },
}
