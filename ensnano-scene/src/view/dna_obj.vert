#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec3 a_normal;

layout(location=0) out vec4 v_color;
layout(location=1) out vec3 v_normal;
layout(location=2) out vec3 v_position;
layout(location=3) out vec4 v_id;


layout(std140, set=0, binding=0)
uniform Uniforms {
    vec3 u_camera_position;
    mat4 u_view;
    mat4 u_proj;
    mat4 u_inversed_view;
    vec4 u_padding1;
    vec3 u_padding2;
    float u_stereography_radius;
    mat4 u_stereography_view;
    float u_aspect_ratio;
    float u_stereography_zoom;
};

layout(set=1, binding=0) buffer ModelBlock {
    readonly mat4 model_matrix2[];
};

struct Instances {
    mat4 model;
    vec4 color;
    vec3 scale;
    uint id;
    mat4 inversed_model;
    float expected_length;
};

layout(std430, set=2, binding=0) 
buffer InstancesBlock {
    readonly Instances instances[];
};

const float LOW_CRIT = 1. / 0.7;
const float HIGH_CRIT = 2. / 0.7;

void main() {
    int model_idx = 0;

    //mat4 model_matrix = model_matrix2[model_idx] * instances[gl_InstanceIndex].model;
    mat4 model_matrix = model_matrix2[model_idx] * instances[gl_InstanceIndex].model;
    mat4 inversed_model_matrix = instances[gl_InstanceIndex].inversed_model;
    mat3 normal_matrix = mat3(transpose(inversed_model_matrix));

    /*Note: I'm currently doing things in world space .
    Doing things in view-space also known as eye-space, is more standard as objects can have
    lighting issues when they are further away from the origin. 
    If we wanted to use view-space, we would use something along the lines
    of mat3(transpose(inverse(view_matrix * model_matrix))).
    Currently we are combining the view matrix and projection matrix before we draw,
    so we'd have to pass those in separately. We'd also have to transform our 
    light's position using something like view_matrix * model_matrix * */
    v_normal = normal_matrix * a_normal;
    v_color = instances[gl_InstanceIndex].color;
    vec3 scale = instances[gl_InstanceIndex].scale;
    float expected_length = instances[gl_InstanceIndex].expected_length;
    if (expected_length > 0.) {
        if (scale.x > expected_length * LOW_CRIT) {
           scale.y *= 1.3;
           scale.z *= 1.3;
           float shade = smoothstep(expected_length * LOW_CRIT, expected_length * HIGH_CRIT, scale.x);
           float grey = 0.25 - 0.25 * shade;
           if (v_color.w > 0.99) {
               v_color = vec4(grey, grey, grey, 1.);
           }
        } 
    }
    vec4 model_space = model_matrix * vec4(a_position * scale, 1.0); 

    /* if (scale.y < 0.8) {
        float dist = length(u_camera_position - model_space.xyz);
        if (abs(scale.x - scale.y) > 1e-5) {
            scale.yz /= max(1., (dist / 10.));
        } else {
          scale /= max(1., (dist / 10.));
        }
        model_space = model_matrix * vec4(a_position * scale, 1.0); 
    }*/

    v_position = model_space.xyz;
    uint id = instances[gl_InstanceIndex].id;
    v_id = vec4(
          float((id >> 16) & 0xFF) / 255.,
          float((id >> 8) & 0xFF) / 255.,
          float(id & 0xFF) / 255.,
          float((id >> 24) & 0xFF) / 255.);
    if (u_stereography_radius > 0.0) {
        vec4 view_space = u_stereography_view * model_space;
        view_space /= u_stereography_radius;
        float dist = length(view_space.xyz);
        vec3 projected = view_space.xyz / dist;
        float close_to_pole = 0.0;
        if (projected.z > (1. - (0.01 / u_stereography_zoom))) {
            close_to_pole = 1.0;
        }
        float z = max(close_to_pole, atan(dist) * 2. / 3.14);
        gl_Position = vec4(projected.x / (1. - projected.z) / u_stereography_zoom / u_aspect_ratio, projected.y / (1. - projected.z) / u_stereography_zoom, z, 1.);
    } else {
        gl_Position = u_proj * u_view * model_space;
    }
}
