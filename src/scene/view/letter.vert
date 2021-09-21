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
    mat4 u_view_inversed;
};

layout(set=1, binding=0) buffer ModelBlock {
    readonly mat4 model_matrix2[];
};

struct Instances {
    vec3 position;
    uint id;
    vec4 color;
    vec3 shift;
    float scale;
};

layout(std430, set=2, binding=0) 
buffer InstancesBlock {
    readonly Instances instances[];
};


void main() {
    v_color = instances[gl_InstanceIndex].color;
    v_tex_coords = a_tex_coords;
    uint model_idx = instances[gl_InstanceIndex].id;

    mat4 model = mat4(1.0);
    model[3] = vec4(instances[gl_InstanceIndex].position, 1.);

    //mat4 model_matrix = model_matrix2[model_idx] * instances[gl_InstanceIndex].model;
    mat4 rotation = mat4(mat3(u_view_inversed));
    mat4 model_matrix = model_matrix2[model_idx] * model * rotation;

    float scale = instances[gl_InstanceIndex].scale;
    vec3 shift = instances[gl_InstanceIndex].shift;
    vec4 model_space = model_matrix * vec4(vec3((a_position + vec2(shift)) * scale * vec2(0.5, -0.5), 0.0), 1.0); 
    gl_Position = u_proj * (u_view * model_space + vec4(0., 0., 0.25, 0.));
}
