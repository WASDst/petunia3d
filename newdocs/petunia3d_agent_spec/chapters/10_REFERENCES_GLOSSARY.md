# 10 — Referências externas, repositório e glossário

> Links abaixo servem como referência de conceito, documentação ou pesquisa. Não significam licença para reutilizar código, ícones, screenshots ou recursos proprietários. Diagramas SVG deste pacote são originais. Bibliotecas/algoritmos precisam de revisão de licença compatível com GPL-3.0-or-later e dependências de distribuição antes de integração.

## 10.1 Repositório e fontes de verdade

- Código: https://github.com/WASDst/petunia3d
- Commit de migração Qt: https://github.com/WASDst/petunia3d/commit/584f83573d3049009524b4718c6f1194b58f1b8a
- `src/main.rs`, `Cargo.toml`, `crates/editor/`, `crates/ui-cxxqt/`, `crates/render/`, `crates/project/`, `crates/core/`.
- `AGENTS.md`, `website/docs/manifest.json`, `website/docs/00-philosophy/decision-register.md`, `website/docs/16-code-agents/implementation-protocol.md`, `website/progress/tasks.json` (revalidar toda revisão).
- CI vista em 15/10/2026: https://github.com/WASDst/petunia3d/actions
- Qt doc de integração histórica: `website/docs/09-ui/qt-integration.md` (tem texto pré-migração; reconciliar).

## 10.2 Referências de produto e UX

- Shapr3D — seção e medição: https://support.shapr3d.com/hc/en-us (buscar `Section View` e `Measure mode` na documentação oficial).
- Rhino — Gumball, clipping planes e display modes: https://docs.mcneel.com/rhino/8/help/en-us/
- Blender — viewport shading, overlays, skinning, curves: https://docs.blender.org/manual/en/latest/
- SketchUp — Scale e Measurements: https://help.sketchup.com/en/sketchup/scaling-geometry
- Autodesk Fusion — section, interference e inspect: https://help.autodesk.com/view/fusion360/ENU/
- Nomad Sculpt — manual de shading e sculpt: https://nomadsculpt.com/manual/
- ZBrush — MatCap/ZSpheres: https://help.maxon.net/zbr/en-us/
- 3D Slicer — layout/slice linked views: https://slicer.readthedocs.io/en/latest/
- VRoid Studio — editor de personagens: https://vroid.com/en/studio
- MakeHuman — humanoides paramétricos: http://www.makehumancommunity.org/
- Substance 3D Modeler — volumes/primitive workflows: https://helpx.adobe.com/substance-3d-modeler/

## 10.3 Pesquisa e algoritmos

- Borosan et al., *RigMesh: Automatic Rigging for Part-Based Shape Modeling and Deformation* (SIGGRAPH Asia 2012): https://cragl.cs.gmu.edu/rigmesh/
- Igarashi et al., *Teddy: A Sketching Interface for 3D Freeform Design* (SIGGRAPH 1999): https://doi.org/10.1145/311535.311602
- DeCarlo et al., *Suggestive Contours for Conveying Shape* (SIGGRAPH 2003): https://gfx.cs.princeton.edu/gfx/pubs/DeCarlo_2003_SCF/index.php
- Shapira et al., *Consistent Mesh Partitioning and Skeletonisation using the Shape Diameter Function* (2008): consultar publicação original/ACM; SDF aqui **não** é Signed Distance Field.
- *Pop-up depth views for improving 3D target acquisition* (Graphics Interface 2011): https://graphicsinterface.org/proceedings/gi2011/gi2011-6/
- libigl tutorial (skinning/BBH/mesh processing): https://libigl.github.io/tutorial/
- CGAL Polygon Mesh Processing / Polygon_mesh_slicer: https://doc.cgal.org/latest/Polygon_mesh_processing/index.html
- UniRig open-source research: https://github.com/VAST-AI-Research/UniRig (avaliar licença/dependências/condições específicas antes de usar)
- RigAnything — pesquisa Adobe: https://research.adobe.com/publication/riganything-template-free-autoregressive-rigging-for-diverse-3d-assets/
- WCAG 2.2, contraste não textual e arrasto (princípios adaptáveis, não certificação Qt): https://www.w3.org/TR/WCAG22/
- Qt Accessibility (referência técnica atual): https://doc.qt.io/qt-6/accessible.html
- Qt QOpenGLWidget: https://doc.qt.io/qt-6/qopenglwidget.html
- CXX-Qt: https://kdab.github.io/cxx-qt/

## 10.4 Glossário normativo

| Termo | Definição neste pacote |
|---|---|
| **SelectionSet** | refs selecionadas, preservadas sem duplicatas e com ordem |
| **ActiveElement** | elemento ativo dentro da seleção; não é toda seleção |
| **ResolvedTransformSet** | targets válidos de transformação após expansão/deduplicação |
| **Pivot** | centro matemático de Rotate/Scale; não é necessariamente posição visual do gizmo |
| **Orientation Frame** | eixos operacionais Local/World/Normal/Custom/etc. |
| **Gizmo** | manipulador universal de Move/Rotate/Scale/Pivot |
| **Snap Candidate** | entidade elegível para restrição/encaixe durante determinada operação |
| **Evaluated Geometry** | geometria após stack de modificadores/avaliação disponível à visualização |
| **Volume Frame** | limites e cotas dimensinais em referencial nomeado |
| **Companion View** | câmera secundária ortográfica ou custom sincronizada, independente da principal |
| **Clipping Plane** | plano que oculta pixels no renderer sem mudar a geometria |
| **Section Contour** | resultado geométrico de interseção plano-superfície |
| **Thickness** | grandeza definida por método/associação, não sinônimo de largura de objeto |
| **Organic Profile** | seção transversal em frame ao longo de linha central |
| **Structural Shape Graph** | estrutura de partes e conexões, independente da malha renderizada |
| **Rest Pose** | estado de referência do rig/bind, distinto da pose atual |
| **Auto-Rig Assisted** | gerar propostas, corrigir, confirmar explicitamente |
| **Live Auto-Rig** | atualizar por mudanças de shape compatíveis, ativado manualmente |
| **Manual Override** | proteção de joints/weights/articulação definidos pelo artista |
| **Rig Role** | classificação semântica de joint (por exemplo, ombro, cotovelo) |
| **Skin Weights** | influência dos joints sobre vértices/superfícies |
| **LBS/DQS** | Linear Blend Skinning / Dual Quaternion Skinning |
| **ExactForRepresentation** | medida determinada para geometria atual, sem implicar realidade física perfeita |
| **Transient/Pinned/Annotation** | dado durante gesto / fixado na sessão / persistido explicitamente no documento |
| **Vertical Slice** | recurso mínimo fim-a-fim com domínio, UI, testes e evidência |

## 10.5 Legibilidade e proveniência das imagens

Os arquivos SVG em `assets/` representam esquemas de arquitetura e UX feitos para esta documentação. Não são captura de tela nem indicam implementação. Em testes de produto, preferir fixtures 3D e screenshots produzidos no próprio Petunia3D, ou assets com atribuição/licença explícita conforme `AGENTS.md`.
