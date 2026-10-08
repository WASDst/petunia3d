# Build, distribuição, compatibilidade e release

> **Status: aprovado em 2026-10-08 para política alvo.** Não significa que os instaladores ou a compatibilidade de todos os drivers já estejam validados.

## Plataformas

**Linux e Windows** são plataformas de primeira classe. macOS e outras plataformas não são promessa de V1. GPU baseline: **desktop OpenGL 3.3 Core**; Slint + FemtoVG integram a viewport com FBO/GL texture. Não perpetuar WGPU como segundo renderer obrigatório nem exigir computadores recentes.

O toolkit alvo permanece Slint **sob os gates de resgate**. Egui fica congelado como fallback histórico, não frontend mantido em paridade nem alternativa de release automática.

## Toolchain e builds

A fonte de verdade de versão Rust é **rust-toolchain.toml** (na inspeção desta decisão: 1.98.1). Cargo workspace/lockfile, features e configurações são os manifests reais; páginas antigas com egui+wgpu como produção não são autoridade desta branch.

- Build dev e release separados, com flags, perfis e warnings documentados.
- Não fazer runtime download de compilador, renderer, shaders, icon pack ou dataset para iniciar o editor.
- Compilar/testar em Linux e Windows em CI quando infraestrutura disponível; smoke manual em drivers reais.
- Evitar "fallback" implícito de GPU que mude a representação do documento ou descarte efeitos silenciosamente.
- Dependência nova exige justificativa: maturidade, mantenibilidade, licença, footprint, Windows/Linux e custo de CI.

## Contexto gráfico e falhas

1. Detectar suporte GL 3.3 Core, extensões exigidas e limites de textura antes de abrir projeto pesado.
2. Falta de capability essencial: mensagem legível com driver/versão detectados e ação recomendada, não crash obscuro.
3. Context loss, resize/HiDPI, minimize/restore e múltiplas janelas/viewports têm lifecycle explícito.
4. Caches GPU descartáveis; reinicializar de DocumentSnapshot, sem alterar estado autoral.
5. Shader compile/link e GL debug messages passam por diagnostics redacted/estruturados, sem log excessivo por frame.
6. Hardware de referência low-end faz parte do release gate, não apenas teste de FPS local de desenvolvimento.

## Packaging

- Instalação offline inicial possível, sem conta obrigatória.
- Distribuição por sistema com binários e assets de runtime versionados; licença/créditos de terceiros e notices incluídos.
- Preferências/configuração e cache em paths específicos do SO, sem misturar com Document.
- Diretório de projeto é escolhido pelo usuário; não escrever no install dir.
- Imports/exports locais são explícitos, não sincronização na nuvem por padrão.
- Package de projeto portátil usa paths relativos/embedded e valida dependências.
- Updater automático, marketplace e telemetria não são requisitos V1.

## Compatibilidade de documentos

- Loader com schema versionado e migrations testadas; não fazer save de projeto futuro com perda silenciosa.
- Release registra versões mínimo/máximo do formato .petunia suportadas e estratégias de upgrade.
- Opening legacy document mantém ID/semântica quando possível; backup/cópia de migração antes de mudanças incompatíveis.
- Export de GLB/OBJ segue contratos de Interchange; não confundir "abrir arquivo" com "modelo visualmente idêntico".
- Assets e caminhos portáteis testados entre Windows/Linux, incluindo unicode e separadores.

## Acessibilidade e internacionalização

- Idiomas configuráveis sem recompile onde a arquitetura permitir; strings UI por tokens de tradução.
- IME, text scaling 100–200%, input de teclado, foco, screen reader semantics, high contrast e reduced motion devem passar no build de release.
- Labels de ícones e tooltips não ficam desativados apenas por densidade de tela/tema.
- Modal de erro crítico nunca impede o usuário de salvar backup/recovery quando ainda viável.

## Release gates

1. Build de produção e dependency/supply-chain checks no toolchain fixado.
2. Testes core + Slint frontend + GL integration + Project round-trip.
3. Smoke em Windows e Linux; distribuição inicia em máquina sem ambiente de desenvolvimento.
4. Device matrix: GL version/driver, DPI e hardware low-end; documentar problemas conhecidos.
5. Autorecovery/Save As e projetos legados com migração.
6. Fluxos básicos MODEL→PAINT→UV→ANIMATE→Export validados.
7. A11y hard gates, logs e crash diagnostics reproduzíveis.
8. Changelog, notas de compatibilidade, licenças, checksums e documentação acessíveis.

## Site de documentação versus release

**website/** é um site estático, de leitura sem build. Publicar um preview da branch não aprova release do app. O workflow legado de GitHub Pages **docs.yml** constrói **docs/**, não **website/**, portanto não assumir que editar website/docs publique automaticamente o site público. Publicação é etapa separada, por host configurado, sem merge forçado à main.

## Decisões fechadas

Windows/Linux; GL 3.3; Slint condicionado ao Go/No-Go; toolchain fixado; offline-first; import de arquivos não confiáveis protegido; não exigir conta/telemetria; recursos portáteis; matriz de compatibilidade com evidências antes de release.
