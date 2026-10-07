# Shape Builder 3D

> **Status: direção estratégica aprovada. Feature potencialmente diferenciadora do Petunia3D.**

O Shape Builder 3D deve tornar composição de formas significativamente mais direta sem introduzir um segundo kernel geométrico ou um sistema procedural excessivamente amplo.

## Objetivo

Permitir que o usuário construa, combine, recorte e reorganize formas por interação direta sobre regiões visuais, usando os kernels existentes.

A experiência deve ser:

```text
hover → entender região
click/drag → escolher intenção
preview → confirmar
```

em vez de exigir configuração manual de várias operações booleanas.

## Dois caminhos internos, uma experiência

### 1. Planar / Surface Shape Builder

Usa:
- arrangements 2D;
- regions;
- imprint;
- faces coplanares;
- holes;
- Push/Pull.

É adequado para:
- formas desenhadas em workplane;
- cortes em faces;
- composição de regiões;
- extração de shapes.

### 2. Solid Shape Builder

Usa os kernels já existentes:
- Boolean Union;
- Difference;
- Intersection;
- Boolean cleanup.

Não cria novo kernel CSG.

A ferramenta apenas interpreta a região/volume sob o cursor e converte a intenção visual do usuário para uma sequência determinística de operações existentes.

## Intenções mínimas

Começar pequeno:

- **Unite** — manter a união das regiões escolhidas;
- **Remove** — remover a região escolhida do resultado;
- **Keep / Extract** — preservar uma região como parte independente quando semanticamente seguro.

Não adicionar dezenas de modes inicialmente.

## Interação

### Hover
A região candidata deve ser destacada antes do clique.

### Drag
Arrastar através de regiões pode acumulá-las para Unite/Remove.

### Modifier
Um único modificador pode inverter a intenção principal quando isso for previsível.

### Preview
Nenhuma operação destrutiva acontece durante hover.
Preview usa geometria derivada/transitória.

### Commit
Um gesto completo = um Undo.

## Regras de escopo

Para evitar bloat:

1. Reutilizar Boolean, arrangement, imprint e cleanup existentes.
2. Não implementar B-Rep/CAD kernel.
3. Não criar grafo procedural como pré-requisito.
4. Não exigir remeshing global.
5. Não prometer regiões volumétricas arbitrárias quando o kernel existente não puder classificá-las de forma robusta.
6. Preferir recusar uma operação ambígua com feedback claro a produzir topologia ruim.
7. Operações devem preservar materiais/UV/paint attachments quando os kernels atuais tiverem informação suficiente.
8. Toda mutação retorna remap/changes explícitos.

## Fases sugeridas

### Fase A — planar robusto
Consolidar o Shape Builder já existente:
- regiões;
- holes;
- Unite;
- Remove;
- Extract;
- imprint;
- Push/Pull.

### Fase B — sólidos simples
Selecionar dois sólidos sobrepostos e oferecer regiões booleanas previsíveis via kernels existentes.

### Fase C — multi-shape gesture
Permitir gesto contínuo sobre múltiplas regiões/sólidos, mantendo um Undo e preview incremental.

### Fase D — refinamento
Somente após métricas e uso real:
- heurísticas melhores de region picking;
- preservação avançada de atributos;
- operações compostas adicionais.

## Não objetivos iniciais

- CAD paramétrico completo;
- B-Rep;
- solver de constraints;
- procedural node graph obrigatório;
- remesh automático pesado;
- boolean history stack complexo;
- dezenas de operadores especializados.

A feature deve ser poderosa porque combina capacidades existentes de forma intuitiva, não porque duplica todo o sistema de modelagem.
