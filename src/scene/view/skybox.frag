#version 450

layout(location=0) in vec3 v_pos;

layout(location=0) out vec4 f_color;

const vec3 SKY_COLOR = vec3(0.207, 0.321, 0.494);
const vec3 SAND_COLOR = vec3(0.368, 0.360, 0.219);

void main() {
    float y = 1. - (v_pos.y + 1.) / 2.;
    vec3 color = mix(SKY_COLOR, SAND_COLOR, sqrt(y));

    f_color = vec4(color, 1.);
}
