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
    mat4 u_inversed_view;
    float u_fog_radius;
    float u_fog_length;
    uint u_make_fog;
    uint u_fog_from_cam;
    vec3 u_fog_center;
};

const float HALF_LIFE = 10.;
const float GREY_VAL = pow(0.8, 2.2);
const vec3 BG_COLOR = vec3(GREY_VAL, GREY_VAL, GREY_VAL);

const vec3 SKY_COLOR = vec3(0.207, 0.321, 0.494);
const vec3 SAND_COLOR = vec3(0.368, 0.360, 0.219);
const vec3 HORIZON = vec3(0.917, 0.917, 0.917);
const vec3 DARK_FOG_COLOR = vec3(0.01, 0.01, 0.03);

void main() {
    vec3 normal = normalize(v_normal);
    vec3 light_position = abs(v_color.w - 1.) < 1e-3 ? u_camera_position : vec3(0., 0., 1000.);
    vec3 light_dir = normalize(light_position - v_position);


    vec3 view_dir = normalize(u_camera_position - v_position);

    if (v_color.w < 0.8 && v_color.w > 0.7) {
        f_color = v_color;
    } else {

        vec3 light_color = vec3(1., 1., 1.);

        float ambient_strength = 0.3;
        vec3 ambient_color = light_color * ambient_strength;

        float diffuse_strength = max(dot(normal, light_dir), 0.0);
        vec3 diffuse_color = light_color * diffuse_strength;

        vec3 reflect_dir = reflect(-light_dir, normal);
        float specular_strength = pow(max(dot(view_dir, reflect_dir), 0.0), 32);
        vec3 specular_color = specular_strength * light_color;

        vec3 result = (ambient_color + diffuse_color + specular_color) * v_color.xyz;

        f_color = vec4(result, v_color.w);
    }


    float visibility;
    if (u_make_fog > 0) {
        float dist;
        if (u_fog_from_cam > 0) {
           dist = length(u_camera_position - v_position);
        } else {
          dist = length(u_fog_center - v_position);
        }
        visibility =  1. - smoothstep(u_fog_length, u_fog_length + u_fog_radius, dist);
    } else {
        visibility = 1.;
    }

    if (visibility < 0.1 && u_make_fog == 1) {
     discard;
    }

    float y;
    if (abs(view_dir.y) > abs(view_dir.x) && abs(view_dir.y) > abs(view_dir.z)) {
       y = view_dir.y > 0 ? 1. : 0.;
    } else if (abs(view_dir.x) > abs(view_dir.z)) {
       y = (view_dir.y/abs(view_dir.x) + 1.) / 2.;
    } else {
       y = (view_dir.y/abs(view_dir.z) + 1.) / 2.;
    }

    vec3 fog_color;
    if (u_make_fog == 1) {
        if (y < 0.33 ) {
         fog_color = SKY_COLOR;
        }
         else if (y < 0.5) {
          float delta = (y - 0.33) / 0.17;
          fog_color = mix(SKY_COLOR, HORIZON, pow(delta, 3.));
        }
          else if (y < 0.66) {
          float delta = 1. - (y - 0.5) / 0.16;
          fog_color = mix(SAND_COLOR, HORIZON, pow(delta, 3.));
        }
        else {
          fog_color = SAND_COLOR;
        }
    } else {
      fog_color = DARK_FOG_COLOR;
    }

    f_color = mix(vec4(fog_color, 1.0), f_color, visibility);

}
