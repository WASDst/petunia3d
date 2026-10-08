# Petunia3D — Documentation Website

Site estático para a documentação técnica, decisões arquiteturais e acompanhamento da refatoração do Petunia3D.

A pasta replica deliberadamente a stack, o design e a filosofia do website do novo Petunia Design: **zero build**, dependências pequenas e versionadas por CDN, leitura previsível, acessibilidade e baixo atrito para manutenção.

## Stack

HTML5, CSS nativo, Web Awesome 3.14.0, Phosphor Icons Web 2.1.2, Alpine.js 3.17.4, Marked 18.0.14, Fuse.js 7.5.0 e Markdown.

Não há Node.js, bundler, package manager, node_modules ou etapa de build.

## Filosofia

Este site é o **caderno vivo da refatoração**. Cada domínio só vira decisão quando for discutido e aprovado. Hipóteses permanecem marcadas como propostas.

## Executar localmente

```bash
python3 -m http.server 8080 -d website
```

## Atualização e publicação

Adicione novos arquivos em website/docs e registre cada página em website/docs/manifest.json. O site é zero-build e usa o manifesto para navegação e busca.

**Atenção:** o workflow existente .github/workflows/docs.yml constrói docs/ (site legado VitePress); ele **não publica website/** automaticamente. Alterações nesta branch de refatoração atualizam a fonte do novo caderno, mas publicação de preview/produção depende de host configurado e não autoriza merge na main.

Links relativos entre capítulos Markdown são convertidos pelo cliente em rotas internas, preservando a navegação do site.

