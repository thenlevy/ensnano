// shader.vert
#version 450

layout(location=0) in vec2 a_position;
layout(location=1) in vec2 a_tex_coords;

layout(location=0) out vec4 v_color;
layout(location=1) out vec2 v_tex_coords;


layout(set=0, binding=0)
uniform Uniforms {
    vec3 u_camera_position;
    mat4 u_view;
    mat4 u_proj;
};

struct Instances {
    mat4 model;
    vec4 color;
    vec4 id;
};

layout(set=1, binding=0) 
buffer InstancesBlock {
    Instances instances[];
};

layout(set=3, binding=0) buffer ModelBlock {
    mat4 model_matrix2[];
};

void main() {
    v_color = instances[gl_InstanceIndex].color;
    v_tex_coords = a_tex_coords;
    int model_idx = int(instances[gl_InstanceIndex].id.w);

    //mat4 model_matrix = model_matrix2[model_idx] * instances[gl_InstanceIndex].model;
    mat4 rotation = mat4(mat3(inverse(u_view)));
    mat4 model_matrix = model_matrix2[model_idx] * instances[gl_InstanceIndex].model * rotation;

    vec4 model_space = model_matrix * vec4(vec3(a_position * vec2(0.5, -0.5), 0.0), 1.0); 
    gl_Position = u_proj * (u_view * model_space + vec4(0., 0., 0.25, 0.));
}
