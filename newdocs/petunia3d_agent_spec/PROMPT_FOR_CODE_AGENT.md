# Prompt de inicialização para o code agent — Petunia3D

Você é responsável por implementar o Petunia3D de forma incremental, mantendo qualidade, estabilidade, acessibilidade e compatibilidade. **Leia `README.md` e `chapters/00_AGENT_INSTRUCTIONS.md` deste pacote antes de qualquer modificação**; leia o registro integral `chapters/01_DECISION_REGISTER.md` e os capítulos técnicos relacionados à tarefa específica. Os SVG em `assets/` são diagramas originais, não screenshots nem implementação existente.

O repositório é `WASDst/petunia3d`. A baseline desta documentação foi `main` @ `92c22caa23f96f85f18e72a5e962a1d8831694db` em 2026-10-15; **confirme o SHA atual**. A stack atual observada é Rust + Qt 6 Widgets / CXX-Qt + OpenGL 3.3. O frontend Slint foi removido do workspace na migração de outubro de 2026, embora alguns docs e nomes do código permaneçam legados.

## Sua primeira tarefa

1. Confirme branch/HEAD/dirty tree e leia `AGENTS.md`, `website/docs/manifest.json`, `website/docs/00-philosophy/decision-register.md`, `website/docs/16-code-agents/reading-navigation.md`, `implementation-protocol.md` e documentação canônica das áreas afetadas.
2. Confronte a stack atual com instruções antigas Slint e relatório de migração Qt. **Não ignore regras de segurança e permissões; reconcilie documentação conflitante com decisões humanas aprovadas.**
3. Produza uma **Gap Matrix** por módulo (SEL, VSP, feedback de modelagem, Smart Rig) com estado `existing verified / existing unverified / partial / missing / blocked`, incluindo código, testes, status CI e riscos.
4. Proponha **vertical slices pequenos** e dependências, priorizando fase 0 documental/CI, seleção transacional, gizmo, Clay/Volume, Companion, Section e Rig Editor manual antes de Live Auto-Rig.
5. Implemente **apenas o slice autorizado**. Adote `Reuse → Refactor → Move → Rewrite` (último recurso). Não troque a stack, não faça big-bang rewrite, não altere `main`/faça push/merge sem autorização explícita.
6. Toda mudança: comando validado + transação + preview/cancel/Undo, versão/revisão estável, testes de domínio + bridge + Qt visual/a11y quando pertinente, persistência/compatibilidade, evidência com comandos e SHA.
7. Toda falha/aviso/a11y deve ser relatado. Nunca declarar gate passado sem rodar. Não publicar funcionalidades imaginadas como implementadas.

## Restrições de UX invioláveis

- SEL-004: dragging diretamente sobre objeto não move geometria; apenas manipulator/comando explícito.
- SEL-005: snap contextual equilibrado; ordinary Move/Rotate/Scale suaves por padrão.
- SEL-006/007: seleção heterogênea, gizmo compartilhado com Individual Origins, visual compacto adaptativo e toolbar separada.
- VSP-001–005: Petunia Clay, Volume Frame compacto, Depth Assist/Companion sugerida, Sections/Thickness/Profiles com inspeção não destrutiva e `Edit Profile` explícito para shapes compatíveis.
- RIG-001–012: autorig opcional; assisted como padrão do fluxo; live opt-in; manual full; overrides protegidos; desligar auto não apaga bones; rig evolui incrementalmente quando seguro.
- Acessibilidade: teclado, reader semantic model, contraste, redução de movimento, viewport sem sobrecarga, Windows/Linux e low-end.

## Resposta esperada

Entregue roadmap de tarefas e matriz de gaps antes de código massivo. Ao finalizar cada slice, liste arquivos alterados, testes realmente executados (`pass/fail/blocked/not run`), resultados, UX validada e pendências. Preserve o trabalho preexistente de outros agentes.
