#version 450

layout(set = 0, binding = 0)
uniform Globals {
    vec2 u_resolution;
    vec2 u_scroll_offset;
    float u_zoom;
};

struct Model {
    vec4 color;
    vec2 translate;
    mat2 rotate;
    int z_index;
    float width;
};

layout(set = 1, binding = 0)
buffer u_models { Model models[]; };

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_normal;
layout(location = 2) in uint a_model_id;

layout(location = 0) out vec4 v_color;

void main() {
    uint id = a_model_id;
    Model model = models[id];

    vec2 invert_y = vec2(1.0, -1.0);

    vec2 local_pos = model.rotate * (a_position + a_normal * model.width);
    vec2 world_pos = local_pos - u_scroll_offset + model.translate;
    vec2 transformed_pos = world_pos * u_zoom / (vec2(0.5, 0.5) * u_resolution) * invert_y;

    float z = float(model.z_index) / 4096.0;
    gl_Position = vec4(transformed_pos, z / 1000.0, 1.0);
    v_color = model.color;
}
