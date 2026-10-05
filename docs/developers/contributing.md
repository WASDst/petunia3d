# Guia de Contribuição

Agradecemos o interesse em contribuir com o **Petunia3D**! Nosso projeto segue regras técnicas rigorosas para garantir a qualidade, segurança e sustentabilidade do código.

---

## Regras de Ouro para Contribuidores

1. **Test-Driven Development**: Todo novo comportamento ou correção de bug deve vir acompanhado de testes automatizados exaustivos.
2. **Zero Hardcoded Colors**: Nunca insira códigos hexadecimais de cor diretamente nos componentes de UI. Utilize sempre os tokens canônicos definidos em `crates/ui/src/tokens.rs`.
3. **Clippy Estrito**: O código deve compilar limpo sem nenhum aviso:
   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```
4. **Formatação Padronizada**:
   ```bash
   cargo fmt --all -- --check
   ```
5. **Documentação Contínua**: Se você alterar atalhos, parâmetros ou adicionar uma ferramenta, a documentação correspondente em `docs/` e o `CHANGELOG.md` devem ser atualizados no mesmo Pull Request.

---

## Fluxo de Trabalho de Pull Request

1. Faça um Fork do repositório no GitHub;
2. Crie um branch para sua funcionalidade: `git checkout -b feature/minha-ferramenta`;
3. Desenvolva sua implementação com testes associados;
4. Valide a conformidade executando:
   ```bash
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   cargo fmt --all -- --check
   cargo run -p xtask -- docs-check
   ```
5. Abra o Pull Request detalhando as alterações e citando as issues relacionadas.
