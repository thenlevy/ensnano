#version 450

layout(location=0) out vec2 v_tex_coords;

const vec2 positions[4] = vec2[4](
    vec2(-1.0, -1.0),
    vec2(-1.0, 1.0),
    vec2(1.0, -1.0),
    vec2(1.0, 1.0)
);

void main() {
    v_tex_coords = positions[gl_VertexIndex] * vec2(1., -1.);
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
}
