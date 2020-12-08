#version 450

layout(location=0) in vec4 v_color;
layout(location=1) in vec3 v_normal;
layout(location=2) in vec3 v_position;
layout(location=3) in vec4 v_id;

layout(location=0) out vec4 f_color;

layout(set=0, binding=0) uniform Uniform {
    uniform vec3 u_camera_position;
    mat4 u_view;
    mat4 u_proj;
};

void main() {
    vec3 normal = normalize(v_normal);
    vec3 light_position = v_color.w == 1. ? u_camera_position : vec3(0., 0., 1000.);
    vec3 light_dir = normalize(light_position - v_position);

    if (v_color.w < 0.8 && v_color.w > 0.7) {
        f_color = v_color;
    } else {

        vec3 light_color = vec3(1., 1., 1.);

        float ambient_strength = 0.3;
        vec3 ambient_color = light_color * ambient_strength;

        float diffuse_strength = max(dot(normal, light_dir), 0.0);
        vec3 diffuse_color = light_color * diffuse_strength;

        vec3 view_dir = normalize(u_camera_position - v_position);
        vec3 reflect_dir = reflect(-light_dir, normal);
        float specular_strength = pow(max(dot(view_dir, reflect_dir), 0.0), 32);
        vec3 specular_color = specular_strength * light_color;

        vec3 result = (ambient_color + diffuse_color + specular_color) * v_color.xyz;

        f_color = vec4(result, v_color.w);
    }

}
