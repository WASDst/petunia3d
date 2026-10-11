layout(location = 0) in vec3 aPosition;
layout(location = 1) in vec2 aUv;
uniform mat4 uViewProj;
out vec2 vUv;
void main() {
    gl_Position = to_gl_clip(uViewProj * vec4(aPosition, 1.0));
    vUv = aUv;
}
