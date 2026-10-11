// Arestas de largura constante em pixels (Bærentzen et al.; Plasticity): cada
// segmento vira uma faixa de dois triângulos expandida na tela; o MSAA
// suaviza as bordas. A largura não muda com zoom, distância ou DPI.
layout(location = 0) in vec3 aA;
layout(location = 1) in vec3 aB;
layout(location = 2) in vec4 aColor;
// x = ponta (0/1), y = lado (-1/+1), z = largura (multiplicador ou px).
layout(location = 3) in vec3 aCorner;
uniform mat4 uViewProj;
uniform mat4 uModel;
uniform vec2 uViewportSize;
// Px físicos por unidade de `aCorner.z`.
uniform float uWidthScale;
uniform float uDepthBias;
out vec4 vColor;
void main() {
    vec4 ca = uViewProj * uModel * vec4(aA, 1.0);
    vec4 cb = uViewProj * uModel * vec4(aB, 1.0);
    vec2 size = max(uViewportSize, vec2(1.0));
    vec2 sa = ca.xy / max(ca.w, 1e-5) * size * 0.5;
    vec2 sb = cb.xy / max(cb.w, 1e-5) * size * 0.5;
    vec2 dir = sb - sa;
    if (length(dir) < 1e-5) {
        dir = vec2(1.0, 0.0);
    }
    dir = normalize(dir);
    vec2 normal = vec2(-dir.y, dir.x);
    vec4 p = aCorner.x > 0.5 ? cb : ca;
    float half_width = max(uWidthScale * aCorner.z, 1.0) * 0.5;
    vec2 offset = normal * aCorner.y * half_width / (size * 0.5);
    gl_Position = to_gl_clip(vec4(p.xy + offset * p.w, p.z - uDepthBias * p.w, p.w));
    vColor = aColor;
}
