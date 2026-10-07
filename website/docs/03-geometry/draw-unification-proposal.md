# Unificação DRAW / POLY

> **Status: proposta em discussão. Não é decisão aprovada ainda.**

## Hipótese

DRAW deixa de ser um segundo sistema de modelagem e passa a ser um **perfil de interação planar/restrito** sobre o mesmo motor de ferramentas 3D.

A intenção é reduzir fragmentação entre:

- ProfileState transitório;
- Profiles/Splines persistentes;
- domínios Shape/Curve/Point/Region;
- Poly Pen;
- ferramentas POLY;
- Shape Builder;
- primitivas;
- booleans.

## Possível modelo

As ferramentas fundamentais seriam compartilhadas:

- Poly Pen;
- Bezier Pen 3D;
- primitivas 3D e formas planas;
- Circle/Plane e demais formas;
- Knife/Cut;
- Push/Pull;
- Shape Builder;
- Booleans;
- snapping/inference.

O workspace DRAW mudaria principalmente:

- projeção da câmera;
- liberdade de navegação;
- workplane ativo;
- apresentação/overlays;
- conjunto contextual de ferramentas.

## Navegação

Em vez de simplesmente bloquear a câmera nos seis eixos globais, a direção preferida é **Planar Navigation**:

- projeção ortográfica;
- câmera normal ao workplane ativo;
- pan e zoom liberados;
- orbit livre desabilitado;
- troca rápida entre planos cardinais;
- possibilidade de alinhar o plano a uma face ou plano local.

Isso mantém a simplicidade de desenho 2D sem impedir desenho sobre superfícies arbitrárias.

## Alerta arquitetural

Curvas/Splines continuam sendo um tipo geométrico persistente quando houver benefício paramétrico. Unificar workspaces não significa converter toda curva imediatamente em mesh.

O objetivo é unificar ferramentas e interação, não perder capacidade de edição não destrutiva.
