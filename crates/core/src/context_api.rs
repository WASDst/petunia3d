//! Superfície de compatibilidade de `AppState` sobre o contexto restrito do editor.
//!
//! As implementações vivem em `impl CommandContext` / `impl EditorView` (sem acesso a UI
//! nem a render); `AppState` apenas delega, aplicando os efeitos ao final.

use crate::context::CommandContext;
use glam::Vec3;
use uuid::Uuid;

use super::*;
use crate::region_push::ProfileGroup;
use petunia_project::{
    Interpolation, MotionGenerator, MotionStyle, Project, ProjectChanges, RootMode, Transform3D,
};

impl AppState {
    #[inline]
    pub fn brush_world_radius(&self, hit: Vec3) -> f32 {
        self.view().brush_world_radius(hit)
    }

    #[inline]
    pub fn world_radius_for_px(&self, hit: Vec3, size_px: f32) -> f32 {
        self.view().world_radius_for_px(hit, size_px)
    }

    #[inline]
    pub fn animate_resolve(&mut self) {
        self.in_context(|ctx| ctx.animate_resolve())
    }

    #[inline]
    pub fn animate_select_skeleton(&mut self, id: Uuid) -> bool {
        self.in_context(|ctx| ctx.animate_select_skeleton(id))
    }

    #[inline]
    pub fn animate_select_bone(&mut self, bone_id: Option<u32>) {
        self.in_context(|ctx| ctx.animate_select_bone(bone_id))
    }

    #[inline]
    pub fn animate_set_ik_mode(&mut self, ik_mode: bool) {
        self.in_context(|ctx| ctx.animate_set_ik_mode(ik_mode))
    }

    #[inline]
    pub fn animate_solve_ik(
        &mut self,
        chain_id: Uuid,
        target_world: [f32; 3],
    ) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_solve_ik(chain_id, target_world))
    }

    #[inline]
    pub fn animate_cancel_ik(&mut self) {
        self.in_context(|ctx| ctx.animate_cancel_ik())
    }

    #[inline]
    pub fn animate_commit_ik(&mut self) -> Result<bool, CommandError> {
        self.in_context(|ctx| ctx.animate_commit_ik())
    }

    #[inline]
    pub fn animate_set_bone_transform(
        &mut self,
        bone_id: u32,
        transform: Transform3D,
    ) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_set_bone_transform(bone_id, transform))
    }

    #[inline]
    pub fn animate_reset_bone_pose(&mut self, bone_id: u32) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_reset_bone_pose(bone_id))
    }

    #[inline]
    pub fn animate_insert_keyframe(&mut self) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_insert_keyframe())
    }

    #[inline]
    pub fn animate_delete_keyframe(&mut self, time: Option<f32>) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_delete_keyframe(time))
    }

    #[inline]
    pub fn animate_set_keyframe_interp(
        &mut self,
        interp: Interpolation,
    ) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_set_keyframe_interp(interp))
    }

    #[inline]
    pub fn animate_clear_bone_track(&mut self) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_clear_bone_track())
    }

    #[inline]
    pub fn animate_set_curve_mode(&mut self, active: bool) {
        self.in_context(|ctx| ctx.animate_set_curve_mode(active))
    }

    #[inline]
    pub fn animate_set_curve_channel(&mut self, channel: petunia_project::CurveChannel) {
        self.in_context(|ctx| ctx.animate_set_curve_channel(channel))
    }

    #[inline]
    pub fn animate_add_motion_layer(
        &mut self,
        name: &str,
        is_tweak: bool,
    ) -> Result<Uuid, CommandError> {
        self.in_context(|ctx| ctx.animate_add_motion_layer(name, is_tweak))
    }

    #[inline]
    pub fn animate_remove_motion_layer(&mut self, layer_id: Uuid) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_remove_motion_layer(layer_id))
    }

    #[inline]
    pub fn animate_set_motion_layer_weight(
        &mut self,
        layer_id: Uuid,
        weight: f32,
    ) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_set_motion_layer_weight(layer_id, weight))
    }

    #[inline]
    pub fn animate_set_motion_layer_blend_mode(
        &mut self,
        layer_id: Uuid,
        mode: petunia_project::BlendMode,
    ) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_set_motion_layer_blend_mode(layer_id, mode))
    }

    #[inline]
    pub fn animate_toggle_motion_layer_enabled(
        &mut self,
        layer_id: Uuid,
    ) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_toggle_motion_layer_enabled(layer_id))
    }

    #[inline]
    pub fn animate_set_onion_skin(&mut self, enabled: bool) {
        self.in_context(|ctx| ctx.animate_set_onion_skin(enabled))
    }

    #[inline]
    pub fn animate_set_motion_trails(&mut self, enabled: bool) {
        self.in_context(|ctx| ctx.animate_set_motion_trails(enabled))
    }

    #[inline]
    pub fn animate_select_motion(&mut self, id: Uuid) -> bool {
        self.in_context(|ctx| ctx.animate_select_motion(id))
    }

    #[inline]
    pub fn animate_add_rig(&mut self, kind: RigPresetKind) -> Result<Uuid, CommandError> {
        self.in_context(|ctx| ctx.animate_add_rig(kind))
    }

    #[inline]
    pub fn animate_add_motion(&mut self, generator: MotionGenerator) -> Result<Uuid, CommandError> {
        self.in_context(|ctx| ctx.animate_add_motion(generator))
    }

    #[inline]
    pub fn animate_duplicate_motion(&mut self) -> Result<Uuid, CommandError> {
        self.in_context(|ctx| ctx.animate_duplicate_motion())
    }

    #[inline]
    pub fn animate_remove_motion(&mut self) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_remove_motion())
    }

    #[inline]
    pub fn animate_param_preview(&mut self, key: &str, value: f32) -> bool {
        self.in_context(|ctx| ctx.animate_param_preview(key, value))
    }

    #[inline]
    pub fn animate_param_commit(&mut self) -> Result<bool, CommandError> {
        self.in_context(|ctx| ctx.animate_param_commit())
    }

    #[inline]
    pub fn animate_param_cancel(&mut self) {
        self.in_context(|ctx| ctx.animate_param_cancel())
    }

    #[inline]
    pub fn animate_set_param(&mut self, key: &str, value: f32) -> Result<bool, CommandError> {
        self.in_context(|ctx| ctx.animate_set_param(key, value))
    }

    #[inline]
    pub fn animate_set_style(&mut self, style: MotionStyle) -> Result<bool, CommandError> {
        self.in_context(|ctx| ctx.animate_set_style(style))
    }

    #[inline]
    pub fn animate_fit_to_model(&mut self) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_fit_to_model())
    }

    #[inline]
    pub fn animate_set_stepped(&mut self, fps: Option<f32>) -> Result<bool, CommandError> {
        self.in_context(|ctx| ctx.animate_set_stepped(fps))
    }

    #[inline]
    pub fn animate_set_root_mode(&mut self, mode: RootMode) -> Result<bool, CommandError> {
        self.in_context(|ctx| ctx.animate_set_root_mode(mode))
    }

    #[inline]
    pub fn animate_toggle_play(&mut self) {
        self.in_context(|ctx| ctx.animate_toggle_play())
    }

    #[inline]
    pub fn animate_seek(&mut self, time: f32) {
        self.in_context(|ctx| ctx.animate_seek(time))
    }

    #[inline]
    pub fn animate_apply_now(&mut self, keep_motion: bool) -> Result<(), CommandError> {
        self.in_context(|ctx| ctx.animate_apply_now(keep_motion))
    }

    #[inline]
    pub fn is_interacting(&self) -> bool {
        self.view().is_interacting()
    }

    #[inline]
    pub fn begin_mesh_preview(&mut self, label: &'static str) -> bool {
        self.in_context(|ctx| ctx.begin_mesh_preview(label))
    }

    #[inline]
    pub fn preview_mesh(&mut self, mesh: Mesh) {
        self.in_context(|ctx| ctx.preview_mesh(mesh))
    }

    #[inline]
    pub fn finish_mesh_preview(&mut self, cancel: bool) {
        self.in_context(|ctx| ctx.finish_mesh_preview(cancel))
    }

    #[inline]
    pub fn begin_modal(&mut self, kind: ModalKind) -> Result<(), ModalError> {
        self.in_context(|ctx| ctx.begin_modal(kind))
    }

    #[inline]
    pub fn set_modal_constraint(&mut self, constraint: ModalConstraint) -> Result<(), ModalError> {
        self.in_context(|ctx| ctx.set_modal_constraint(constraint))
    }

    #[inline]
    pub fn update_modal(&mut self, translation: Vec3, value: f32) -> Result<(), ModalError> {
        self.in_context(|ctx| ctx.update_modal(translation, value))
    }

    #[inline]
    pub fn update_modal_snapped(
        &mut self,
        translation: Vec3,
        value: f32,
        snap: Option<SnapKind>,
    ) -> Result<(), ModalError> {
        self.in_context(|ctx| ctx.update_modal_snapped(translation, value, snap))
    }

    #[inline]
    pub fn rigid_preview_transforms(&self) -> &[(uuid::Uuid, glam::Mat4)] {
        self.view().rigid_preview_transforms()
    }

    #[inline]
    pub fn update_modal_components(&mut self, components: Vec3) -> Result<(), ModalError> {
        self.in_context(|ctx| ctx.update_modal_components(components))
    }

    #[inline]
    pub fn commit_modal(&mut self) -> bool {
        self.in_context(|ctx| ctx.commit_modal())
    }

    #[inline]
    pub fn cancel_modal(&mut self) -> bool {
        self.in_context(|ctx| ctx.cancel_modal())
    }

    #[inline]
    pub fn poly_pen_add_polygon(
        &mut self,
        points: &[PenPoint],
    ) -> Result<usize, PolyPenCommandError> {
        self.in_context(|ctx| ctx.poly_pen_add_polygon(points))
    }

    #[inline]
    pub fn poly_pen_melt_point(&mut self, point: u32) -> Result<(), PolyPenCommandError> {
        self.in_context(|ctx| ctx.poly_pen_melt_point(point))
    }

    #[inline]
    pub fn poly_pen_split_edge(
        &mut self,
        a: u32,
        b: u32,
        t: f32,
    ) -> Result<u32, PolyPenCommandError> {
        self.in_context(|ctx| ctx.poly_pen_split_edge(a, b, t))
    }

    #[inline]
    pub fn begin_poly_pen_edge_extrude(
        &mut self,
        a: u32,
        b: u32,
    ) -> Result<(), PolyPenCommandError> {
        self.in_context(|ctx| ctx.begin_poly_pen_edge_extrude(a, b))
    }

    #[inline]
    pub fn begin_poly_pen_strip(
        &mut self,
        a: u32,
        b: u32,
    ) -> Result<PenStrip, PolyPenCommandError> {
        self.in_context(|ctx| ctx.begin_poly_pen_strip(a, b))
    }

    #[inline]
    pub fn poly_pen_strip_to(
        &mut self,
        strip: &mut PenStrip,
        target: Vec3,
    ) -> Result<usize, PolyPenCommandError> {
        self.in_context(|ctx| ctx.poly_pen_strip_to(strip, target))
    }

    #[inline]
    pub fn finish_poly_pen_strip(&mut self, strip: PenStrip) -> bool {
        self.in_context(|ctx| ctx.finish_poly_pen_strip(strip))
    }

    #[inline]
    pub fn cancel_poly_pen_strip(&mut self, strip: PenStrip) {
        self.in_context(|ctx| ctx.cancel_poly_pen_strip(strip))
    }

    #[inline]
    pub fn query_scene_hierarchy(&self) -> SceneHierarchyDto {
        self.view().query_scene_hierarchy()
    }

    #[inline]
    pub fn query_selection_details(&self) -> SelectionDetailsDto {
        self.view().query_selection_details()
    }

    #[inline]
    pub fn query_uv_diagnostics(&self) -> UvDiagnosticsDto {
        self.view().query_uv_diagnostics()
    }

    #[inline]
    pub fn set_active_asset_by_id(&mut self, id: Uuid) -> bool {
        self.in_context(|ctx| ctx.set_active_asset_by_id(id))
    }

    #[inline]
    pub fn delete_asset_by_id(&mut self, id: Uuid) -> bool {
        self.in_context(|ctx| ctx.delete_asset_by_id(id))
    }

    #[inline]
    pub fn find_asset_by_id(&self, id: Uuid) -> Option<&petunia_project::Asset> {
        self.view().find_asset_by_id(id)
    }

    #[inline]
    pub fn profile_region_planes(&self) -> RegionPlanes {
        self.view().profile_region_planes()
    }

    #[inline]
    pub fn profile_region_at_ray(&self, origin: Vec3, direction: Vec3) -> Option<RegionHit> {
        self.view().profile_region_at_ray(origin, direction)
    }

    #[inline]
    pub fn region_host_face(&self, hit: &RegionHit) -> Option<usize> {
        self.view().region_host_face(hit)
    }

    #[inline]
    pub fn begin_region_push_pull(&mut self, hit: &RegionHit) -> Result<(), RegionPushError> {
        self.in_context(|ctx| ctx.begin_region_push_pull(hit))
    }

    #[inline]
    pub fn shape_planes(&self) -> Vec<ShapePlane> {
        self.view().shape_planes()
    }

    #[inline]
    pub fn apply_shape_edit(
        &mut self,
        hint: Option<&RegionPlane>,
        edit: &ShapeEdit,
    ) -> Result<ShapeEditReport, ShapeEditError> {
        self.in_context(|ctx| ctx.apply_shape_edit(hint, None, edit))
    }

    #[inline]
    pub fn apply_shape_edit_with_operands(
        &mut self,
        hint: Option<&RegionPlane>,
        operands: Option<&[uuid::Uuid]>,
        edit: &ShapeEdit,
    ) -> Result<ShapeEditReport, ShapeEditError> {
        self.in_context(|ctx| ctx.apply_shape_edit(hint, operands, edit))
    }

    #[inline]
    pub fn document_session(&self) -> &DocumentSession {
        self.view().document_session()
    }

    #[inline]
    pub fn editor_session(&self) -> &EditorSession {
        self.view().editor_session()
    }

    #[inline]
    pub fn viewport_session(&self) -> ViewportSession {
        self.view().viewport_session()
    }

    #[inline]
    pub fn set_viewport_session(&mut self, vs: ViewportSession) {
        self.in_context(|ctx| ctx.set_viewport_session(vs))
    }

    #[inline]
    pub fn tool_session_manager(&self) -> &ToolSessionManager {
        self.view().tool_session_manager()
    }

    #[inline]
    pub fn scene_tris(&self) -> usize {
        self.view().scene_tris()
    }

    #[inline]
    pub fn scene_verts(&self) -> usize {
        self.view().scene_verts()
    }

    #[inline]
    pub fn primitive_session_valid(&self) -> bool {
        self.view().primitive_session_valid()
    }

    #[inline]
    pub fn begin_primitive(
        &mut self,
        kind: crate::command::PrimitiveKind,
        name: Option<String>,
    ) -> bool {
        self.in_context(|ctx| ctx.begin_primitive(kind, name))
    }

    #[inline]
    pub fn update_primitive(
        &mut self,
        descriptor: crate::primitive_session::PrimitiveDescriptor,
    ) -> bool {
        self.in_context(|ctx| ctx.update_primitive(descriptor))
    }

    #[inline]
    pub fn active_is_parametric(&self) -> bool {
        self.view().active_is_parametric()
    }

    #[inline]
    pub fn active_is_composition(&self) -> bool {
        self.view().active_is_composition()
    }

    #[inline]
    pub fn active_primitive_descriptor(
        &self,
    ) -> Option<crate::primitive_session::PrimitiveDescriptor> {
        self.view().active_primitive_descriptor()
    }

    #[inline]
    pub fn freeze_active_primitive(&mut self) -> bool {
        self.in_context(|ctx| ctx.freeze_active_primitive())
    }

    #[inline]
    pub fn update_active_primitive(
        &mut self,
        descriptor: crate::primitive_session::PrimitiveDescriptor,
        create_checkpoint: bool,
    ) -> bool {
        self.in_context(|ctx| ctx.update_active_primitive(descriptor, create_checkpoint))
    }

    #[inline]
    pub fn confirm_primitive(&mut self) -> bool {
        self.in_context(|ctx| ctx.confirm_primitive())
    }

    #[inline]
    pub fn cancel_primitive(&mut self) -> bool {
        self.in_context(|ctx| ctx.cancel_primitive())
    }

    #[inline]
    pub fn reopen_last_primitive(&mut self) -> bool {
        self.in_context(|ctx| ctx.reopen_last_primitive())
    }

    #[inline]
    pub fn finalize_primitive_session(&mut self) {
        self.in_context(|ctx| ctx.finalize_primitive_session())
    }

    #[inline]
    pub fn switch_workspace(&mut self, next: Workspace) {
        self.in_context(|ctx| ctx.switch_workspace(next))
    }

    #[inline]
    pub fn is_axis_locked(&self, axis: usize) -> bool {
        self.view().is_axis_locked(axis)
    }

    #[inline]
    pub fn active_axis_constraint_label(&self) -> Option<(&'static str, [u8; 3])> {
        self.view().active_axis_constraint_label()
    }

    #[inline]
    pub fn toggle_axis_lock(&mut self, axis: usize) {
        self.in_context(|ctx| ctx.toggle_axis_lock(axis))
    }

    #[inline]
    pub fn is_document_dirty(&self) -> bool {
        self.view().is_document_dirty()
    }

    #[inline]
    pub fn mark_document_dirty(&mut self) {
        self.in_context(|ctx| ctx.mark_document_dirty())
    }

    #[inline]
    pub fn mark_document_clean(&mut self) {
        self.in_context(|ctx| ctx.mark_document_clean())
    }

    #[inline]
    pub fn checkpoint(&mut self, label: &str) {
        self.in_context(|ctx| ctx.checkpoint(label))
    }

    #[inline]
    pub fn undo(&mut self) -> bool {
        self.in_context(|ctx| ctx.undo())
    }

    #[inline]
    pub fn redo(&mut self) -> bool {
        self.in_context(|ctx| ctx.redo())
    }

    #[inline]
    pub fn emit_mesh_changed(&mut self) {
        self.in_context(|ctx| ctx.emit_mesh_changed())
    }

    #[inline]
    pub fn emit_positions_changed(&mut self) {
        self.in_context(|ctx| ctx.emit_positions_changed())
    }

    #[inline]
    pub fn emit_uv_changed(&mut self) {
        self.in_context(|ctx| ctx.emit_uv_changed())
    }

    #[inline]
    pub fn emit_color_changed(&mut self) {
        self.in_context(|ctx| ctx.emit_color_changed())
    }

    #[inline]
    pub fn emit_material_changed(&mut self) {
        self.in_context(|ctx| ctx.emit_material_changed())
    }

    #[inline]
    pub fn emit_texture_changed(&mut self) {
        self.in_context(|ctx| ctx.emit_texture_changed())
    }

    #[inline]
    pub fn emit_texture_changed_regions(&mut self, regions: Vec<TextureDirtyRect>) {
        self.in_context(|ctx| ctx.emit_texture_changed_regions(regions))
    }

    #[inline]
    pub fn emit_project_changed(&mut self, changes: ProjectChanges) {
        self.in_context(|ctx| ctx.emit_project_changed(changes))
    }

    #[inline]
    pub fn sync_selection(&mut self) -> bool {
        self.in_context(|ctx| ctx.sync_selection())
    }

    #[inline]
    pub fn apply_component_selection(
        &mut self,
        set: &crate::ComponentSet,
        op: crate::SelectionOp,
    ) -> Result<(), crate::CommandError> {
        self.in_context(|ctx| ctx.apply_component_selection(set, op))
    }

    #[inline]
    pub fn select_hover_if_empty(&mut self) -> bool {
        self.in_context(|ctx| ctx.select_hover_if_empty())
    }

    #[inline]
    pub fn uv_click(&mut self, point: glam::Vec2, radius: f32, extend: bool) -> bool {
        self.in_context(|ctx| ctx.uv_click(point, radius, extend))
    }

    #[inline]
    pub fn uv_press(&mut self, point: glam::Vec2, radius: f32, extend: bool) -> bool {
        self.in_context(|ctx| ctx.uv_press(point, radius, extend))
    }

    #[inline]
    pub fn uv_box_select(
        &mut self,
        min: glam::Vec2,
        max: glam::Vec2,
        extend: bool,
        subtract: bool,
    ) -> bool {
        self.in_context(|ctx| ctx.uv_box_select(min, max, extend, subtract))
    }

    #[inline]
    pub fn uv_select_faces(&mut self, faces: &[usize], extend: bool) -> bool {
        self.in_context(|ctx| ctx.uv_select_faces(faces, extend))
    }

    #[inline]
    pub fn set_uv_sync(&mut self, enabled: bool) {
        self.in_context(|ctx| ctx.set_uv_sync(enabled))
    }

    #[inline]
    pub fn select_object(&mut self, index: Option<usize>, extend: bool) {
        self.in_context(|ctx| ctx.select_object(index, extend))
    }

    #[inline]
    pub fn selection_domain(&self) -> SelectionDomain {
        self.view().selection_domain()
    }

    #[inline]
    pub fn set_edit_mode(&mut self, mode: EditMode) {
        self.in_context(|ctx| ctx.set_edit_mode(mode))
    }

    #[inline]
    pub fn set_selection_domain(&mut self, domain: SelectionDomain) {
        self.in_context(|ctx| ctx.set_selection_domain(domain))
    }

    #[inline]
    pub fn cycle_selection_domain(&mut self) {
        self.in_context(|ctx| ctx.cycle_selection_domain())
    }

    #[inline]
    pub fn calculate_pivot(&self, pivot: PivotPoint) -> glam::Vec3 {
        self.view().calculate_pivot(pivot)
    }

    #[inline]
    pub fn calculate_transform_frame(&self, orientation: TransformOrientation) -> TransformFrame {
        self.view().calculate_transform_frame(orientation)
    }

    #[inline]
    pub fn animate_selected_bone_pivot(&self) -> Option<glam::Vec3> {
        self.view().animate_selected_bone_pivot()
    }

    #[inline]
    pub fn set_origin_geometry(&mut self) -> Result<(), &'static str> {
        self.in_context(|ctx| ctx.set_origin_geometry())
    }

    #[inline]
    pub fn set_origin_bottom(&mut self) -> Result<(), &'static str> {
        self.in_context(|ctx| ctx.set_origin_bottom())
    }

    #[inline]
    pub fn set_origin_cursor(&mut self) -> Result<(), &'static str> {
        self.in_context(|ctx| ctx.set_origin_cursor())
    }

    #[inline]
    pub fn set_origin_selection(&mut self) -> Result<(), &'static str> {
        self.in_context(|ctx| ctx.set_origin_selection())
    }

    #[inline]
    pub fn set_origin_world(&mut self) -> Result<(), &'static str> {
        self.in_context(|ctx| ctx.set_origin_world())
    }

    #[inline]
    pub fn set_origin_point(&mut self, point: glam::Vec3) -> Result<(), &'static str> {
        self.in_context(|ctx| ctx.set_origin_point(point))
    }

    #[inline]
    pub fn set_geometry_to_origin(&mut self) -> Result<(), &'static str> {
        self.in_context(|ctx| ctx.set_geometry_to_origin())
    }

    #[inline]
    pub fn toggle_edit_pivot(&mut self) -> bool {
        self.in_context(|ctx| ctx.toggle_edit_pivot())
    }

    #[inline]
    pub fn current_tool_feedback(&self) -> Option<crate::modal_feedback::ToolFeedback> {
        self.view().current_tool_feedback()
    }

    #[inline]
    pub fn brush_settings(&self) -> crate::BrushSettings {
        self.view().brush_settings()
    }

    #[inline]
    pub fn paint_at(&mut self, center: Vec3) {
        self.in_context(|ctx| ctx.paint_at(center))
    }

    #[inline]
    pub fn begin_paint_stroke(&mut self) {
        self.in_context(|ctx| ctx.begin_paint_stroke())
    }

    #[inline]
    pub fn finish_paint_stroke(&mut self, cancel: bool) {
        self.in_context(|ctx| ctx.finish_paint_stroke(cancel))
    }

    #[inline]
    pub fn pick_vertex(&self, origin: Vec3, dir: Vec3) -> Option<(usize, Vec3)> {
        self.view().pick_vertex(origin, dir)
    }

    #[inline]
    pub fn save_active_as_asset(&mut self) -> bool {
        self.in_context(|ctx| ctx.save_active_as_asset())
    }

    #[inline]
    pub fn save_selection_as_prefab(&mut self, name: Option<&str>) -> Option<uuid::Uuid> {
        self.in_context(|ctx| ctx.save_selection_as_prefab(name))
    }

    #[inline]
    pub fn instantiate_prefab(
        &mut self,
        prefab_id: uuid::Uuid,
        position: Option<[f32; 3]>,
    ) -> bool {
        self.in_context(|ctx| ctx.instantiate_prefab(prefab_id, position))
    }

    #[inline]
    pub fn delete_prefab(&mut self, prefab_id: uuid::Uuid) -> bool {
        self.in_context(|ctx| ctx.delete_prefab(prefab_id))
    }

    #[inline]
    pub fn rename_prefab(&mut self, prefab_id: uuid::Uuid, name: &str) -> bool {
        self.in_context(|ctx| ctx.rename_prefab(prefab_id, name))
    }

    #[inline]
    pub fn set_prefab_favorite(&mut self, prefab_id: uuid::Uuid, favorite: bool) -> bool {
        self.in_context(|ctx| ctx.set_prefab_favorite(prefab_id, favorite))
    }

    #[inline]
    pub fn update_prefab_from_selection(&mut self, prefab_id: uuid::Uuid) -> bool {
        self.in_context(|ctx| ctx.update_prefab_from_selection(prefab_id))
    }

    #[inline]
    pub fn apply_solid_shape_builder(
        &mut self,
        explicit_operand: Option<uuid::Uuid>,
        op: crate::solid_commands::SolidOperation,
    ) -> Result<crate::solid_commands::SolidShapeBuilderReport, crate::command::CommandError> {
        self.in_context(|ctx| ctx.apply_solid_shape_builder(explicit_operand, op))
    }

    #[inline]
    pub fn create_solid_composition(
        &mut self,
        base_id: Option<uuid::Uuid>,
        operands: &[uuid::Uuid],
        op: crate::solid_commands::SolidOperation,
        budget: Option<petunia_mesh::ShapeBuilderBudget>,
    ) -> Result<(), crate::command::CommandError> {
        self.in_context(|ctx| ctx.create_solid_composition(base_id, operands, op, budget))
    }

    #[inline]
    pub fn bake_solid_composition(
        &mut self,
        target: Option<uuid::Uuid>,
    ) -> Result<(), crate::command::CommandError> {
        self.in_context(|ctx| ctx.bake_solid_composition(target))
    }

    #[inline]
    pub fn toggle_solid_composition_step(
        &mut self,
        target: Option<uuid::Uuid>,
        step_id: uuid::Uuid,
        enabled: bool,
    ) -> Result<(), crate::command::CommandError> {
        self.in_context(|ctx| ctx.toggle_solid_composition_step(target, step_id, enabled))
    }

    #[inline]
    pub fn reorder_solid_composition_steps(
        &mut self,
        target: Option<uuid::Uuid>,
        from: usize,
        to: usize,
    ) -> Result<(), crate::command::CommandError> {
        self.in_context(|ctx| ctx.reorder_solid_composition_steps(target, from, to))
    }

    #[inline]
    pub fn align_workplane_to_face(
        &mut self,
        target: Option<uuid::Uuid>,
        face_index: usize,
    ) -> Result<(), crate::command::CommandError> {
        self.in_context(|ctx| ctx.align_workplane_to_face(target, face_index))
    }

    #[inline]
    pub fn align_workplane_to_selection(&mut self) -> Result<(), crate::command::CommandError> {
        self.in_context(|ctx| ctx.align_workplane_to_selection())
    }

    #[inline]
    pub fn imprint_profile_on_surface(
        &mut self,
        profile_id: uuid::Uuid,
        target: Option<uuid::Uuid>,
    ) -> Result<(), crate::command::CommandError> {
        self.in_context(|ctx| ctx.imprint_profile_on_surface(profile_id, target))
    }

    #[inline]
    pub fn contextual_push_pull(
        &mut self,
        target: Option<uuid::Uuid>,
        distance: f32,
    ) -> Result<(), crate::command::CommandError> {
        self.in_context(|ctx| ctx.contextual_push_pull(target, distance))
    }

    /// Outro objeto selecionado, distinto do ativo: o operando implícito das
    /// operações de dois objetos (Join, Shape Builder sólido).
    #[inline]
    pub fn second_selected_asset(&self) -> Option<uuid::Uuid> {
        let view = self.view();
        let active = view.document.active().map(|asset| asset.id);
        view.editor
            .selection
            .assets
            .iter()
            .copied()
            .find(|id| Some(*id) != active && view.document.assets.iter().any(|a| a.id == *id))
    }

    #[inline]
    pub fn join_selected_into_active(&mut self) -> Result<usize, &'static str> {
        self.in_context(|ctx| ctx.join_selected_into_active())
    }

    #[inline]
    pub fn rename_active_asset(&mut self, name: &str) -> Result<bool, AssetRenameError> {
        self.in_context(|ctx| ctx.rename_active_asset(name))
    }

    #[inline]
    pub fn frame_selection(&mut self) {
        self.in_context(|ctx| ctx.frame_selection())
    }

    #[inline]
    pub fn frame_all(&mut self) {
        self.in_context(|ctx| ctx.frame_all())
    }

    #[inline]
    pub fn instantiate_asset_at_cursor(&mut self, asset_index: usize) -> bool {
        self.in_context(|ctx| ctx.instantiate_asset_at_cursor(asset_index))
    }

    #[inline]
    pub fn instantiate_asset_by_id(
        &mut self,
        asset_id: uuid::Uuid,
        position: Option<[f32; 3]>,
    ) -> bool {
        self.in_context(|ctx| ctx.instantiate_asset_by_id(asset_id, position))
    }

    #[inline]
    pub fn toggle_isolate(&mut self) {
        self.in_context(|ctx| ctx.toggle_isolate())
    }

    #[inline]
    pub fn toggle_paint_isolate_ghost(&mut self) {
        self.in_context(|ctx| ctx.toggle_paint_isolate_ghost())
    }

    /// Pedido de importação para o host abrir o diálogo de arquivos.
    #[inline]
    pub fn request_import(&mut self, filter: &str) {
        self.in_context(|ctx| ctx.request_import(filter))
    }

    #[inline]
    pub fn refresh_isolation(&mut self) {
        self.in_context(|ctx| ctx.refresh_isolation())
    }

    #[inline]
    pub fn is_active_locked(&self) -> bool {
        self.view().is_active_locked()
    }

    #[inline]
    pub fn toggle_lock_active(&mut self) {
        self.in_context(|ctx| ctx.toggle_lock_active())
    }

    #[inline]
    pub fn commit_modal_gesture(&mut self) -> Option<LastOperation> {
        self.in_context(|ctx| ctx.commit_modal_gesture())
    }

    #[inline]
    pub fn last_operation_is_current(&self, last: &LastOperation) -> bool {
        self.view().last_operation_is_current(last)
    }

    #[inline]
    pub fn adjust_last_operation(
        &mut self,
        last: &LastOperation,
        value: f32,
    ) -> Result<Option<LastOperation>, ModalError> {
        self.in_context(|ctx| ctx.adjust_last_operation(last, value))
    }

    /// Inspeciona a operação em andamento e devolve o descritor canônico de feedback (H28 / MF-001–008).
    #[inline]
    pub fn current_modeling_feedback(&self) -> Option<crate::modeling_feedback::OperationFeedback> {
        crate::modeling_feedback::ModelingFeedbackInspector::inspect_current_operation(self)
    }

    /// Preflight de operação Booleana (MF-003 / MF-T04).
    #[inline]
    pub fn preflight_boolean_feedback(
        &self,
        op_a: uuid::Uuid,
        op_b: uuid::Uuid,
    ) -> crate::modeling_feedback::OperationFeedback {
        crate::modeling_feedback::ModelingFeedbackInspector::preflight_boolean(self, op_a, op_b)
    }

    /// Preflight de simetria / Mirror (MF-004 / MF-T05).
    #[inline]
    pub fn preflight_mirror_feedback(
        &self,
        asset_id: uuid::Uuid,
        normal: [f32; 3],
        offset: f32,
    ) -> crate::modeling_feedback::OperationFeedback {
        crate::modeling_feedback::ModelingFeedbackInspector::preflight_mirror(
            self, asset_id, normal, offset,
        )
    }

    /// Preflight de pincel de escultura / deformação (MF-005 / MF-T06).
    #[inline]
    pub fn preflight_sculpt_feedback(
        &self,
        radius: f32,
        strength: f32,
        hit: [f32; 3],
        normal: [f32; 3],
    ) -> crate::modeling_feedback::OperationFeedback {
        crate::modeling_feedback::ModelingFeedbackInspector::preflight_sculpt(
            radius, strength, hit, normal,
        )
    }

    /// Preflight de remesh sobre malha com skin bind (MF-007 / MF-T07).
    #[inline]
    pub fn preflight_remesh_skin_feedback(
        &self,
        asset_id: uuid::Uuid,
    ) -> crate::modeling_feedback::OperationFeedback {
        crate::modeling_feedback::ModelingFeedbackInspector::preflight_remesh_skin(self, asset_id)
    }

    /// Preflight de seleção mista ou parcialmente incompatível (MF-007 / MF-T08).
    #[inline]
    pub fn preflight_partial_selection_feedback(
        &self,
        requested_domain: &str,
    ) -> crate::modeling_feedback::OperationFeedback {
        crate::modeling_feedback::ModelingFeedbackInspector::preflight_partial_selection(
            self,
            requested_domain,
        )
    }

    #[inline]
    pub fn query_volume_frame(
        &self,
        asset_id: uuid::Uuid,
        frame: crate::spatial_query::SpatialFrameKind,
        rep: crate::spatial_query::GeometryRepresentation,
    ) -> Option<crate::spatial_query::VolumeFrame> {
        self.view().query_volume_frame(asset_id, frame, rep)
    }

    #[inline]
    pub fn query_spatial(
        &self,
        query: crate::spatial_query::SpatialQueryKind,
    ) -> Result<crate::spatial_query::SpatialQueryResult, CommandError> {
        self.view().query_spatial(query)
    }

    #[inline]
    pub fn query_section(
        &self,
        asset_id: uuid::Uuid,
        plane: &crate::section_analysis::SectionPlane,
    ) -> Result<crate::section_analysis::SectionResult, CommandError> {
        self.view().query_section(asset_id, plane)
    }

    #[inline]
    pub fn probe_thickness(
        &self,
        asset_id: uuid::Uuid,
        query: &crate::section_analysis::ThicknessProbeQuery,
    ) -> Result<crate::section_analysis::ThicknessProbeResult, CommandError> {
        self.view().probe_thickness(asset_id, query)
    }

    pub fn scale_to_dimension(
        &mut self,
        asset_id: uuid::Uuid,
        axis: usize,
        target_dimension: f32,
        uniform: bool,
        frame: crate::spatial_query::SpatialFrameKind,
    ) -> Result<(), CommandError> {
        let cmd = crate::spatial_query::ScaleToDimensionCmd::new(
            asset_id,
            axis,
            target_dimension,
            uniform,
            frame,
        );
        crate::command::CommandDispatcher::dispatch(self, &cmd)
    }

    #[inline]
    pub fn open_companion_view(&mut self) {
        self.session.companion_view.enabled = true;
    }

    #[inline]
    pub fn close_companion_view(&mut self) {
        self.session.companion_view.enabled = false;
    }

    #[inline]
    pub fn toggle_companion_view(&mut self) {
        self.session.companion_view.enabled = !self.session.companion_view.enabled;
    }

    #[inline]
    pub fn is_companion_view_open(&self) -> bool {
        self.session.companion_view.enabled
    }

    #[inline]
    pub fn companion_view_state(&self) -> &crate::spatial_query::CompanionViewState {
        &self.session.companion_view
    }

    #[inline]
    pub fn set_show_xray(&mut self, enabled: bool) {
        self.session.show_xray = enabled;
        // Invariante VSP-004: X-Ray visual nunca altera a política de seleção!
    }

    #[inline]
    pub fn set_select_through(&mut self, enabled: bool) {
        self.session.select_policy = if enabled {
            crate::SelectPolicy::Through
        } else {
            crate::SelectPolicy::VisibleOnly
        };
        // Invariante VSP-004: a política de seleção nunca altera show_xray!
    }

    /// Orientação dos eixos do gizmo e das restrições de G/R/S (B15).
    #[inline]
    pub fn set_transform_orientation(&mut self, orientation: crate::TransformOrientation) {
        self.session.transform_orientation = orientation;
    }

    /// Escolhe a política de seleção (B10): Automática, Só visível ou Através.
    #[inline]
    pub fn set_select_policy(&mut self, policy: crate::SelectPolicy) {
        self.session.select_policy = policy;
    }

    #[inline]
    pub fn active_section_plane(&self) -> Option<&crate::section_analysis::SectionPlane> {
        self.session.active_section.as_ref()
    }

    #[inline]
    pub fn set_active_section_plane(&mut self, plane: crate::section_analysis::SectionPlane) {
        self.session.active_section = Some(plane);
    }

    #[inline]
    pub fn clear_active_section_plane(&mut self) {
        self.session.active_section = None;
    }

    #[inline]
    pub fn visual_assistance_profile(&self) -> crate::visual_assistance::VisualAssistanceProfile {
        self.session.visual_assistance
    }

    #[inline]
    pub fn set_visual_assistance_profile(
        &mut self,
        profile: crate::visual_assistance::VisualAssistanceProfile,
    ) {
        self.session.visual_assistance = profile;
        // Invariante VSP-001: nunca altera keymap, ferramentas ou documento!
    }

    #[inline]
    pub fn clay_mode(&self) -> crate::visual_assistance::ClayMode {
        self.session.clay_settings.mode
    }

    #[inline]
    pub fn set_clay_mode(&mut self, mode: crate::visual_assistance::ClayMode) {
        self.session.clay_settings.mode = mode;
        // Invariante VSP-002: modo de trabalho desacoplado do documento!
    }

    #[inline]
    pub fn clay_lighting(&self) -> crate::visual_assistance::LightingAnchor {
        self.session.clay_settings.lighting
    }

    #[inline]
    pub fn set_clay_lighting(&mut self, anchor: crate::visual_assistance::LightingAnchor) {
        self.session.clay_settings.lighting = anchor;
    }

    #[inline]
    pub fn set_companion_view_trigger(
        &mut self,
        trigger: crate::visual_assistance::CompanionViewTrigger,
    ) {
        self.session.companion_view.trigger_mode = trigger;
    }

    #[inline]
    pub fn gizmo_accessibility_feedback(
        &self,
    ) -> crate::visual_assistance::GizmoAccessibilityFeedback {
        crate::visual_assistance::evaluate_gizmo_accessibility(
            self.modal.as_ref(),
            self.session.transform_orientation,
            self.session.snap_enabled,
        )
    }
}

#[allow(dead_code)]
impl CommandContext<'_> {
    #[inline]
    pub fn brush_world_radius(&self, hit: Vec3) -> f32 {
        self.view().brush_world_radius(hit)
    }

    #[inline]
    pub fn world_radius_for_px(&self, hit: Vec3, size_px: f32) -> f32 {
        self.view().world_radius_for_px(hit, size_px)
    }

    #[inline]
    pub fn is_interacting(&self) -> bool {
        self.view().is_interacting()
    }

    #[inline]
    pub(crate) fn modal_flips_when_negative(&self) -> bool {
        self.view().modal_flips_when_negative()
    }

    #[inline]
    pub(crate) fn modal_prelude_state(&self) -> Option<(Project, Selection)> {
        self.view().modal_prelude_state()
    }

    #[inline]
    pub fn rigid_preview_transforms(&self) -> &[(uuid::Uuid, glam::Mat4)] {
        self.view().rigid_preview_transforms()
    }

    #[inline]
    pub(crate) fn poly_pen_viewer(&self) -> Vec3 {
        self.view().poly_pen_viewer()
    }

    #[inline]
    pub fn query_scene_hierarchy(&self) -> SceneHierarchyDto {
        self.view().query_scene_hierarchy()
    }

    #[inline]
    pub fn query_selection_details(&self) -> SelectionDetailsDto {
        self.view().query_selection_details()
    }

    #[inline]
    pub fn query_uv_diagnostics(&self) -> UvDiagnosticsDto {
        self.view().query_uv_diagnostics()
    }

    #[inline]
    pub fn find_asset_by_id(&self, id: Uuid) -> Option<&petunia_project::Asset> {
        self.view().find_asset_by_id(id)
    }

    #[inline]
    pub fn profile_region_planes(&self) -> RegionPlanes {
        self.view().profile_region_planes()
    }

    #[inline]
    pub(crate) fn profile_groups(&self, editable_only: bool) -> Vec<ProfileGroup> {
        self.view().profile_groups(editable_only)
    }

    #[inline]
    pub fn profile_region_at_ray(&self, origin: Vec3, direction: Vec3) -> Option<RegionHit> {
        self.view().profile_region_at_ray(origin, direction)
    }

    #[inline]
    pub fn region_host_face(&self, hit: &RegionHit) -> Option<usize> {
        self.view().region_host_face(hit)
    }

    #[inline]
    pub fn shape_planes(&self) -> Vec<ShapePlane> {
        self.view().shape_planes()
    }

    #[inline]
    pub fn document_session(&self) -> &DocumentSession {
        self.view().document_session()
    }

    #[inline]
    pub fn editor_session(&self) -> &EditorSession {
        self.view().editor_session()
    }

    #[inline]
    pub fn viewport_session(&self) -> ViewportSession {
        self.view().viewport_session()
    }

    #[inline]
    pub fn tool_session_manager(&self) -> &ToolSessionManager {
        self.view().tool_session_manager()
    }

    #[inline]
    pub fn scene_tris(&self) -> usize {
        self.view().scene_tris()
    }

    #[inline]
    pub fn scene_verts(&self) -> usize {
        self.view().scene_verts()
    }

    #[inline]
    pub fn primitive_session_valid(&self) -> bool {
        self.view().primitive_session_valid()
    }

    #[inline]
    pub fn query_volume_frame(
        &self,
        asset_id: uuid::Uuid,
        frame: crate::spatial_query::SpatialFrameKind,
        rep: crate::spatial_query::GeometryRepresentation,
    ) -> Option<crate::spatial_query::VolumeFrame> {
        self.view().query_volume_frame(asset_id, frame, rep)
    }

    #[inline]
    pub fn query_spatial(
        &self,
        query: crate::spatial_query::SpatialQueryKind,
    ) -> Result<crate::spatial_query::SpatialQueryResult, CommandError> {
        self.view().query_spatial(query)
    }

    #[inline]
    pub fn query_section(
        &self,
        asset_id: uuid::Uuid,
        plane: &crate::section_analysis::SectionPlane,
    ) -> Result<crate::section_analysis::SectionResult, CommandError> {
        self.view().query_section(asset_id, plane)
    }

    #[inline]
    pub fn probe_thickness(
        &self,
        asset_id: uuid::Uuid,
        query: &crate::section_analysis::ThicknessProbeQuery,
    ) -> Result<crate::section_analysis::ThicknessProbeResult, CommandError> {
        self.view().probe_thickness(asset_id, query)
    }

    pub fn scale_to_dimension(
        &mut self,
        asset_id: uuid::Uuid,
        axis: usize,
        target_dimension: f32,
        uniform: bool,
        frame: crate::spatial_query::SpatialFrameKind,
    ) -> Result<(), CommandError> {
        let cmd = crate::spatial_query::ScaleToDimensionCmd::new(
            asset_id,
            axis,
            target_dimension,
            uniform,
            frame,
        );
        crate::command::CommandDispatcher::dispatch_context(self, &cmd)
    }

    #[inline]
    pub fn active_is_parametric(&self) -> bool {
        self.view().active_is_parametric()
    }

    #[inline]
    pub fn active_primitive_descriptor(
        &self,
    ) -> Option<crate::primitive_session::PrimitiveDescriptor> {
        self.view().active_primitive_descriptor()
    }

    #[inline]
    pub fn is_axis_locked(&self, axis: usize) -> bool {
        self.view().is_axis_locked(axis)
    }

    #[inline]
    pub fn active_axis_constraint_label(&self) -> Option<(&'static str, [u8; 3])> {
        self.view().active_axis_constraint_label()
    }

    #[inline]
    pub fn is_document_dirty(&self) -> bool {
        self.view().is_document_dirty()
    }

    #[inline]
    pub fn selection_domain(&self) -> SelectionDomain {
        self.view().selection_domain()
    }

    #[inline]
    pub fn calculate_pivot(&self, pivot: PivotPoint) -> glam::Vec3 {
        self.view().calculate_pivot(pivot)
    }

    #[inline]
    pub fn calculate_transform_frame(&self, orientation: TransformOrientation) -> TransformFrame {
        self.view().calculate_transform_frame(orientation)
    }

    #[inline]
    pub fn animate_selected_bone_pivot(&self) -> Option<glam::Vec3> {
        self.view().animate_selected_bone_pivot()
    }

    #[inline]
    pub fn current_tool_feedback(&self) -> Option<crate::modal_feedback::ToolFeedback> {
        self.view().current_tool_feedback()
    }

    #[inline]
    pub fn current_modeling_feedback(&self) -> Option<crate::modeling_feedback::OperationFeedback> {
        self.view().current_modeling_feedback()
    }

    #[inline]
    pub fn brush_settings(&self) -> crate::BrushSettings {
        self.view().brush_settings()
    }

    #[inline]
    pub fn pick_vertex(&self, origin: Vec3, dir: Vec3) -> Option<(usize, Vec3)> {
        self.view().pick_vertex(origin, dir)
    }

    #[inline]
    pub fn pick_edge(&self, origin: Vec3, dir: Vec3) -> Option<((u32, u32), Vec3)> {
        self.view().pick_edge(origin, dir)
    }

    #[inline]
    pub(crate) fn selected_or_active_asset_ids(&self) -> Vec<uuid::Uuid> {
        self.view().selected_or_active_asset_ids()
    }

    #[inline]
    pub fn is_active_locked(&self) -> bool {
        self.view().is_active_locked()
    }

    #[inline]
    pub fn last_operation_is_current(&self, last: &LastOperation) -> bool {
        self.view().last_operation_is_current(last)
    }

    #[inline]
    pub fn open_companion_view(&mut self) {
        self.editor.companion_view.enabled = true;
    }

    #[inline]
    pub fn close_companion_view(&mut self) {
        self.editor.companion_view.enabled = false;
    }

    #[inline]
    pub fn toggle_companion_view(&mut self) {
        self.editor.companion_view.enabled = !self.editor.companion_view.enabled;
    }

    #[inline]
    pub fn is_companion_view_open(&self) -> bool {
        self.editor.companion_view.enabled
    }

    #[inline]
    pub fn companion_view_state(&self) -> &crate::spatial_query::CompanionViewState {
        &self.editor.companion_view
    }

    #[inline]
    pub fn set_show_xray(&mut self, enabled: bool) {
        self.editor.show_xray = enabled;
        // Invariante VSP-004: X-Ray visual nunca altera a política de seleção!
    }

    #[inline]
    pub fn set_select_through(&mut self, enabled: bool) {
        self.editor.select_policy = if enabled {
            crate::SelectPolicy::Through
        } else {
            crate::SelectPolicy::VisibleOnly
        };
        // Invariante VSP-004: a política de seleção nunca altera show_xray!
    }

    #[inline]
    pub fn active_section_plane(&self) -> Option<&crate::section_analysis::SectionPlane> {
        self.editor.active_section.as_ref()
    }

    #[inline]
    pub fn set_active_section_plane(&mut self, plane: crate::section_analysis::SectionPlane) {
        self.editor.active_section = Some(plane);
    }

    #[inline]
    pub fn clear_active_section_plane(&mut self) {
        self.editor.active_section = None;
    }

    #[inline]
    pub fn visual_assistance_profile(&self) -> crate::visual_assistance::VisualAssistanceProfile {
        self.editor.visual_assistance
    }

    #[inline]
    pub fn set_visual_assistance_profile(
        &mut self,
        profile: crate::visual_assistance::VisualAssistanceProfile,
    ) {
        self.editor.visual_assistance = profile;
        // Invariante VSP-001: nunca altera keymap, ferramentas ou documento!
    }

    #[inline]
    pub fn clay_mode(&self) -> crate::visual_assistance::ClayMode {
        self.editor.clay_settings.mode
    }

    #[inline]
    pub fn set_clay_mode(&mut self, mode: crate::visual_assistance::ClayMode) {
        self.editor.clay_settings.mode = mode;
        // Invariante VSP-002: modo de trabalho desacoplado do documento!
    }

    #[inline]
    pub fn clay_lighting(&self) -> crate::visual_assistance::LightingAnchor {
        self.editor.clay_settings.lighting
    }

    #[inline]
    pub fn set_clay_lighting(&mut self, anchor: crate::visual_assistance::LightingAnchor) {
        self.editor.clay_settings.lighting = anchor;
    }

    #[inline]
    pub fn set_companion_view_trigger(
        &mut self,
        trigger: crate::visual_assistance::CompanionViewTrigger,
    ) {
        self.editor.companion_view.trigger_mode = trigger;
    }
}
