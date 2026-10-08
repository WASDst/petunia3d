# ADR 009 — Slint como frontend ativo e egui congelado como fallback

**Status:** APPROVED — 2026-10-08  
**Escopo:** frontend desktop do Petunia3D, UI/UX, acessibilidade e estratégia de contingência.  
**Autoridade relacionada:** [cap. 23](../../bible/foundations/23-macroarquitetura-interface.md), [cap. 24](../../bible/foundations/24-design-system-tokens-estados.md), [cap. 25](../../bible/foundations/25-biblioteca-componentes-interacao.md), [cap. 36](../../bible/foundations/36-ui-baseline-temas-plugin-panels.md) e [plano de refatoração Slint](../../development/ui-ux-refactor-plan-2026-10-04.md).

## Contexto

O Petunia3D já possui investimento substancial no frontend Slint e o shell atual está funcional o suficiente para justificar uma rodada adicional de consolidação em vez de outra migração de toolkit. A análise de 2026-10-08 concluiu que os principais problemas observados são predominantemente de **execução de UI/UX, acessibilidade, hierarquia visual e integração**, não uma prova de inviabilidade estrutural do Slint.

A alternativa egui continua tecnicamente útil como referência histórica e opção de recuperação, mas desenvolver dois frontends em paralelo aumenta custo, divergência, testes duplicados e risco de regras de produto vazarem para a camada visual.

## Decisão

### 1. Slint continua sendo o frontend de produção

O Petunia3D continua investindo em `petunia_ui_slint` (`crates/ui-slint/`) como **única superfície ativa de produto**. Não haverá reescrita de GUI neste momento.

A estratégia é incremental: preservar o que já funciona, corrigir o delta comprovado e melhorar a interface atual por componentes, tokens, view-models, intents e testes. Reescrita completa só volta à mesa se houver bloqueador estrutural medido.

### 2. egui deixa de ser frontend de transição ativo

egui passa a ser **fallback congelado**, não uma segunda implementação mantida em paridade.

Política aprovada:

- não implementar novas features de produto em egui;
- não corrigir diferenças cosméticas para acompanhar Slint;
- preservar uma versão conhecida e recuperável em branch/tag dedicada fora da linha ativa de desenvolvimento do Slint;
- manter na documentação o procedimento e os critérios para reavaliação;
- enquanto a extração física da crate egui da branch ativa ainda não tiver sido concluída, sua presença no workspace e os flags `--legacy-egui` / `PETUNIA_LEGACY_EGUI=1` são **compatibilidade temporária**, não autorização para evolução paralela.

O fallback não é rollback automático: falha em um detalhe de UI exige correção do Slint antes de considerar troca de toolkit.

### 3. Fronteira arquitetural obrigatória

Slint apresenta estado e emite intenção; não contém regra de negócio.

```text
Slint callback
→ UiIntent
→ validação / CommandId
→ comando/transação no core
→ AppEvent / revision / query DTO
→ ShellViewModel
→ Slint + adapter de viewport
```

`petunia_core`, `petunia_commands`, `petunia_project`, `petunia_config` e o renderer não podem depender de Slint nem de egui. Essa separação mantém a possibilidade de trocar a UI no futuro sem reescrever domínio, ferramentas, documento ou comandos.

### 4. Direção de UI/UX aprovada para a consolidação do Slint

A rodada de melhoria deve priorizar:

- **viewport-first**, preservando aproximadamente `480 × 360` logical px antes de ceder espaço a painéis;
- reduzir ruído e controles permanentes; ações raras vão para menu, popover, contexto ou Command Palette;
- uma hierarquia visual clara, com accent reservado para seleção, foco e estado realmente ativo;
- textos legíveis, sem truncamento como solução de layout;
- ícones ambíguos acompanhados de rótulo ou tooltip explicativo;
- Inspector contextual, rolagem real, listas virtualizadas quando necessário e persistência por workspace;
- drawers/painéis com estado explícito de aberto, recolhido, pin e foco; nenhum docking irrestrito é reintroduzido;
- componentes reutilizáveis e design tokens como linguagem visual única;
- nenhum enum, id técnico, string de debug ou detalhe de implementação exposto ao usuário;
- estados disabled devem explicar por que a ação está indisponível e, quando possível, o que fazer para habilitá-la.

A interface deve continuar profissional, simples e convidativa, sem virar um clone reduzido do Blender e sem esconder complexidade essencial atrás de comportamento mágico.

### 5. Acessibilidade é gate, não acabamento

A consolidação do Slint só é considerada bem-sucedida se cobrir:

- navegação completa por teclado e ordem de foco previsível;
- foco visível e recuperação de foco após fechar popovers/modais;
- nomes, roles, estados e descrições semânticas para tecnologias assistivas;
- atalhos resolvidos pelo keymap, nunca hardcoded no markup;
- Dark e High Contrast como superfícies verificadas;
- UI scaling `100 / 125 / 150 / 175 / 200%` e HiDPI;
- reduced-motion;
- hit targets adequados e alternativas a interações dependentes apenas de precisão do ponteiro;
- tooltips ricos, hints e mensagens de erro/disabled compreensíveis;
- validação de screen reader, teclado, foco, IME e input real nas plataformas suportadas sempre que o toolkit/OS permitir.

Limitações do toolkit devem ser registradas como gaps concretos, com reprodução e impacto, nunca tratadas apenas por impressão subjetiva.

### 6. Renderização e viewport

O backend do shell Slint (incluindo FemtoVG/OpenGL quando aplicável) não é motivo isolado para abandonar o toolkit. O viewport 3D continua atrás de um adapter toolkit-neutro e do renderer próprio do Petunia3D.

A integração GPU deve ser preferida quando puder evitar cópias e stalls, mas o estado atual não deve ser descrito como zero-copy sem evidência. Caminhos de readback, fallback de software, resize, aspect ratio, shading e wireframe precisam ser medidos e testados.

Performance deve ser comprovada em hardware modesto com métricas de frame time/FPS, latência p95, memória, abertura/save e operações relevantes; não se aprovam números inventados apenas para cumprir meta.

### 7. Gates objetivos para continuar com Slint

Slint permanece aprovado enquanto a consolidação demonstrar progresso nos seguintes gates:

1. fluxos centrais completos de criação/modelagem, seleção, transformação, Paint/UV quando aplicável, save/reopen e export;
2. Undo/Redo e transações corretos nos fluxos cobertos;
3. paridade suficiente com os contratos de produto, não necessariamente paridade pixel-a-pixel com o antigo egui;
4. arquivos protegidos contra perda/corrupção nos fluxos principais;
5. viewport utilizável e responsivo nos backends suportados;
6. conformance de teclado, foco, acessibilidade, scaling, dark/high-contrast e input;
7. testes de resultado/E2E e regressão visual suficientes para detectar quebra real;
8. manutenção do desacoplamento entre domínio, renderer e toolkit.

### 8. Quando reabrir a discussão de toolkit

A troca de toolkit só pode ser reaberta se, após tentativa incremental documentada, um ou mais gates acima continuarem bloqueados por **limitação estrutural reproduzível do Slint** e não por código incompleto do Petunia3D.

Exemplos válidos: acessibilidade essencial impossível de entregar, integração de input/IME inviável para plataformas-alvo, performance de viewport/interface fora do orçamento mesmo após profiling e correções, ou custo de manutenção comprovadamente desproporcional por limitação do toolkit.

Nesse caso, o snapshot egui congelado serve como referência/fallback para acelerar comparação e recuperação. A decisão de voltar ao egui ou escolher outro toolkit exige novo ADR; não acontece automaticamente.

## Ordem de implementação

1. confiabilidade, erros, dirty-state, save/reopen e transações;
2. manter o runtime compartilhado e remover regras dos callbacks;
3. fechar um vertical slice de modelagem com Undo/Redo;
4. estabilizar viewport GPU/software e medir gargalos;
5. fechar o caminho shape-first e os fluxos centrais;
6. completar Paint/UV conforme o roadmap;
7. consolidar design system, acessibilidade e polish;
8. executar matriz de conformance e só então decidir se os gates foram atendidos.

## Consequências

- preservamos o investimento já feito em Slint;
- evitamos custo permanente de dois frontends ativos;
- egui continua disponível como seguro técnico sem contaminar a evolução diária;
- a arquitetura toolkit-neutra passa a ser também requisito de recuperabilidade;
- problemas de UI devem gerar issues/gaps objetivos e testes, não migrações impulsivas de stack;
- qualquer documentação que descreva egui como frontend paralelo, alternativa mantida em paridade ou destino de novas features fica obsoleta por esta decisão.
