in vec2 vUv;
out vec4 oColor;
uniform sampler2D uTexture;
uniform float uOpacity;
void main() {
    vec4 c = texture(uTexture, vUv);
    oColor = vec4(c.rgb, c.a * uOpacity);
}
