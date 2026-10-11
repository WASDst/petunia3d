// Blur 5×5 que respeita a profundidade (não mistura AO entre objetos em
// distâncias diferentes).
uniform sampler2D uAoRaw;
out vec4 oColor;
void main() {
    ivec2 dims = ivec2(uSize.zw);
    ivec2 p = ivec2(gl_FragCoord.xy);
    vec2 scale = uSize.xy / uSize.zw;
    float dc = depth_at((vec2(p) + vec2(0.5)) * scale);
    if (dc >= 1.0) {
        oColor = vec4(1.0, 0.0, 0.0, 1.0);
        return;
    }
    float zc = view_pos((vec2(p) + vec2(0.5)) * scale, dc).z;
    float tolerance = abs(zc) * 0.04 + 1.0e-4;
    float sum = 0.0;
    float weight = 0.0;
    for (int dy = -2; dy <= 2; dy++) {
        for (int dx = -2; dx <= 2; dx++) {
            ivec2 q = clamp(p + ivec2(dx, dy), ivec2(0), dims - ivec2(1));
            vec2 center = (vec2(q) + vec2(0.5)) * scale;
            float dq = depth_at(center);
            if (dq >= 1.0) {
                continue;
            }
            float w = exp(-abs(view_pos(center, dq).z - zc) / tolerance);
            sum += texelFetch(uAoRaw, q, 0).r * w;
            weight += w;
        }
    }
    oColor = vec4(weight > 1.0e-5 ? sum / weight : 1.0, 0.0, 0.0, 1.0);
}
