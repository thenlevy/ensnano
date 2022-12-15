#version 450

layout(location=0) in vec2 a_position;

layout(location=0) out vec2 v_tex_pos;
layout(location=1) out float graduation_unit;
layout(location=2) out float rotation_radius;


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
    float min_x;
    float max_x;
    float min_y;
    float max_y;
    float graduation_unit;
    float rotation_radius;
};

layout(set=2, binding=0) 
buffer InstancesBlock {
    readonly Instances instances[];
};


void main() {
    float min_x = instances[gl_InstanceIndex].min_x - 0.025;
    float max_x = instances[gl_InstanceIndex].max_x + 0.025;
    float min_y = instances[gl_InstanceIndex].max_y - 0.025;
    float max_y = instances[gl_InstanceIndex].min_y + 0.025;

    vec2 plane_position = vec2((max_x - min_x) * a_position.x + min_x,
                         (max_y - min_y) * a_position.y + min_y);

    v_tex_pos = plane_position;
    graduation_unit = instances[gl_InstanceIndex].graduation_unit;
    rotation_radius = instances[gl_InstanceIndex].rotation_radius;

    mat4 model_matrix = instances[gl_InstanceIndex].model;

    vec4 model_space = model_matrix * vec4(0., plane_position.y, plane_position.x, 1.0); 
    gl_Position = u_proj * u_view * model_space;
}
