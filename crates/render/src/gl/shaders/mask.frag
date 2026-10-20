// Máscara do contorno: RGB = cor do contorno do objeto, A = coberto.
uniform vec4 uColor;
out vec4 oColor;
void main() {
    oColor = vec4(uColor.rgb, 1.0);
}
