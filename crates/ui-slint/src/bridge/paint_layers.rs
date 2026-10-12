//! Pilha de camadas PAINT.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Cria uma camada de efeito com parâmetros padrão para o tipo pedido.
    pub fn add_paint_effect_layer(&mut self, kind: &str) -> bool {
        use petunia_project::paint_layers::PaintEffect;
        let effect = match kind {
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
            _ => return false,
        };
        self.dispatch_paint_stack(
            "add effect layer",
            petunia_module_paint::PaintStackOp::AddEffect {
                name: kind.to_string(),
                effect,
            },
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
