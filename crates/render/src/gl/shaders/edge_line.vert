// Arestas do objeto: a mesma faixa de largura constante do `wide_line`, mais o
// descarte das arestas cujas duas faces vizinhas estão de costas. O deslocamento
// de profundidade (que evita a briga com a face coplanar) deixava passar os
// primeiros pixels dessas arestas de trás nos cantos ("tocos"); descartá-las é
// exato para o Sólido. Normal zero = borda aberta/wireframe: sempre desenhada.
layout(location = 0) in vec3 aA;
layout(location = 1) in vec3 aB;
layout(location = 2) in vec4 aColor;
layout(location = 3) in vec3 aCorner;
layout(location = 4) in vec3 aN1;
layout(location = 5) in vec3 aN2;
uniform mat4 uViewProj;
uniform mat4 uModel;
uniform mat3 uNormalModel;
uniform vec2 uViewportSize;
uniform float uWidthScale;
uniform float uDepthBias;
uniform vec3 uEye;
uniform vec3 uViewDir;
uniform float uOrtho;
uniform float uCullBack;
out vec4 vColor;

bool facing(vec3 n, vec3 toEye) {
    vec3 world = uNormalModel * n;
    return dot(world, toEye) > 0.0;
}

void main() {
    vColor = aColor;
    bool known = dot(aN1, aN1) > 0.0 && dot(aN2, aN2) > 0.0;
    if (uCullBack > 0.5 && known) {
        vec3 mid = (uModel * vec4((aA + aB) * 0.5, 1.0)).xyz;
        vec3 toEye = uOrtho > 0.5 ? -uViewDir : uEye - mid;
        if (!facing(aN1, toEye) && !facing(aN2, toEye)) {
            // Fora do volume de recorte: a faixa inteira some.
            gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
            return;
        }
    }
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
}
