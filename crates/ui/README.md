# Crate `petunia_ui` (`crates/ui/`) — [ARQUIVADA / LEGACY]

> ⚠️ **Status: Interface legada arquivada.**  
> A interface principal do Petunia3D foi promovida para o frontend declarativo moderno em Slint (`crates/ui-slint`).  
> Esta crate permanece no repositório como implementação de referência histórica e fallback acessível via `--legacy-egui` ou variável de ambiente `PETUNIA_LEGACY_EGUI=1`.

Camada de apresentação e interface gráfica construída com `egui`:
- Barra de cabeçalho superior com seleção de workspaces por pílulas (MODEL, PAINT, UV, EXPORT).
- Motor de iconografia vetorial procedural canônica (`icons.rs`, `icon_registry.rs`) e eliminação de emojis.
- Barra de viewport organizada em 7 clusters funcionais responsivos: seletor de domínio unificado de 4 pílulas (P3D-015), menus contextuais, orientação/pivô tipados, snapping magnético e edição proporcional segmentados com popovers de ajuste fino (P3D-079, P3D-080) e popover de overlays integrado (`viewport_bar.rs`, P3D-010).
- Menus padronizados (`PetuniaMenuItem`, `PetuniaMenuCheckboxItem`, `PetuniaMenuRadioItem`) com layout profissional, submenus e atalhos dinâmicos.
- Paleta de comandos (`command_palette.rs`, P3D-081) com busca fuzzy, filtros por categoria, navegação por teclado e validação contextual.
- Gerenciador visual de conjuntos de referências (`reference_manager.rs`, P3D-013 / P3D-014) com suporte aos 6 slots ortográficos canônicos, calibração fina e miniaturas.
- Navigation HUD e gizmo 3D (`nav_gizmo.rs`, P3D-005 / P3D-007) com orientação nominal de câmera e leitura de ângulos em tempo real.
- Orquestração de painéis laterais de ferramentas através do registro dinâmico `ModuleRegistry`.
- Área central de viewport 3D interativo com suporte a eventos de ponteiro, gestos, desenho de overlays de controle e telemetria modal em tempo real (`ToolFeedback`, P3D-131).


## Interação direta e Camada Apresentacional
 
`modal_viewport` traduz mouse/teclado para a máquina de estados `PointerSession` e transações modais do domínio, desenhando o HUD e guias.
`gizmo` projeta e testa eixos, planos, anéis e escala. `viewport_interaction` arbitra picking/hover, navegação e pintura.
`cutting` delega o ciclo de corte e deslizamento diretamente para a máquina de estados `CutSession` em `AppState`.
`file_dialog_service` centraliza os diálogos nativos/in-canvas de arquivos e os encaminha ao `ProjectService`.
`reference_manager` provê interface modal rica para upload, pré-visualização, alinhamento de vista e configuração de referências ortográficas.
Testes egui nos módulos `*_tests` injetam eventos reais e verificam geometria, sessões e undo.
Consulte [o manual](../../docs/manual/usage.md) para atalhos e limitações efetivos.
