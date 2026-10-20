//! Pilha de camadas PAINT.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Cria uma camada de efeito (ajuste) com parâmetros padrão para o tipo.
    pub fn add_paint_effect_layer(&mut self, kind: &str) -> bool {
        let Some(effect) = petunia_module_paint::effect_for_kind(kind) else {
            return false;
        };
        self.dispatch_paint_stack(
            "add effect layer",
            petunia_module_paint::PaintStackOp::AddEffect {
                name: kind.to_string(),
                effect,
            },
        )
    }

    /// Acrescenta um efeito de sub-camada à camada ativa (C1).
    pub fn add_layer_effect(&mut self, kind: &str) -> bool {
        let Some(effect) = petunia_module_paint::effect_for_kind(kind) else {
            return false;
        };
        self.dispatch_paint_stack(
            "add layer effect",
            petunia_module_paint::PaintStackOp::AddLayerEffect { effect },
        )
    }

    pub fn remove_layer_effect(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.dispatch_paint_stack(
            "remove layer effect",
            petunia_module_paint::PaintStackOp::RemoveLayerEffect { effect: id },
        )
    }

    pub fn set_layer_effect_enabled(&mut self, id: &str, enabled: bool) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.dispatch_paint_stack(
            "layer effect enabled",
            petunia_module_paint::PaintStackOp::SetLayerEffectEnabled {
                effect: id,
                enabled,
            },
        )
    }

    pub fn set_layer_effect_opacity(&mut self, id: &str, opacity: f32) -> bool {
        if !opacity.is_finite() {
            return false;
        }
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.dispatch_paint_stack(
            "layer effect opacity",
            petunia_module_paint::PaintStackOp::SetLayerEffectOpacity {
                effect: id,
                opacity,
            },
        )
    }

    pub fn set_layer_effect_param(&mut self, id: &str, key: &str, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.dispatch_paint_stack(
            "layer effect parameter",
            petunia_module_paint::PaintStackOp::SetLayerEffectParam {
                effect: id,
                key: key.to_string(),
                value,
            },
        )
    }

    /// Alvo de máscara: string vazia = máscara da camada; senão id do efeito.
    fn paint_mask_target(effect: &str) -> Option<petunia_module_paint::MaskTarget> {
        if effect.is_empty() {
            return Some(petunia_module_paint::MaskTarget::Layer);
        }
        uuid::Uuid::parse_str(effect)
            .ok()
            .map(petunia_module_paint::MaskTarget::Effect)
    }

    pub fn add_paint_mask(&mut self, effect: &str) -> bool {
        let Some(target) = Self::paint_mask_target(effect) else {
            return false;
        };
        self.dispatch_paint_stack(
            "add paint mask",
            petunia_module_paint::PaintStackOp::AddMask { target },
        )
    }

    pub fn remove_paint_mask(&mut self, effect: &str) -> bool {
        let Some(target) = Self::paint_mask_target(effect) else {
            return false;
        };
        self.dispatch_paint_stack(
            "remove paint mask",
            petunia_module_paint::PaintStackOp::RemoveMask { target },
        )
    }

    pub fn set_paint_mask_enabled(&mut self, effect: &str, enabled: bool) -> bool {
        let Some(target) = Self::paint_mask_target(effect) else {
            return false;
        };
        self.dispatch_paint_stack(
            "paint mask enabled",
            petunia_module_paint::PaintStackOp::SetMaskEnabled { target, enabled },
        )
    }

    pub fn set_paint_mask_inverted(&mut self, effect: &str, inverted: bool) -> bool {
        let Some(target) = Self::paint_mask_target(effect) else {
            return false;
        };
        self.dispatch_paint_stack(
            "paint mask inverted",
            petunia_module_paint::PaintStackOp::SetMaskInverted { target, inverted },
        )
    }

    /// Para onde o pincel pinta: `content`, `layer` ou `effect` (com `id`).
    pub fn set_paint_target(&mut self, target: &str, id: &str) -> bool {
        use petunia_project::paint_layers::PaintTarget;
        let target = match target {
            "content" => PaintTarget::Content,
            "layer" => PaintTarget::LayerMask,
            "effect" => {
                let Ok(id) = uuid::Uuid::parse_str(id) else {
                    return false;
                };
                PaintTarget::EffectMask(id)
            }
            _ => return false,
        };
        self.dispatch_paint_stack(
            "paint target",
            petunia_module_paint::PaintStackOp::SetPaintTarget { target },
        )
    }

    /// Cria uma camada de preenchimento por gradiente (C3).
    pub fn add_gradient_layer(&mut self, kind: &str) -> bool {
        use petunia_project::paint_layers::{GradientFill, GradientKind};
        let Some(kind) = GradientKind::from_id(kind) else {
            return false;
        };
        let color = self.state.session.tools.paint_color;
        let rgb = color.map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8);
        let mut fill = GradientFill::new([rgb[0], rgb[1], rgb[2], 255], [0, 0, 0, 0]);
        fill.kind = kind;
        let name = format!("Gradient {}", self.paint_layer_count() + 1);
        self.dispatch_paint_stack(
            "add gradient layer",
            petunia_module_paint::PaintStackOp::AddGradient { name, fill },
        )
    }

    pub fn set_gradient_kind(&mut self, kind: &str) -> bool {
        let Some(kind) = petunia_project::paint_layers::GradientKind::from_id(kind) else {
            return false;
        };
        self.dispatch_paint_stack(
            "gradient kind",
            petunia_module_paint::PaintStackOp::SetGradientKind { kind },
        )
    }

    pub fn set_gradient_repeat(&mut self, repeat: bool) -> bool {
        self.dispatch_paint_stack(
            "gradient repeat",
            petunia_module_paint::PaintStackOp::SetGradientRepeat { repeat },
        )
    }

    pub fn set_gradient_geometry(&mut self, start: [f32; 2], end: [f32; 2]) -> bool {
        if !start.iter().all(|value| value.is_finite())
            || !end.iter().all(|value| value.is_finite())
        {
            return false;
        }
        self.dispatch_paint_stack(
            "gradient geometry",
            petunia_module_paint::PaintStackOp::SetGradientGeometry { start, end },
        )
    }

    /// Ajusta um eixo do início/fim do gradiente ativo (as duas coordenadas
    /// vivem em comandos separados para a UI editar X e Y em campos próprios).
    fn set_gradient_axis(&mut self, start: bool, axis: usize, value: f32) -> bool {
        use petunia_project::paint_layers::LayerKind;
        if !value.is_finite() || axis > 1 {
            return false;
        }
        let fill = self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .and_then(|layer| match &layer.kind {
                LayerKind::Gradient(fill) => Some(fill.clone()),
                _ => None,
            });
        let Some(fill) = fill else {
            return false;
        };
        let mut start_point = fill.start;
        let mut end_point = fill.end;
        if start {
            start_point[axis] = value;
        } else {
            end_point[axis] = value;
        }
        self.dispatch_paint_stack(
            "gradient geometry",
            petunia_module_paint::PaintStackOp::SetGradientGeometry {
                start: start_point,
                end: end_point,
            },
        )
    }

    pub fn set_gradient_start_axis(&mut self, axis: usize, value: f32) -> bool {
        self.set_gradient_axis(true, axis, value)
    }

    pub fn set_gradient_end_axis(&mut self, axis: usize, value: f32) -> bool {
        self.set_gradient_axis(false, axis, value)
    }

    pub fn set_gradient_stop_position(&mut self, index: usize, position: f32) -> bool {
        if !position.is_finite() {
            return false;
        }
        self.dispatch_paint_stack(
            "gradient stop position",
            petunia_module_paint::PaintStackOp::SetGradientStopPosition { index, position },
        )
    }

    pub fn set_gradient_stop_color(&mut self, index: usize, color: &str) -> bool {
        let Some(color) = parse_hex_color(color) else {
            return false;
        };
        self.dispatch_paint_stack(
            "gradient stop color",
            petunia_module_paint::PaintStackOp::SetGradientStopColor { index, color },
        )
    }

    pub fn add_gradient_stop(&mut self, position: f32) -> bool {
        if !position.is_finite() {
            return false;
        }
        self.dispatch_paint_stack(
            "add gradient stop",
            petunia_module_paint::PaintStackOp::AddGradientStop {
                position: position.clamp(0.0, 1.0),
                color: [255, 255, 255, 255],
            },
        )
    }

    pub fn remove_gradient_stop(&mut self, index: usize) -> bool {
        self.dispatch_paint_stack(
            "remove gradient stop",
            petunia_module_paint::PaintStackOp::RemoveGradientStop { index },
        )
    }

    /// Ajusta um parâmetro do efeito da camada ativa.
    pub fn set_paint_effect_param(&mut self, key: &str, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        self.dispatch_paint_stack(
            "effect parameter",
            petunia_module_paint::PaintStackOp::SetEffectParam {
                key: key.to_string(),
                value,
            },
        )
    }

    /// Operações na pilha de camadas do workspace PAINT, por `PaintStackCmd`
    /// (uma entrada de Undo, recomposição da textura). O raster canônico é
    /// `Asset.paint_stack`; `Asset.texture` é só o cache composto. Recusa
    /// ("nada mudaria", objeto travado) devolve `false` sem Undo nem status.
    pub(crate) fn dispatch_paint_stack(
        &mut self,
        label: &'static str,
        op: petunia_module_paint::PaintStackOp,
    ) -> bool {
        self.state
            .dispatch(&petunia_module_paint::PaintStackCmd { label, op })
            .is_ok()
    }

    pub fn add_paint_layer(&mut self) -> bool {
        let side = petunia_module_paint::DEFAULT_TEXTURE_SIDE;
        let (w, h) = self.paint_canvas_dimensions().unwrap_or((side, side));
        let name = format!("Layer {}", self.paint_layer_count() + 1);
        self.dispatch_paint_stack(
            "add paint layer",
            petunia_module_paint::PaintStackOp::AddLayer {
                name,
                width: w,
                height: h,
            },
        )
    }

    pub fn add_paint_group(&mut self) -> bool {
        let name = format!("Group {}", self.paint_layer_count() + 1);
        self.dispatch_paint_stack(
            "add paint group",
            petunia_module_paint::PaintStackOp::AddGroup { name },
        )
    }

    /// Importa uma imagem do disco como camada de decalque projetada (P3D-133).
    ///
    /// Valida a decodificação (erro vira status, o documento não muda), limita o
    /// lado maior a 512 texels (a imagem é persistida no projeto) e preserva a
    /// proporção na escala UV.
    pub fn import_decal_image(&mut self, path: &std::path::Path) -> bool {
        let Some(source) = self.read_decal_source(path) else {
            return false;
        };
        let (w, h) = (source.image.w, source.image.h);
        let aspect = h as f32 / w as f32;
        let scale_uv = if aspect <= 1.0 {
            [0.3, (0.3 * aspect).max(0.01)]
        } else {
            [(0.3 / aspect).max(0.01), 0.3]
        };
        let anchor = self.new_decal_anchor();
        let name = source.name.clone();
        let label = if source.source_svg.is_some() {
            "import SVG decal"
        } else {
            "import decal"
        };
        let mut decal =
            petunia_project::paint_layers::DecalLayer::new(source.image, [0.5, 0.5], scale_uv, 0.0);
        decal.source_svg = source.source_svg;
        decal.anchor = anchor;
        let ok = self.dispatch_paint_stack(
            label,
            petunia_module_paint::PaintStackOp::AddDecal {
                name: source.name,
                decal: Box::new(decal),
            },
        );
        if ok {
            self.state.set_status(crate::tr::fill(
                &self
                    .state
                    .t_id(petunia_config::text_id::STATUS_DECAL_IMPORTED_X),
                &[("name", name), ("w", format!("{w}")), ("h", format!("{h}"))],
            ));
        }
        ok
    }

    pub fn add_decal_layer(&mut self) -> bool {
        let anchor = self.new_decal_anchor();
        let decal_img = petunia_project::Canvas::new(64, 64, [255, 200, 50, 255]);
        let mut decal = petunia_project::paint_layers::DecalLayer::new(
            decal_img,
            [0.5, 0.5],
            [0.25, 0.25],
            0.0,
        );
        decal.anchor = anchor;
        let name = format!("Decal {}", self.paint_layer_count() + 1);
        self.dispatch_paint_stack(
            "add decal layer",
            petunia_module_paint::PaintStackOp::AddDecal {
                name,
                decal: Box::new(decal),
            },
        )
    }

    pub fn set_paint_layer_active(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.dispatch_paint_stack(
            "activate paint layer",
            petunia_module_paint::PaintStackOp::SetActive { layer: id },
        )
    }

    pub fn toggle_paint_layer_visibility(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.dispatch_paint_stack(
            "toggle paint layer visibility",
            petunia_module_paint::PaintStackOp::ToggleVisible { layer: id },
        )
    }

    pub fn toggle_paint_layer_lock(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.dispatch_paint_stack(
            "toggle paint layer lock",
            petunia_module_paint::PaintStackOp::ToggleLock { layer: id },
        )
    }

    pub fn remove_paint_layer(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        // A última camada é a base do raster: o comando recusa removê-la.
        self.dispatch_paint_stack(
            "remove paint layer",
            petunia_module_paint::PaintStackOp::Remove { layer: id },
        )
    }

    pub fn merge_down_paint_layer(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.dispatch_paint_stack(
            "merge down paint layer",
            petunia_module_paint::PaintStackOp::MergeDown { layer: id },
        )
    }

    /// Move a camada em `delta` posições na ordem de composição.
    pub fn move_paint_layer(&mut self, id: &str, delta: i32) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.dispatch_paint_stack(
            "reorder paint layer",
            petunia_module_paint::PaintStackOp::Move { layer: id, delta },
        )
    }

    pub fn set_paint_layer_opacity(&mut self, id: &str, opacity: f32) -> bool {
        if !opacity.is_finite() {
            return false;
        }
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.dispatch_paint_stack(
            "paint layer opacity",
            petunia_module_paint::PaintStackOp::SetOpacity { layer: id, opacity },
        )
    }

    /// Modo de mesclagem da camada (índice de `LayerBlendMode`).
    pub fn set_paint_layer_blend(&mut self, id: &str, blend: i32) -> bool {
        use petunia_project::paint_layers::LayerBlendMode as Blend;
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let blend = match blend {
            0 => Blend::Normal,
            1 => Blend::Multiply,
            2 => Blend::Add,
            3 => Blend::Screen,
            _ => return false,
        };
        self.dispatch_paint_stack(
            "paint layer blend",
            petunia_module_paint::PaintStackOp::SetBlend { layer: id, blend },
        )
    }

    /// Camadas que a pilha do objeto ativo terá ao executar o comando (nomes
    /// padrão "Layer N"): sem pilha, a conversão cria a camada "Base".
    fn paint_layer_count(&self) -> usize {
        self.state
            .project
            .active()
            .and_then(|asset| asset.paint_stack.as_ref())
            .map_or(1, |stack| stack.layers.len())
    }
}

/// `#RRGGBB` (ou `RRGGBB`) → RGBA opaco. Entrada da UI é hostil.
fn parse_hex_color(text: &str) -> Option<[u8; 4]> {
    let text = text.trim().trim_start_matches('#');
    if text.len() != 6 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |range: std::ops::Range<usize>| u8::from_str_radix(&text[range], 16).ok();
    Some([channel(0..2)?, channel(2..4)?, channel(4..6)?, 255])
}
