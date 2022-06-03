#version 450

layout(set = 0, binding = 0)
uniform Globals {
    vec2 u_resolution;
    vec2 u_scroll_offset;
    float u_zoom;
};

struct Instances {
    vec2 position;
    float depth;
    int padd;
    mat2 rotation;
    vec4 color;
};

layout(set=1, binding=0) 
buffer InstancesBlock {
    readonly Instances instances[];
};

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_normal;


layout(location = 0) out vec4 v_color;

void main() {
    vec2 invert_y = vec2(1.0, -1.0);

    float nb_pixel = 3.;
    float cst = max(0.1125, nb_pixel / ( 4. * u_zoom));

    vec2 position = instances[gl_InstanceIndex].position;
    mat2 rotation = instances[gl_InstanceIndex].rotation;
    float depth = instances[gl_InstanceIndex].depth;

    vec2 local_pos = rotation * (a_position + a_normal * cst);
    vec2 world_pos = local_pos - u_scroll_offset + position;
    vec2 transformed_pos = world_pos * u_zoom / (vec2(0.5, 0.5) * u_resolution) * invert_y;

// formula for the grid:
//float z = (float(model.z_index * 1000 + a_model_id) + background_depth);
// We want the strand to have the depth of the grid - 0.25.

    float z = depth * 1000.;
    gl_Position = vec4(transformed_pos, z / 1e6, 1.0);
    v_color = instances[gl_InstanceIndex].color;
}
