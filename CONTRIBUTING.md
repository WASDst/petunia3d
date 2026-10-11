# Contribuindo com o Petunia3D

Obrigado pelo interesse em contribuir! Este arquivo é o ponto de partida.
O guia técnico completo (regras de código, UI, segurança) vive em
[`docs/developers/contributing.md`](docs/developers/contributing.md) — leia os dois.

## Fluxo rápido

1. Faça um fork e crie um branch a partir de `main`:
   ```bash
   git checkout -b feature/minha-ferramenta   # ou fix/..., docs/...
   ```
2. Implemente com testes (TDD: comportamento novo ou bugfix vem com teste).
3. Rode os portões antes de abrir o PR:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --lib
   cargo run -p xtask -- arch-check
   cargo run -p xtask -- docs-check
   cargo run -p xtask -- ui-lint
   ```
4. Abra o Pull Request contra `main` descrevendo o quê/porquê e citando issues.

## Convenções de commit

Prefixo + escopo, no imperativo e de forma descritiva. Exemplos do histórico:

- `feat(modeling): ...`, `feat(shell): ...`, `feat(primitives): ...`
- `fix(ui/viewport): ...`, `fix(watch): ...`
- `docs(ui-map): ...`, `perf(app): ...`, `ci: ...`, `style: ...`

## Checklist do PR

- [ ] Testes adicionados/atualizados e passando (`cargo test --workspace --lib`).
- [ ] `clippy` e `fmt` limpos (zero warnings; sem `#[allow]` para esconder problema).
- [ ] `CHANGELOG.md` atualizado na seção `[Unreleased]`.
- [ ] Docs atualizadas no mesmo PR (atalhos, parâmetros, ferramentas; contratos canônicos em `website/docs/`).
- [ ] i18n: chaves novas em `assets/locales/en.toml` **e** `pt-BR.toml` (paridade cobrada em CI).
- [ ] UI: sem cores hardcoded (tokens em `crates/ui-slint/ui/tokens.slint`, cobrado pelo `ui-lint`), sem glifos-fontes frágeis, foco/teclado preservados.
- [ ] Sem dependências novas sem justificativa (fronteiras P0/P1 do `arch-check`).
- [ ] Sem segredos, chaves ou dados pessoais no diff.

## Onde encontrar as coisas

- Visão e escopo: `docs/product/vision.md`, `docs/product/scope.md`
- Especificações (fonte da verdade): `docs/bible/` (155 specs P3D)
- Mapa da UI: `docs/developers/ui-component-map.md` + `docs/public/ui-map.json`
- Atalhos: `docs/shortcuts/`, `assets/keybinds/`, `assets/keymaps/`
- Segurança: `docs/security/`

## Reportando bugs

Abra uma issue com: versão/commit, SO e backend (`PETUNIA_BACKEND`), passo a passo
mínimo para reproduzir, comportamento esperado vs obtido e, se possível, o
projeto `.petunia` mínimo que reproduz o problema.

## Código de conduta

Seja respeitoso e objetivo. Críticas ao código, nunca às pessoas. Discussões
técnicas se resolvem com evidência: reproduza, meça e mostre.
