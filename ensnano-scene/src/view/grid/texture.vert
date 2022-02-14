#version 450

layout(location=0) in vec2 a_position;
layout(location=1) in vec2 a_normal;

void main() {
    vec2 position = a_position + 0.05 * a_normal;
    gl_Position = vec4(position, 0.0, 1.0);
}
