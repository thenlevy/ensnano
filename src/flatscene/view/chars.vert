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
    mat2 rotation;
    float size;
    int z_index;
};

layout(set=1, binding=0) 
buffer InstancesBlock {
    Instances instances[];
};

layout(set=2, binding = 2) uniform TexSize {
   vec2 tex_size;
};

void main() {
    float min_x = 0.;
    float max_x = 1.;
    float min_y = 0.;
    float max_y = 1.;

    vec2 position[4] = vec2[4](
        vec2(min_x, min_y),
        vec2(min_x, max_y),
        vec2(max_x, min_y),
        vec2(max_x, max_y)
    );

    v_tex_coords = position[gl_VertexIndex] * vec2(1., 1.) * tex_size / 512.;

    mat2 rotate = instances[gl_InstanceIndex].rotation;
    float size = instances[gl_InstanceIndex].size;

    vec2 local_pos = instances[gl_InstanceIndex].center + rotate *((position[gl_VertexIndex] * 2. - 1.) * tex_size / 512. * size);
    vec2 world_pos = local_pos - u_scroll_offset; 
    vec2 zoom_factor = u_zoom / (vec2(0.5, 0.5) * u_resolution);
    vec2 transformed_pos = world_pos * zoom_factor * vec2(1., -1);
    float z_index = float(instances[gl_InstanceIndex].z_index);
    float z = z_index >= 0 ? z_index / 1000. : 1e-7;
    gl_Position = vec4(transformed_pos, z, 1.0);
}
