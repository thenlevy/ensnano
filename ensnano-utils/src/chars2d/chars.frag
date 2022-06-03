// shader.frag
#version 450

layout(location=0) in vec2 v_tex_coords;
layout(location=1) in vec4 v_color;

layout(location=0) out vec4 f_color;

layout(set = 2, binding = 0) uniform texture2D t_circle;
layout(set = 2, binding = 1) uniform sampler s_circle;


void main() {
    vec4 alpha;

    alpha = texture(sampler2D(t_circle, s_circle), v_tex_coords);

    if (alpha.w < 0.01) {
    discard;
    }

    f_color = vec4(v_color.xyz, alpha.w);
}
