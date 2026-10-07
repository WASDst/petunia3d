# Validation, Repair, Cleanup e Import Boundaries

> **Status: aprovado.**

## Problema atual

`Mesh::validate()` hoje mistura quatro responsabilidades:

- checar invariantes;
- sanitizar valores não finitos;
- reparar UVs;
- remover faces/edges inválidos.

Além disso, comandos internos chamam `validate()` depois de editar geometria.

Isso pode esconder bugs do próprio Petunia.

## Regra principal

Separar rigorosamente:

```text
Internal authoring data
→ validate invariants
→ failure = bug / command error

External/untrusted data
→ inspect
→ repair/sanitize when policy allows
→ explicit report
→ validate invariants
```

## 1. Strict validation

Introduzir um contrato sem mutação:

```rust
pub fn validate_invariants(mesh: &Mesh) -> Result<(), MeshValidationError>
```

Exemplos de erro:

- non-finite vertex position/color;
- face com menos de 3 corners;
- corner apontando para vertex inexistente;
- FaceCorner com UV não finita;
- face degenerada por índices repetidos;
- seam referindo edge inexistente;
- pin referindo FaceCorner inexistente;
- topology structure inconsistente quando aplicável.

Validação nunca corrige.

## 2. External repair

Import/load legado pode usar:

```rust
pub fn repair_external_mesh(
    mesh: &mut Mesh,
    policy: RepairPolicy,
) -> RepairReport
```

`RepairReport` deve registrar pelo menos:

- vertices sanitized;
- faces dropped;
- invalid references removed;
- UV defaults generated;
- seams/pins dropped;
- duplicate/coincident points merged quando explicitamente permitido;
- warnings não reparáveis.

Nenhum reparo silencioso.

## RepairPolicy

Não criar dezenas de flags.

Começar com poucos perfis explícitos:

```text
StrictImport
TolerantImport
LegacyProjectMigration
```

Cada importer escolhe uma política conscientemente.

## StrictImport

Rejeita:

- NaN/Inf em geometry;
- indices fora dos limites;
- buffers truncados;
- faces estruturalmente inválidas;
- overflow/size limit.

Não substitui posição inválida por `[0,0,0]` silenciosamente.

## TolerantImport

Pode descartar primitives/faces isoladamente inválidas quando o restante do documento continuar coerente.

Sempre gera warnings.

Não inventa transform/geometry estrutural relevante sem avisar.

## LegacyProjectMigration

Existe para compatibilidade de formatos antigos do Petunia.

Pode:

- preencher campos novos com defaults conhecidos;
- migrar `verts + uv` para FaceCorner;
- converter collection String para CollectionId;
- migrar IDs/tipos antigos.

Não deve ser reutilizado como mecanismo geral de conserto de bugs atuais.

## 3. Cleanup explícito

Cleanup não pertence à validação.

Operações de cleanup são mutações geométricas intencionais e retornam `TopologyResult`/relatório.

### Safe cleanup

Conjunto inicial:

- remove degenerate faces;
- remove orphan vertices;
- Weld / Merge by Distance;
- Limited Dissolve;
- safe coplanar dissolve em resultados específicos;
- optional duplicate-face detection/removal quando comprovadamente seguro.

Cada ação é separada e testável.

## 4. Weld / Merge by Distance

O algoritmo já existe e deve ser preservado/refatorado.

Problema atual: ele muta Mesh diretamente e remapeia seleção interna.

No modelo novo:

```text
weld(mesh, tolerance)
→ Mesh
+ TopologyResult
+ ElementRemap
```

SelectionState e SurfaceAttachments são atualizados pela Application usando remap.

### Regras

- tolerance finita e >= 0;
- determinístico;
- não fundir vertices além da tolerância;
- preservar FaceCorner UV por corner sempre que faces sobreviverem;
- remover faces colapsadas explicitamente no resultado;
- seams/pins remapeados ou reportados como invalidados.

## 5. Limited Dissolve

Gap atual real: não há uma implementação geral consolidada.

Objetivo:

remover edges/vertices que não contribuem geometricamente de forma perceptível, preservando silhouette e boundaries importantes.

### V1 enxuta

Critérios conservadores:

- edge interno;
- exatamente duas faces adjacentes;
- faces coplanares dentro de tolerance angular;
- material slot compatível;
- não atravessar UV seam;
- não atravessar boundary;
- não destruir hole/border;
- merged polygon precisa ser simples e triangulável.

Se qualquer condição for ambígua:

→ não dissolve.

Isso é melhor que heurística agressiva.

### Uso

Limited Dissolve serve especialmente para:

- contours/trace que geraram segmentos redundantes;
- imported triangulated planar surfaces;
- cleanup pós-boolean quando provenance segura não estiver disponível;
- simplificação manual/assistida.

Não substituir Decimate.

## 6. Boolean cleanup

`boolean_cleanup.rs` é uma base forte e deve ser preservada.

Ele já segue uma política saudável:

- weld controlado;
- degenerates;
- dissolve coplanar somente quando a origem é conhecida;
- fallback para resultado cru se volume/topologia não baterem;
- sem remesh global.

Classificação:

**REUSE + FaceCorner/newtypes/remap refactor.**

Não transformar o cleanup booleano em `repair_external_mesh()` genérico; ele possui contexto/provenance específicos.

## 7. Degenerate cleanup

Separar explicitamente:

```rust
remove_degenerate_faces(...)
remove_orphan_vertices(...)
```

Uma face é degenerada estruturalmente quando:

- possui <3 vertices distintos;
- índice inválido;
- área abaixo de epsilon configurado relativo à escala.

Área quase zero precisa de epsilon baseado na escala/bounds, não literal global rígido.

## 8. Contour cleanup versus Mesh cleanup

Preservar a distinção já aprovada.

### Antes de gerar Mesh

```text
raw contour
→ duplicate point removal
→ collinear cleanup
→ Douglas–Peucker
→ adaptive resample
→ clean PlanarShape/Spline
```

### Depois de gerar Mesh

```text
Mesh
→ Weld
→ Limited Dissolve
→ Degenerate cleanup
→ strict validation
```

Evitar resolver excesso de vertices de desenho apenas com cleanup pós-mesh.

## 9. Imports

Importer é uma trust boundary.

Fluxo alvo:

```text
decode format
→ format validation
→ build intermediate/import mesh
→ repair according to import policy
→ strict Mesh validation
→ Document objects
```

### OBJ

O importer atual já faz bem várias coisas:

- finite positions;
- bounds checks;
- size limits;
- parse/validation errors.

Preservar a postura strict.

Melhoria futura: preservar UV/materials corretamente com FaceCorner quando `single_index` não for suficiente para a semântica desejada.

### glTF

Preservar:

- size limits;
- buffer validation;
- unsupported primitive warnings;
- material/image limits.

Alterar sanitizações silenciosas relevantes para warnings explícitos.

Por exemplo, transform não finito não deve simplesmente virar Identity sem constar em `ImportReport`.

## ImportReport

Unificar feedback dos importers:

```rust
pub struct ImportReport {
    pub warnings: Vec<ImportWarning>,
    pub repairs: Vec<RepairAction>,
}
```

A UI pode mostrar resumo simples e permitir detalhes.

Plugins/MCP também recebem report estruturado.

## 10. Project load

Carregar `.petunia` atual não deve chamar um `validate()` mutante genérico que poda dados silenciosamente.

Fluxo:

```text
deserialize
→ detect file version
→ migrate known legacy versions
→ strict document validation
→ fail with actionable error if current-format data is corrupt
```

Para autosave/recovery pode haver modo tolerant específico, mas sempre reportado.

## 11. Commands internos

Depois de Knife, Poly Pen, Loop Cut, Weld, Bevel etc.:

```text
operation
→ TopologyResult
→ debug/test strict validation
→ commit
```

Não:

```text
operation
→ Mesh::validate() repairs whatever went wrong
→ commit
```

Em builds de desenvolvimento/teste, validação completa pode ser agressiva.

Em release, operações críticas podem validar invariantes locais/estruturais conforme custo medido.

## 12. Diagnostics

Separar:

- `MeshValidationError` — invalidez real;
- `MeshDiagnostics` — qualidade/suspeitas sem impedir uso;
- `RepairReport` — o que foi alterado;
- `CleanupReport` — o que uma operação intencional removeu/fundiu.

Exemplos de diagnostics não fatais:

- non-manifold;
- open boundary;
- coincident vertices;
- flipped/inconsistent winding;
- very small faces;
- self-intersection suspeita.

Nem todo diagnostic deve causar repair automático.

## 13. Ownership

### Geometry

- strict Mesh validation;
- repair primitives;
- Weld;
- Limited Dissolve;
- degenerate/orphan cleanup;
- topology diagnostics;
- Boolean cleanup.

### IO / Project boundary

- choose RepairPolicy;
- format limits;
- ImportReport;
- legacy migrations.

### Application

- commands/Undo;
- present reports;
- update Selection/Attachments from TopologyResult.

## Migração recomendada

1. criar `MeshValidationError` + `validate_invariants()`;
2. criar `RepairReport` e `repair_external_mesh()`;
3. remover uso de `Mesh::validate()` de commands internos;
4. adaptar OBJ/glTF/load para a nova boundary;
5. refatorar Weld para retornar remap;
6. implementar Limited Dissolve conservador;
7. separar remove-degenerate/remove-orphan;
8. migrar Boolean cleanup para FaceCorner/newtypes;
9. integrar reports à UI/MCP/plugins;
10. remover `Mesh::validate()` mutante antigo.

## Regra anti-bloat

Repair existe na fronteira externa.

Cleanup é operação explícita.

Validation não muda dados.

Nenhum sistema tenta transformar automaticamente qualquer malha ruim em malha perfeita.

## Decisões fechadas

1. Validation não modifica dados.
2. Repair fica restrito a fronteiras externas, legacy e recovery.
3. Todo Repair produz report explícito.
4. Cleanup é operação geométrica explícita.
5. O `Mesh::validate()` mutante atual será removido.
6. Weld será reutilizado e passará a retornar `TopologyResult / ElementRemap`.
7. Limited Dissolve será implementado de forma conservadora.
8. Degenerate cleanup e orphan cleanup são operações separadas.
9. Boolean Cleanup existente é preservado como algoritmo especializado.
10. Não haverá remesh/retopology automática como consequência genérica de cleanup.
11. Contour cleanup ocorre preferencialmente antes da geração de Mesh.
12. OBJ mantém postura strict.
13. glTF pode operar de forma tolerante quando necessário, mas toda sanitização relevante gera warning/report.
14. Arquivos .petunia atuais inválidos falham explicitamente; versões antigas usam migrations conhecidas.
15. Commands internos terminam em strict validation, nunca em auto-repair.
