# Petunia3D Qt adapter

Frontend experimental sobre os mesmos crates funcionais do editor. O Qt não é a autoridade de Mesh/Project nem uma cópia do motor. Canonical: `website/docs/09-ui/qt-integration.md` (Q05).

Desenvolvimento: worktree `petunia3d-qt-integration`, branch `codex/qt-integration`, no repositório `petunia3d-refact`. A pasta `petunia3d-cxxqt` anterior é origem preservada.

```bash
cargo run -p petunia_ui_cxxqt --example cxxqt_widgets_studio
```

Requer Qt 6 (Widgets, OpenGL, OpenGLWidgets, Quick), C++ e Rust. A entrada padrão continua Slint. Build/test não certifica paridade visual/funcional, caneta, Windows, a11y nem performance. O Qt Quick preview é separado do Studio Widgets.

Antes de sincronizar features, preservar mudanças locais, confirmar branch/status e integrar os commits funcionais pelo Git. Não copiar implementações antigas de Core/Geometry/Project para resolver erros do adapter.
