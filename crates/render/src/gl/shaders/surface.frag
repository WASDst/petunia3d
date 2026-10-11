// Iluminação de estúdio/cena, matcap procedural em espaço de vista, oclusão
// ambiente amostrada do passe de AO, orientação de faces e checker de UV.
in vec3 vNormal;
in vec3 vColor;
in vec2 vUv;
out vec4 oColor;

uniform float uOpacity;
uniform vec3 uLightDir;
uniform float uAmbient;
uniform float uDiffuse;
uniform bool uMatcap;
uniform vec3 uViewRight;
uniform vec3 uViewUp;
uniform bool uTextured;
uniform sampler2D uTexture;
uniform bool uAmbientOcclusion;
uniform sampler2D uAoTexture;
// x, y = origem do viewport no framebuffer; z, w = tamanho (px físicos).
uniform vec4 uViewportRect;
uniform bool uFaceOrientation;
uniform bool uUvChecker;

const vec3 FRONT_FACE = vec3(0.2, 0.45, 0.95);
const vec3 BACK_FACE = vec3(0.95, 0.2, 0.2);

vec3 uv_checker(vec2 uv) {
    int u_cell = int(floor(uv.x * 16.0));
    int v_cell = int(floor(uv.y * 16.0));
    bool even = ((u_cell + v_cell) & 1) == 0;
    bool u_macro = (u_cell & 7) == 0;
    bool v_macro = (v_cell & 7) == 0;
    if (even) {
        if (u_macro) return vec3(0.95, 0.55, 0.45);
        if (v_macro) return vec3(0.45, 0.75, 0.95);
        return vec3(0.85);
    }
    if (u_macro) return vec3(0.45, 0.20, 0.18);
    if (v_macro) return vec3(0.18, 0.28, 0.45);
    return vec3(0.25);
}

float occlusion() {
    if (!uAmbientOcclusion) {
        return 1.0;
    }
    vec2 uv = (gl_FragCoord.xy - uViewportRect.xy) / uViewportRect.zw;
    return texture(uAoTexture, uv).r;
}

vec3 lit(vec3 base, vec3 normal) {
    float ao = occlusion();
    if (length(normal) < 0.1) {
        return base * ao;
    }
    vec3 n = normalize(normal);
    if (uMatcap) {
        vec3 right = uViewRight;
        vec3 up = uViewUp;
        vec3 vn = vec3(dot(n, right), dot(n, up), dot(n, cross(right, up)));
        vec3 key_dir = normalize(vec3(-0.45, 0.6, 0.66));
        float key = max(dot(vn, key_dir), 0.0);
        float sky = 0.5 + 0.5 * vn.y;
        float rim = pow(1.0 - clamp(abs(vn.z), 0.0, 1.0), 3.0);
        float spec = pow(max(dot(reflect(vec3(0.0, 0.0, -1.0), vn), key_dir), 0.0), 32.0);
        return (base * (0.16 + 0.58 * key + 0.24 * sky) + vec3(rim * 0.16 + spec * 0.22)) * ao;
    }
    float diffuse = max(dot(n, normalize(uLightDir)), 0.0);
    return base * (uAmbient * ao + uDiffuse * diffuse * mix(1.0, ao, 0.5));
}

void main() {
    vec3 base = vColor;
    if (uFaceOrientation) {
        base = gl_FrontFacing ? FRONT_FACE : BACK_FACE;
    } else if (uUvChecker) {
        base = uv_checker(vUv);
    } else if (uTextured) {
        base *= texture(uTexture, vUv).rgb;
    }
    oColor = vec4(lit(base, vNormal), uOpacity);
}
