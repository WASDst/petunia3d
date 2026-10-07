# Shape Builder 3D

> **Status: aprovado. Direção estratégica e arquitetura sólida inicial fechadas.**

O Shape Builder 3D deve ser uma feature diferenciadora do Petunia3D por transformar composição geométrica em interação direta, sem introduzir um segundo kernel CAD.

## Princípio

A ferramenta não inventa uma nova matemática booleana.

Ela cria uma camada de interação sobre capacidades existentes:

- planar arrangement;
- regions;
- imprint;
- Push/Pull;
- Boolean Union;
- Boolean Difference;
- Boolean Intersection;
- Boolean cleanup.

## Dois caminhos internos, uma experiência

### Planar / Surface Shape Builder

Usa regiões coplanares, holes, imprint e Push/Pull.

### Solid Shape Builder

Usa o kernel boolean existente e apresenta suas partes de forma visual.

---

# Proposta detalhada para Solid Shape Builder

> **Status: aprovado para a primeira implementação sólida.**

## 1. Dois operandos por sessão

A primeira versão trabalha com exatamente dois objetos sólidos ativos:

```text
A + B
```

Não tenta construir uma partição volumétrica arbitrária de N objetos ao mesmo tempo.

Isso evita crescimento combinatório e mantém picking, preview, Undo e preservação de atributos compreensíveis.

Um terceiro objeto pode ser incorporado em um gesto posterior.

## 2. Três regiões booleanas fundamentais

Para dois sólidos, derivar:

```text
A_ONLY   = A \ B
OVERLAP  = A ∩ B
B_ONLY   = B \ A
```

Essas três regiões formam o vocabulário sólido básico.

Não é necessário criar um novo kernel de regiões volumétricas.

## 3. Componentes desconectados

Cada resultado booleano pode conter múltiplos componentes desconectados.

A Geometry deverá oferecer uma query de componentes conectados por faces:

```rust
connected_face_components(mesh: &Mesh) -> Vec<MeshComponent>
```

Cada componente visualmente separado pode virar uma região interativa.

Isso é uma query derivada; não introduz identidade persistente de componente no documento.

## 4. Identidade transitória da região

Uma região de preview pertence somente à Tool Session.

Exemplo conceitual:

```rust
pub struct SolidRegionCandidate {
    pub source: SolidRegionSource,
    pub component_index: ComponentIndex,
    pub preview_mesh: Mesh,
    pub bounds: Bounds3,
}

pub enum SolidRegionSource {
    FirstOnly,
    Intersection,
    SecondOnly,
}
```

Nada disso é persistido no arquivo .petunia.

## 5. Picking

Picking deve acontecer sobre as meshes de preview derivadas.

Pipeline:

```text
pointer ray
   ↓
candidate bounds
   ↓
triangle hit
   ↓
nearest visible SolidRegionCandidate
```

O hover destaca a região inteira correspondente ao componente conectado, não somente o triângulo atingido.

## 6. Preview

Ao iniciar a sessão:

1. capturar A e B;
2. calcular A_ONLY, OVERLAP e B_ONLY;
3. executar boolean cleanup;
4. separar componentes conectados;
5. construir cache de picking/bounds;
6. renderizar como preview transitório.

O documento não muda durante hover ou seleção de regiões.

## 7. Estado da ferramenta

```rust
pub struct SolidShapeBuilderSession {
    pub first_operand: AssetId,
    pub second_operand: AssetId,
    pub candidates: Vec<SolidRegionCandidate>,
    pub selected_regions: HashSet<SolidRegionIndex>,
    pub intent: ShapeBuilderIntent,
}
```

O cache deve ser reconstruído somente quando os operandos/revisões mudarem.

## 8. Intenções iniciais

Manter somente três ações públicas:

### Unite
As regiões tocadas entram no resultado final como um único objeto quando a união for válida.

### Remove
As regiões tocadas deixam de fazer parte do resultado.

### Extract
As regiões tocadas tornam-se objeto(s) independentes.

A UI não deve expor diretamente A_ONLY / OVERLAP / B_ONLY; isso é implementação.

## 9. Modelo de seleção por máscara

Internamente, a sessão mantém quais regiões sobreviverão.

Exemplo:

```text
A_ONLY   keep
OVERLAP  remove
B_ONLY   keep
```

produz duas partes externas sem a interseção.

Outro gesto:

```text
A_ONLY   keep
OVERLAP  keep
B_ONLY   keep
```

equivale visualmente a Union.

Isso permite uma gramática única sem criar dezenas de operadores.

## 10. Resultado

No commit:

- combinar apenas as regiões marcadas para manter;
- aplicar cleanup;
- reconstruir assets necessários;
- produzir remap/changes explícitos;
- transferir material/UV/paint quando possível;
- registrar um único Undo.

## 11. Atributos

A origem da região é conhecida:

- FirstOnly deriva principalmente de A;
- SecondOnly deriva principalmente de B;
- Intersection possui superfícies herdadas dos dois operandos e superfícies novas de corte.

Reutilizar a infraestrutura atual de Boolean material/texture transfer em vez de duplicá-la.

FaceCorner torna essa transferência mais segura porque vertex + UV de corner permanecem unidos estruturalmente.

## 12. Ambiguidade e falha

Recusar claramente quando:

- operandos não forem sólidos adequados;
- boolean kernel falhar;
- resultado for vazio;
- topologia resultante for inválida;
- número de componentes ultrapassar orçamento seguro;
- preview exceder orçamento configurado.

Nunca tentar “consertar” silenciosamente com remesh global.

## 13. Performance

Não recalcular boolean a cada movimento do mouse.

```text
tool begin / operand changed
        ↓
calculate candidate regions once
        ↓
hover only performs picking
        ↓
gesture changes selection mask
        ↓
commit reuses cached result when valid
```

Essa separação é essencial para hardware low-end.

## 14. Multi-object

Não fazer partição combinatória de vários objetos na V1.

Fluxo:

```text
A + B
→ commit
→ Result + C
→ commit
```

A interação continua rápida e previsível.

Multi-object verdadeiro só deve ser considerado se uso real demonstrar necessidade.

## 15. Relação com Planar Shape Builder

A experiência visual deve ser a mesma:

```text
hover region
→ highlight
→ drag/click
→ preview
→ commit
```

Mas o backend pode ser diferente:

```text
Planar → arrangement/regions
Solid  → boolean decomposition
```

A uniformidade é de interação, não de algoritmo.

## 16. Não objetivos

- B-Rep;
- CAD constraint solver;
- volumetric arrangement genérico;
- histórico boolean procedural obrigatório;
- remeshing global automático;
- N-object boolean partition na primeira versão;
- novo kernel CSG;
- persistência de regiões transitórias.

## Critério de sucesso

O usuário deve conseguir combinar formas complexas sem pensar em “Union/Difference/Intersection” na maior parte do tempo, enquanto a implementação continua sendo uma camada pequena e testável sobre os kernels existentes.


## Decisões fechadas — Solid Shape Builder V1

As seguintes regras são normativas:

1. A primeira versão sólida trabalha com **dois operandos por sessão**.
2. A decomposição fundamental é **A_ONLY / INTERSECTION / B_ONLY**.
3. Componentes desconectados de cada resultado são tratados como regiões distintas.
4. Regiões sólidas são **transitórias da Tool Session** e nunca persistem no documento.
5. Booleans e cleanup são recalculados quando os operandos/revisões mudarem; **movimento do mouse faz apenas picking/seleção de região**.
6. A linguagem pública inicial possui somente **Unite, Remove e Extract**.

Expansões futuras só entram após medição de uso real. Multi-object combinatório, novos kernels, B-Rep e remeshing global permanecem explicitamente fora do escopo inicial.
