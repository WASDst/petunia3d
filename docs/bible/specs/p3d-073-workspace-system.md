# P3D-073 — Workspace System

<aside>
🧩

Estado: **parcial; precisa adaptação contextual** · Prioridade: P1.

</aside>

## Objetivo

Draw, Poly, Paint, UV e Animation como composições contextuais de painéis/tools, sem duplicar estado do projeto. DRAW e POLY substituem Model desde 2026-09-29 ([ADR 006](../../architecture/adr/006-workspaces-draw-poly-e-gramatica-unica.md)); até estarem funcionais, a UI mantém Model.

## Regras

- workspace troca layout/contexto, não cria cópia de scene/material/selection;
- painéis podem ser personalizados, redimensionados e ocultados;
- layout inicial continua simples e previsível;
- Export não vira workspace permanente sem uma necessidade real;
- DRAW e POLY compartilham documento, seleção de objeto, câmera, snapping, gramática de ferramenta e Inspector; mudam o trilho de ferramentas, o conjunto de seleção e a aparência do viewport;
- a passagem de forma (DRAW) para malha editável (POLY) é explícita e reversível por Undo.

## Dependências

P3D-078, P3D-079, P3D-083.

## Testes / DoD

Troca repetida sem perder seleção/estado (inclusive DRAW ↔ POLY), layout persistido como UI settings e funcionamento em 1366×768.