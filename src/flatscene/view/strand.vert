#version 450

layout(set = 0, binding = 0)
uniform Globals {
    vec2 u_resolution;
    vec2 u_scroll_offset;
    float u_zoom;
};

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_normal;
layout(location = 2) in vec4 a_color;


layout(location = 0) out vec4 v_color;

void main() {
    vec2 invert_y = vec2(1.0, -1.0);

    vec2 local_pos = a_position + a_normal * 0.075;
    vec2 world_pos = local_pos - u_scroll_offset;
    vec2 transformed_pos = world_pos * u_zoom / (vec2(0.5, 0.5) * u_resolution) * invert_y;

    float z = 0. / 4096.0;
    gl_Position = vec4(transformed_pos, z / 1000.0, 1.0);
    v_color = a_color;
}
