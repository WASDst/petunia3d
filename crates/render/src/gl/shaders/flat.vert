// Preenchimentos translúcidos do overlay (faces selecionadas, workplane).
layout(location = 0) in vec3 aPosition;
layout(location = 1) in vec4 aColor;
uniform mat4 uViewProj;
out vec4 vColor;
void main() {
    vec4 clip = uViewProj * vec4(aPosition, 1.0);
    gl_Position = to_gl_clip(vec4(clip.xy, clip.z - 0.00005 * clip.w, clip.w));
    vColor = aColor;
}
