# P3D-138 — Animation Retargeting / External Compatibility

<aside>
🧩

Novo item; Mixamo é workflow de referência, não dependência proprietária · Prioridade: P3.

</aside>

## Objetivo

Importar/retargetear animações externas para rigs Petunia suportados.

## Contrato

Retarget profiles mapeiam bone semantics/naming e transform conventions. Formatos são tratados por importers; não integrar ao site Mixamo como requisito do core.

## Compatibilidade

Documentar claramente rigs/formats suportados, root motion e limitações.

## Dependências

P3D-135, P3D-136, P3D-071.

## Testes / DoD

Clips externos representativos, mapping incompleto, scaling/orientation, playback comparativo e erros compreensíveis.

## Extensão pós-V1 (2026-09-30)

O mapeamento de retarget passa a usar **Rig Roles** ([P3D-169](p3d-169-rig-roles-ik-foundation.md)) como vocabulário comum, em vez de depender de nomes de osso. Importar e exportar **skin e animação em glTF** é pré-requisito de "game-ready" e faz parte da fase F0 do [cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md); o teste de round-trip pertence a P3D-124.
