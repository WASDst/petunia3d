//! Pilha de camadas PAINT por comando.
//!
//! Antes, o bridge da UI convertia a textura em pilha, mutava a pilha numa
//! closure e gravava um snapshot. Agora a operação é um valor tipado e o
//! dispatcher cuida de Undo e rollback: uma operação que não muda nada falha
//! e desfaz também a conversão textura → pilha, sem Undo vazio.

use petunia_core::{Command, CommandContext, CommandError, EditorView};
use petunia_project::ProjectChanges;
use petunia_project::paint_layers::{
    DecalLayer, DecalVariant, GradientFill, GradientKind, GradientStop, LayerEffect, LayerKind,
    LayerMask, MAX_GRADIENT_STOPS, MAX_LAYER_EFFECTS, PaintEffect, PaintLayer, PaintLayerStack,
    PaintTarget,
};
use uuid::Uuid;

/// Onde uma máscara vive: na camada ativa ou num dos seus efeitos de sub-camada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskTarget {
    Layer,
    Effect(Uuid),
}

/// Efeito de sub-camada com parâmetros padrão para o tipo pedido.
pub fn effect_for_kind(kind: &str) -> Option<PaintEffect> {
    Some(match kind {
        "Pixelate" => PaintEffect::Pixelate { cell_size: 4 },
        "Posterize" => PaintEffect::Posterize { levels: 6 },
        "Invert" => PaintEffect::Invert,
        "Grain" => PaintEffect::Grain {
            intensity: 0.15,
            seed: 1,
        },
        "Levels" => PaintEffect::Levels {
            in_min: 0.0,
            in_max: 1.0,
            gamma: 1.0,
            out_min: 0.0,
            out_max: 1.0,
        },
        "BrightnessContrast" => PaintEffect::BrightnessContrast {
            brightness: 0.0,
            contrast: 0.0,
        },
        "HueSaturation" => PaintEffect::HueSaturation {
            hue_shift_deg: 0.0,
            saturation: 0.0,
        },
        _ => return None,
    })
}

/// Edição do decalque da camada ativa.
#[derive(Clone, Debug)]
pub enum DecalEdit {
    /// Rerasteriza o SVG de origem com o lado maior `size`.
    Rerasterize {
        size: u32,
    },
    /// Acrescenta uma variante e passa a mostrá-la.
    AddVariant {
        variant: Box<DecalVariant>,
    },
    SetVariant {
        index: usize,
    },
    /// Remove a variante mostrada (a última não sai).
    RemoveShownVariant,
    /// Grava a variante mostrada no quadro `frame` da trilha.
    SetKey {
        frame: u32,
    },
    RemoveKey {
        frame: u32,
    },
    /// Quadros por segundo da trilha, `1..=60`.
    TrackFps {
        fps: f32,
    },
    /// Duração do ciclo, nunca menor que a última chave.
    TrackLength {
        frames: u32,
    },
}

impl DecalEdit {
    fn apply(&self, decal: &mut DecalLayer, layer_name: &str) -> bool {
        match self {
            Self::Rerasterize { size } => decal.rerasterize(*size).unwrap_or(false),
            Self::AddVariant { variant } => {
                let index = decal.add_variant(layer_name, (**variant).clone());
                decal.set_variant(index)
            }
            Self::SetVariant { index } => decal.set_variant(*index),
            Self::RemoveShownVariant => decal.remove_variant(decal.variant_index),
            Self::SetKey { frame } => {
                let variant = decal.variant_index as u32;
                let track = decal.track.get_or_insert_with(Default::default);
                let before = track.clone();
                track.set_key(*frame, variant);
                *track != before
            }
            Self::RemoveKey { frame } => decal
                .track
                .as_mut()
                .is_some_and(|track| track.remove_key(*frame)),
            Self::TrackFps { fps } => {
                let fps = fps.round().clamp(1.0, 60.0);
                let track = decal.track.get_or_insert_with(Default::default);
                let changed = track.fps != fps;
                track.fps = fps;
                changed
            }
            Self::TrackLength { frames } => {
                let track = decal.track.get_or_insert_with(Default::default);
                let last_key = track.keys.last().map_or(0, |key| key.frame + 1);
                let length = (*frames).clamp(1, 600).max(last_key);
                let changed = track.length_frames != length;
                track.length_frames = length;
                changed
            }
        }
    }
}

/// Operação sobre a pilha de camadas do objeto ativo.
#[derive(Clone, Debug)]
pub enum PaintStackOp {
    AddLayer {
        name: String,
        width: u32,
        height: u32,
    },
    AddGroup {
        name: String,
    },
    AddEffect {
        name: String,
        effect: PaintEffect,
    },
    /// Efeito de sub-camada na camada ativa (aplicado antes da mescla, C1).
    AddLayerEffect {
        effect: PaintEffect,
    },
    RemoveLayerEffect {
        effect: Uuid,
    },
    SetLayerEffectEnabled {
        effect: Uuid,
        enabled: bool,
    },
    SetLayerEffectOpacity {
        effect: Uuid,
        opacity: f32,
    },
    SetLayerEffectParam {
        effect: Uuid,
        key: String,
        value: f32,
    },
    /// Cria/remove/edita a máscara da camada ativa ou de um efeito dela.
    AddMask {
        target: MaskTarget,
    },
    RemoveMask {
        target: MaskTarget,
    },
    SetMaskEnabled {
        target: MaskTarget,
        enabled: bool,
    },
    SetMaskInverted {
        target: MaskTarget,
        inverted: bool,
    },
    /// Para onde o pincel pinta na camada ativa (conteúdo ou máscara).
    SetPaintTarget {
        target: PaintTarget,
    },
    /// Camada de preenchimento por gradiente (C3).
    AddGradient {
        name: String,
        fill: GradientFill,
    },
    SetGradientKind {
        kind: GradientKind,
    },
    SetGradientRepeat {
        repeat: bool,
    },
    SetGradientGeometry {
        start: [f32; 2],
        end: [f32; 2],
    },
    SetGradientStop {
        index: usize,
        position: f32,
        color: [u8; 4],
    },
    SetGradientStopPosition {
        index: usize,
        position: f32,
    },
    SetGradientStopColor {
        index: usize,
        color: [u8; 4],
    },
    AddGradientStop {
        position: f32,
        color: [u8; 4],
    },
    RemoveGradientStop {
        index: usize,
    },
    /// Parâmetro `key` do efeito da camada ativa, limitado à faixa válida.
    SetEffectParam {
        key: String,
        value: f32,
    },
    AddDecal {
        name: String,
        decal: Box<DecalLayer>,
    },
    SetActive {
        layer: Uuid,
    },
    ToggleVisible {
        layer: Uuid,
    },
    ToggleLock {
        layer: Uuid,
    },
    /// A última camada é a base do raster e não sai.
    Remove {
        layer: Uuid,
    },
    MergeDown {
        layer: Uuid,
    },
    Move {
        layer: Uuid,
        delta: i32,
    },
    SetOpacity {
        layer: Uuid,
        opacity: f32,
    },
    SetBlend {
        layer: Uuid,
        blend: petunia_project::paint_layers::LayerBlendMode,
    },
    EditActiveDecal(DecalEdit),
}

fn set_effect_param(effect: &mut PaintEffect, key: &str, value: f32) -> bool {
    match (effect, key) {
        (PaintEffect::Pixelate { cell_size }, "cell_size") => {
            *cell_size = (value.round() as u32).clamp(1, 64);
        }
        (PaintEffect::Posterize { levels }, "levels") => {
            *levels = (value.round() as u8).clamp(2, 32);
        }
        (PaintEffect::Grain { intensity, .. }, "intensity") => *intensity = value.clamp(0.0, 1.0),
        (PaintEffect::Grain { seed, .. }, "seed") => *seed = value.max(0.0).round() as u32,
        (PaintEffect::Levels { in_min, in_max, .. }, "in_min") => {
            *in_min = value.clamp(0.0, (*in_max - 0.01).max(0.0));
        }
        (PaintEffect::Levels { in_min, in_max, .. }, "in_max") => {
            *in_max = value.clamp((*in_min + 0.01).min(1.0), 1.0);
        }
        (PaintEffect::Levels { gamma, .. }, "gamma") => *gamma = value.clamp(0.1, 4.0),
        (PaintEffect::Levels { out_min, .. }, "out_min") => *out_min = value.clamp(0.0, 1.0),
        (PaintEffect::Levels { out_max, .. }, "out_max") => *out_max = value.clamp(0.0, 1.0),
        (PaintEffect::BrightnessContrast { brightness, .. }, "brightness") => {
            *brightness = value.clamp(-1.0, 1.0);
        }
        (PaintEffect::BrightnessContrast { contrast, .. }, "contrast") => {
            *contrast = value.clamp(-1.0, 1.0);
        }
        (PaintEffect::HueSaturation { hue_shift_deg, .. }, "hue_shift_deg") => {
            *hue_shift_deg = value.clamp(-180.0, 180.0);
        }
        (PaintEffect::HueSaturation { saturation, .. }, "saturation") => {
            *saturation = value.clamp(-1.0, 1.0);
        }
        _ => return false,
    }
    true
}

/// Máscara da camada ou de um efeito dela (C1).
fn mask_mut(layer: &mut PaintLayer, target: MaskTarget) -> Option<&mut LayerMask> {
    match target {
        MaskTarget::Layer => layer.mask.as_mut(),
        MaskTarget::Effect(id) => layer
            .effects
            .iter_mut()
            .find(|effect| effect.id == id)
            .and_then(|effect| effect.mask.as_mut()),
    }
}

/// Dimensões de referência para uma máscara nova: o maior canvas disponível.
fn mask_dimensions(stack: &PaintLayerStack) -> (u32, u32) {
    stack
        .layers
        .iter()
        .rev()
        .find_map(|layer| layer.canvas().map(|canvas| (canvas.w, canvas.h)))
        .unwrap_or((256, 256))
}

fn gradient_mut(layer: &mut PaintLayer) -> Option<&mut GradientFill> {
    match &mut layer.kind {
        LayerKind::Gradient(fill) => Some(fill),
        _ => None,
    }
}

fn sort_stops(fill: &mut GradientFill) {
    fill.stops.sort_by(|a, b| a.position.total_cmp(&b.position));
}

impl PaintStackOp {
    /// Aplica a operação; `false` quando nada mudaria.
    fn apply(&self, stack: &mut PaintLayerStack, mesh: Option<&petunia_core::Mesh>) -> bool {
        let find = |stack: &mut PaintLayerStack, id: Uuid| {
            stack.layers.iter().position(|layer| layer.id == id)
        };
        match self {
            Self::AddLayer {
                name,
                width,
                height,
            } => {
                stack.add_layer(PaintLayer::new(name.clone(), *width, *height, [0; 4]));
                true
            }
            Self::AddGroup { name } => {
                stack.add_group(name.clone());
                true
            }
            Self::AddEffect { name, effect } => {
                stack.add_layer(PaintLayer::new_effect(name.clone(), *effect));
                true
            }
            Self::AddLayerEffect { effect } => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                if layer.is_group
                    || matches!(layer.kind, LayerKind::Effect(_))
                    || layer.effects.len() >= MAX_LAYER_EFFECTS
                {
                    return false;
                }
                layer.effects.push(LayerEffect::new(*effect));
                true
            }
            Self::RemoveLayerEffect { effect } => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(index) = layer.effects.iter().position(|entry| entry.id == *effect) else {
                    return false;
                };
                layer.effects.remove(index);
                if layer.paint_target == PaintTarget::EffectMask(*effect) {
                    layer.paint_target = PaintTarget::Content;
                }
                true
            }
            Self::SetLayerEffectEnabled { effect, enabled } => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(entry) = layer.effects.iter_mut().find(|entry| entry.id == *effect) else {
                    return false;
                };
                if entry.enabled == *enabled {
                    return false;
                }
                entry.enabled = *enabled;
                if !*enabled && layer.paint_target == PaintTarget::EffectMask(*effect) {
                    layer.paint_target = PaintTarget::Content;
                }
                true
            }
            Self::SetLayerEffectOpacity { effect, opacity } => {
                if !opacity.is_finite() {
                    return false;
                }
                let opacity = opacity.clamp(0.0, 1.0);
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(entry) = layer.effects.iter_mut().find(|entry| entry.id == *effect) else {
                    return false;
                };
                if (entry.opacity - opacity).abs() < f32::EPSILON {
                    return false;
                }
                entry.opacity = opacity;
                true
            }
            Self::SetLayerEffectParam { effect, key, value } => {
                if !value.is_finite() {
                    return false;
                }
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(entry) = layer.effects.iter_mut().find(|entry| entry.id == *effect) else {
                    return false;
                };
                set_effect_param(&mut entry.effect, key, *value)
            }
            Self::AddMask { target } => {
                let (w, h) = mask_dimensions(stack);
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                match target {
                    MaskTarget::Layer => {
                        if layer.mask.is_some() {
                            return false;
                        }
                        layer.mask = Some(LayerMask::new(w, h));
                        layer.paint_target = PaintTarget::LayerMask;
                    }
                    MaskTarget::Effect(id) => {
                        {
                            let Some(entry) =
                                layer.effects.iter_mut().find(|entry| entry.id == *id)
                            else {
                                return false;
                            };
                            if entry.mask.is_some() {
                                return false;
                            }
                            entry.mask = Some(LayerMask::new(w, h));
                        }
                        layer.paint_target = PaintTarget::EffectMask(*id);
                    }
                }
                true
            }
            Self::RemoveMask { target } => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                match target {
                    MaskTarget::Layer => {
                        if layer.mask.take().is_none() {
                            return false;
                        }
                        if layer.paint_target == PaintTarget::LayerMask {
                            layer.paint_target = PaintTarget::Content;
                        }
                    }
                    MaskTarget::Effect(id) => {
                        let removed = {
                            let Some(entry) =
                                layer.effects.iter_mut().find(|entry| entry.id == *id)
                            else {
                                return false;
                            };
                            entry.mask.take().is_some()
                        };
                        if !removed {
                            return false;
                        }
                        if layer.paint_target == PaintTarget::EffectMask(*id) {
                            layer.paint_target = PaintTarget::Content;
                        }
                    }
                }
                true
            }
            Self::SetMaskEnabled { target, enabled } => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(mask) = mask_mut(layer, *target) else {
                    return false;
                };
                if mask.enabled == *enabled {
                    return false;
                }
                mask.enabled = *enabled;
                true
            }
            Self::SetMaskInverted { target, inverted } => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(mask) = mask_mut(layer, *target) else {
                    return false;
                };
                if mask.inverted == *inverted {
                    return false;
                }
                mask.inverted = *inverted;
                true
            }
            Self::SetPaintTarget { target } => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let valid = match target {
                    PaintTarget::Content => true,
                    PaintTarget::LayerMask => layer.mask.is_some(),
                    PaintTarget::EffectMask(id) => layer
                        .effects
                        .iter()
                        .any(|effect| effect.id == *id && effect.mask.is_some()),
                };
                if !valid || layer.paint_target == *target {
                    return false;
                }
                layer.paint_target = *target;
                true
            }
            Self::AddGradient { name, fill } => {
                stack.add_layer(PaintLayer::new_gradient(name.clone(), fill.clone()));
                true
            }
            Self::SetGradientKind { kind } => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(fill) = gradient_mut(layer) else {
                    return false;
                };
                if fill.kind == *kind {
                    return false;
                }
                fill.kind = *kind;
                true
            }
            Self::SetGradientRepeat { repeat } => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(fill) = gradient_mut(layer) else {
                    return false;
                };
                if fill.repeat == *repeat {
                    return false;
                }
                fill.repeat = *repeat;
                true
            }
            Self::SetGradientGeometry { start, end } => {
                if !start.iter().all(|value| value.is_finite())
                    || !end.iter().all(|value| value.is_finite())
                {
                    return false;
                }
                let start = start.map(|value| value.clamp(-8.0, 8.0));
                let end = end.map(|value| value.clamp(-8.0, 8.0));
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(fill) = gradient_mut(layer) else {
                    return false;
                };
                if fill.start == start && fill.end == end {
                    return false;
                }
                fill.start = start;
                fill.end = end;
                true
            }
            Self::SetGradientStop {
                index,
                position,
                color,
            } => {
                if !position.is_finite() {
                    return false;
                }
                let position = position.clamp(0.0, 1.0);
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(fill) = gradient_mut(layer) else {
                    return false;
                };
                let Some(stop) = fill.stops.get_mut(*index) else {
                    return false;
                };
                if stop.position == position && stop.color == *color {
                    return false;
                }
                stop.position = position;
                stop.color = *color;
                sort_stops(fill);
                true
            }
            Self::SetGradientStopPosition { index, position } => {
                if !position.is_finite() {
                    return false;
                }
                let position = position.clamp(0.0, 1.0);
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(fill) = gradient_mut(layer) else {
                    return false;
                };
                let Some(stop) = fill.stops.get_mut(*index) else {
                    return false;
                };
                if stop.position == position {
                    return false;
                }
                stop.position = position;
                sort_stops(fill);
                true
            }
            Self::SetGradientStopColor { index, color } => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(fill) = gradient_mut(layer) else {
                    return false;
                };
                let Some(stop) = fill.stops.get_mut(*index) else {
                    return false;
                };
                if stop.color == *color {
                    return false;
                }
                stop.color = *color;
                true
            }
            Self::AddGradientStop { position, color } => {
                if !position.is_finite() || *position < 0.0 || *position > 1.0 {
                    return false;
                }
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(fill) = gradient_mut(layer) else {
                    return false;
                };
                if fill.stops.len() >= MAX_GRADIENT_STOPS {
                    return false;
                }
                fill.stops.push(GradientStop {
                    position: *position,
                    color: *color,
                });
                sort_stops(fill);
                true
            }
            Self::RemoveGradientStop { index } => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let Some(fill) = gradient_mut(layer) else {
                    return false;
                };
                if fill.stops.len() <= 2 || *index >= fill.stops.len() {
                    return false;
                }
                fill.stops.remove(*index);
                true
            }
            Self::SetEffectParam { key, value } => {
                if !value.is_finite() {
                    return false;
                }
                match stack.active_mut().map(|layer| &mut layer.kind) {
                    Some(LayerKind::Effect(effect)) => set_effect_param(effect, key, *value),
                    _ => false,
                }
            }
            Self::AddDecal { name, decal } => {
                stack.add_layer(PaintLayer::new_decal(name.clone(), (**decal).clone()));
                true
            }
            Self::SetActive { layer } => stack.set_active(*layer),
            Self::ToggleVisible { layer } => match find(stack, *layer) {
                Some(index) => {
                    let layer = &mut stack.layers[index];
                    layer.visible = !layer.visible;
                    true
                }
                None => false,
            },
            Self::ToggleLock { layer } => match find(stack, *layer) {
                Some(index) => {
                    let layer = &mut stack.layers[index];
                    layer.locked = !layer.locked;
                    true
                }
                None => false,
            },
            Self::Remove { layer } => stack.layers.len() > 1 && stack.remove_layer(*layer),
            Self::MergeDown { layer } => match find(stack, *layer) {
                Some(index) if index > 0 => stack.merge_down_on(index, mesh),
                _ => false,
            },
            Self::Move { layer, delta } => {
                let Some(from) = find(stack, *layer) else {
                    return false;
                };
                let to = from as i32 + delta;
                to >= 0 && (to as usize) < stack.layers.len() && stack.move_layer(from, to as usize)
            }
            Self::SetOpacity { layer, opacity } => {
                if !opacity.is_finite() {
                    return false;
                }
                let opacity = opacity.clamp(0.0, 1.0);
                match find(stack, *layer) {
                    Some(index)
                        if (stack.layers[index].opacity - opacity).abs() >= f32::EPSILON =>
                    {
                        stack.layers[index].opacity = opacity;
                        true
                    }
                    _ => false,
                }
            }
            Self::SetBlend { layer, blend } => match find(stack, *layer) {
                Some(index) if stack.layers[index].blend != *blend => {
                    stack.layers[index].blend = *blend;
                    true
                }
                _ => false,
            },
            Self::EditActiveDecal(edit) => {
                let Some(layer) = stack.active_mut() else {
                    return false;
                };
                let name = layer.name.clone();
                match &mut layer.kind {
                    LayerKind::Decal(decal) => edit.apply(decal, &name),
                    _ => false,
                }
            }
        }
    }
}

/// Uma operação de camada = uma entrada de Undo, com recomposição da textura.
#[derive(Clone, Debug)]
pub struct PaintStackCmd {
    pub label: &'static str,
    pub op: PaintStackOp,
}

impl Command for PaintStackCmd {
    fn label(&self) -> &'static str {
        self.label
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::TEXTURES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        match state.document.active() {
            None => Err("no active object"),
            Some(asset) if asset.locked => Err("object is locked"),
            Some(_) => Ok(()),
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        crate::PaintModule::ensure_stack_in(state);
        // Só a fusão precisa da malha (para sangrar entre ilhas UV).
        let mesh = matches!(self.op, PaintStackOp::MergeDown { .. })
            .then(|| {
                state
                    .document
                    .active()
                    .map(|asset| asset.mesh.to_owned_value())
            })
            .flatten();
        let stack = state
            .document
            .active_mut()
            .and_then(|asset| asset.paint_stack.as_mut())
            .ok_or(CommandError::NoActiveAsset)?;
        if !self.op.apply(stack, mesh.as_ref()) {
            // O dispatcher desfaz também a conversão textura → pilha.
            return Err(CommandError::Execution(
                "paint layer: nothing to change".into(),
            ));
        }
        crate::PaintModule::composite_active_in(state);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack_with_raster() -> PaintLayerStack {
        let mut stack = PaintLayerStack::new();
        stack.add_layer(PaintLayer::new("Camada", 8, 8, [255, 0, 0, 255]));
        stack
    }

    fn gradient_of(stack: &PaintLayerStack) -> &GradientFill {
        match &stack.active().expect("camada ativa").kind {
            LayerKind::Gradient(fill) => fill,
            other => panic!("esperava gradiente, veio {other:?}"),
        }
    }

    #[test]
    fn layer_effects_follow_the_layer_kind_and_the_cap() {
        let mut stack = stack_with_raster();
        let effect = PaintEffect::Grain {
            intensity: 0.5,
            seed: 1,
        };
        for _ in 0..MAX_LAYER_EFFECTS {
            assert!(PaintStackOp::AddLayerEffect { effect }.apply(&mut stack, None));
        }
        assert!(
            !PaintStackOp::AddLayerEffect { effect }.apply(&mut stack, None),
            "limite de efeitos por camada"
        );

        let id = stack.active().expect("ativa").effects[0].id;
        assert!(
            PaintStackOp::SetLayerEffectOpacity {
                effect: id,
                opacity: 0.5
            }
            .apply(&mut stack, None)
        );
        assert!(
            !PaintStackOp::SetLayerEffectOpacity {
                effect: id,
                opacity: f32::NAN
            }
            .apply(&mut stack, None)
        );
        assert!(
            PaintStackOp::SetLayerEffectParam {
                effect: id,
                key: "intensity".into(),
                value: 0.9
            }
            .apply(&mut stack, None)
        );
        assert!(
            !PaintStackOp::SetLayerEffectParam {
                effect: id,
                key: "intensity".into(),
                value: f32::INFINITY
            }
            .apply(&mut stack, None),
            "valor não finito é recusado"
        );
        assert!(
            !PaintStackOp::SetLayerEffectParam {
                effect: uuid::Uuid::new_v4(),
                key: "intensity".into(),
                value: 0.2
            }
            .apply(&mut stack, None)
        );
        assert!(
            !PaintStackOp::SetLayerEffectParam {
                effect: id,
                key: "chave_inexistente".into(),
                value: 1.0
            }
            .apply(&mut stack, None)
        );

        // Grupo e camada de ajuste não têm conteúdo próprio: sem sub-efeitos.
        let mut groups = PaintLayerStack::new();
        groups.add_group("Grupo");
        assert!(!PaintStackOp::AddLayerEffect { effect }.apply(&mut groups, None));
        groups.add_layer(PaintLayer::new_effect("Ajuste", PaintEffect::Invert));
        assert!(!PaintStackOp::AddLayerEffect { effect }.apply(&mut groups, None));

        // Remover o efeito que é alvo do pincel devolve o alvo ao conteúdo.
        let mut stack = stack_with_raster();
        assert!(PaintStackOp::AddLayerEffect { effect }.apply(&mut stack, None));
        let id = stack.active().expect("ativa").effects[0].id;
        assert!(
            PaintStackOp::AddMask {
                target: MaskTarget::Effect(id)
            }
            .apply(&mut stack, None)
        );
        assert_eq!(
            stack.active().expect("ativa").paint_target,
            PaintTarget::EffectMask(id)
        );
        assert!(PaintStackOp::RemoveLayerEffect { effect: id }.apply(&mut stack, None));
        assert_eq!(
            stack.active().expect("ativa").paint_target,
            PaintTarget::Content
        );
    }

    #[test]
    fn paint_target_only_accepts_a_mask_that_exists() {
        let mut stack = stack_with_raster();
        assert!(
            !PaintStackOp::SetPaintTarget {
                target: PaintTarget::LayerMask
            }
            .apply(&mut stack, None)
        );

        assert!(
            PaintStackOp::AddMask {
                target: MaskTarget::Layer
            }
            .apply(&mut stack, None)
        );
        assert_eq!(
            stack.active().expect("ativa").paint_target,
            PaintTarget::LayerMask,
            "criar a máscara já aponta o pincel para ela"
        );
        assert!(
            !PaintStackOp::AddMask {
                target: MaskTarget::Layer
            }
            .apply(&mut stack, None),
            "não há duas máscaras na mesma camada"
        );

        assert!(
            PaintStackOp::SetPaintTarget {
                target: PaintTarget::Content
            }
            .apply(&mut stack, None)
        );
        assert!(
            !PaintStackOp::SetPaintTarget {
                target: PaintTarget::Content
            }
            .apply(&mut stack, None),
            "repetir o mesmo alvo não muda nada"
        );
        assert!(
            !PaintStackOp::SetPaintTarget {
                target: PaintTarget::EffectMask(uuid::Uuid::new_v4())
            }
            .apply(&mut stack, None)
        );

        let effect = PaintEffect::Invert;
        assert!(PaintStackOp::AddLayerEffect { effect }.apply(&mut stack, None));
        let id = stack.active().expect("ativa").effects[0].id;
        assert!(
            PaintStackOp::AddMask {
                target: MaskTarget::Effect(id)
            }
            .apply(&mut stack, None),
            "máscara do efeito criada"
        );
        assert!(
            PaintStackOp::SetPaintTarget {
                target: PaintTarget::Content
            }
            .apply(&mut stack, None)
        );
        assert!(
            PaintStackOp::SetPaintTarget {
                target: PaintTarget::EffectMask(id)
            }
            .apply(&mut stack, None)
        );
        assert!(
            PaintStackOp::SetMaskEnabled {
                target: MaskTarget::Effect(id),
                enabled: false
            }
            .apply(&mut stack, None)
        );
        assert!(
            !PaintStackOp::SetMaskEnabled {
                target: MaskTarget::Effect(id),
                enabled: false
            }
            .apply(&mut stack, None)
        );
        assert!(
            PaintStackOp::SetMaskInverted {
                target: MaskTarget::Effect(id),
                inverted: true
            }
            .apply(&mut stack, None)
        );

        // Remover a máscara da camada não muda o alvo (que está no efeito).
        assert!(
            PaintStackOp::RemoveMask {
                target: MaskTarget::Layer
            }
            .apply(&mut stack, None)
        );
        assert_eq!(
            stack.active().expect("ativa").paint_target,
            PaintTarget::EffectMask(id)
        );
        assert!(
            !PaintStackOp::RemoveMask {
                target: MaskTarget::Layer
            }
            .apply(&mut stack, None)
        );
    }

    #[test]
    fn gradient_ops_validate_geometry_and_keep_two_stops() {
        let mut stack = stack_with_raster();
        let fill = GradientFill::new([255, 0, 0, 255], [0, 0, 255, 255]);
        assert!(
            PaintStackOp::AddGradient {
                name: "Grad".into(),
                fill
            }
            .apply(&mut stack, None)
        );
        assert!(matches!(
            stack.active().expect("ativa").kind,
            LayerKind::Gradient(_)
        ));

        assert!(
            PaintStackOp::SetGradientKind {
                kind: GradientKind::Radial
            }
            .apply(&mut stack, None)
        );
        assert!(
            !PaintStackOp::SetGradientKind {
                kind: GradientKind::Radial
            }
            .apply(&mut stack, None),
            "mesmo tipo não muda"
        );
        assert!(PaintStackOp::SetGradientRepeat { repeat: true }.apply(&mut stack, None));
        assert!(
            !PaintStackOp::SetGradientGeometry {
                start: [f32::NAN, 0.0],
                end: [1.0, 0.5]
            }
            .apply(&mut stack, None)
        );
        assert!(
            PaintStackOp::SetGradientGeometry {
                start: [0.2, 0.5],
                end: [0.9, 0.5]
            }
            .apply(&mut stack, None)
        );

        // Posição fora da faixa é clampada; não finita é recusada.
        assert!(
            PaintStackOp::SetGradientStopPosition {
                index: 0,
                position: 5.0
            }
            .apply(&mut stack, None)
        );
        assert_eq!(
            gradient_of(&stack).stops.first().expect("parada").position,
            1.0
        );
        assert!(
            !PaintStackOp::SetGradientStopPosition {
                index: 0,
                position: f32::NAN
            }
            .apply(&mut stack, None)
        );
        assert!(
            !PaintStackOp::SetGradientStopPosition {
                index: 99,
                position: 0.5
            }
            .apply(&mut stack, None)
        );

        // Cor muda sem mover a parada.
        let position = gradient_of(&stack).stops[0].position;
        assert!(
            PaintStackOp::SetGradientStopColor {
                index: 0,
                color: [0, 255, 0, 255]
            }
            .apply(&mut stack, None)
        );
        assert_eq!(gradient_of(&stack).stops[0].position, position);
        assert_eq!(gradient_of(&stack).stops[0].color, [0, 255, 0, 255]);

        while gradient_of(&stack).stops.len() < MAX_GRADIENT_STOPS {
            assert!(
                PaintStackOp::AddGradientStop {
                    position: 0.5,
                    color: [1, 2, 3, 255]
                }
                .apply(&mut stack, None)
            );
        }
        assert!(
            !PaintStackOp::AddGradientStop {
                position: 0.5,
                color: [1, 2, 3, 255]
            }
            .apply(&mut stack, None)
        );
        assert!(
            !PaintStackOp::AddGradientStop {
                position: f32::NAN,
                color: [1, 2, 3, 255]
            }
            .apply(&mut stack, None)
        );

        while (PaintStackOp::RemoveGradientStop { index: 0 }).apply(&mut stack, None) {}
        assert_eq!(
            gradient_of(&stack).stops.len(),
            2,
            "o gradiente nunca desce de duas paradas"
        );
    }

    #[test]
    fn effect_for_kind_covers_the_panel_choices() {
        for kind in [
            "Pixelate",
            "Posterize",
            "Invert",
            "Grain",
            "Levels",
            "BrightnessContrast",
            "HueSaturation",
        ] {
            assert!(effect_for_kind(kind).is_some(), "{kind}");
        }
        assert!(
            effect_for_kind("Gradient").is_none(),
            "gradiente é camada, não efeito"
        );
        assert!(effect_for_kind("").is_none());
    }
}
