#version 450


layout(location=0) in vec2 a_position;
layout(location=1) in vec2 a_tex_coords;

layout(location=0) out vec2 v_tex_coords;
layout(location=1) out vec4 v_color;

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
    vec4 color;
};

layout(set=1, binding=0) 
buffer InstancesBlock {
    readonly Instances instances[];
};

void main() {
    mat2 rotate = instances[gl_InstanceIndex].rotation;

    v_tex_coords = a_tex_coords;

    Instances this_letter = instances[gl_InstanceIndex];
    float size = this_letter.size;
    v_color = this_letter.color;

    if (u_zoom < 7.) {
       size *= 2.;
    }

    vec2 local_pos = instances[gl_InstanceIndex].center + ( a_position * size);
    vec2 world_pos = local_pos - u_scroll_offset; 
    vec2 zoom_factor = u_zoom / (vec2(0.5, 0.5) * u_resolution);
    vec2 transformed_pos = world_pos * zoom_factor * vec2(1., -1.);
    float z_index = float(instances[gl_InstanceIndex].z_index);
    float z = z_index >= 0 ? z_index / 10000. : 1e-7;
    gl_Position = vec4(transformed_pos, z, 1.0);
}
