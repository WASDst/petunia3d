# P3D-013 — Imagens de referência

<aside>
🧩

Estado inicial: **implementado de forma rudimentar / UX inadequada** · Prioridade: P0.

</aside>

## Objetivo

Transformar a importação de referências em um workflow organizado e previsível, não em seleção isolada de uma imagem.

## Decisão de produto

Criar **Reference Set Manager** como modal/utility window. Slots independentes: Front, Back, Left, Right, Top e Bottom. Nenhum slot é obrigatório.

## Propriedades por referência

Arquivo, visibility, opacity, lock, position, rotation, scale e offset, além do vínculo semântico com a vista correspondente.

## Arquitetura

ReferenceSet/ReferenceImage são dados de projeto com IDs estáveis. File picker e layout do modal ficam no frontend. O renderer recebe descriptors neutros e cacheia texturas.

## UX

Preview e edição devem ser simples, com reset por campo/vista e possibilidade de substituir/remover imagem sem recriar o set inteiro.

## Dependências

P3D-014, P3D-006, P3D-125.

## Testes / DoD

Load/remove/replace, formatos inválidos, arquivo ausente, persistência, transforms independentes e comportamento após mover o projeto.

## Extensão pós-V1 (2026-09-30)

Uma referência pode ser uma **sequência de imagens** ligada ao tempo (Reference Frames): importação em lote, ordenação natural, mapeamento imagem→frame, filmstrip e marcação de key poses. Ver [P3D-172](p3d-172-reference-image-sequence.md). O contrato desta página (slots, propriedades e persistência) permanece válido para cada imagem da sequência.
