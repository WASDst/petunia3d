// Prefixo de todos os shaders do viewport (GLSL 3.30 core).
//
// A câmera usa a convenção do glam (`*_rh`): profundidade de clip em [0, 1].
// O OpenGL recorta em [-1, 1]; `to_gl_clip` remapeia z para que o plano
// near recorte de fato e o depth buffer guarde o mesmo valor em [0, 1] que a
// projeção produz (os passes de AO reconstroem a posição com a inversa dela).
vec4 to_gl_clip(vec4 clip) {
    return vec4(clip.xy, clip.z * 2.0 - clip.w, clip.w);
}
