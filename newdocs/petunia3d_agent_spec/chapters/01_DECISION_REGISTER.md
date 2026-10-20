# 01 — Registro consolidado das decisões aprovadas

**Status:** decisões aprovadas pelo responsável durante a especificação, independentemente do estado de implementação. **NÃO** inferir que o código possui todos os recursos. Este arquivo é fonte do produto; os demais capítulos detalham algoritmos, interações, testes e riscos.

## 1.1 Seleção e transformação — SEL-001 a SEL-007

| ID | Aprovação | Regras imperativas |
|---|---|---|
| **SEL-001** | Gizmo universal integrado e contextual | Combinar Move/Rotate/Scale em um mesmo manipulador compacto e permanente quando há seleção válida; operações especializadas são contextuais, não um amontoado de alças. Referências Shapr3D e Rhino Gumball. |
| **SEL-002** | Seleção geométrica contextual | Escolha direta entre objeto/face/aresta/vértice/shape com prioridade de picking; pré-highlight; filtros; ambiguidade tratada; oclusão; Select Through explícito; evitar Edit Mode obrigatório. |
| **SEL-003 A** | Pivô/orientação automáticos + controle manual | Gizmo posiciona-se conforme tipo selecionado; `TransformPivot`, `GizmoDisplayPosition` e `OrientationFrame` independentes. Suporte World/Local/Normal/View/Custom/Auto; pivô manual persistente quando válido. Congelar frame durante drag. |
| **SEL-004 A** | Arrasto protegido | Arrastar geometria **não** transforma automaticamente. Só alças ou comando transform ativo; área vazia = box selection; Esc cancela transação; navegação explícita e separada; valor numérico; uma interação = um Undo. |
| **SEL-005 B** | Snapping equilibrado contextual | Pivô e Point-to-Point podem usar snap automático; Move/Rotate/Scale comuns suaves por padrão, snap explícito/temporário. Snap Engine único: vértice, aresta, midpoint, face, eixo, grade, superfície, distância, ângulo; tolerâncias e histerese. |
| **SEL-006 C** | Seleção heterogênea e gizmo compartilhado | Um gizmo compartilhado por padrão, troca rápida para pivôs individuais; seleção mista de objetos e subelementos, `ActiveElement`, ordem de seleção, conjuntos transformáveis resolvidos sem duplicar vértices; hierarquias sem transformações duplas; ações incompatíveis identificadas. |
| **SEL-007 B** | Universal compacto adaptativo | Move/Rotate/Scale sempre acessíveis, controles secundários visualmente discretos, ênfase no hover/drag; barra contextual separada, reposicionável/fixável; alças visuais independentes do hit testing; tamanho, contraste, DPI e teclado configuráveis. |

### Detalhes fixados por essas decisões

- **Prioridade de picking recomendada:** gizmo efetivamente habilitado sob o ponteiro > vértice próximo > aresta próxima > face > objeto. Filtros e candidatos por profundidade/oclusão podem alterar a elegibilidade, não gerar ações escondidas.
- **Pivot default:** objeto = centro do bbox; face = centro geométrico + normal; aresta = midpoint + tangente; vértice = posição; seleção múltipla = centro de bounds; edição individual usa referências por componente/região.
- **Pivôs compartilhados:** Rotate/Scale orbitam/escalam também o arranjo dos elementos; pivôs individuais preservam posições de origem em operações por elemento. Componentes de uma mesma mesh conexa devem ser agrupados em ilhas conforme política da operação.
- **Mescla objeto + parte:** seleção do objeto inteiro prevalece sobre seleção simultânea de parte dele para transform global; hierarquia e elementos repetidos não multiplicam a transformação. Componentes da malha referem-se a vértices únicos deduplicados.
- **Preview:** snapshot inicial + transformação derivada; nunca aplicar incrementalmente a cada frame ao resultado do frame anterior (evita drift); Esc = rollback atômico.
- **Operações contextuais:** face (Extrude, Offset, Inset, Bevel), aresta (Bevel, Chamfer, Loop, Bridge), objeto (Align, Duplicate, Group, Mirror) — filtradas por capacidades reais, com ações parcialmente compatíveis explicitadas.
- **Estética:** convenção inicial X vermelho, Y verde, Z azul, mas não confiar somente em cores; círculos/arcos, setas, planos e cubos de escala; controles com hit regions mais amplas do que o desenho quando possível sem conflitos.

## 1.2 Percepção espacial — VSP-001 a VSP-005

| ID | Aprovação | Regras imperativas |
|---|---|---|
| **VSP-001 B** | Assistência espacial contextual adaptativa | Sombreamento e silhueta discretos. Mostrar informações relevantes conforme seleção/ação, não todos os overlays. Companion View sugerida; abertura automática opcional. Preferências do usuário predominam. |
| **VSP-002 B** | Clay Soft adaptativo | Key/fill suaves, silhueta discreta, cavity leve, realce geométrico conforme contexto. Modos Clay Soft/Form/Technical/MatCap; controle manual persistente; iluminação de trabalho independente das luzes da cena. |
| **VSP-003 B** | Volume Frame compacto adaptativo | Ao selecionar, limites discretos + resumo dimensional. Durante transformação, destaque à dimensão afetada. Expansão sob demanda ou pinning; referenciais World/Local/Custom/Structural; medidas exatas/aproximadas/indisponíveis/desatualizadas. |
| **VSP-004 B** | Depth Assist contextual equilibrado | Feedback discreto quando profundidade pouco legível; comparação espacial precisa sob demanda; Companion View sugerida, fixável, auto-abertura apenas opt-in; não mover câmera principal. |
| **VSP-005 C** | Inspeção não destrutiva primeiro, edição contextual explícita | Section Assist, Thickness Analysis e Organic Profiles informativos por padrão; em shapes com perfis paramétricos editáveis, `Edit Profile` explícito; preserve representação de origem, histórico, transações e Undo/Redo. |

### VSP-002: regras adicionais de sombreamento e contorno

- Clay Soft é **modo de trabalho**, não material da cena nem fonte de iluminação exportada.
- Light Rig de modelagem com `camera-relative` ou `world-fixed`; inspeção de luz separada da câmera, sem modificar luzes autorais.
- Contornos categorizados: silhouette, feature edge, suggestive contour (avançado), occluded edges (inspeção), interactive outlines (hover/active/selected). Silhueta e feature edges antes de contornos de curvatura complexos.
- Cavity/Ridge distintos por escala; macroforma preferencial em blockout; microdetalhe opcional em escultura.
- `Geometry Truth` como modo/diagnóstico diferenciar geometria real, normais suavizadas, normal maps, sombras e interferência real.

### VSP-003: regras adicionais de dimensão

- Não confundir `World AABB`, bounds locais e bounds orientados por frame do objeto (nem supor PCA exata para todos os casos).
- Avaliação padrão das dimensões sobre geometria **evaluated** quando suportada; `source` e parâmetros originais disponíveis de modo distinto.
- Alterar **medida exibida** não implica alterar geometria; `Transform Dimension` e `Edit Shape Parameter` são operações distintas.
- Perfis orgânicos usam linha central e seção local. Áreas somente quando contornos válidos fechados; múltiplos contornos conservados. Sem inventar anatomia em malhas importadas.
- Medidas transient, pinned e document annotation possuem persistência distinta; referências revisadas em alteração topológica.

### VSP-004: regras adicionais de profundidade

- Distinguir distância euclidiana, deslocamento projetado em um eixo, profundidade na câmera, distância mínima entre superfícies, contato e interseção volumétrica.
- `Depth X-Ray` não altera política de Select Through. `Depth Map` exige legenda e espaço de referência; depth buffer das superfícies visíveis não implica mapa de tudo que está oculto.
- Companion View é uma câmera secundária com seleção/preview compartilhados e estado de câmera independente; ortográfica por padrão, pinned ou temporária, manipulável por teclado; sem sugestão repetitiva e sem mudar de orientação durante drag.
- Heurística pode estimar variação projetada via jacobiana da câmera ou método equivalente. Auto-view não é obrigatória; usuário mantém controle.

### VSP-005: regras adicionais de seção/espessura

- Separar `GPU clipping`, interseção geométrica plano-superfície e `Geometry Split` real. Seção sintética **não é face selecionável** como geometria modelável.
- Seção plana pode ter zero, um ou vários contornos; buracos e contornos abertos não devem ser preenchidos como sólido fechado fictício.
- `Wall Thickness`, `Directional Thickness`, `Section Width/Depth` e `Shape Diameter` são grandezas diferentes. Qualidade e método sempre rastreáveis.
- `Thickness Probe`, mapas de espessura, seções de membros, inspeção de perfil ao longo de linhas centrais manuais ou estruturais; sem exigência de extração automática de skeleton para MVP.
- Controle de plano de corte separado do gizmo universal; coordenadas próprias, inversão de lado e entrada numérica; seções salvas/nomeadas permitidas; Companion View de VSP-004 reaproveitada.

## 1.3 Modelagem — VSP-006 (decisões derivadas pelo responsável)

**Contexto:** após aprovar VSP-005 C, o responsável autorizou explicitamente tomar decisões adicionais coerentes. Estas **não** significam implementação existente.

| ID interno | Diretriz derivada |
|---|---|
| MF-001 | Previews não destrutivas e reversíveis para operações de geometria. |
| MF-002 | Extrude/Inset/Bevel: região ativa, estado anterior, direção/parâmetro dominante. |
| MF-003 | Boolean: resultado ghosted, volumes de interseção e diagnósticos separados de efeitos visuais. |
| MF-004 | Mirror/Symmetry: plano discreto e mapeamento de correspondência; não impor simetria. |
| MF-005 | Deformações orgânicas: campo de influência, silhueta, referências dimensionais. |
| MF-006 | Avisos geométricos contextuais, sem interrupção e sem corrigir automaticamente. |
| MF-007 | Ações incompatíveis detectadas na fase de prévia; nada é silenciosamente ignorado. |
| MF-008 | Ativar ajuda não muda ferramentas/keymaps e não altera dados sem comando explícito. |

## 1.4 Petunia Smart Rig — RIG-001 a RIG-012

| ID | Decisão aprovada | Obrigação |
|---|---|---|
| RIG-001 | Autorig opcional | Rig nunca obrigatório para modelar. |
| RIG-002 | Assisted Auto-Rig como padrão *do fluxo de autorig* | Sugerir skeleton/joints/pesos e permitir revisar/corrigir antes de confirmar. |
| RIG-003 | Live Auto-Rig opt-in | Rig evolui incrementalmente com alterações compatíveis de partes estruturais, mediante ativação explícita. |
| RIG-004 | Rig manual completo | Usuário pode começar do zero, adicionar/editar/remover joints e pesos sem IA ou preset. |
| RIG-005 | Manual Overrides protegidos | Correções e bloqueios manuais prevalecem sobre regeneração. |
| RIG-006 | Rig incremental ligado a Structural Shape Graph | Partes/volumes têm identidade e papéis semânticos distintos da mesh renderizada. |
| RIG-007 | Weights por regiões afetadas | Recalcular incrementalmente quando possível, globalmente apenas com motivo e preview. |
| RIG-008 | Pose Test durante modelagem | Pode experimentar poses antes de finalizar a forma, preservando rest pose. |
| RIG-009 | Geração/bind/regeneração transacionais | Preview, confirmação, cancel, Undo/Redo atômicos. |
| RIG-010 | IA opcional | Nenhum serviço externo nem GPU de IA obrigatório para workflow básico. |
| RIG-011 | Anatomias variadas | Humanoides, quadrúpedes, aves, peixes, criaturas não convencionais, estruturas personalizadas. |
| RIG-012 | Off nunca apaga rig | Desabilitar autoatualização, desabilitar deformação e remover rig são três operações diferentes. |

**Decisões adicionais de coerência:** edição de hierarquia e `RigRole`, IK, weight painting, simetria, influência rígida por parte, preview de proteção; preservação de `bone_id` e animações na evolução; para formas importadas, presets/landmarks e IA opcional. Nenhuma topologia nova pode reatribuir pesos silenciosamente a índices reutilizados.

## 1.5 O que não foi decidido / restrições de projeto

- Nenhum keymap completo, tamanho definitivo de hit target, valores absolutos de tolerância de snap, frames por segundo garantidos, algoritmo definitivo de contornos, kernel CAD definitivo ou provedores de IA obrigatórios foram aprovados. São itens **a prototipar, medir e validar**, não valores normativos.
- Não há autorização implícita para substituir Qt Widgets, OpenGL ou o formato `.petunia`.
- A abordagem Shape Skeleton/Structural Shape Graph é aprovada como direção de produto, mas sua API, forma persistida e geometria de blend precisam ser validadas contra o domínio atual.
- O status de `RIG`/VSP/SEL é **Approved**; implementação deve ser marcada por recurso após auditoria, jamais por aprovação documental.
