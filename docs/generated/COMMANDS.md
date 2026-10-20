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

Total de comandos registrados no motor: **241**.

## Tabela Geral de Comandos

| ID (`CommandId`) | Rótulo | Categoria | Destrutivo | Tópico Docs | Descrição |
| :--- | :--- | :---: | :---: | :--- | :--- |
| `app.redo` | **Redo** | `Edit` | Não | — | Redo last undone modification |
| `app.undo` | **Undo** | `Edit` | Não | — | Undo previous modification |
| `edit.delete` | **Delete** | `Edit` | Sim | — | Delete selected elements or active object |
| `edit.delete_selected` | **Delete** | `Edit` | Sim | — | Delete selected elements or active object |
| `edit.duplicate` | **Duplicate** | `Edit` | Sim | — | Duplicate selected elements or active object |
| `edit.duplicate_selected` | **Duplicate** | `Edit` | Sim | — | Duplicate selected elements or active object |
| `edit.redo` | **Redo** | `Edit` | Não | — | Redo last undone modification |
| `edit.undo` | **Undo** | `Edit` | Não | — | Undo previous modification |
| `global.redo` | **Redo** | `Edit` | Não | — | Redo last undone modification |
| `global.undo` | **Undo** | `Edit` | Não | — | Undo previous modification |
| `model.delete` | **Delete** | `Edit` | Sim | — | Delete selected elements or active object |
| `model.delete_selected` | **Delete** | `Edit` | Sim | — | Delete selected elements or active object |
| `model.duplicate` | **Duplicate** | `Edit` | Sim | — | Duplicate selected elements or active object |
| `model.duplicate_selected` | **Duplicate** | `Edit` | Sim | — | Duplicate selected elements or active object |
| `file.export_glb` | **Export GLB** | `File` | Não | ImportExport | Export scene to binary glTF format |
| `file.export_obj` | **Export OBJ** | `File` | Não | ImportExport | Export active mesh to Wavefront OBJ format |
| `file.import_gltf` | **Import glTF / GLB** | `File` | Não | ImportExport | Import 3D mesh with materials and textures from glTF or GLB file |
| `file.import_obj` | **Import OBJ** | `File` | Não | ImportExport | Import 3D mesh from Wavefront OBJ file |
| `file.new` | **New Project** | `File` | Não | GettingStarted | Create a blank 3D project |
| `file.open` | **Open Project** | `File` | Não | GettingStarted | Open a Petunia3D project from disk |
| `file.save` | **Save Project** | `File` | Não | GettingStarted | Save active project to disk |
| `file.save_as` | **Save Project As** | `File` | Não | — | Save active project to a new file |
| `file.save_asset` | **Save Active Model as Asset** | `File` | Não | Assets | Save active mesh to project asset library |
| `help.documentation` | **Documentation** | `Help` | Não | GettingStarted | Open official documentation online |
| `add.capsule` | **Add Capsule** | `Model` | Não | Modeling | Add a capsule primitive |
| `add.circle` | **Add Circle** | `Model` | Não | Modeling | Add a circle/disc primitive |
| `add.cone` | **Add Cone** | `Model` | Não | Modeling | Add a cone primitive |
| `add.cube` | **Add Cube** | `Model` | Não | Modeling | Add a 3D box primitive |
| `add.cylinder` | **Add Cylinder** | `Model` | Não | Modeling | Add a cylinder primitive |
| `add.icosphere` | **Add Icosphere** | `Model` | Não | Modeling | Add an icosphere primitive |
| `add.plane` | **Add Plane** | `Model` | Não | Modeling | Add a flat plane primitive |
| `add.sphere` | **Add Sphere** | `Model` | Não | Modeling | Add a low-poly sphere primitive |
| `add.torus` | **Add Torus** | `Model` | Não | Modeling | Add a torus primitive |
| `add.wedge` | **Add Wedge** | `Model` | Não | Modeling | Add a wedge/ramp primitive |
| `align.back` | **Align Back (Min Z)** | `Model` | Sim | Modeling | Align back edges of selected objects along Z axis |
| `align.bottom` | **Align Bottom (Min Y)** | `Model` | Sim | Modeling | Align bottom base of selected objects along Y axis |
| `align.center_x` | **Align Centers X** | `Model` | Sim | Modeling | Align centers of selected objects along X axis |
| `align.center_y` | **Align Centers Y** | `Model` | Sim | Modeling | Align centers of selected objects along Y axis |
| `align.center_z` | **Align Centers Z** | `Model` | Sim | Modeling | Align centers of selected objects along Z axis |
| `align.front` | **Align Front (Max Z)** | `Model` | Sim | Modeling | Align front edges of selected objects along Z axis |
| `align.left` | **Align Left (Min X)** | `Model` | Sim | Modeling | Align minimum edges of selected objects along X axis |
| `align.right` | **Align Right (Max X)** | `Model` | Sim | Modeling | Align maximum edges of selected objects along X axis |
| `align.top` | **Align Top (Max Y)** | `Model` | Sim | Modeling | Align top edges of selected objects along Y axis |
| `asset.join_into_active` | **Join Asset Into Active** | `Model` | Sim | Modeling | Merge another asset's mesh into the active asset and remove the source asset |
| `asset.set_transform` | **Set Object Transform** | `Model` | Sim | Modeling | Set absolute position, rotation (Euler XYZ) and scale of an object |
| `distribute.x` | **Distribute Objects X** | `Model` | Sim | Modeling | Distribute spacing between selected objects along X axis |
| `distribute.y` | **Distribute Objects Y** | `Model` | Sim | Modeling | Distribute spacing between selected objects along Y axis |
| `distribute.z` | **Distribute Objects Z** | `Model` | Sim | Modeling | Distribute spacing between selected objects along Z axis |
| `draw.exclude` | **Exclude** | `Model` | Sim | Modeling | Remove the areas where shapes overlap |
| `draw.intersect` | **Intersect Shapes** | `Model` | Sim | Modeling | Keep only the area all closed shapes share |
| `draw.subtract` | **Subtract** | `Model` | Sim | Modeling | Cut the shapes in front out of the shape at the back |
| `draw.unite` | **Unite** | `Model` | Sim | Modeling | Merge the closed shapes of the sketch plane into one |
| `material.assign_slot` | **Assign Material Slot** | `Model` | Sim | Materials | Assign a material slot to the active object or selected faces |
| `material.create` | **Create Material** | `Model` | Sim | Materials | Create a new standard PBR material in the project |
| `material.duplicate` | **Duplicate Material** | `Model` | Sim | Materials | Duplicate the active or selected material |
| `material.edit` | **Edit Material** | `Model` | Sim | Materials | Edit a scalar or color property of a material |
| `material.remove` | **Remove Material** | `Model` | Sim | Materials | Remove the specified material from the project |
| `material.set_properties` | **Set Material Properties** | `Model` | Sim | Materials | Set multiple PBR properties of a material in a single atomic transaction |
| `measure.clear` | **Clear Measurements** | `Model` | Sim | Modeling | Remove every measurement made with the Measure tool |
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
| `model.align_center_x` | **Align Centers X** | `Model` | Sim | Modeling | Align centers of selected objects along X axis |
| `model.align_center_y` | **Align Centers Y** | `Model` | Sim | Modeling | Align centers of selected objects along Y axis |
| `model.align_center_z` | **Align Centers Z** | `Model` | Sim | Modeling | Align centers of selected objects along Z axis |
| `model.align_max_x` | **Align Right (Max X)** | `Model` | Sim | Modeling | Align maximum edges of selected objects along X axis |
| `model.align_max_y` | **Align Top (Max Y)** | `Model` | Sim | Modeling | Align top edges of selected objects along Y axis |
| `model.align_max_z` | **Align Front (Max Z)** | `Model` | Sim | Modeling | Align front edges of selected objects along Z axis |
| `model.align_min_x` | **Align Left (Min X)** | `Model` | Sim | Modeling | Align minimum edges of selected objects along X axis |
| `model.align_min_y` | **Align Bottom (Min Y)** | `Model` | Sim | Modeling | Align bottom base of selected objects along Y axis |
| `model.align_min_z` | **Align Back (Min Z)** | `Model` | Sim | Modeling | Align back edges of selected objects along Z axis |
| `model.align_objects` | **Align Objects** | `Model` | Sim | Modeling | Align selected objects in 3D space |
| `model.align_workplane_to_face` | **Align Workplane to Face** | `Model` | Não | Modeling | Align drawing workplane to the specified planar face |
| `model.align_workplane_to_selection` | **Align Workplane to Selection** | `Model` | Não | Modeling | Align drawing workplane to selected faces or opposing face gap |
| `model.apply_modifier` | **Apply Modifier** | `Model` | Sim | Modeling | Bake modifier deformation permanently into base mesh |
| `model.assign_material` | **Assign Material Slot** | `Model` | Sim | Materials | Assign a material slot to the active object or selected faces |
| `model.assign_material_slot` | **Assign Material Slot** | `Model` | Sim | Materials | Assign a material slot to the active object or selected faces |
| `model.bevel` | **Bevel Edges** | `Model` | Sim | Bevel | Bevel selected mesh edges |
| `model.clear_all_transforms` | **Clear All Transforms** | `Model` | Sim | Modeling | Reset object position, rotation and scale to default |
| `model.clear_location` | **Clear Location** | `Model` | Sim | Modeling | Reset object location to [0, 0, 0] |
| `model.clear_rotation` | **Clear Rotation** | `Model` | Sim | Modeling | Reset object rotation to [0, 0, 0] |
| `model.clear_scale` | **Clear Scale** | `Model` | Sim | Modeling | Reset object scale to [1, 1, 1] |
| `model.clear_sharp` | **Clear Sharp** | `Model` | Sim | Modeling | Remove the sharp mark from the selected edges |
| `model.connect` | **Connect Loops** | `Model` | Sim | Modeling | Connect two faces or boundary loops with quads and triangles |
| `model.contextual_push_pull` | **Contextual Push/Pull** | `Model` | Não | Modeling | Push or pull surface region: boundary cap stretches or embedded region extrudes |
| `model.create_material` | **Create Material** | `Model` | Sim | Materials | Create a new standard PBR material in the project |
| `model.dissolve` | **Dissolve** | `Model` | Sim | Modeling | Dissolve selected edges, faces, or vertices cleanly |
| `model.distribute_objects_x` | **Distribute Objects X** | `Model` | Sim | Modeling | Distribute spacing between selected objects along X axis |
| `model.distribute_objects_y` | **Distribute Objects Y** | `Model` | Sim | Modeling | Distribute spacing between selected objects along Y axis |
| `model.distribute_objects_z` | **Distribute Objects Z** | `Model` | Sim | Modeling | Distribute spacing between selected objects along Z axis |
| `model.duplicate_material` | **Duplicate Material** | `Model` | Sim | Materials | Duplicate the active or selected material |
| `model.extrude` | **Extrude** | `Model` | Sim | Extrude | Extrude selected faces along surface normal |
| `model.extrude_individual` | **Extrude Individual** | `Model` | Sim | Extrude | Extrude selected faces individually |
| `model.flip_diagonal` | **Flip Diagonal** | `Model` | Sim | Modeling | Flip quad internal diagonal or triangle edge |
| `model.flip_faces` | **Flip Faces** | `Model` | Sim | Modeling | Reverse the winding of the selected faces |
| `model.flip_normals` | **Flip Normals** | `Model` | Sim | Modeling | Reverse orientation of face normals |
| `model.geometry_to_origin` | **Geometry to Origin** | `Model` | Sim | — | Action Geometry to Origin |
| `model.imprint_profile` | **Imprint Profile on Surface** | `Model` | Não | Modeling | Imprint profile contour and holes onto coplanar surface faces |
| `model.inset` | **Inset Faces** | `Model` | Sim | Modeling | Inset selected faces towards interior |
| `model.instantiate_asset` | **Instantiate Asset** | `Model` | Sim | Assets | Instantiate a library asset into the active 3D scene |
| `model.join` | **Join** | `Model` | Não | Modeling | Merge the other selected objects into the active object keeping every topology |
| `model.knife` | **Knife** | `Model` | Não | Knife | Cut the active mesh along edge points picked in the viewport |
| `model.loop_cut` | **Loop Cut** | `Model` | Sim | LoopCut | Insert evenly spaced cuts along a quad ring |
| `model.make_face` | **Make Face / Edge** | `Model` | Sim | Modeling | Create a new face or edge connecting selected vertices or edges (F) |
| `model.mark_sharp` | **Mark Sharp** | `Model` | Sim | Modeling | Mark the selected edges as sharp (limits smooth shading) |
| `model.merge` | **Merge Center** | `Model` | Sim | Modeling | Merge selected vertices into center point |
| `model.mirror` | **Mirror Mesh** | `Model` | Sim | Modeling | Mirror the active mesh across an axis and optionally weld the plane |
| `model.orient_consistently` | **Orient Consistently** | `Model` | Sim | Modeling | Make face winding consistent across connected faces |
| `model.orient_outward` | **Orient Outward** | `Model` | Sim | Modeling | Orient a closed mesh so faces point outside |
| `model.origin_bottom` | **Set Origin to Bottom** | `Model` | Sim | — | Action Set Origin to Bottom |
| `model.origin_cursor` | **Set Origin to 3D Cursor** | `Model` | Sim | — | Action Set Origin to 3D Cursor |
| `model.origin_geometry` | **Set Origin to Geometry** | `Model` | Sim | — | Action Set Origin to Geometry |
| `model.origin_selection` | **Set Origin to Selection** | `Model` | Sim | — | Action Set Origin to Selection |
| `model.push_pull` | **Push/Pull** | `Model` | Não | Modeling | Push or pull selected faces along the surface normal |
| `model.push_pull_apply` | **Apply Push/Pull** | `Model` | Sim | Modeling | Displace selected faces along their normal by a given distance |
| `model.remove_material` | **Remove Material** | `Model` | Sim | Materials | Remove the specified material from the project |
| `model.revolve` | **Revolve 360** | `Model` | Sim | Modeling | Revolve selected profile 360 degrees around an axis |
| `model.scale_selection` | **Scale Selection** | `Model` | Sim | Modeling | Scale selected geometry uniformly around its center |
| `model.scale_to_dimension` | **Scale To Dimension** | `Model` | Sim | Modeling | Scale object along an axis to target dimension in the specified coordinate frame |
| `model.separate_selection` | **Separate Selection** | `Model` | Sim | Modeling | Separate selected geometry into a new object |
| `model.set_material_properties` | **Set Material Properties** | `Model` | Sim | Materials | Set multiple PBR properties of a material in a single atomic transaction |
| `model.set_selection_color` | **Set Object Selection Color** | `Model` | Sim | Interface | Set authored custom selection highlight color for an object |
| `model.set_transform` | **Set Object Transform** | `Model` | Sim | Modeling | Set absolute position, rotation (Euler XYZ) and scale of an object |
| `model.shade_flat` | **Shade Flat** | `Model` | Sim | Modeling | Use faceted normals on the active object |
| `model.shade_smooth` | **Shade Smooth** | `Model` | Sim | Modeling | Interpolate normals across faces of the active object |
| `model.shape_builder_solid` | **Solid Shape Builder** | `Model` | Não | Modeling | Execute 3D solid partition operations with transient RegionPlan |
| `model.slice_plane` | **Slice Plane** | `Model` | Sim | Modeling | Slice the active mesh along a plane through the selection center |
| `model.smooth_by_angle` | **Smooth by Angle** | `Model` | Sim | Modeling | Shade smooth and mark edges above the angle as sharp |
| `model.solid_composition_bake` | **Bake Solid Composition** | `Model` | Não | Modeling | Explicitly bake solid composition recipe to editable static mesh |
| `model.solid_composition_create` | **Create Solid Composition** | `Model` | Não | Modeling | Create a re-editable solid composition from stable source operands |
| `model.solid_composition_reorder_step` | **Reorder Solid Composition Steps** | `Model` | Não | Modeling | Reorder evaluation steps in the solid composition recipe |
| `model.solid_composition_toggle_step` | **Toggle Solid Composition Step** | `Model` | Não | Modeling | Enable or disable a specific step in the solid composition recipe |
| `model.solid_edit` | **Solid Shape Builder** | `Model` | Não | Modeling | Execute 3D solid partition operations with transient RegionPlan |
| `model.solid_extract_overlap` | **Solid Extract Overlap** | `Model` | Não | Modeling | Extract overlapping region as a new scene object |
| `model.solid_remove_overlap` | **Solid Remove Overlap** | `Model` | Não | Modeling | Remove overlapping regions from all solid operands |
| `model.solid_unite` | **Solid Unite** | `Model` | Não | Modeling | Unite all partition regions into a single solid mesh |
| `model.spin` | **Spin** | `Model` | Sim | Modeling | Extrude and spin selected geometry in an arc around axis |
| `model.subdivide` | **Subdivide** | `Model` | Sim | LoopCut | Subdivide selected geometry |
| `model.surface_imprint` | **Imprint Profile on Surface** | `Model` | Não | Modeling | Imprint profile contour and holes onto coplanar surface faces |
| `model.symmetrize` | **Symmetrize** | `Model` | Sim | Modeling | Copy one side to the other across an axis |
| `model.translate_selection` | **Translate Selection** | `Model` | Sim | Modeling | Translate selected vertices by a delta vector |
| `model.triangulate` | **Triangulate** | `Model` | Sim | Modeling | Triangulate faces of the active mesh |
| `model.weld` | **Merge by Distance** | `Model` | Sim | Modeling | Weld duplicate vertices within distance |
| `modifier.apply` | **Apply Modifier** | `Model` | Sim | Modeling | Bake modifier deformation permanently into base mesh |
| `object.align` | **Align Objects** | `Model` | Sim | Modeling | Align selected objects in 3D space |
| `object.set_selection_color` | **Set Object Selection Color** | `Model` | Sim | Interface | Set authored custom selection highlight color for an object |
| `object.set_transform` | **Set Object Transform** | `Model` | Sim | Modeling | Set absolute position, rotation (Euler XYZ) and scale of an object |
| `push_pull.contextual` | **Contextual Push/Pull** | `Model` | Não | Modeling | Push or pull surface region: boundary cap stretches or embedded region extrudes |
| `scene.group` | **Group** | `Model` | Sim | Modeling | Gather the selected objects into a new group of the scene structure |
| `scene.ungroup` | **Ungroup** | `Model` | Sim | Modeling | Dissolve the group of the active object, keeping its objects |
| `solid.bake` | **Bake Solid Composition** | `Model` | Não | Modeling | Explicitly bake solid composition recipe to editable static mesh |
| `solid.composition_bake` | **Bake Solid Composition** | `Model` | Não | Modeling | Explicitly bake solid composition recipe to editable static mesh |
| `solid.composition_create` | **Create Solid Composition** | `Model` | Não | Modeling | Create a re-editable solid composition from stable source operands |
| `solid.composition_reorder` | **Reorder Solid Composition Steps** | `Model` | Não | Modeling | Reorder evaluation steps in the solid composition recipe |
| `solid.composition_toggle` | **Toggle Solid Composition Step** | `Model` | Não | Modeling | Enable or disable a specific step in the solid composition recipe |
| `solid.edit` | **Solid Shape Builder** | `Model` | Não | Modeling | Execute 3D solid partition operations with transient RegionPlan |
| `solid.extract_overlap` | **Solid Extract Overlap** | `Model` | Não | Modeling | Extract overlapping region as a new scene object |
| `solid.remove_overlap` | **Solid Remove Overlap** | `Model` | Não | Modeling | Remove overlapping regions from all solid operands |
| `solid.unite` | **Solid Unite** | `Model` | Não | Modeling | Unite all partition regions into a single solid mesh |
| `surface.imprint` | **Imprint Profile on Surface** | `Model` | Não | Modeling | Imprint profile contour and holes onto coplanar surface faces |
| `transform.scale_to_dimension` | **Scale To Dimension** | `Model` | Sim | Modeling | Scale object along an axis to target dimension in the specified coordinate frame |
| `workplane.align_face` | **Align Workplane to Face** | `Model` | Não | Modeling | Align drawing workplane to the specified planar face |
| `workplane.align_selection` | **Align Workplane to Selection** | `Model` | Não | Modeling | Align drawing workplane to selected faces or opposing face gap |
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
| `tool.toggle_proportional` | **Toggle Proportional Editing** | `Tools` | Não | Modeling | Toggle proportional editing with falloff radius |
| `tool.toggle_snap` | **Toggle Magnet Snap** | `Tools` | Não | Modeling | Toggle snapping to grid, vertices, edges or faces |
| `tools.toggle_proportional` | **Toggle Proportional Editing** | `Tools` | Não | Modeling | Toggle proportional editing with falloff radius |
| `tools.toggle_snap` | **Toggle Magnet Snap** | `Tools` | Não | Modeling | Toggle snapping to grid, vertices, edges or faces |
| `uv.equalize_texel_density` | **Equalize Texel Density** | `Tools` | Sim | UvUnwrapping | Equalize texel density across UV islands |
| `uv.normalize_texel_density` | **Normalize Texel Density** | `Tools` | Sim | UvUnwrapping | Normalize texel density to a target value for a given texture width |
| `uv.pack_islands` | **Pack UV Islands** | `Tools` | Sim | UvUnwrapping | Pack UV islands into 0..1 without overlaps |
| `uv.project_cube` | **Cube UV Projection** | `Tools` | Sim | UvUnwrapping | Project UVs using box mapping on the active mesh |
| `uv.project_cylindrical` | **Cylindrical UV Projection** | `Tools` | Sim | UvUnwrapping | Project UVs cylindrically around the Y axis on selected faces (or all) |
| `uv.project_planar` | **Planar UV Projection** | `Tools` | Sim | UvUnwrapping | Project UVs planarly on the active mesh |
| `uv.project_reference` | **Project From Reference** | `Tools` | Sim | UvUnwrapping | Project UVs from the active reference image or fallback to camera view |
| `uv.project_spherical` | **Spherical UV Projection** | `Tools` | Sim | UvUnwrapping | Project UVs spherically from the bounding center on selected faces (or all) |
| `uv.project_view` | **Project From View** | `Tools` | Sim | UvUnwrapping | Project UVs from the current camera view |
| `uv.relax` | **Relax UVs** | `Tools` | Sim | UvUnwrapping | Relax UV islands toward the 3D shape (as-rigid-as-possible), keeping pins and seams |
| `uv.split` | **Split UVs** | `Tools` | Sim | UvUnwrapping | Mark seams around the selected UV faces without duplicating vertices |
| `uv.stitch` | **Stitch UV Seams** | `Tools` | Sim | UvUnwrapping | Weld and align separated UV seams on shared 3D edges |
| `uv.unwrap_auto` | **Auto UV** | `Tools` | Sim | UvUnwrapping | Unwrap the active mesh with the generic UV provider |
| `view.toggle_snap` | **Toggle Magnet Snap** | `Tools` | Não | Modeling | Toggle snapping to grid, vertices, edges or faces |
| `model.clear_section_plane` | **Clear Section Plane** | `View` | Não | Modeling | Clear active section inspection plane |
| `model.flip_section_plane` | **Flip Section Plane** | `View` | Não | Modeling | Invert active section inspection plane normal |
| `model.offset_section_plane` | **Offset Section Plane** | `View` | Não | Modeling | Adjust active section plane offset numerically |
| `model.set_section_plane` | **Set Section Plane** | `View` | Não | Modeling | Set or update active non-destructive section inspection plane |
| `view.back` | **View Back** | `View` | Não | Navigation | Align camera to Back orthographic view |
| `view.bottom` | **View Bottom** | `View` | Não | Navigation | Align camera to Bottom orthographic view |
| `view.camera_back` | **View Back** | `View` | Não | Navigation | Align camera to Back orthographic view |
| `view.camera_bottom` | **View Bottom** | `View` | Não | Navigation | Align camera to Bottom orthographic view |
| `view.camera_front` | **View Front** | `View` | Não | Navigation | Align camera to Front orthographic view |
| `view.camera_left` | **View Left** | `View` | Não | Navigation | Align camera to Left orthographic view |
| `view.camera_right` | **View Right** | `View` | Não | Navigation | Align camera to Right orthographic view |
| `view.camera_top` | **View Top** | `View` | Não | Navigation | Align camera to Top orthographic view |
| `view.clear_section_plane` | **Clear Section Plane** | `View` | Não | Modeling | Clear active section inspection plane |
| `view.flip_section_plane` | **Flip Section Plane** | `View` | Não | Modeling | Invert active section inspection plane normal |
| `view.frame_all` | **Frame All** | `View` | Não | Navigation | Center 3D camera on all visible scene geometry |
| `view.frame_selection` | **Frame Selection** | `View` | Não | Navigation | Center 3D camera on selected geometry |
| `view.front` | **View Front** | `View` | Não | Navigation | Align camera to Front orthographic view |
| `view.isometric_ne` | **Isometric NE** | `View` | Não | Navigation | Align camera to North-East isometric view |
| `view.isometric_nw` | **Isometric NW** | `View` | Não | Navigation | Align camera to North-West isometric view |
| `view.isometric_se` | **Isometric SE** | `View` | Não | Navigation | Align camera to South-East isometric view |
| `view.isometric_sw` | **Isometric SW** | `View` | Não | Navigation | Align camera to South-West isometric view |
| `view.left` | **View Left** | `View` | Não | Navigation | Align camera to Left orthographic view |
| `view.offset_section_plane` | **Offset Section Plane** | `View` | Não | Modeling | Adjust active section plane offset numerically |
| `view.pseudo_isometric_2to1` | **Pseudo-Isometric 2:1** | `View` | Não | Navigation | Align camera to classic 2:1 pseudo-isometric game view (26.565°) |
| `view.reset_camera` | **Reset Camera** | `View` | Não | Navigation | Reset 3D camera to default isometric view |
| `view.right` | **View Right** | `View` | Não | Navigation | Align camera to Right orthographic view |
| `view.set_section_plane` | **Set Section Plane** | `View` | Não | Modeling | Set or update active non-destructive section inspection plane |
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

