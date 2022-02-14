#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec4 a_color;

layout(location=0) out vec4 v_color;

layout(set=0, binding=0)
uniform Uniforms {
    vec3 u_camera_position;
    mat4 u_view;
    mat4 u_proj;
};

layout(set=1, binding=0) buffer ModelBlock {
    readonly mat4 model_matrix2[];
};

struct Instances {
    mat4 model;
    vec4 color;
    float radius;
    uint identifier;
};

layout(set=2, binding=0) 
buffer InstancesBlock {
    readonly Instances instances[];
};

void main() {
    v_color = a_color * instances[gl_InstanceIndex].color;
    uint model_idx = instances[gl_InstanceIndex].identifier;

    mat4 model_matrix = model_matrix2[model_idx] * instances[gl_InstanceIndex].model;
    float radius = instances[gl_InstanceIndex].radius;

    vec4 model_space = model_matrix * vec4(radius * a_position, 1.0); 
    gl_Position = u_proj * u_view * model_space;
}
