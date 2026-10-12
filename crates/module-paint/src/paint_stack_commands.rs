//! Pilha de camadas PAINT por comando.
//!
//! Antes, o bridge da UI convertia a textura em pilha, mutava a pilha numa
//! closure e gravava um snapshot. Agora a operação é um valor tipado e o
//! dispatcher cuida de Undo e rollback: uma operação que não muda nada falha
//! e desfaz também a conversão textura → pilha, sem Undo vazio.

use petunia_core::{Command, CommandContext, CommandError, EditorView};
use petunia_project::ProjectChanges;
use petunia_project::paint_layers::{
    DecalLayer, DecalVariant, LayerKind, PaintEffect, PaintLayer, PaintLayerStack,
};
use uuid::Uuid;

/// Edição do decalque da camada ativa.
#[derive(Clone, Debug)]
pub enum DecalEdit {
    /// Rerasteriza o SVG de origem com o lado maior `size`.
    Rerasterize { size: u32 },
    /// Acrescenta uma variante e passa a mostrá-la.
    AddVariant { variant: Box<DecalVariant> },
    SetVariant { index: usize },
    /// Remove a variante mostrada (a última não sai).
    RemoveShownVariant,
    /// Grava a variante mostrada no quadro `frame` da trilha.
    SetKey { frame: u32 },
    RemoveKey { frame: u32 },
    /// Quadros por segundo da trilha, `1..=60`.
    TrackFps { fps: f32 },
    /// Duração do ciclo, nunca menor que a última chave.
    TrackLength { frames: u32 },
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
    AddLayer { name: String, width: u32, height: u32 },
    AddGroup { name: String },
    AddEffect { name: String, effect: PaintEffect },
    /// Parâmetro `key` do efeito da camada ativa, limitado à faixa válida.
    SetEffectParam { key: String, value: f32 },
    AddDecal { name: String, decal: Box<DecalLayer> },
    SetActive { layer: Uuid },
    ToggleVisible { layer: Uuid },
    ToggleLock { layer: Uuid },
    /// A última camada é a base do raster e não sai.
    Remove { layer: Uuid },
    MergeDown { layer: Uuid },
    Move { layer: Uuid, delta: i32 },
    SetOpacity { layer: Uuid, opacity: f32 },
    SetBlend { layer: Uuid, blend: petunia_project::paint_layers::LayerBlendMode },
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
                    Some(index) if (stack.layers[index].opacity - opacity).abs() >= f32::EPSILON => {
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
            .then(|| state.document.active().map(|asset| asset.mesh.to_owned_value()))
            .flatten();
        let stack = state
            .document
            .active_mut()
            .and_then(|asset| asset.paint_stack.as_mut())
            .ok_or(CommandError::NoActiveAsset)?;
        if !self.op.apply(stack, mesh.as_ref()) {
            // O dispatcher desfaz também a conversão textura → pilha.
            return Err(CommandError::Execution("paint layer: nothing to change".into()));
        }
        crate::PaintModule::composite_active_in(state);
        Ok(())
    }
}
