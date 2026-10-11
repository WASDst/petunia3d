// Superfícies: posição, normal de canto, cor e UV já resolvidas.
layout(location = 0) in vec3 aPosition;
layout(location = 1) in vec3 aNormal;
layout(location = 2) in vec3 aColor;
layout(location = 3) in vec2 aUv;
uniform mat4 uViewProj;
uniform mat4 uModel;
uniform mat3 uNormalMatrix;
out vec3 vNormal;
out vec3 vColor;
out vec2 vUv;
void main() {
    gl_Position = to_gl_clip(uViewProj * uModel * vec4(aPosition, 1.0));
    // Normal nula continua nula: marca superfícies unlit.
    vNormal = uNormalMatrix * aNormal;
    vColor = aColor;
    vUv = aUv;
}
