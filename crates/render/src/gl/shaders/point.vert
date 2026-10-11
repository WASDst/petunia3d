// Marcador de ponto: quad orientado à tela com raio em px lógicos.
layout(location = 0) in vec3 aCenter;
layout(location = 1) in vec4 aColor;
// xy = canto em [-1, 1], z = raio em px lógicos.
layout(location = 2) in vec3 aCorner;
uniform mat4 uViewProj;
uniform vec2 uViewportSize;
uniform float uPxScale;
out vec4 vColor;
out vec2 vLocal;
out float vRadius;
void main() {
    vec4 c = uViewProj * vec4(aCenter, 1.0);
    float radius = aCorner.z * uPxScale;
    // Meio px de margem para a borda suavizada.
    float reach = radius + 1.0;
    vec2 offset = aCorner.xy * reach / (max(uViewportSize, vec2(1.0)) * 0.5);
    gl_Position = to_gl_clip(vec4(c.xy + offset * c.w, c.z - 0.0002 * c.w, c.w));
    vColor = aColor;
    vLocal = aCorner.xy * reach;
    vRadius = radius;
}
