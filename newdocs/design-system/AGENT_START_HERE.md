# Petunia3D UI Design System — Instructions for code agent

Task: implement the design system **incrementally** based on `PETUNIA_UI_001_DESIGN_SYSTEM.md` after checking the latest `WASDst/petunia3d` main branch. Do not treat decisions as existing implementations. **Do not replace the frontend**. Existing stack: Rust + Qt 6 Widgets + CXX/CXX-Qt + QOpenGLWidget/OpenGL.

## First request to perform

1. Inspect `AGENTS.md`, theme/i18n/preferences modules, `crates/ui-cxxqt` Qt C++ widgets, editor bridge, `build.rs`, all GUI tests, existing SVG assets and dock manager. Identify stale references to Slint and conflicting duplicated QSS/theme sources.
2. Produce an **inventory** and proposed code-level migration plan (actual files/classes, owners, dependency ordering), with existing behavior preserved. Show baseline native screenshots from all five workspaces and Preferences if executable in environment.
3. Implement or outline a component gallery with P0 components and semantic theme snapshot, without mutating the document through presentation controls.
4. Migrate exactly one vertical slice first: Inspector Transform + NumberField/Vector3Input + Undo/Redo and mixed selection, then run tests/screenshots. Only continue to other panels when this slice works.
5. Before introducing Qlementine, Qt Advanced Docking System, KDDockWidgets, a breaking theme schema, or large workspace layout behavior change, explicitly report the tradeoffs and request approval.

## Acceptance gates
- Component library has contract, stable IDs, localization and accessibility semantics, no arbitrary inline color literals.
- User theme IDs & TOML preferences backwards compatible.
- Ui command paths and selection/Undo behavior unchanged.
- Dark/light/high contrast, pt-BR/en/pseudo-long and HiDPI tested.
- Tests show accessible, responsive UI with zero clipped primary labels.
- Layouts re-open safely and can be restored without loss of project state.

Read the full spec and original SVG architectural diagrams before editing. Record deviations with reasons. Report manual UX tests not run as outstanding, rather than claiming completion.
