# P3D-095 — Blender-like

<aside>
🧩

Perfil de familiaridade, não cópia cega · Prioridade: P1.

</aside>

## Objetivo

Aproximar muscle memory de Blender onde isso não conflita com a filosofia simplificada do Petunia.

## Direção

G/R/S, E, I, Ctrl+B, Ctrl+R, Shift+D, X/Delete, A e vistas por numpad quando disponível. Commands inexistentes no Petunia não são inventados só para completar o perfil.

Neste perfil, G/R/S e E/I/Ctrl+B iniciam o estado `Latched` da gramática única (constituição 11): a operação segue o mouse e o clique confirma, como no Blender. É o mesmo código das ferramentas persistentes, não um segundo ciclo. RMB continua abrindo menu de contexto; cancelar é Esc.

## Dependências

P3D-090–091, P3D-015.

## Testes / DoD

Conflicts, modal transforms via estado `Latched`, Object/Face/Edge/Point unificado e labels “Blender-like” na UI/documentação.