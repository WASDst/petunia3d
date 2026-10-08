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
- OBJ: positions, faces, groups, UV, normals e MTL quando disponíveis.
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
