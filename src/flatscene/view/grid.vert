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
buffer u_models { readonly Model models[]; };

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_normal;
layout(location = 2) in uint a_model_id;
layout(location = 3) in uint a_is_background;

layout(location = 0) out vec4 v_color;

void main() {
    uint id = a_model_id;
    Model model = models[id];

    vec2 invert_y = vec2(1.0, -1.0);

    vec2 zoom_factor = u_zoom / (vec2(0.5, 0.5) * u_resolution);
    vec2 local_pos = model.rotate * (a_position + a_normal * model.width / max(zoom_factor, 0.3));
    vec2 world_pos = local_pos - u_scroll_offset + model.translate;
    vec2 transformed_pos = world_pos * zoom_factor * invert_y;

    float background_depth = a_is_background > 0 ? 0.5 : 0.25;
    float z = (float(model.z_index * 1000 + a_model_id) + background_depth);
    gl_Position = vec4(transformed_pos, z / 1e6, 1.0);
    v_color = a_is_background > 0 ? vec4(0.95, 0.95, 0.95, 1.) : model.color;
}
