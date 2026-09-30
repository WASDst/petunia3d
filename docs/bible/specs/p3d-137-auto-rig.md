# P3D-137 — Auto-Rig

<aside>
🧩

Novo item; implementar somente após Rig Core/Presets · Prioridade: P3. **Estado (2026-09-30, F2b do [cap. 45](../foundations/45-pos-v1-animate-acessivel-animacao-procedural.md)):** existe um primeiro **Fit to model** genérico (qualquer preset ao volume do modelo, pesos por osso mais próximo com raio de mistura), editável e em um passo de Undo; landmarks, correção guiada e pintura de pesos seguem pendentes.

</aside>

## Objetivo

Assistir criação/posicionamento de rig sem virar caixa-preta.

## Regras

- resultado sempre editável;
- indicar falhas/ambiguidade;
- permitir correção manual antes de bind;
- não ocultar limitações de topologia/pose.

## Estratégia

Começar por fitting simples de presets com landmarks/manual hints; técnicas mais automáticas só após benchmark de qualidade.

## Dependências

P3D-135, P3D-136.

## Testes / DoD

Modelos humanos/quadrúpedes simples, proporções diferentes, falha segura, undo e comparação com rig manual.