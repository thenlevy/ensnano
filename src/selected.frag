// shader.frag
#version 450

layout(location=0) in vec3 v_color;
layout(location=1) in vec3 v_normal;
layout(location=2) in vec3 v_position;

layout(location=0) out vec4 f_color;

layout(set=0, binding=0) uniform Uniform {
    uniform vec3 u_camera_position;
    mat4 u_view_proj;
};

layout(set=2, binding = 0) uniform Light {
    vec3 light_position;
    vec3 light_color;
};


void main() {
    f_color = vec4(1., 0., 0., 0.75);
}
