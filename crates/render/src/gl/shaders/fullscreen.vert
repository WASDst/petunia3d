// Triângulo que cobre o viewport (sem atributos).
const vec2 CORNERS[3] = vec2[3](vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
void main() {
    gl_Position = vec4(CORNERS[gl_VertexID], 0.0, 1.0);
}
