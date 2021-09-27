#version 450

layout(location=0) out vec2 v_coords;
layout(location=1) out float v_angle;
layout(location=2) out vec4 v_color;

layout(set = 0, binding = 0)
uniform Globals {
    vec2 u_resolution;
    vec2 u_scroll_offset;
    float u_zoom;
};

struct Instances {
    vec2 center;
    float radius;
    float angle;
    int z_index;
    uint color;
};

layout(std430, set=1, binding=0) 
buffer InstancesBlock {
    readonly Instances instances[];
};

void main() {
    float min_x = -1.;
    float max_x = 1.;
    float min_y = -1;
    float max_y = 1.;

    vec2 position[4] = vec2[4](
        vec2(min_x, min_y),
        vec2(min_x, max_y),
        vec2(max_x, min_y),
        vec2(max_x, max_y)
    );

    v_angle = instances[gl_InstanceIndex].angle;
    uint color = instances[gl_InstanceIndex].color;
    v_color = vec4(
          float((color >> 16) & 0xFF) / 255.,
          float((color >> 8) & 0xFF) / 255.,
          float(color & 0xFF) / 255.,
          float((color >> 24) & 0xFF) / 255.);
    v_coords = position[gl_VertexIndex] * vec2(1, -1);

    float radius = instances[gl_InstanceIndex].radius;
    vec2 local_pos = instances[gl_InstanceIndex].center + position[gl_VertexIndex] * radius;
    vec2 world_pos = local_pos - u_scroll_offset; 
    vec2 zoom_factor = u_zoom / (vec2(0.5, 0.5) * u_resolution);
    vec2 transformed_pos = world_pos * zoom_factor * vec2(1., -1);
    float z_index = float(instances[gl_InstanceIndex].z_index) + 0.5;
    float z = z_index >= 0 ? z_index / 10000. : 1e-7;

    gl_Position = vec4(transformed_pos, z, 1.0);
}
