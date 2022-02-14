/*
This file contains fragment of code that were originally published in the `lyon` crate
Original source: https://github.com/nical/lyon/blob/master/examples/wgpu/shaders/background.vert.glsl
The original source was distributed under the MIT License by Nicolas Silva.
A copy of the original license is available in thirdparties/lyon/LICENSE
*/
#version 450

layout(set = 0, binding = 0) uniform Globals {
    vec2 u_resolution;
    vec2 u_scroll_offset;
    float u_zoom;
    float u_tilt;
    vec2 u_symetry;
};

layout(location = 0) in vec2 a_position;
layout(location = 0) out vec2 v_position;

layout(location = 1) flat out vec2 v_resolution;
layout(location = 2) flat out vec2 v_scroll_offset;
layout(location = 3) flat out float v_zoom;
layout(location = 4) flat out float v_tilt;

mat2 rotation(float angle) {
   float c = cos(angle);
   float s = sin(angle);
   return mat2(c, s, -s, c);
}

void main() {
    gl_Position = vec4(a_position,1. - 0.0000001, 1.0);
    v_position = a_position * u_symetry;
    v_resolution = u_resolution;
    v_scroll_offset = u_scroll_offset;
    v_zoom = u_zoom;
    v_tilt = u_tilt;
}
