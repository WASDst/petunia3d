# Qualidade e estilo de código

## Código autoexplicativo

O código deve priorizar leitura e intenção. Abreviações locais ou inventadas de nomes de variáveis, parâmetros, funções e tipos não são aceitas.

Evitar:

```rust
fn orbit(&mut self, dx: f32, dy: f32)
```

Preferir:

```rust
fn orbit(
    &mut self,
    horizontal_pointer_delta: f32,
    vertical_pointer_delta: f32,
)
```

Também preferimos `vertex_index`, `asset_identifier`, `texture_width`, `event`, `command` e `context` a nomes como `i`, `idx`, `tex_w`, `ev`, `cmd` e `ctx`.

## Newtypes e aliases

Newtypes são preferidos quando o tipo primitivo não carrega significado suficiente:

```rust
pub struct AssetId(Uuid);
pub struct MaterialId(Uuid);
pub struct VertexIndex(u32);
```

Aliases podem ser usados quando melhorarem a leitura sem fingir segurança de tipo que não existe.

## Funções

Uma função deve representar uma responsabilidade compreensível. Funções grandes devem ser divididas quando possuírem fases distintas, múltiplos níveis de decisão, muitos temporários ou efeitos diferentes.

A regra não é um limite artificial de linhas. Clean Code é ferramenta, não religião.

## Documentação

Comentários não devem repetir o código. Devem explicar intenção, invariantes, decisões e restrições.

APIs públicas devem possuir Rustdoc instrutivo, deixando claro o contrato, invariantes, erros, efeitos colaterais, unidades e sistemas de coordenadas quando relevantes.

## Parâmetros booleanos

Chamadas opacas como `process(mesh, true, false, true)` devem ser evitadas. Preferir tipos ou estruturas de opções com nomes explícitos.

## Side effects

Criação de structs de domínio não deve executar I/O implicitamente. Filesystem, relógio, threads, diálogos e recursos do sistema entram por serviços explícitos.
