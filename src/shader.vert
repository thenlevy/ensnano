// shader.vert
#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec3 a_normal;

layout(location=0) out vec3 v_color;
layout(location=1) out vec3 v_normal;
layout(location=2) out vec3 v_position;


layout(set=0, binding=0)
uniform Uniforms {
    vec3 u_camera_position;
    mat4 u_view_proj;
};

layout(set=1, binding=0) 
buffer Instances {
    mat4 s_models[];
};

layout(set=2, binding=0) uniform Light {
    vec3 light_position;
    vec3 light_color;
};

void main() {
    v_color = vec3(0.5, 0.1, 0.5);

    mat4 model_matrix = s_models[gl_InstanceIndex];
    mat3 normal_matrix = mat3(transpose(inverse(model_matrix)));

    /*Note: I'm currently doing things in world space .
    Doing things in view-space also known as eye-space, is more standard as objects can have
    lighting issues when they are further away from the origin. 
    If we wanted to use view-space, we would use something along the lines
    of mat3(transpose(inverse(view_matrix * model_matrix))).
    Currently we are combining the view matrix and projection matrix before we draw,
    so we'd have to pass those in separately. We'd also have to transform our 
    light's position using something like view_matrix * model_matrix * */
    v_normal = normal_matrix * a_normal;
    vec4 model_space = s_models[gl_InstanceIndex] * vec4(a_position, 1.0); 
    v_position = model_space.xyz;
    gl_Position = u_view_proj * model_space;
}
