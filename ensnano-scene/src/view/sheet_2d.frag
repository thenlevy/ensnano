// shader.frag
#version 450

layout(location=0) in vec2 v_tex_pos;
layout(location=1) in float graduation_unit;

layout(location=0) out vec4 out_color;

void main() {
    vec4 color;

    // #005fa4
    float vignette = clamp(0.7 * length(v_tex_pos), 0.0, 1.0);
    out_color = mix(
        vec4(0.9, 0.9, 0.9, 1.0),
        vec4(0.64, 0.64, 0.64, 1.0),
        vignette
    );

    float grid_scale = graduation_unit;

    float grid_width = grid_scale * 0.1;

    float small_bar_coeff = min(
            abs(mod(v_tex_pos.x + grid_width / 2., grid_scale)),
            abs(mod(v_tex_pos.y + grid_width / 2., grid_scale))
            );
    float darken = 1.2 - 0.2 * smoothstep(grid_width * 0.9, grid_width, small_bar_coeff);
    out_color /= darken;

    float big_bar_coeff = min(
            abs(mod(v_tex_pos.x + grid_width / 2., 5.0 * grid_scale)),
            abs(mod(v_tex_pos.y + grid_width / 2., 5.0 * grid_scale))
            );
    darken = 1.2 - 0.2 * smoothstep(grid_width * 0.9, grid_width, big_bar_coeff);
    out_color /= darken;
    out_color.w = 0.8;
}