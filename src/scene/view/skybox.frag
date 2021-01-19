#version 450

layout(location=0) in vec3 v_pos;

layout(location=0) out vec4 f_color;

const vec3 SKY_COLOR = vec3(0.207, 0.321, 0.494);
const vec3 SAND_COLOR = vec3(0.368, 0.360, 0.219);
const vec3 HORIZON = vec3(0.917, 0.917, 0.917);

void main() {
    float y = 1. - (v_pos.y + 1.) / 2.;
    vec3 color;
    if (y < 0.33 ) {
     color = SKY_COLOR;
    }
     else if (y < 0.5) {
      float delta = (y - 0.33) / 0.17;
      color = mix(SKY_COLOR, HORIZON, pow(delta, 3.));
    }
      else if (y < 0.66) {
      float delta = 1. - (y - 0.5) / 0.16;
      color = mix(SAND_COLOR, HORIZON, pow(delta, 3.));
    }
    else {
      color = SAND_COLOR;
    }

    f_color = vec4(color, 1.);
}
