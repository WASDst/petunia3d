# P3D-139 — Animation Asset Library

<aside>
🧩

Novo item · Biblioteca reutilizável de clips · Prioridade: P3.

</aside>

## Objetivo

Salvar/importar clips uma vez e reutilizá-los sem retornar continuamente a sites externos.

## UX

Grid/list, search, tags, preview, rig compatibility e Apply/Retarget. Reutilizar padrões da Project Model Library sem duplicar backend desnecessariamente.

## Modelo

AnimationAssetId, source/format, clip metadata, duration, tags, compatibility/retarget profile e preview cache.

## Dependências

P3D-003, P3D-138.

## Testes / DoD

Import, tag/search, missing file/relink, preview, apply compatível/incompatível e metadata persistida.

## Extensão pós-V1 (2026-09-30)

A biblioteca também guarda **Motion Recipes** ([P3D-170](p3d-170-procedural-motion-generators.md)) — vivos, com parâmetros e Style — além de clipes com keys. Grade com prévia ao vivo é a porta de entrada da camada 1 do Animate (cap. 45).
