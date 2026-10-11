// Só a posição: máscara do contorno e pré-passe de profundidade do AO.
layout(location = 0) in vec3 aPosition;
uniform mat4 uViewProj;
uniform mat4 uModel;
void main() {
    gl_Position = to_gl_clip(uViewProj * uModel * vec4(aPosition, 1.0));
}
