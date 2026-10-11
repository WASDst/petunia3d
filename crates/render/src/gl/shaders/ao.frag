// Oclusão ambiente por horizontes (GTAO, Jimenez et al. 2016): para cada fatia
// de direção em tela, os dois horizontes mais altos limitados à normal integram
// a visibilidade com peso cosseno. Normal reconstruída do depth; 4 fatias × 6
// passos com ruído intercalado, em meia resolução.
out vec4 oColor;

const float PI = 3.14159265;
const int SLICES = 4;
const int STEPS = 6;

float noise(vec2 p) {
    return fract(52.9829189 * fract(dot(p, vec2(0.06711056, 0.00583715))));
}

void main() {
    vec2 scale = uSize.xy / uSize.zw;
    vec2 center = (floor(gl_FragCoord.xy) + vec2(0.5)) * scale;
    float d = depth_at(center);
    if (d >= 1.0) {
        oColor = vec4(1.0, 0.0, 0.0, 1.0);
        return;
    }
    vec3 pos = view_pos(center, d);
    vec2 ex = vec2(1.0, 0.0);
    vec2 ey = vec2(0.0, 1.0);
    vec3 right = view_pos(center + ex, depth_at(center + ex)) - pos;
    vec3 left = pos - view_pos(center - ex, depth_at(center - ex));
    vec3 up = view_pos(center + ey, depth_at(center + ey)) - pos;
    vec3 down = pos - view_pos(center - ey, depth_at(center - ey));
    vec3 dx = abs(left.z) < abs(right.z) ? left : right;
    vec3 dy = abs(down.z) < abs(up.z) ? down : up;
    bool ortho = uParams.y > 0.5;
    vec3 view_dir = ortho ? vec3(0.0, 0.0, 1.0) : normalize(-pos);
    vec3 n = normalize(cross(dx, dy));
    if (dot(n, view_dir) < 0.0) {
        n = -n;
    }
    float radius = uParams.z;
    float depth_scale = ortho ? 1.0 : max(-pos.z, 1.0e-4);
    float radius_px = min(radius * uParams.x * 0.5 * uSize.y / depth_scale, uLimits.x);
    if (radius_px < 1.5) {
        oColor = vec4(1.0, 0.0, 0.0, 1.0);
        return;
    }
    float rotation = noise(gl_FragCoord.xy);
    float jitter = fract(rotation * 7.31);
    float visibility = 0.0;
    for (int slice = 0; slice < SLICES; slice++) {
        float phi = (float(slice) + rotation) * PI / float(SLICES);
        vec2 dir2 = vec2(cos(phi), sin(phi));
        // Origem embaixo-esquerda: y da tela coincide com y de vista.
        vec3 dir3 = vec3(dir2, 0.0);
        vec3 slice_ortho = dir3 - dot(dir3, view_dir) * view_dir;
        vec3 axis = cross(dir3, view_dir);
        vec3 proj_n = n - axis * dot(n, axis);
        float proj_len = length(proj_n);
        if (proj_len < 1.0e-4) {
            visibility += 1.0;
            continue;
        }
        float cos_n = clamp(dot(proj_n, view_dir) / proj_len, -1.0, 1.0);
        float n_angle = sign(dot(slice_ortho, proj_n)) * acos(cos_n);
        vec2 max_cos = vec2(-1.0);
        for (int s_index = 0; s_index < STEPS; s_index++) {
            float t = (float(s_index) + jitter + 0.5) / float(STEPS);
            for (int side = 0; side < 2; side++) {
                float s = side == 1 ? 1.0 : -1.0;
                vec2 sample_px = center + dir2 * (s * t * radius_px);
                float sd = depth_at(sample_px);
                if (sd >= 1.0) {
                    continue;
                }
                vec3 delta = view_pos(sample_px, sd) - pos;
                float dist = length(delta);
                if (dist < 1.0e-5) {
                    continue;
                }
                float falloff = clamp(1.0 - dist * dist / (radius * radius), 0.0, 1.0);
                float c = mix(-1.0, dot(delta / dist, view_dir), falloff);
                if (side == 0) {
                    max_cos.x = max(max_cos.x, c);
                } else {
                    max_cos.y = max(max_cos.y, c);
                }
            }
        }
        float h1 = n_angle + max(-acos(max_cos.x) - n_angle, -PI * 0.5);
        float h2 = n_angle + min(acos(max_cos.y) - n_angle, PI * 0.5);
        float sin_n = sin(n_angle);
        float arc1 = -cos(2.0 * h1 - n_angle) + cos_n + 2.0 * h1 * sin_n;
        float arc2 = -cos(2.0 * h2 - n_angle) + cos_n + 2.0 * h2 * sin_n;
        visibility += proj_len * 0.25 * (arc1 + arc2);
    }
    float v = clamp(visibility / float(SLICES), 0.0, 1.0);
    oColor = vec4(pow(v, uParams.w), 0.0, 0.0, 1.0);
}
