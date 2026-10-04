# Guia de Deploy e Empacotamento — Petunia3D

Procedimentos de compilação em modo release, empacotamento, distribuição de binários e checklist pré-lançamento.

---

## 1. Requisitos de Release

Antes de gerar qualquer release pública:
1. `cargo test --workspace` passando 100% (23+ testes).
2. `cargo clippy --workspace --all-targets` limpo com 0 warnings.
3. `cargo fmt --all --check` limpo.
4. Gates de documentação: `cargo run -p xtask -- docs-check` e `cargo run -p xtask -- bible-check` sem erros.
5. Versão incrementada no `Cargo.toml` e documentada no `CHANGELOG.md`.

---

## 2. Procedimento de Compilação Otimizada

Para gerar um binário leve, rápido e com símbolos reduzidos:

```bash
# Compilação em modo release
cargo build --release

# Otimização opcional com strip de símbolos de debug
strip target/release/simple3d-modeling
```

---

## 3. Empacotamento e Distribuição

- **Linux**:
  - Binário único autocontido acompanhado da pasta `assets/` (locales, keybinds, themes, tools.toml).
  - Empacotamento em tarball `.tar.gz` ou formato AppImage para portabilidade entre distribuições (Ubuntu, Fedora, Arch).
- **Windows**:
  - Arquivo `.zip` portátil contendo `simple3d-modeling.exe` e a pasta `assets/`.
- **Integridade Criptográfica**:
  - Cada arquivo de release deve ser acompanhado de seu respectivo digest SHA-256 (`sha256sum simple3d-modeling-v0.1.0-linux.tar.gz > SHA256SUMS`).
