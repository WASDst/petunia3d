use petunia_config::keybinds::{Keybinds, Mods2, parse_binding, winit_keys::KeyCode};
use petunia_core::{AppState, RigPresetKind, FillScope, BrushSettings, BrushType, AnimatePreview};
use petunia_module_paint::PaintModule;
use petunia_project::{Canvas, Transform3D, AnimationAsset};

#[test]
fn literal_brackets_parse() {
    for key in ["[", "]", "Shift+[", "Shift+]"] {
        assert!(parse_binding(key).is_some(), "unparsed: {key}");
    }
}
#[test]
fn default_brush_shortcuts_exist() {
    let kb=Keybinds::load_profile("petunia-default");
    for id in ["paint.size_decrease", "paint.size_increase", "paint.hardness_decrease", "paint.hardness_increase"] {
        assert!(kb.binding_for(id).is_some(), "absent: {id}");
    }
}
#[test]
fn numpad_plus_roundtrips() {
    let binding=parse_binding("NumpadAdd").unwrap();
    assert_eq!(parse_binding(&binding.to_shortcut_string()), Some(binding));
}
#[test]
fn removed_binding_survives_save_reload() {
    let dir=std::path::Path::new("/tmp/petunia-tools-audit-2026-10-15/keymaps");
    let mut kb=Keybinds::defaults();
    kb.remove_binding("model.extrude");
    let id=kb.save_user_profile_in(dir,"Audit removal", "synthetic audit").unwrap();
    assert!(Keybinds::load_profile_in(&id,dir).binding_for("model.extrude").is_none());
}
#[test]
fn view_overlap_is_reported_consistently() {
    let mut kb=Keybinds::default();
    let b=parse_binding("F").unwrap();
    kb.set_binding("view.frame_selection",b.clone());
    kb.set_binding("model.make_face",b.clone());
    assert_eq!(kb.conflicts_for("model.make_face",&b).len(),1);
    assert_eq!(kb.detect_conflicts().len(),1);
}
#[test]
fn application_handles_select_tool_action() {
    let mut app=petunia_application::Application::new(Default::default());
    app.set_tool("transform",None);
    app.run_action("model.tool_select");
    assert_eq!(app.state.active_tool,"select");
}
#[test]
fn workspace_bindings_cover_uv_and_animation() {
    let kb=Keybinds::load_profile("petunia-default");
    assert_eq!(kb.find_in_context(KeyCode::KeyG,Mods2::default(),"uv"),Some("uv.move"));
    assert_eq!(kb.find_in_context(KeyCode::Space,Mods2::default(),"animate"),Some("animate.toggle_play"));
}
fn paint_state() -> AppState {
    let mut state=AppState::new("en");
    let active=state.project.active;
    let asset=state.project.assets.get_mut(active).unwrap();
    asset.texture=Some(Canvas::new(16,16,[0,0,0,255]));
    for face in &mut asset.mesh.faces { face.selected=false; }
    for vert in &mut asset.mesh.verts { vert.selected=false; }
    PaintModule::ensure_stack(&mut state);
    state.paint_color=[1.0,0.0,0.0];
    state.session.tools.paint_color=[1.0,0.0,0.0];
    state
}
fn pixels(state:&AppState)->Vec<u8>{state.project.active().unwrap().texture.as_ref().unwrap().pixels.to_vec()}
#[test]
fn selected_faces_fill_with_empty_selection_changes_nothing() {
    let mut state=paint_state(); let before=pixels(&state);
    state.begin_paint_stroke();
    PaintModule::canvas_fill_scoped(&mut state,None,Some((8,8)),FillScope::SelectedFaces);
    state.finish_paint_stroke(false);
    let after=pixels(&state);
    assert_eq!(before.iter().zip(&after).filter(|(a,b)|a!=b).count(),0,"empty selection broadened fill");
}
#[test]
fn invalid_face_fill_changes_nothing() {
    let mut state=paint_state(); let before=pixels(&state);
    state.begin_paint_stroke();
    PaintModule::canvas_fill_scoped(&mut state,Some(99999),None,FillScope::Face);
    state.finish_paint_stroke(false);
    let after=pixels(&state);
    assert_eq!(before.iter().zip(&after).filter(|(a,b)|a!=b).count(),0,"invalid face broadened fill");
}
#[test]
fn low_level_fill_brush_honors_empty_selection_mask() {
    let mut state=paint_state(); let before=pixels(&state);
    state.session.tools.paint_isolate_selection=true;
    state.begin_paint_stroke();
    PaintModule::canvas_brush_with_settings(&mut state,8,8,BrushSettings{kind:BrushType::Fill,..Default::default()});
    state.finish_paint_stroke(false);
    let after=pixels(&state);
    assert_eq!(before.iter().zip(&after).filter(|(a,b)|a!=b).count(),0,"Fill bypassed mask");
}
fn rig_state()->(AppState,u32){
    let mut state=AppState::default();
    let sid=state.animate_add_rig(RigPresetKind::Humanoid).unwrap();
    let bone=state.project.project.get_skeleton(sid).unwrap().bones[1].id;
    state.animate_select_bone(Some(bone));
    (state,bone)
}
#[test]
fn delete_key_preserves_neighboring_frames_at_30fps(){
    let (mut state,bone)=rig_state();
    for time in [0.0,1.0/30.0,2.0/30.0]{state.session.animate.time=time;state.animate_insert_keyframe().unwrap();}
    state.animate_delete_keyframe(Some(1.0/30.0)).unwrap();
    let sid=state.session.animate.skeleton.unwrap();
    let track=state.project.project.get_animation_for_skeleton(sid).unwrap().clip.get_track(bone).unwrap();
    assert_eq!(track.translations.len(),2,"deleting one key must preserve adjacent frames");
}
#[test]
fn insert_key_preserves_evaluated_pose(){
    let (mut state,bone)=rig_state();
    state.session.animate.time=0.0;
    state.animate_insert_keyframe().unwrap();
    let mut moved=Transform3D::from_euler_degrees(0.0,0.0,0.0);
    moved.translation=[10.0,0.0,0.0];
    state.session.animate.time=1.0;
    state.session.animate.set_bone_draft(bone,moved);
    state.animate_insert_keyframe().unwrap();
    state.session.animate.time=0.5;
    let mut preview=AnimatePreview::default();
    let sid=state.session.animate.skeleton.unwrap();
    let idx=state.project.project.get_skeleton(sid).unwrap().bone_index(bone).unwrap();
    let before=preview.local_pose(&state.project.project,&state.session.animate).unwrap().1[idx];
    state.animate_insert_keyframe().unwrap();
    let after=preview.local_pose(&state.project.project,&state.session.animate).unwrap().1[idx];
    assert_eq!(before,after,"inserting a key changed the visible pose");
}
#[test]
fn delete_key_only_changes_active_clip(){
    let (mut state,bone)=rig_state();
    state.animate_insert_keyframe().unwrap();
    let original=state.project.project.animations[0].clone();
    let mut second=AnimationAsset::new("Second",original.clip.clone());
    second.tags=original.tags.clone();
    state.session.animate.active_clip=Some(second.id);
    state.project.project.add_animation(second);
    state.animate_delete_keyframe(Some(0.0)).unwrap();
    let original_after=state.project.project.get_animation(original.id).unwrap();
    assert!(original_after.clip.get_track(bone).unwrap().has_keyframe_at(0.0,1e-4),"inactive clip changed");
}
#[test]
fn brush_undo_and_cancel_control(){
    let mut state=paint_state();let before=pixels(&state);let depth=state.project.undo.depth();
    state.begin_paint_stroke();
    PaintModule::canvas_brush_with_settings(&mut state,8,8,BrushSettings{kind:BrushType::Pixel,..Default::default()});
    assert_ne!(pixels(&state),before);
    state.finish_paint_stroke(true);
    assert_eq!(pixels(&state),before);
    assert_eq!(state.project.undo.depth(),depth);
}

#[test]
fn maya_w_activates_move_instead_of_inherited_select(){
    let kb=Keybinds::load_profile("maya");
    assert_eq!(kb.find_in_context(KeyCode::KeyW,Mods2::default(),"model"),Some("model.transform"));
}
#[test]
fn animation_space_resolves_to_playback(){
    let kb=Keybinds::load_profile("petunia-default");
    assert_eq!(kb.find_in_context(KeyCode::Space,Mods2::default(),"animate"),Some("animate.toggle_play"));
}
#[test]
fn insert_key_only_changes_active_clip(){
    let (mut state,bone)=rig_state();state.animate_insert_keyframe().unwrap();
    let original=state.project.project.animations[0].clone();
    let mut second=AnimationAsset::new("Second",original.clip.clone());second.tags=original.tags.clone();
    let second_id=second.id;state.project.project.add_animation(second);
    state.session.animate.active_clip=Some(second_id);state.session.animate.time=1.0;
    state.animate_insert_keyframe().unwrap();
    let original_after=state.project.project.get_animation(original.id).unwrap();
    assert!(!original_after.clip.get_track(bone).unwrap().has_keyframe_at(1.0,1e-4),"inactive clip changed");
    assert!(state.project.project.get_animation(second_id).unwrap().clip.get_track(bone).unwrap().has_keyframe_at(1.0,1e-4));
}
