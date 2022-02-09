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
layout(location = 4) flat in float v_tilt;

layout(location = 0) out vec4 out_color;

mat2 rotation(float angle) {
   float c = cos(angle);
   float s = sin(angle);
   return mat2(c, s, -s, c);
}

void main() {
    vec2 invert_y = vec2(1.0, -1.0);
    vec2 center_to_point = v_position * v_resolution * 0.5 * invert_y;

    // #005fa4
    float vignette = clamp(0.7 * length(v_position), 0.0, 1.0);
    out_color = mix(
        vec4(0.9, 0.9, 0.9, 1.0),
        vec4(0.64, 0.64, 0.64, 1.0),
        vignette
    );

    // TODO: properly adapt the grid while zooming in and out.
    float grid_scale = 1.;
    if (v_zoom < 2.5) {
        grid_scale = 5.;
    }

    float grid_width = grid_scale * 0.1;

    if (v_zoom > 7.) {

        vec2 pos = (rotation(-v_tilt) * center_to_point / v_zoom) + v_scroll_offset;

        float small_bar_coeff = min(
            abs(mod(pos.x + grid_width / 2., grid_scale)),
            abs(mod(pos.y + grid_width / 2., grid_scale))
        );
        float darken = 1.2 - 0.2 * smoothstep(grid_width * 0.9, grid_width, small_bar_coeff);
        out_color /= darken;

        float big_bar_coeff = min(
            abs(mod(pos.x + grid_width / 2., 5.0 * grid_scale)),
            abs(mod(pos.y + grid_width / 2., 5.0 * grid_scale))
        );
        darken = 1.2 - 0.2 * smoothstep(grid_width * 0.9, grid_width, big_bar_coeff);
        out_color /= darken;
    }
}
