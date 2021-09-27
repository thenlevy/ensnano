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
    readonly mat4 model_matrix2[];
};

struct Instances {
    mat4 model;
    vec4 color;
    vec3 scale;
    uint id;
    mat4 model_inversed;
};

layout(std430, set=2, binding=0) 
buffer InstancesBlock {
    readonly Instances instances[];
};

const float LOW_CRIT = 1.01;
const float HIGH_CRIT = 1.4;

void main() {
    int model_idx = int(instances[gl_InstanceIndex].id >> 24);

    //mat4 model_matrix = model_matrix2[model_idx] * instances[gl_InstanceIndex].model;
    mat4 model_matrix = model_matrix2[model_idx] * instances[gl_InstanceIndex].model;
    mat4 inversed_model_matrix = instances[gl_InstanceIndex].model_inversed;
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
    vec3 outline = vec3(1.2);
    if (scale.x > LOW_CRIT && abs(scale.x - scale.y) > 1e-5) {
       scale.y *= 1.3;
       scale.z *= 1.3;
       float shade = smoothstep(LOW_CRIT, HIGH_CRIT, scale.x);
       float grey = 0.25 - 0.25 * shade;
       outline.x = 1.;
       if (v_color.w > 0.99) {
           v_color = vec4(grey, grey, grey, 1.);
       }
    } 
    vec4 model_space = model_matrix * vec4((a_position + v_normal * 0.02) * scale * outline, 1.0); 

    if (scale.y < 0.8) {
        float dist = length(u_camera_position - model_space.xyz);
        if (abs(scale.x - scale.y) > 1e-5) {
            scale.yz /= max(1., (dist / 10.));
        } else {
          scale /= max(1., (dist / 10.));
        }
        model_space = model_matrix * vec4((a_position + v_normal * 0.02) * scale * outline, 1.0); 
    }

    v_position = model_space.xyz;
    uint id = instances[gl_InstanceIndex].id;
    v_id = vec4(
          float((id >> 16) & 0xFF) / 255.,
          float((id >> 8) & 0xFF) / 255.,
          float(id & 0xFF) / 255.,
          float((id >> 24) & 0xFF) / 255.);
    gl_Position = u_proj * u_view * model_space;
}
