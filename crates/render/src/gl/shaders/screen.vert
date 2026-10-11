// Overlay de tela (gizmo) em px físicos a partir do topo-esquerdo.
layout(location = 0) in vec2 aPosition;
layout(location = 1) in vec4 aColor;
uniform vec2 uViewportSize;
out vec4 vColor;
void main() {
    vec2 size = max(uViewportSize, vec2(1.0));
    gl_Position = vec4(aPosition.x / size.x * 2.0 - 1.0, 1.0 - aPosition.y / size.y * 2.0, 0.0, 1.0);
    vColor = aColor;
}
