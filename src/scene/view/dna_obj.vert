#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec3 a_normal;

layout(location=0) out vec4 v_color;
layout(location=1) out vec3 v_normal;
layout(location=2) out vec3 v_position;
layout(location=3) out vec4 v_id;


layout(set=0, binding=0)
uniform Uniforms {
    vec3 u_camera_position;
    mat4 u_view;
    mat4 u_proj;
};

layout(set=1, binding=0) buffer ModelBlock {
    mat4 model_matrix2[];
};

struct Instances {
    mat4 model;
    vec4 color;
    vec3 scale;
    uint id;
};

layout(std430, set=2, binding=0) 
buffer InstancesBlock {
    Instances instances[];
};

const float DIST_NUCL = 0.67;
const float TOLERANCE = 1.3;
const float CRITICAL_DIST = DIST_NUCL * TOLERANCE;

void main() {
    int model_idx = int(instances[gl_InstanceIndex].id >> 24);

    //mat4 model_matrix = model_matrix2[model_idx] * instances[gl_InstanceIndex].model;
    mat4 model_matrix = model_matrix2[model_idx] * instances[gl_InstanceIndex].model;
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
    vec3 scale = instances[gl_InstanceIndex].scale;
    if (scale.x > CRITICAL_DIST && abs(scale.x - scale.y) > 1e-5) {
       scale.y *= 1.3;
       scale.z *= 1.3;
       v_color = vec4(0., 0., 0., 1.);
    } else {
        v_color = instances[gl_InstanceIndex].color;
    }
    vec4 model_space = model_matrix * vec4(a_position * scale, 1.0); 
    v_position = model_space.xyz;
    uint id = instances[gl_InstanceIndex].id;
    v_id = vec4(
          float((id >> 16) & 0xFF) / 255.,
          float((id >> 8) & 0xFF) / 255.,
          float(id & 0xFF) / 255.,
          float((id >> 24) & 0xFF) / 255.);
    gl_Position = u_proj * u_view * model_space;
}
