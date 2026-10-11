// Funções comuns aos passes de AO e de blur.
uniform sampler2D uDepth;
uniform mat4 uInvProj;
// x = proj[1][1], y = ortográfica (1/0), z = raio em mundo, w = intensidade.
uniform vec4 uParams;
// xy = tamanho do depth (px físicos), zw = tamanho do AO.
uniform vec4 uSize;
// x = raio máximo em px físicos do depth.
uniform vec4 uLimits;

float depth_at(vec2 px) {
    ivec2 dims = ivec2(uSize.xy);
    ivec2 p = clamp(ivec2(px), ivec2(0), dims - ivec2(1));
    return texelFetch(uDepth, p, 0).r;
}

// Origem embaixo-esquerda (GL); a profundidade já está em [0, 1] (glam).
vec3 view_pos(vec2 px, float depth) {
    vec2 uv = px / uSize.xy;
    vec4 v = uInvProj * vec4(uv.x * 2.0 - 1.0, uv.y * 2.0 - 1.0, depth, 1.0);
    return v.xyz / v.w;
}
