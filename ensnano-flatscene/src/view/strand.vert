#version 450

layout(set = 0, binding = 0)
uniform Globals {
    vec2 u_resolution;
    vec2 u_scroll_offset;
    float u_zoom;
    float u_tilt;
    vec2 u_symetry;
};

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_normal;
layout(location = 2) in vec4 a_color;
layout(location = 3) in float a_depth;
layout(location = 4) in float a_width;


layout(location = 0) out vec4 v_color;

mat2 rotation(float angle) {
   float c = cos(angle);
   float s = sin(angle);
   return mat2(c, s, -s, c);
}

void main() {
    vec2 invert_y = vec2(1.0, -1.0);

    float nb_pixel = 3.;
    float cst = max(0.1125, nb_pixel / ( a_width * 4. * u_zoom));

    vec2 local_pos = a_position + a_normal * cst * a_width;
    vec2 world_pos = local_pos - u_scroll_offset;
    vec2 transformed_pos = (rotation(u_tilt) * world_pos) * u_zoom / (vec2(0.5, 0.5) * u_resolution) * invert_y * u_symetry;

// formula for the grid:
//float z = (float(model.z_index * 1000 + a_model_id) + background_depth);
// We want the strand to have the depth of the grid - 0.25.

    float z = a_depth * 1000.;
    gl_Position = vec4(transformed_pos, z / 1e6, 1.0);
    v_color = a_color;
}
