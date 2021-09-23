// shader.frag
#version 450

layout(location=0) in vec4 v_color;
layout(location=1) in vec3 v_normal;
layout(location=2) in vec3 v_position;
layout(location=3) in vec4 v_id;

layout(location=0) out vec4 f_color;

layout(set=0, binding=0) uniform Uniform {
    uniform vec3 u_camera_position;
    mat4 u_view;
    mat4 u_proj;
    mat4 u_view_inversed;
    float u_fog_radius;
    float u_fog_length;
    uint u_make_fog;
    uint u_fog_from_cam;
    vec3 u_fog_center;
};


void main() {
    float visibility;
    if (u_make_fog > 0) {
        float dist;
        if (u_fog_from_cam > 0) {
           dist = length(u_camera_position - v_position);
        } else {
          dist = length(u_fog_center - v_position);
        }
        visibility =  1. - smoothstep(u_fog_length, u_fog_length + u_fog_radius, dist);
    } else {
        visibility = 1.;
    }

    if (visibility < 0.9) {
     discard;
    }
    f_color = vec4(0., 0., 0., 1.);
}
