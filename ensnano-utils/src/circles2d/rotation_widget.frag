// shader.frag
#version 450

layout(location=0) in vec2 v_coords;
layout(location=1) in float v_angle;

layout(location=0) out vec4 f_color;


void main() {

    float dist = length(v_coords);
    float delta = fwidth(dist);
    float alpha_in = smoothstep(1. - delta, 1., dist);
    float alpha_out = smoothstep(0.9 - delta, 0.9, dist);
    float alpha = (1. - alpha_in) * (alpha_out);

    mat2 rotation = mat2(vec2(cos(-v_angle), -sin(-v_angle)), vec2(sin(-v_angle), cos(-v_angle)));
    vec2 coords = rotation * v_coords;

    float x = abs(coords.x);
    float delta_x = fwidth(x);
    float alpha_x = (1. - alpha_in) * (1. - smoothstep(0.1 - delta_x, 0.1, x));

    float y = abs(coords.y);
    float delta_y = fwidth(y);
    float alpha_y = (1. - alpha_in) * (1. - smoothstep(0.1 - delta_y, 0.1, y));


    vec4 color = vec4(0.0129, 0.4129, 0.5742, max(alpha, max(alpha_x, alpha_y)));
    f_color = color;
}
