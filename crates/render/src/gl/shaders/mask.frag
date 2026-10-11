// Máscara do contorno: R = selecionado, G = ativo.
uniform vec2 uValue;
out vec4 oColor;
void main() {
    oColor = vec4(uValue, 0.0, 1.0);
}
