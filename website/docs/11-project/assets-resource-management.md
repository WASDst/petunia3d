# Assets, recursos compartilhados e biblioteca

> **Status: aprovado em 2026-10-08 para o modelo alvo.** A migração de Asset legado para SceneObject e recursos reutilizáveis é incremental.

## Definições canônicas

O nome **SceneObject** identifica uma instância colocada no documento; na GUI, **Part** pode continuar sendo o nome amigável. **Asset** identifica conteúdo reutilizável/importado/prefab, não o objeto colocado. Não perpetuar o Asset legado como fonte simultânea de geometry, material, texture, paint, rig e cache.

| Entidade | Identidade e ownership | Exemplo |
|---|---|---|
| SceneObject | ObjectId persistido no Document | instância de uma cadeira |
| Geometry Source | uma única fonte autoral por SceneObject | EditableMesh, Spline, PlanarShape ou Generator |
| Library Asset | AssetId estável e metadata/versão | modelo reutilizável |
| Prefab | definição reutilizável + overrides explícitos | coleção de partes |
| Material | MaterialId em resource store | material de madeira |
| Texture | TextureId e conteúdo/caminho validado | atlas/albedo |
| Spline/PlanarShape | IDs próprios no Document | perfil extrudável |
| Reference | ReferenceId/ReferenceSet | blueprint |
| DecalSet / Paint | IDs próprios no respectivo domínio | facial sprite set |
| Resource cache | chaves por IDs/revisions; **não persistido** | thumbnail, texture GPU |

Não introduzir ECS nem store universal com Any/trait objects só para manter tudo num mesmo registry. **Stores tipadas por domínio** e um resolvedor pequeno bastam.

## Library versus Document

- Document possui recursos autorais locais e bindings de suas instâncias.
- Library indexa assets reutilizáveis, importados ou instalados, com metadata e preview; não assume posse da seleção nem câmera.
- Inserir um asset cria SceneObject/instância por Command com origem rastreável.
- Editar definição compartilhada altera outras instâncias **apenas quando o usuário escolheu shared edit**.
- Make Unique quebra vínculo de modo explícito. Import não converte um asset vinculado em cópias aleatórias.
- Prefab/linked instance é feature modular: oferecer vínculo e overrides somente onde tiverem semântica implementável e testada.

## IDs, URI e resolução de dependências

Preferir IDs tipados e um descriptor de localização: **Embedded**, **ProjectRelative** ou **ExternalExplicit**; localização não é identidade. URI/path passa por normalize, containment, limite de tamanho e validação de tipo.

Ordem para resolver recurso:
1. embedded por ID e hash de integridade se houver;
2. project-relative resolvido à raiz confiável do projeto;
3. external explícito autorizado pelo usuário;
4. unresolved com diagnóstico, placeholder e ação **Relink**; nunca escolher outro arquivo silenciosamente.

Resolução não executa scripts, não acessa rede sem capability e não concatena paths não confiáveis. Symlink/zip traversal devem passar pelos checks de trust boundary.

## Reuso, cache e invalidação

- Decodificação de imagem por resource key + revision; GPU textures pertencem ao Renderer.
- Thumbnail e preview regeneráveis não são Document state.
- Geometry caches pertencem à avaliação, não ao Library Asset.
- Mudanças em fonte propagam invalidation explicitamente por dependências; evitar scan global por frame.
- Dirty rectangles/tiles para Paint/texture, com partial upload quando suportado.
- LRU ou budgets são infraestrutura adaptativa, não formato serializado.
- IDs persistentes não mudam porque arquivo mudou de pasta; fingerprint/hash não cria identidade nova sem ação.

## Import, drag/drop e referências

Drag de um asset para Scene é **UI Intent → Command → SceneObject**; undo remove instância sem apagar recurso compartilhado. Preview não altera documento; drop só confirma se recurso é válido.

A Library deve mostrar:
- origem, tipo, versão/compatibilidade, tamanho/triangles quando disponível;
- thumbnails com fallback e status de erro;
- busca e filtros por tags/tipo/coleção;
- scope explícito Project/Local/External, itens recentes e favoritos quando houver suporte;
- Relink, Make Unique, Locate, Missing Dependencies;
- estados acessíveis, ações por teclado e explicação ao desabilitar opções.

**Nenhuma biblioteca remota obrigatória, login ou telemetria automática** para criar/editar assets.

## Reference, materiais, Decals e rigs

Compartilhamento é por recurso tipado:
- Surface projection referencia ReferenceView/Texture por ID.
- Material slot vincula MaterialId com FaceCorner/corner attrs mantidos em Geometry.
- DecalPlacement referencia DecalSet/Variant; sem cópia de pixels para cada placement.
- PaintDocument possui seus targets/layers; materiais apenas ligam recursos e channels.
- Rig/Animation usa IDs persistentes e bindings explícitos; nenhum acoplamento silencioso entre shape generator e skin.

## Formato e export

**Package Project** agrega dependências elegíveis em cópia portátil, com manifest, IDs e recursos necessários; não embute paths absolutos sensíveis. **Export asset** trabalha sobre snapshot avaliado e não sobre a definição da Library mutável; ver [Import/export](../12-interchange/import-export-game-ready.md).

## Migrar código atual

1. Inventariar usos do atual Asset (SceneObject, store/Library, thumbnails, resource bindings).
2. Adotar ObjectId para scene selection, sem forçar rename global antes dos consumidores.
3. Extrair vínculo e stores por material, texture e reference gradualmente.
4. Migrar Asset Library para apresentação do catalog de recursos, sem estado de domínio no Slint.
5. Preservar import/model library existentes; remover duplicações depois de round-trip e UI parity.
6. Verificar projetos legados e IDs; não apagar tipos antes de migration.

## Gates

- Duas instâncias da mesma Library Asset têm transforms distintos.
- Shared edit e Make Unique distinguíveis/Undoáveis; nenhuma edição surpresa.
- Mover pacote entre Linux/Windows preserva resource refs relativos e scene IDs.
- Relink manual sem perder material/paint/decal attachments.
- Thumbnail faltante não impede abrir documento ou mover asset.
- UI keyboard-only e status legíveis em qualquer tema.

## Decisões fechadas

Asset é biblioteca/reuso; SceneObject é instância; typed resources e stable IDs; vinculação e overrides explícitos; cache reconstruível; dependências nunca são resolvidas silenciosamente; acesso a arquivos externos exige autorização do usuário.
