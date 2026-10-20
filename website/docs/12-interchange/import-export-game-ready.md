# Import, Export, Validation e Game-Ready

> **Status: aprovado em 2026-10-08.** Define contratos de produto e boundaries; não afirma suporte completo de todos os canais/formatos no build atual.

## Princípio

**Exportar é compilar uma cópia do authoring**, nunca destrutivamente converter o Document. Importar é validar dados externos antes de publicá-los. Bibliotecas de formato ficam em Infrastructure/adapter e não contaminam Geometry, Materials, Paint ou SceneObject.

## Formatos e escopo

| Formato | Papel | Regra |
|---|---|---|
| glTF 2.0 / GLB | Intercâmbio principal de modelos game-ready | Prioridade V1; validar extensão e canais realmente suportados |
| OBJ + MTL | Intercâmbio geométrico secundário | Não promete rig, animation, decals ou full-PBR |
| SVG | Importação de paths/contornos quando suportada | Converte para Spline/PlanarShape explicitamente |
| PNG/JPEG e imagens autorizadas | Textures, Reference e projeção | Validar limites e color space |
| .petunia | Projeto nativo autoral | Não tratar como export final/game-ready |
| FBX | Fora da baseline V1 | Reavaliar apenas por demanda e custo de dependência |

Implementar contratos Importer/Exporter por formato sem criar um sistema dinâmico de plugins desnecessário. Algoritmos e converters existentes devem ser reutilizados.

## Pipeline de exportação

1. Receber DocumentSnapshot, ObjectIds explícitos e ExportOptions tipadas.
2. Resolver instâncias, transformações e dependências.
3. Avaliar Geometry Source + Modifiers + Skin/Pose conforme escopo, **em cópia**.
4. Checar meshes, hierarquia, triangles, normals, winding, UV/FaceCorners, materials, textures.
5. Resolver projeções e decals por uma política de Bake/Export declarada; nunca prometer efeito não suportado.
6. Triangular deterministamente usando política compartilhada por Renderer e picking.
7. Calcular tangents quando aplicáveis, com conversão de coordenadas/unidades explícita.
8. Converter para ExportModel agnóstico ao formato; backend de GLB/OBJ serializa.
9. Validar resultado, reportar warnings e escrever atomicamente onde aplicável.
10. Nunca editar Document original, source generators, UVs autorais nem material slots para atender exporter.

O mesmo pipeline suporta **Single, Multiple e Batch Export**, variando apenas a seleção e opções. Não duplicar uma implementação por modo.

## Política de conversão e perda

Sempre distinguir:
- **Preservado:** formato de destino consegue representar fielmente.
- **Bake explícito:** recurso derivado transformado conforme opção selecionada.
- **Aproximado:** perda controlada com descrição mensurável.
- **Não suportado:** avisar ou falhar quando necessário para integridade.

Exemplos:
- Surface Decal / Animated Decal podem exigir Bake Texture, Flipbook, Animation Frames ou sidecar metadata quando exportados fora do formato nativo.
- Material Toon/Glass pode exigir conversão parcial; nunca rotular como GLB visualmente idêntico sem conformance.
- Mesh com FaceCorner UV deve preservar seams/overlaps intencionais mediante vertex splitting derivado quando requerido pelo formato.
- Animation/Rig Export requer suporte explícito por skin, bones e tracks; não marcar "Export Animation" como implementado só porque a arquitetura o admite.
- Recursos SVG vetoriais não precisam virar mesh até o target requerer surface.

## Importação, staging e mensagens

Padrão: file picker ou fonte autorizada → IO bounds/decoding → DTO de staging → strict validation + repair policy explícita → preview → Command/Transaction.

- Limits: tamanho de arquivo, count de vértices, texture dimensions, depth de nodes e decompression bomb.
- Path traversal e referências externas resolvidas via um resolver com containment e consentimento.
- glTF: malhas, nodes, UV, colors, materials/textures, skins/animation somente quando efetivamente suportados.
- OBJ: positions, faces, groups, UV, normals e MTL quando disponíveis. O `.mtl` é lido de verdade (materiais por objeto, `Kd`, `Ns`→rugosidade, `map_Kd` e `map_Bump` como texturas) e o material do objeto entra no asset, com a textura espelhada no asset para a viewport. `.mtl` e texturas são resolvidos **dentro da pasta do modelo** (mesmo confinamento do glTF): `..`, caminho absoluto e URI remota são recusados e viram aviso, não erro fatal — a geometria entra mesmo sem material.
- Regras de winding, transforms, units e tangent-space são explicitadas por importer e testadas.
- Dados malformados nunca criam um Document parcialmente publicado.
- Topologia irreparável gera erro estruturado. Reparos seguros no legado são classificados; **não fazer remesh, weld, close hole ou repack UV silenciosos**.

## Game-Ready Validator

Diagnósticos de **Error / Warning / Info** possuem código estável, mensagem simples, target ObjectId, localização quando aplicável, risco e ação recomendada.

| Gravidade | Exemplos | Resultado |
|---|---|---|
| Error | refs inválidas, topology impossível, textura essencial ausente, parse incorreto | Bloqueia export daquele target |
| Warning | non-manifold, UV stretch, overlap suspeito, materiais sem correspondência, budget excedido | Export sujeito a decisão |
| Info | counts, resolução, texel density, materials, triangles | Transparência sem bloqueio |

Auto-fix somente quando inequívoco ou por opção expressa; preservar document undo/preview se alterar authoring. O validator não decide a estética do artista.

## UX e acessibilidade

Export dialog com:
- destino, formato, seleção de objetos, units/axis, options e presets;
- pré-verificação, seção compacta de problemas e link "ver objeto";
- previsão clara de Bake/Unsupported/Excluded antes de executar;
- progress, cancel, retry, detalhes copiáveis e resumo final;
- keyboard-only, foco restaurado e estados que não dependem só de cor;
- operações caras em worker com snapshot/revision, cancelamento real e resultado stale descartado.

## Testes e critérios

- Fixtures golden/round-trip (GLB/OBJ) para triangles/quads/n-gons, UV seam, sharp normals e material slots.
- GLB validado por Khronos Validator e smoke em engines-alvo quando disponível, com evidência de versão.
- Teste de export com modificadores, generators, rig/skin e project photo: recusa explicada onde ainda não houver suporte.
- Batch determinístico em ordem/nomes, sem sobrescrever arquivo sem confirmação.
- Import malicious, oversized, textures quebradas, paths fora do projeto e cancelamento.
- Document original preservado byte/semanticamente após export, exceto preferências de UI não autorais.

## Decisões fechadas

GLB/glTF principal; OBJ secundário; FBX não baseline; Import/Export são adapters; ExportModel snapshot intermediário; validação explicável; sem baking/conversão silenciosa; Batch usa pipeline único; integridade e portabilidade antes de conveniência.

## Matriz de implementação e conformidade (I01 / I02)

| Tarefa | Escopo                                                      | Status    | Evidência                                                                             |
|--------|-------------------------------------------------------------|-----------|---------------------------------------------------------------------------------------|
| I01    | ExportModel intermediário avaliado e cópia estrita          | [x] DONE  | `crates/project/src/export_model.rs`, `test_gate1_snapshot_evaluated_authoring_intact`|
| I01    | Tangentes ortonormais determinísticas (MikkTSpace 4D)       | [x] DONE  | `crates/project/src/export_model.rs`, `test_gate2_orthonormal_tangents_and_vertex_splitting`|
| I01    | Vertex splitting em FaceCorner UV seams e sharp edges       | [x] DONE  | `crates/project/src/export_model.rs`, `test_gate2_orthonormal_tangents_and_vertex_splitting`|
| I01    | Pipeline único (Single, Multiple, Batch e Snapshot)         | [x] DONE  | `crates/project/src/pipeline.rs`, `export_snapshot`, `test_gate1_snapshot_evaluated_authoring_intact`|
| I02    | Game-Ready Validator com severidade acionável               | [x] DONE  | `crates/project/src/game_ready.rs`, `validate_asset`, `i02-game-ready-validator`     |
| I02    | Confinamento seguro contra path traversal e rede            | [x] DONE  | `crates/project/src/import_gltf.rs`, `resolve_safe_path`, `test_gate4_safe_io_path_traversal_and_malicious_input`|
| I02    | Khronos glTF 2.0 / GLB conformance com PBR e TANGENT        | [x] DONE  | `crates/project/src/export_model.rs`, `test_gate3_khronos_glb_conformance_and_materials`|
| I02    | Política estrita de sobrescrita (AlreadyExists) e atômica   | [x] DONE  | `crates/project/src/pipeline.rs`, `test_gate5_atomic_integrity_and_overwrite_policy` |
| I02    | Round-trip OBJ e GLB preservando topologia e UVs            | [x] DONE  | `crates/project/tests/interchange_i01_i02_gates_tests.rs::test_gate6_roundtrip_obj_and_glb_preserves_topology_and_uvs`|
| I03    | Importação de modelo no shell: OBJ com `.mtl` e textura, glTF/GLB com material e textura, entrada no menu Arquivo | [x] DONE | `crates/project/tests/pipeline_tests.rs` (`test_import_obj_reads_the_mtl_material_and_its_texture`, `test_import_obj_refuses_a_texture_outside_the_model_folder`), `crates/ui-cxxqt/tests/import_model.rs` (8 testes), `shell_v2` (`file:menu-offers-the-model-import`)|

### Status da bateria e evidências de Import/Export (I01, 2026-10-17)

Todos os gates de `I01` foram formalizados e validados:
- **Snapshot Avaliado e Imutabilidade do Authoring**: `ExportModel` compila uma cópia avaliada do projeto a partir de `DocumentSnapshot` sem alterar o documento original (`test_gate1_snapshot_evaluated_authoring_intact`).
- **Tangentes Ortonormais Game-Ready 4D**: Geração determinística de tangentes estilo MikkTSpace com componentes XYZ unitários e ortogonais às normais do corner (`dot(n, t) < 1e-3`) e handedness W estritamente em `{-1.0, 1.0}` (`test_gate2_orthonormal_tangents_and_vertex_splitting`).
- **Preservação de Seams e Sharp Edges por Vertex Splitting**: Vértices duplicados apenas onde posições, UVs ou normais divergem, garantindo sombreamento nítido e continuidade de textura nos motores de jogo.
- **Pipeline Unificado de Exportação**: `DeliveryPipeline::export_snapshot`, `export_single_asset`, `export_multiple_assets` e `batch_export` utilizam o mesmo modelo avaliado com escrita atômica via `io_atomic`.

### Status da bateria e evidências de Game-Ready e IO Seguro (I02, 2026-10-17)

Todos os gates de `I02` foram formalizados e validados:
- **Game-Ready Validator Acionável**: `validate_asset` classifica inconsistências em `Error` (bloqueante), `Warning` (reportado no manifesto) e `Info`.
- **Confinamento Seguro contra Path Traversal**: `resolve_safe_path` rejeita tentativas de escape com `..` (ex: `../../etc/passwd`), caminhos absolutos e URIs de rede sem pânico (`Never crash`), protegendo buffers e texturas externas (`test_gate4_safe_io_path_traversal_and_malicious_input`).
- **Conformidade Khronos glTF 2.0 / GLB**: Header binário de 12 bytes, chunks JSON e BIN alinhados a 4 bytes, accessors tipados (`POSITION`, `NORMAL`, `TEXCOORD_0`, `TANGENT`, `JOINTS_0`, `WEIGHTS_0`), canais PBR e metadados (`test_gate3_khronos_glb_conformance_and_materials`).
- **Política Estrita de Sobrescrita e Integridade Atômica**: Rejeição determinística com `PipelineError::AlreadyExists` quando `overwrite == false`, sem truncar arquivos pré-existentes (`test_gate5_atomic_integrity_and_overwrite_policy`).
- **Round-trip Fiel**: Leitura e escrita de OBJ com coordenadas de textura (`face.uv`) e GLB com materiais PBR validados em round-trip (`test_gate6_roundtrip_obj_and_glb_preserves_topology_and_uvs`).

### Status da importação de modelo no shell (I03, 2026-10-19)

Relato do usuário: "a importação de modelos não está funcionando para nenhum formato e, principalmente, se tiver textura". A auditoria achou quatro defeitos independentes, todos corrigidos:

| Defeito | Causa | Correção |
|---|---|---|
| OBJ vinha sem material e sem textura | o importador ignorava o `.mtl` (material loader vazio) | `import_obj::import_obj_file` lê o `.mtl`, converte `Kd`/`Ns`/`map_Kd`/`map_Bump` e liga o material por face; a textura espelha no asset |
| Um OBJ com vários objetos trazia só o primeiro | o importador ficava com o primeiro modelo do `tobj` | todos os objetos entram, cada um com seu material; o nome vem do `o` do arquivo (como no Blender) e, sem `o`, do nome do arquivo |
| `.gltf` com skin/animation e buffers externos falhava inteiro | o rig era importado antes da tolerância a falha | rig que não pode ser lido vira **aviso**; o modelo entra de qualquer forma |
| Não había porta de entrada para importar | o menu Arquivo não tinha Importar e os comandos da paleta eram stubs | `Arquivo > Importar` (modelo 3D, paleta) e os comandos `file.import_obj`/`file.import_gltf` passam a **pedir** a importação (`import_request` no snapshot); a interface abre o diálogo (comando programático nunca abre janela) |

Feedback: arquivo sem malha é recusado com status próprio ("nenhuma malha encontrada"), e o import continua desfazível por Undo.

