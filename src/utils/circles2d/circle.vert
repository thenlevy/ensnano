#version 450

layout(location=0) out vec2 v_tex_coords;

layout(set = 0, binding = 0)
uniform Globals {
    vec2 u_resolution;
    vec2 u_scroll_offset;
    float u_zoom;
};

struct Instances {
    vec2 center;
    float radius;
};

layout(std430, set=1, binding=0) 
buffer InstancesBlock {
    Instances instances[];
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

    // The conversion to [0, 1] x [0, 1] is done in the fragment shader
    v_tex_coords = position[gl_VertexIndex] * vec2(1., -1.);

    float radius = instances[gl_InstanceIndex].radius;
    vec2 local_pos = instances[gl_InstanceIndex].center + position[gl_VertexIndex] * radius;
    vec2 world_pos = local_pos - u_scroll_offset; 
    vec2 zoom_factor = u_zoom / (vec2(0.5, 0.5) * u_resolution);
    vec2 transformed_pos = world_pos * zoom_factor * vec2(1., -1);

    gl_Position = vec4(transformed_pos, 0.000001, 1.0);
}
