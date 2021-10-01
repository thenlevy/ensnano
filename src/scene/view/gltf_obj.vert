#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec3 a_normal;
layout(location=2) in vec4 a_color;

layout(location=0) out vec4 v_color;
layout(location=1) out vec3 v_normal;
layout(location=2) out vec3 v_position;


layout(set=0, binding=0)
uniform Uniforms {
    vec3 u_camera_position;
    mat4 u_view;
    mat4 u_proj;
    mat4 u_inversed_view;
};

void main() {
    mat4 model_matrix = mat4(1.); //identity for now, we will use real model later
    mat4 inversed_model_matrix = mat4(1.); // identity for now
    mat3 normal_matrix = mat3(transpose(inversed_model_matrix));

    v_normal = normal_matrix * a_normal;
    v_color = a_color;
    vec4 model_space = model_matrix * vec4(a_position, 1.0); 

    v_position = model_space.xyz;
    gl_Position = u_proj * u_view * model_space;
}
