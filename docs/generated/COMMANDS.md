---
title: Catálogo de Comandos & Ferramentas
description: Catálogo canônico de comandos gerados a partir do CommandDispatcher (P3D-119)
---

<!--
  ARQUIVO GERADO AUTOMATICAMENTE — NÃO EDITE MANUALMENTE!
  Gerado deterministicamente por `cargo xtask docs` (P3D-119).
  Para atualizar execute: cargo run -p xtask -- docs
-->

# Catálogo Canônico de Comandos e Ações (`CommandId`)

> **Single Source of Truth (P3D-100, P3D-119)**
> Todos os comandos do Petunia3D são registrados centralmente no `CommandDispatcher`, permitindo despacho transacional com histórico (Undo/Redo), Command Palette e telemetria.

Total de comandos registrados no motor: **97**.

## Tabela Geral de Comandos

| ID (`CommandId`) | Rótulo | Categoria | Destrutivo | Tópico Docs | Descrição |
| :--- | :--- | :---: | :---: | :--- | :--- |
| `edit.delete` | **Delete** | `Edit` | Sim | — | Delete selected elements or active object |
| `edit.duplicate` | **Duplicate** | `Edit` | Sim | — | Duplicate selected elements or active object |
| `edit.redo` | **Redo** | `Edit` | Não | — | Redo last undone modification |
| `edit.undo` | **Undo** | `Edit` | Não | — | Undo previous modification |
| `global.redo` | **Redo** | `Edit` | Não | — | Redo last undone modification |
| `global.undo` | **Undo** | `Edit` | Não | — | Undo previous modification |
| `model.delete` | **Delete** | `Edit` | Sim | — | Delete selected elements or active object |
| `model.duplicate` | **Duplicate** | `Edit` | Sim | — | Duplicate selected elements or active object |
| `file.export_glb` | **Export GLB** | `File` | Não | ImportExport | Export scene to binary glTF format |
| `file.export_obj` | **Export OBJ** | `File` | Não | ImportExport | Export active mesh to Wavefront OBJ format |
| `file.import_obj` | **Import OBJ** | `File` | Não | ImportExport | Import 3D mesh from Wavefront OBJ file |
| `file.new` | **New Project** | `File` | Sim | GettingStarted | Create a blank 3D project |
| `file.open` | **Open Project** | `File` | Não | GettingStarted | Open a Petunia3D project from disk |
| `file.save` | **Save Project** | `File` | Sim | GettingStarted | Save active project to disk |
| `file.save_as` | **Save Project As** | `File` | Não | — | Save active project to a new file |
| `file.save_asset` | **Save Active Model as Asset** | `File` | Sim | Assets | Save active mesh to project asset library |
| `help.documentation` | **Documentation** | `Help` | Não | GettingStarted | Open official documentation online |
| `model.add_capsule` | **Add Capsule** | `Model` | Não | Modeling | Add a capsule primitive |
| `model.add_circle` | **Add Circle** | `Model` | Não | Modeling | Add a circle/disc primitive |
| `model.add_cone` | **Add Cone** | `Model` | Não | Modeling | Add a cone primitive |
| `model.add_cube` | **Add Cube** | `Model` | Não | Modeling | Add a 3D box primitive |
| `model.add_cylinder` | **Add Cylinder** | `Model` | Não | Modeling | Add a cylinder primitive |
| `model.add_icosphere` | **Add Icosphere** | `Model` | Não | Modeling | Add an icosphere primitive |
| `model.add_plane` | **Add Plane** | `Model` | Não | Modeling | Add a flat plane primitive |
| `model.add_sphere` | **Add Sphere** | `Model` | Não | Modeling | Add a low-poly sphere primitive |
| `model.add_torus` | **Add Torus** | `Model` | Não | Modeling | Add a torus primitive |
| `model.add_wedge` | **Add Wedge** | `Model` | Não | Modeling | Add a wedge/ramp primitive |
| `model.bevel` | **Bevel Edges** | `Model` | Sim | Bevel | Bevel selected mesh edges |
| `model.connect` | **Connect Loops** | `Model` | Sim | Modeling | Connect two faces or boundary loops with quads and triangles |
| `model.cut` | **Cut** | `Model` | Não | Modeling | Subtract the boolean operand from the active object |
| `model.dissolve` | **Dissolve** | `Model` | Sim | Modeling | Dissolve selected edges, faces, or vertices cleanly |
| `model.extrude` | **Extrude** | `Model` | Sim | Extrude | Extrude selected faces along surface normal |
| `model.extrude_individual` | **Extrude Individual** | `Model` | Sim | Extrude | Extrude selected faces individually |
| `model.flip_diagonal` | **Flip Diagonal** | `Model` | Sim | Modeling | Flip quad internal diagonal or triangle edge |
| `model.flip_normals` | **Flip Normals** | `Model` | Sim | Modeling | Reverse orientation of face normals |
| `model.fuse` | **Fuse** | `Model` | Não | Modeling | Combine the active object with the boolean operand into one |
| `model.inset` | **Inset Faces** | `Model` | Sim | Modeling | Inset selected faces towards interior |
| `model.instantiate_asset` | **Instantiate Asset** | `Model` | Sim | Assets | Instantiate a library asset into the active 3D scene |
| `model.intersect` | **Intersect** | `Model` | Não | Modeling | Keep only the volume shared with the boolean operand |
| `model.join` | **Join** | `Model` | Não | Modeling | Merge the operand into the active object keeping both topologies |
| `model.knife` | **Knife** | `Model` | Não | Knife | Cut the active mesh along edge points picked in the viewport |
| `model.loop_cut` | **Loop Cut** | `Model` | Sim | LoopCut | Insert evenly spaced cuts along a quad ring |
| `model.merge` | **Merge Center** | `Model` | Sim | Modeling | Merge selected vertices into center point |
| `model.push_pull` | **Push/Pull** | `Model` | Não | Modeling | Push or pull selected faces along the surface normal |
| `model.revolve` | **Revolve 360** | `Model` | Sim | Modeling | Revolve selected profile 360 degrees around an axis |
| `model.scale_selection` | **Scale Selection** | `Model` | Sim | Modeling | Scale selected geometry uniformly around its center |
| `model.separate_selection` | **Separate Selection** | `Model` | Sim | Modeling | Separate selected geometry into a new object |
| `model.spin` | **Spin** | `Model` | Sim | Modeling | Extrude and spin selected geometry in an arc around axis |
| `model.subdivide` | **Subdivide** | `Model` | Sim | LoopCut | Subdivide selected geometry |
| `model.symmetrize` | **Symmetrize** | `Model` | Sim | Modeling | Copy one side to the other across an axis |
| `model.weld` | **Merge by Distance** | `Model` | Sim | Modeling | Weld duplicate vertices within distance |
| `model.deselect_all` | **Deselect All** | `Select` | Não | — | Clear current geometry selection |
| `model.invert_selection` | **Invert Selection** | `Select` | Não | — | Invert geometry selection in active mesh |
| `model.select_all` | **Select All** | `Select` | Não | — | Select all geometry elements in active mesh |
| `select.all` | **Select All** | `Select` | Não | — | Select all geometry elements in active mesh |
| `select.cycle_domain` | **Cycle Selection Domain** | `Select` | Não | — | Toggle between Object and last component domain |
| `select.domain_edge` | **Select Domain: Edge** | `Select` | Não | — | Switch interaction to Edge domain |
| `select.domain_face` | **Select Domain: Face** | `Select` | Não | — | Switch interaction to Face domain |
| `select.domain_object` | **Select Domain: Object** | `Select` | Não | — | Switch interaction to Object domain |
| `select.domain_vertex` | **Select Domain: Point** | `Select` | Não | — | Switch interaction to Point domain |
| `select.invert` | **Invert Selection** | `Select` | Não | — | Invert geometry selection in active mesh |
| `select.linked` | **Select Linked** | `Select` | Não | — | Select connected geometry elements |
| `select.none` | **Deselect All** | `Select` | Não | — | Clear current geometry selection |
| `paint.bake_decal` | **Bake Decal to Raster** | `Tools` | Sim | TexturePainting | Bake a live decal layer into a static raster layer |
| `paint.bake_reference` | **Bake Reference to Texture** | `Tools` | Sim | TexturePainting | Project UVs and bake visible reference image into the active texture |
| `paint.set_decal_transform` | **Set Decal Transform** | `Tools` | Sim | TexturePainting | Update position, scale and rotation of a decal layer |
| `tools.toggle_proportional` | **Toggle Proportional Editing** | `Tools` | Não | Modeling | Toggle proportional editing with falloff radius |
| `tools.toggle_snap` | **Toggle Magnet Snap** | `Tools` | Não | Modeling | Toggle snapping to grid, vertices, edges or faces |
| `uv.pack_islands` | **Pack UV Islands** | `Tools` | Sim | UvUnwrapping | Pack UV islands into 0..1 without overlaps |
| `uv.project_reference` | **Project From Reference** | `Tools` | Sim | UvUnwrapping | Project UVs from the active reference image or fallback to camera view |
| `uv.project_view` | **Project From View** | `Tools` | Sim | UvUnwrapping | Project UVs from the current camera view |
| `uv.relax` | **Relax UVs** | `Tools` | Sim | UvUnwrapping | Smooth and relax UV coordinates to reduce stretching |
| `uv.stitch` | **Stitch UV Seams** | `Tools` | Sim | UvUnwrapping | Weld and align separated UV seams on shared 3D edges |
| `uv.unwrap_auto` | **Auto UV** | `Tools` | Sim | UvUnwrapping | Unwrap the active mesh with the generic UV provider |
| `view.back` | **View Back** | `View` | Não | Navigation | Align camera to Back orthographic view |
| `view.bottom` | **View Bottom** | `View` | Não | Navigation | Align camera to Bottom orthographic view |
| `view.frame_all` | **Frame All** | `View` | Não | Navigation | Center 3D camera on all visible scene geometry |
| `view.frame_selection` | **Frame Selection** | `View` | Não | Navigation | Center 3D camera on selected geometry |
| `view.front` | **View Front** | `View` | Não | Navigation | Align camera to Front orthographic view |
| `view.isometric_ne` | **Isometric NE** | `View` | Não | Navigation | Align camera to North-East isometric view |
| `view.isometric_nw` | **Isometric NW** | `View` | Não | Navigation | Align camera to North-West isometric view |
| `view.isometric_se` | **Isometric SE** | `View` | Não | Navigation | Align camera to South-East isometric view |
| `view.isometric_sw` | **Isometric SW** | `View` | Não | Navigation | Align camera to South-West isometric view |
| `view.left` | **View Left** | `View` | Não | Navigation | Align camera to Left orthographic view |
| `view.reset_camera` | **Reset Camera** | `View` | Não | Navigation | Reset 3D camera to default isometric view |
| `view.right` | **View Right** | `View` | Não | Navigation | Align camera to Right orthographic view |
| `view.toggle_face_orientation` | **Toggle Face Orientation** | `View` | Não | Navigation | Display front faces in blue and back faces in red |
| `view.toggle_nav_hud` | **Toggle Navigation HUD** | `View` | Não | Navigation | Toggle display of viewport orientation angle badge |
| `view.toggle_projection` | **Toggle Projection** | `View` | Não | Navigation | Toggle perspective or orthographic view |
| `view.toggle_uv_checker` | **Toggle UV Checker** | `View` | Não | UvUnwrapping | Display procedural checkerboard pattern for UV inspection |
| `view.toggle_wire_overlay` | **Toggle Wire Overlay** | `View` | Não | Navigation | Display mesh edges over the active shading mode |
| `view.toggle_wireframe` | **Toggle Wireframe** | `View` | Não | Navigation | Toggle wireframe display on active mesh |
| `view.toggle_xray` | **Toggle X-Ray** | `View` | Não | Navigation | Toggle semi-transparent see-through mesh display |
| `view.top` | **View Top** | `View` | Não | Navigation | Align camera to Top orthographic view |
| `window.command_palette` | **Command Palette** | `Window` | Não | Interface | Open rapid search and execute palette |
| `window.reference_manager` | **Reference Set Manager** | `Window` | Não | Interface | Open reference images manager window |
| `window.settings` | **Preferences** | `Window` | Não | Themes | Open application preferences modal |

