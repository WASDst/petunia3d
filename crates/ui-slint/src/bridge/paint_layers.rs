//! Pilha de camadas PAINT.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Cria uma camada de efeito com parâmetros padrão para o tipo pedido.
    pub fn add_paint_effect_layer(&mut self, kind: &str) -> bool {
        use petunia_project::paint_layers::{PaintEffect, PaintLayer};
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
        self.mutate_paint_stack("add effect layer", |stack| {
            stack.add_layer(PaintLayer::new_effect(kind.to_string(), effect));
            true
        })
    }

    /// Ajusta um parâmetro do efeito da camada ativa.
    pub fn set_paint_effect_param(&mut self, key: &str, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        self.mutate_paint_stack("effect parameter", |stack| {
            let Some(layer) = stack.active_mut() else {
                return false;
            };
            let petunia_project::paint_layers::LayerKind::Effect(effect) = &mut layer.kind else {
                return false;
            };
            use petunia_project::paint_layers::PaintEffect;
            match (effect, key) {
                (PaintEffect::Pixelate { cell_size }, "cell_size") => {
                    *cell_size = (value.round() as u32).clamp(1, 64);
                }
                (PaintEffect::Posterize { levels }, "levels") => {
                    *levels = (value.round() as u8).clamp(2, 32);
                }
                (PaintEffect::Grain { intensity, .. }, "intensity") => {
                    *intensity = value.clamp(0.0, 1.0);
                }
                (PaintEffect::Grain { seed, .. }, "seed") => {
                    *seed = value.max(0.0).round() as u32;
                }
                (PaintEffect::Levels { in_min, in_max, .. }, "in_min") => {
                    *in_min = value.clamp(0.0, (*in_max - 0.01).max(0.0));
                }
                (PaintEffect::Levels { in_min, in_max, .. }, "in_max") => {
                    *in_max = value.clamp((*in_min + 0.01).min(1.0), 1.0);
                }
                (PaintEffect::Levels { gamma, .. }, "gamma") => {
                    *gamma = value.clamp(0.1, 4.0);
                }
                (PaintEffect::Levels { out_min, .. }, "out_min") => {
                    *out_min = value.clamp(0.0, 1.0);
                }
                (PaintEffect::Levels { out_max, .. }, "out_max") => {
                    *out_max = value.clamp(0.0, 1.0);
                }
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
        })
    }

    /// Mutações do stack de camadas do workspace PAINT.
    ///
    /// Todas passam por `ensure_stack` + checkpoint + recomposição: o raster
    /// canônico é `Asset.paint_stack` e `Asset.texture` é só o cache composto.
    pub(crate) fn mutate_paint_stack(
        &mut self,
        label: &str,
        mutate: impl FnOnce(&mut petunia_project::paint_layers::PaintLayerStack) -> bool,
    ) -> bool {
        if self.state.project.active().is_none_or(|a| a.locked) {
            return false;
        }
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let before = self.state.project.project.clone();
        let active = self.state.project.active;
        let Some(asset) = self.state.project.assets.get_mut(active) else {
            return false;
        };
        let Some(stack) = asset.paint_stack.as_mut() else {
            return false;
        };
        if !mutate(stack) {
            return false;
        }
        self.state.project.checkpoint_snapshot(label, &before);
        petunia_module_paint::PaintModule::composite_active(&mut self.state);
        self.state.mark_dirty();
        true
    }

    pub fn add_paint_layer(&mut self) -> bool {
        let side = petunia_module_paint::DEFAULT_TEXTURE_SIDE;
        let (w, h) = self.paint_canvas_dimensions().unwrap_or((side, side));
        self.mutate_paint_stack("add paint layer", |stack| {
            stack.add_layer(petunia_project::paint_layers::PaintLayer::new(
                format!("Layer {}", stack.layers.len() + 1),
                w,
                h,
                [0, 0, 0, 0],
            ));
            true
        })
    }

    pub fn add_paint_group(&mut self) -> bool {
        self.mutate_paint_stack("add paint group", |stack| {
            stack.add_group(format!("Group {}", stack.layers.len() + 1));
            true
        })
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
        let ok = self.mutate_paint_stack(label, |stack| {
            let mut decal = petunia_project::paint_layers::DecalLayer::new(
                source.image,
                [0.5, 0.5],
                scale_uv,
                0.0,
            );
            decal.source_svg = source.source_svg;
            decal.anchor = anchor;
            stack.add_layer(petunia_project::paint_layers::PaintLayer::new_decal(
                source.name,
                decal,
            ));
            true
        });
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
        self.mutate_paint_stack("add decal layer", |stack| {
            let decal_img = petunia_project::Canvas::new(64, 64, [255, 200, 50, 255]);
            let mut decal = petunia_project::paint_layers::DecalLayer::new(
                decal_img,
                [0.5, 0.5],
                [0.25, 0.25],
                0.0,
            );
            decal.anchor = anchor;
            stack.add_layer(petunia_project::paint_layers::PaintLayer::new_decal(
                format!("Decal {}", stack.layers.len() + 1),
                decal,
            ));
            true
        })
    }

    pub fn set_paint_layer_active(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("activate paint layer", |stack| stack.set_active(id))
    }

    pub fn toggle_paint_layer_visibility(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("toggle paint layer visibility", |stack| {
            let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) else {
                return false;
            };
            layer.visible = !layer.visible;
            true
        })
    }

    pub fn toggle_paint_layer_lock(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("toggle paint layer lock", |stack| {
            let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) else {
                return false;
            };
            layer.locked = !layer.locked;
            true
        })
    }

    pub fn remove_paint_layer(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("remove paint layer", |stack| {
            // A última camada é a base do raster: removê-la deixaria o asset sem
            // superfície de pintura.
            if stack.layers.len() <= 1 {
                return false;
            }
            stack.remove_layer(id)
        })
    }

    pub fn merge_down_paint_layer(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let mesh = self.state.project.active().map(|asset| asset.mesh.clone());
        self.mutate_paint_stack("merge down paint layer", |stack| {
            let Some(pos) = stack.layers.iter().position(|layer| layer.id == id) else {
                return false;
            };
            if pos == 0 {
                return false;
            }
            stack.merge_down_on(pos, mesh.as_deref())
        })
    }

    /// Move a camada em `delta` posições na ordem de composição.
    pub fn move_paint_layer(&mut self, id: &str, delta: i32) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("reorder paint layer", |stack| {
            let Some(from) = stack.layers.iter().position(|layer| layer.id == id) else {
                return false;
            };
            let to = from as i32 + delta;
            if to < 0 || to as usize >= stack.layers.len() {
                return false;
            }
            stack.move_layer(from, to as usize)
        })
    }

    pub fn set_paint_layer_opacity(&mut self, id: &str, opacity: f32) -> bool {
        if !opacity.is_finite() {
            return false;
        }
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let opacity = opacity.clamp(0.0, 1.0);
        self.mutate_paint_stack("paint layer opacity", |stack| {
            let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) else {
                return false;
            };
            if (layer.opacity - opacity).abs() < f32::EPSILON {
                return false;
            }
            layer.opacity = opacity;
            true
        })
    }
}
