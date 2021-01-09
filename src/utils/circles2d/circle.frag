// shader.frag
#version 450

layout(location=0) in vec2 v_coords;
layout(location=1) in float v_angle;
layout(location=2) in vec4 v_color;

layout(location=0) out vec4 f_color;


void main() {

    float dist = length(v_coords);
    float delta = fwidth(dist);
    float alpha = smoothstep(1. - delta, 1., dist);
    
    if (alpha > 0.5) {
        discard;
    }

    vec4 color = vec4(v_color.xyz, (1. - alpha) * v_color.w);
    f_color = color;
}
