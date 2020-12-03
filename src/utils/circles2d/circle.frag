// shader.frag
#version 450

layout(location=0) in vec2 v_coords;
layout(location=1) in float v_angle;

layout(location=0) out vec4 f_color;


void main() {

    float dist = length(v_coords);
    float delta = fwidth(dist);
    float alpha = smoothstep(1. - delta, 1., dist);

    vec4 color = vec4(0.0129, 0.4129, 0.5742, 1. - alpha);
    f_color = color;
}
