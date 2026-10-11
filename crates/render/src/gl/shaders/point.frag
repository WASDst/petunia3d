in vec4 vColor;
in vec2 vLocal;
in float vRadius;
out vec4 oColor;
void main() {
    float alpha = clamp(vRadius + 0.5 - length(vLocal), 0.0, 1.0);
    if (alpha <= 0.0) {
        discard;
    }
    oColor = vec4(vColor.rgb, vColor.a * alpha);
}
