#version 450

layout(location=0) out uint v_grid_type;
layout(location=1) out vec2 v_tex_honney_coords;
layout(location=2) out vec2 v_tex_square_coords;


layout(set=0, binding=0)
uniform Uniforms {
    vec3 u_camera_position;
    mat4 u_view;
    mat4 u_proj;
};

struct Instances {
    mat4 model;
    float min_x;
    float max_x;
    float min_y;
    float max_y;
    uint grid_type;
};

layout(set=1, binding=0) 
buffer InstancesBlock {
    Instances instances[];
};

layout(set = 3, binding = 0)
uniform Parameters {
    float u_helix_radius;
    float u_inter_helix_gap;
};

void main() {
    v_grid_type = instances[gl_InstanceIndex].grid_type;
    float r = u_helix_radius + u_inter_helix_gap / 2.;

    float min_x = instances[gl_InstanceIndex].min_x;
    float max_x = instances[gl_InstanceIndex].max_x;
    float min_y = instances[gl_InstanceIndex].min_y;
    float max_y = instances[gl_InstanceIndex].max_y;

    vec2 position[4] = vec2[4](
        vec2(min_x, min_y),
        vec2(min_x, max_y),
        vec2(max_x, min_y),
        vec2(max_x, max_y)
    );
    v_tex_honney_coords = position[gl_VertexIndex] * vec2(1., 1./3.);
    v_tex_square_coords = position[gl_VertexIndex];

    mat4 model_matrix = instances[gl_InstanceIndex].model;
    
    vec2 coeff = v_grid_type == 0 ? vec2(2. * r, 2. * r) : vec2(sqrt(3) * r, r);

    vec2 pos = position[gl_VertexIndex] * coeff;
    vec4 model_space = model_matrix * vec4(0., pos.y, pos.x, 1.0); 
    gl_Position = u_proj * u_view * model_space;
}
