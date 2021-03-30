// shader.frag
#version 450

layout(location=0) flat in uint v_grid_type;
layout(location=1) in vec2 v_tex_honey_coords;
layout(location=2) in vec2 v_tex_square_coords;
layout(location=3) in vec3 v_color;
layout(location=4) flat in uint v_fake;
layout(location=5) flat in uint v_design_id;

layout(location=0) out vec4 f_color;

layout(set = 3, binding = 0) uniform texture2D t_square;
layout(set = 3, binding = 1) uniform sampler s_square;
layout(set = 3, binding = 2) uniform texture2D t_honney;
layout(set = 3, binding = 3) uniform sampler s_honney;


void main() {
    vec4 color;

    if (v_grid_type == 0) {
       color = texture(sampler2D(t_square, s_square), v_tex_square_coords);
    } else if (v_grid_type == 1) {
       color = texture(sampler2D(t_honney, s_honney), v_tex_honey_coords);
    } else {
       color = vec4(1., 1., 1., 0.4);
    }

    if (color.w < 0.01) {
    discard;
    }

    f_color = color * vec4(v_color, 1.);
    if (v_fake > 0) {
      f_color = vec4(v_color, float(v_design_id) / 255.);
    }
}
