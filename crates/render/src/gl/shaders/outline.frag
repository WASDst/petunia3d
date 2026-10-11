// Contorno de largura constante em pixels ao redor da máscara: cada pixel fora
// dela procura a máscara mais próxima num raio pequeno (para 1–6 px a busca
// direta basta; jump flooding só valeria para raios grandes). A borda externa
// é suavizada pela distância.
uniform sampler2D uMask;
uniform vec4 uColor;
uniform vec4 uActiveColor;
uniform float uRadius;
// Origem do viewport no framebuffer principal (px físicos).
uniform vec2 uOrigin;
out vec4 oColor;
void main() {
    ivec2 dims = textureSize(uMask, 0);
    ivec2 p = ivec2(gl_FragCoord.xy - uOrigin);
    if (texelFetch(uMask, p, 0).r > 0.5) {
        discard;
    }
    int reach = int(ceil(uRadius)) + 1;
    float best = 1e9;
    float nearest_active = 0.0;
    for (int dy = -reach; dy <= reach; dy++) {
        for (int dx = -reach; dx <= reach; dx++) {
            ivec2 q = p + ivec2(dx, dy);
            if (q.x < 0 || q.y < 0 || q.x >= dims.x || q.y >= dims.y) {
                continue;
            }
            vec4 m = texelFetch(uMask, q, 0);
            if (m.r > 0.5) {
                float d = length(vec2(float(dx), float(dy)));
                if (d < best || (d == best && m.g > nearest_active)) {
                    best = d;
                    nearest_active = m.g;
                }
            }
        }
    }
    float alpha = clamp(uRadius + 1.0 - best, 0.0, 1.0);
    if (alpha <= 0.0) {
        discard;
    }
    vec4 color = mix(uColor, uActiveColor, nearest_active);
    oColor = vec4(color.rgb, color.a * alpha);
}
