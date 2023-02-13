// shader.frag
#version 450

layout(location=0) in vec2 v_tex_pos;
layout(location=1) in float graduation_unit;
layout(location=2) in float revolution_axis_position;

layout(location=0) out vec4 out_color;

float arith_mod(float x, float y) {
    if (x >= 0) {
        return mod(x, y);
    } else {
        return y + mod(x, y);
    }
}

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
            arith_mod(v_tex_pos.x + grid_width / 2., grid_scale),
            arith_mod(v_tex_pos.y + grid_width / 2., grid_scale)
            );
    float darken = 1.2 - 0.2 * smoothstep(grid_width * 0.9, grid_width, small_bar_coeff);
    out_color /= darken;

    float big_bar_coeff = min(
            arith_mod(v_tex_pos.x + grid_width / 2., 5.0 * grid_scale),
            arith_mod(v_tex_pos.y + grid_width / 2., 5.0 * grid_scale)
            );
    darken = 1.2 - 0.2 * smoothstep(grid_width * 0.9, grid_width, big_bar_coeff);
    out_color /= darken;
    out_color.w = 0.8;

    float x = v_tex_pos.x;
    float y = v_tex_pos.y;

    bool is_in_dotted = (mod(abs(y) / 6., 1.) < 0.7);
    if (abs((x - revolution_axis_position)) < grid_width / 6. && is_in_dotted) {
      out_color = vec4(0., 0., 1., 0.9);

    }

    bool is_in_center = (min(abs(x), abs(y)) < 2. && max(abs(x), abs(y)) < 5.);
    if (is_in_center) {
        out_color += vec4(0., 1., 0., 0.);
    }
}

