/*
This file contains fragment of code that were originally published in the `lyon` crate
Original source: https://github.com/nical/lyon/blob/master/examples/wgpu/shaders/background.frag.glsl
The original source was distributed under the MIT License by Nicolas Silva.
A copy of the original license is available in thirdparties/lyon/LICENSE
*/
#version 450

layout(location = 0) in vec2 v_position;
layout(location = 1) flat in vec2 v_resolution;
layout(location = 2) flat in vec2 v_scroll_offset;
layout(location = 3) flat in float v_zoom;

layout(location = 0) out vec4 out_color;


void main() {
    vec2 invert_y = vec2(1.0, -1.0);
    vec2 px_position = v_position * v_resolution * 0.5 * invert_y;

    if (v_position.y < -0.97 || v_position.y > 0.97 ) {
       out_color = vec4(0., 0., 0., 1.);
    } else {
        discard;
    }
}
