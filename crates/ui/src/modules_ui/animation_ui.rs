//! Painel de Animação e Rigging (`Workspace::Animate`) — P3D-066, P3D-067, P3D-135 a P3D-139.
//! Interface desacoplada de edição de esqueletos, bind pose, poses, trilhas e clipes.

use crate::tokens;
use egui::{CollapsingHeader, DragValue, Grid, RichText, Ui};
use petunia_core::state::AppState;
use petunia_project::animation::{
    AnimationClip, AnimationLibrary, RigPreset, auto_fit_humanoid, compute_auto_skin_weights,
};

/// Renderiza o painel lateral de propriedades do workspace Animate.
pub fn draw_animation_panel(ui: &mut Ui, state: &mut AppState) {
    ui.add_space(4.0);

    // 1. Cabeçalho de Armatures / Esqueletos
    CollapsingHeader::new(
        RichText::new("Armatures & Esqueletos")
            .size(12.0)
            .color(tokens::TEXT_PRIMARY),
    )
    .default_open(true)
    .show(ui, |ui| {
        draw_skeletons_section(ui, state);
    });

    ui.add_space(4.0);

    // 2. Inspetor do Osso Ativo
    CollapsingHeader::new(
        RichText::new("Inspetor de Ossos")
            .size(12.0)
            .color(tokens::TEXT_PRIMARY),
    )
    .default_open(true)
    .show(ui, |ui| {
        draw_bone_inspector_section(ui, state);
    });

    ui.add_space(4.0);

    // 3. Clipes de Animação & Biblioteca
    CollapsingHeader::new(
        RichText::new("Clipes & Biblioteca de Animação")
            .size(12.0)
            .color(tokens::TEXT_PRIMARY),
    )
    .default_open(true)
    .show(ui, |ui| {
        draw_animation_clips_section(ui, state);
    });

    ui.add_space(4.0);

    // 4. Transporte & Keyframing
    CollapsingHeader::new(
        RichText::new("⏱️ Transporte & Keyframes")
            .size(12.0)
            .color(tokens::TEXT_PRIMARY),
    )
    .default_open(true)
    .show(ui, |ui| {
        draw_transport_section(ui, state);
    });
}

fn draw_skeletons_section(ui: &mut Ui, state: &mut AppState) {
    let mut dirty = false;

    // Presets rápidos
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new("Presets:")
                .size(10.5)
                .color(tokens::TEXT_SECONDARY),
        );

        if ui.button("Humanoide").clicked() {
            let skel = RigPreset::humanoid(1.0);
            state.project.add_skeleton(skel);
            dirty = true;
        }

        if ui.button("Quadrúpede").clicked() {
            let skel = RigPreset::quadruped(1.0);
            state.project.add_skeleton(skel);
            dirty = true;
        }

        if ui.button("Multi-Leg").clicked() {
            let skel = RigPreset::multi_leg(8, 1.0);
            state.project.add_skeleton(skel);
            dirty = true;
        }
    });

    ui.add_space(3.0);

    // Auto-Rig e Auto-Skin sobre o ativo selecionado
    let active_idx = state.project.active;
    let has_mesh = state.project.assets.get(active_idx).is_some();

    if has_mesh {
        ui.horizontal(|ui| {
            if ui
                .button("Auto-Rig Malha Ativa")
                .on_hover_text(
                    "Ajusta e dimensiona um esqueleto humanoide às dimensões do modelo ativo",
                )
                .clicked()
            {
                let active_asset = state.project.assets.get_mut(active_idx);
                if let Some(asset) = active_asset {
                    let skel = auto_fit_humanoid(&asset.mesh);
                    let skel_id = skel.id;
                    let skin = compute_auto_skin_weights(&asset.mesh, &skel);
                    asset.skeleton_id = Some(skel_id);
                    asset.skin_data = Some(skin);
                    state.project.add_skeleton(skel);
                    dirty = true;
                }
            }

            if ui
                .button("Auto-Skin")
                .on_hover_text("Calcula pesos automáticos por proximidade geométrica")
                .clicked()
            {
                let skel_opt = state
                    .project
                    .assets
                    .get(active_idx)
                    .and_then(|a| a.skeleton_id)
                    .and_then(|id| state.project.get_skeleton(id))
                    .cloned();

                if let (Some(skel), Some(asset)) =
                    (skel_opt, state.project.assets.get_mut(active_idx))
                {
                    let skin = compute_auto_skin_weights(&asset.mesh, &skel);
                    asset.skin_data = Some(skin);
                    dirty = true;
                }
            }
        });
    }

    ui.add_space(4.0);

    // Lista de esqueletos na cena
    if state.project.skeletons.is_empty() {
        ui.label(
            RichText::new("Nenhum esqueleto no projeto. Use os presets acima para adicionar.")
                .size(10.5)
                .color(tokens::TEXT_SECONDARY),
        );
        if dirty {
            state.mark_dirty();
        }
        return;
    }

    let mut remove_id = None;
    for (idx, skel) in state.project.skeletons.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{}.", idx + 1));
            ui.text_edit_singleline(&mut skel.name);
            ui.label(
                RichText::new(format!("({} ossos)", skel.bones.len()))
                    .size(9.5)
                    .color(tokens::TEXT_SECONDARY),
            );
            if ui.button("×").on_hover_text("Excluir esqueleto").clicked() {
                remove_id = Some(skel.id);
            }
        });
    }

    if let Some(id) = remove_id {
        state.project.remove_skeleton(id);
        dirty = true;
    }

    if dirty {
        state.mark_dirty();
    }
}

fn draw_bone_inspector_section(ui: &mut Ui, state: &mut AppState) {
    let project = &mut state.project;
    if project.skeletons.is_empty() {
        ui.label(
            RichText::new("Nenhum esqueleto selecionado.")
                .size(10.5)
                .color(tokens::TEXT_SECONDARY),
        );
        return;
    }

    // Primeiro esqueleto ou o vinculado ao ativo
    let skel = &mut project.skeletons[0];
    if skel.bones.is_empty() {
        ui.label(
            RichText::new("O esqueleto não possui ossos cadastrados.")
                .size(10.5)
                .color(tokens::TEXT_SECONDARY),
        );
        return;
    }

    ui.label(
        RichText::new(format!("Esqueleto: {}", skel.name))
            .size(11.0)
            .color(tokens::TEXT_PRIMARY),
    );

    // Seleção de osso ativo via combo
    let mut selected_bone_id = ui.data_mut(|d| {
        d.get_temp::<u32>(egui::Id::new("anim.selected_bone"))
            .unwrap_or(0)
    });

    egui::ComboBox::from_label("Osso Ativo")
        .width(ui.available_width().clamp(96.0, 180.0))
        .selected_text(
            skel.get_bone(selected_bone_id)
                .map(|b| b.name.as_str())
                .unwrap_or("Selecione..."),
        )
        .show_ui(ui, |ui| {
            for b in &skel.bones {
                ui.selectable_value(&mut selected_bone_id, b.id, &b.name);
            }
        });

    ui.data_mut(|d| d.insert_temp(egui::Id::new("anim.selected_bone"), selected_bone_id));

    if let Some(bone) = skel.get_bone_mut(selected_bone_id) {
        ui.add_space(3.0);
        Grid::new("bone_details_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Nome:")
                        .size(10.5)
                        .color(tokens::TEXT_SECONDARY),
                );
                ui.text_edit_singleline(&mut bone.name);
                ui.end_row();

                ui.label(
                    RichText::new("Pai:")
                        .size(10.5)
                        .color(tokens::TEXT_SECONDARY),
                );
                ui.label(match bone.parent {
                    Some(p) => format!("Bone ID #{p}"),
                    None => "Raiz (Nenhum)".to_string(),
                });
                ui.end_row();

                ui.label(
                    RichText::new("Head [X, Y, Z]:")
                        .size(10.5)
                        .color(tokens::TEXT_SECONDARY),
                );
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut bone.head[0]).speed(0.02).prefix("X: "));
                    ui.add(DragValue::new(&mut bone.head[1]).speed(0.02).prefix("Y: "));
                    ui.add(DragValue::new(&mut bone.head[2]).speed(0.02).prefix("Z: "));
                });
                ui.end_row();

                ui.label(
                    RichText::new("Tail [X, Y, Z]:")
                        .size(10.5)
                        .color(tokens::TEXT_SECONDARY),
                );
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut bone.tail[0]).speed(0.02).prefix("X: "));
                    ui.add(DragValue::new(&mut bone.tail[1]).speed(0.02).prefix("Y: "));
                    ui.add(DragValue::new(&mut bone.tail[2]).speed(0.02).prefix("Z: "));
                });
                ui.end_row();

                ui.label(
                    RichText::new("Comprimento:")
                        .size(10.5)
                        .color(tokens::TEXT_SECONDARY),
                );
                ui.label(format!("{:.3} m", bone.length()));
                ui.end_row();
            });
    }
}

fn draw_animation_clips_section(ui: &mut Ui, state: &mut AppState) {
    let mut dirty = false;

    ui.horizontal(|ui| {
        if ui.button("+ Novo Clipe").clicked() {
            let clip = AnimationClip::new("New_Animation", 2.0);
            state
                .project
                .add_animation(petunia_project::animation::AnimationAsset::new(
                    "New_Animation",
                    clip,
                ));
            dirty = true;
        }

        if !state.project.skeletons.is_empty() {
            if ui.button("Carregar Idle").clicked() {
                let clip = AnimationLibrary::humanoid_idle(&state.project.skeletons[0]);
                state
                    .project
                    .add_animation(petunia_project::animation::AnimationAsset::new(
                        "Humanoid_Idle",
                        clip,
                    ));
                dirty = true;
            }

            if ui.button("Carregar Walk").clicked() {
                let clip = AnimationLibrary::humanoid_walk(&state.project.skeletons[0]);
                state
                    .project
                    .add_animation(petunia_project::animation::AnimationAsset::new(
                        "Humanoid_Walk",
                        clip,
                    ));
                dirty = true;
            }
        }
    });

    ui.add_space(4.0);

    if state.project.animations.is_empty() {
        ui.label(
            RichText::new("Nenhum clipe de animação cadastrado no projeto.")
                .size(10.5)
                .color(tokens::TEXT_SECONDARY),
        );
        if dirty {
            state.mark_dirty();
        }
        return;
    }

    let mut remove_anim_id = None;
    for (idx, asset) in state.project.animations.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("{}.", idx + 1));
            ui.text_edit_singleline(&mut asset.name);
            ui.add(
                DragValue::new(&mut asset.clip.duration)
                    .speed(0.1)
                    .range(0.1..=60.0)
                    .suffix("s"),
            );
            ui.checkbox(&mut asset.clip.looping, "Loop");
            if ui.button("×").on_hover_text("Remover clipe").clicked() {
                remove_anim_id = Some(asset.id);
            }
        });
    }

    if let Some(id) = remove_anim_id {
        state.project.remove_animation(id);
        dirty = true;
    }

    if dirty {
        state.mark_dirty();
    }
}

fn draw_transport_section(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        let (play_label, play_color) = if state.ui.timeline_playing {
            ("Pausar", tokens::ACCENT_BLUE)
        } else {
            ("Reproduzir", tokens::TEXT_PRIMARY)
        };

        if ui
            .button(RichText::new(play_label).color(play_color))
            .clicked()
        {
            state.ui.timeline_playing = !state.ui.timeline_playing;
            state.mark_dirty();
        }

        if ui.button("⏮ Início").clicked() {
            state.ui.timeline_frame = state.ui.timeline_start;
            state.mark_dirty();
        }

        if ui.button("⏭ Fim").clicked() {
            state.ui.timeline_frame = state.ui.timeline_end;
            state.mark_dirty();
        }
    });

    ui.add_space(3.0);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Frame:")
                .size(10.5)
                .color(tokens::TEXT_SECONDARY),
        );
        ui.add(
            DragValue::new(&mut state.ui.timeline_frame)
                .range(state.ui.timeline_start..=state.ui.timeline_end),
        );

        ui.label(
            RichText::new("Alcance:")
                .size(10.5)
                .color(tokens::TEXT_SECONDARY),
        );
        ui.add(DragValue::new(&mut state.ui.timeline_start).range(0..=state.ui.timeline_end));
        ui.label("..");
        ui.add(DragValue::new(&mut state.ui.timeline_end).range(state.ui.timeline_start..=1000));
    });

    // Inserção rápida de Keyframe de Pose
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Keyframe:")
                .size(10.5)
                .color(tokens::TEXT_SECONDARY),
        );

        if ui
            .button("• Inserir Pose Key")
            .on_hover_text("Grava a transformação do osso ativo no frame atual")
            .clicked()
        {
            let selected_bone_id = ui.data_mut(|d| {
                d.get_temp::<u32>(egui::Id::new("anim.selected_bone"))
                    .unwrap_or(0)
            });

            // Chave local absoluta: usa o repouso do osso (AN-16), não a origem.
            let rest = state
                .project
                .skeletons
                .iter()
                .find_map(|s| s.get_bone(selected_bone_id))
                .map(|b| b.local_transform)
                .unwrap_or_default();
            if let Some(anim) = state.project.animations.first_mut() {
                let time = (state.ui.timeline_frame as f32 / anim.clip.fps).max(0.0);
                let track = anim
                    .clip
                    .get_or_create_track(selected_bone_id, &format!("Bone_{selected_bone_id}"));
                track.add_translation(time, rest.translation);
                track.add_rotation(time, rest.rotation);
                track.add_scale(time, rest.scale);
                state.mark_dirty();
            }
        }
    });
}
