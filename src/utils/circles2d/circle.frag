// shader.frag
#version 450

layout(location=0) in vec2 v_coords;
layout(location=1) in float v_angle;
layout(location=2) in float v_visible;

layout(location=0) out vec4 f_color;


void main() {

    float dist = length(v_coords);
    float delta = fwidth(dist);
    float alpha = smoothstep(1. - delta, 1., dist);
    
    if (alpha > 0.5) {
        discard;
    }

    vec4 color = v_visible > 0. ?  vec4(0.0129, 0.4129, 0.5742, 1. - alpha) : vec4(0.3, 0.3, 0.3, 1. - alpha);
    f_color = color;
}
