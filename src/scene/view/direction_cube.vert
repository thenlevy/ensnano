#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec2 a_tex_pos;

layout(location=0) out vec2 v_tex_pos;

layout(set=0, binding=0)
uniform Uniforms {
    vec3 u_camera_position;
    mat4 u_view;
    mat4 u_proj;
    mat4 u_inverted_view;
};

layout(set=1, binding=0) buffer ModelBlock {
    readonly mat4 model_matrix2[];
};

struct Instances {
    float dist;
};

layout(std430, set=2, binding=0) 
buffer InstancesBlock {
    readonly Instances instances[];
};

void main() {
    v_tex_pos = a_tex_pos;
    float dist = instances[0].dist;
    vec4 position = u_inverted_view * vec4(0., 0., -dist - 1., 0.) + vec4(u_camera_position, 1.) + vec4(a_position, 0.);
    gl_Position = u_proj * u_view * position;
}
