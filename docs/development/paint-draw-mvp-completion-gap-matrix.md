# PAINT/DRAW — reconciliação do MVP (02/10/2026)

Escopo autorizado: corrigir booleanos; Shape Builder poligonal (curvas fiéis em V1); SVG decal/perfil, Projection/Stencil e Path Paint no MVP; completar adicionais aprovados. Preservar aparência da UI. Gates finais aguardam instrução do responsável.

| Requisito | Estado observado antes da retomada | Delta necessário |
| --- | --- | --- |
| Boolean cleanup | PARTIALLY_COMPLIANT | Auditar UV/material/cor de pontos novos |
| Shape Builder / Pathfinder | BROKEN | Persistir furos; Delete e XOR não podem recriar vazios; preservar formas não afetadas |
| SVG decal/perfil | PARTIALLY_COMPLIANT (worktree isolado) | Recuperar código, migração e bridge Slint |
| Projection/Stencil | RUDIMENTARY (rascunho isolado) | Revisar, integrar preview, transformação, restrições, commit/Undo |
| Path Paint | PARTIALLY_COMPLIANT (núcleo isolado) | Integrar ferramenta, preview, attachment e Undo |
| Alpha lock; linha; pixel perfect; rampas/dithering | MISSING | Motor e controles funcionais |
| UV health; export bleed | PARTIALLY_COMPLIANT | Expor diagnósticos existentes; padding configurável de exportação |
| Round corners; mirror; presets; dimensões; simplify; trace | MISSING | Operações DRAW e entradas funcionais |
| Documentação MVP | OBSOLETE | Reconciliar decisão com P3D-165 e matrizes |

Evidência da inspeção: HEAD inicial 4a41951; SVG 0e1c83e e Path Paint 8645a8f em worktrees separados; Projection sem commit. Nenhum gate executado nesta retomada.
