# Petunia3D — Especificação de Produto e Implementação para Code Agents

**Pacote:** PETUNIA-AGENT-SPEC-2026-10-15  
**Idioma:** pt-BR  
**Tipo:** especificação de produto + arquitetura proposta + plano incremental (não é evidência de implementação)  
**Repositório:** https://github.com/WASDst/petunia3d  
**Baseline verificada na elaboração:** `main` @ `92c22caa23f96f85f18e72a5e962a1d8831694db` (2026-10-15)  
**Stack principal verificada:** Rust + Qt 6 Widgets / CXX-Qt + OpenGL 3.3 (`glow`); Qt Quick/QML existe no projeto, mas não substitui o shell Qt Widgets.

> **IMPORTANTE:** Esta especificação consolida decisões do responsável pelo projeto, mas **não declara os recursos implementados**. Valide a `main`, o diff e os contratos canônicos atuais *antes* de implementar. O repositório contém documentos herdados da fase Slint que contradizem o código. A verdade operacional é a árvore de código e os commits recentes; atualizar a documentação canônica é tarefa explícita da fase 0. Não sobrescrever políticas de segurança/permissão do repositório nem executar merges sem autorização explícita.

## Leitura progressiva sugerida ao agente

1. [Instruções e protocolo de execução](chapters/00_AGENT_INSTRUCTIONS.md) — primeiro arquivo para qualquer code agent.
2. [Registro integral de decisões](chapters/01_DECISION_REGISTER.md) — IDs aprovados e invariantes.
3. [Seleção e transformação](chapters/02_SELECTION_TRANSFORM.md) — SEL-001 a SEL-007, interações e transações.
4. [Percepção espacial na viewport](chapters/03_VIEWPORT_PERCEPTION.md) — VSP-001 a VSP-005, Clay, medidas, profundidade e seção.
5. [Feedback de ferramentas de modelagem](chapters/04_MODELING_OPERATION_FEEDBACK.md) — UX e estados de prévia para operações.
6. [Petunia Smart Rig](chapters/05_SMART_RIG.md) — rig inspirado em RigMesh; automático assistido, live e manual.
7. [Arquitetura Qt/Rust/OpenGL](chapters/06_ARCHITECTURE_INTEGRATION.md) — mapa de integração real com o código.
8. [Acessibilidade e design de interação](chapters/07_ACCESSIBILITY_UX.md).
9. [Roadmap e tarefas para agentes](chapters/08_IMPLEMENTATION_ROADMAP.md).
10. [Matriz de testes e evidências](chapters/09_ACCEPTANCE_TEST_MATRIX.md).
11. [Referências, proveniência e glossário](chapters/10_REFERENCES_GLOSSARY.md).

## Diagramas SVG originais

| Arquivo | Assunto |
|---|---|
| [system-architecture.svg](assets/system-architecture.svg) | Bounded contexts, Rust, Qt e renderização |
| [gizmo-and-selection.svg](assets/gizmo-and-selection.svg) | Gizmo universal, picking, seleção e pivô |
| [viewport-spatial-feedback.svg](assets/viewport-spatial-feedback.svg) | Clay, guias dimensionais e Companion View |
| [section-and-thickness.svg](assets/section-and-thickness.svg) | Seção 3D, seção 2D, espessura e perfis |
| [operation-lifecycle.svg](assets/operation-lifecycle.svg) | Hover → prévia → validação → commit/cancel |
| [smart-rig-pipeline.svg](assets/smart-rig-pipeline.svg) | Shape Graph, skeleton, pesos, pose e overrides |

Os SVGs são **diagramas originais**, não recursos extraídos de softwares de terceiros. Os fluxogramas Mermaid estão embutidos nos arquivos Markdown.

## Regras de interpretação

- **APROVADO:** decisão de produto feita durante as conversas.
- **DERIVADO:** decisão complementar delegada expressamente pelo responsável e coerente com as aprovações.
- **PROPOSTO:** desenho técnico ainda dependente de confronto com código/teste de UX.
- **OBSERVADO:** arquivo/componente encontrado na baseline GitHub, sem presumir paridade runtime.
- **VERIFICADO:** funcionalidade comprovada por gate executado com evidência; este pacote não declara novos recursos verificados.

A prioridade em conflito é: **segurança/dados + instrução humana explícita vigente → código atual + contratos canônicos reconciliados → decisões aprovadas deste pacote → recomendações derivadas/propostas**. Se houver conflito entre *novas* decisões aprovadas e documentação histórica, abrir a reconciliação documental antes da implementação.

## Escopo do pacote

Cobre seleção contextual e heterogênea, gizmo Move/Rotate/Scale, pivôs, snapping, dragging protegido, UX de parâmetros; Petunia Clay e percepção da forma; Volume Frame, referenciais e dimensões; Depth Assist e Companion View; Section Assist, thickness e perfis orgânicos; feedbacks de modelagem; rigging incremental inspirado em RigMesh, opções manuais, IK e pintura de pesos; arquitetura de componentes, protocolos de comando e renderização; acessibilidade; testes; roadmap; riscos e migração documental Qt.

**Fora do escopo de implementação imediata:** criar o produto inteiro em um único PR, troca do renderer, troca do toolkit, serviço cloud de IA obrigatório, rig neuronal obrigatório, refazer os módulos de pintura/UV, criar ECS especulativo, publicar ou efetuar merge no GitHub sem autorização.
