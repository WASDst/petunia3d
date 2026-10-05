//! Ferramenta de decalque no bridge (cap. 39, P3D-133/156, constituição 11).
//!
//! Com uma camada de decalque ativa no PAINT, a viewport segue a gramática
//! única: o [`petunia_core::ToolSession`] decide clique × arrasto; clicar
//! posiciona o decalque na superfície; arrastar em qualquer lugar o move;
//! as alças dos cantos escalam e a alça de cima gira. Durante o gesto, o
//! valor digitado (largura em m ou rotação em graus, `Tab` troca) vence o
//! ponteiro; `Esc` restaura; cada gesto é uma entrada de Undo, ajustável no
//! card "Última operação" sem aumentar o histórico.
//!
//! Mover o decalque recompõe só os tiles que ele cobria ou cobre (D5); ao
//! confirmar, variantes vindas de SVG são re-rasterizadas na resolução que a
//! área ocupada pede (D6). O Decal Set (variantes + trilha em degraus, D4)
//! também vive aqui.

use glam::Vec3;
use petunia_core::{PressTarget, Workspace};
use petunia_project::Canvas;
use petunia_project::paint_layers::{DecalLayer, DecalVariant, LayerKind};

use crate::projection::{pick_face_hit, project_world_point};
use crate::{PetuniaViewport, SlintUiBridge, ToolGesture};

/// Alça do centro (mover).
pub(crate) const DECAL_HANDLE_MOVE: u8 = 20;
/// Alças dos cantos (escalar a partir do centro).
pub(crate) const DECAL_HANDLE_SCALE: u8 = 21;
/// Alça acima da borda de cima (girar em torno do centro).
pub(crate) const DECAL_HANDLE_ROTATE: u8 = 22;
/// Raio de acerto das alças: alvo de 24 px (WCAG 2.2, 2.5.8).
pub(crate) const DECAL_HANDLE_RADIUS_PX: f32 = 12.0;
/// Distância (px) da alça de girar até a borda de cima.
const DECAL_ROTATE_OFFSET_PX: f32 = 28.0;
/// Abaixo desta distância (px) do centro, o arrasto usa o deslocamento em
/// vez da geometria radial (evita divisão por quase zero).
const RADIAL_MIN_PX: f32 = 4.0;
/// Maior largura aceita para um decalque de superfície (m).
const MAX_SURFACE_WIDTH: f32 = 10_000.0;
/// Maior lado (px) de uma imagem importada como decalque ou variante.
pub(crate) const MAX_DECAL_IMAGE_PX: u32 = 1024;

/// O que o gesto faz com o decalque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecalMode {
    Move,
    Scale,
    Rotate,
}

/// Parâmetro que recebe o valor digitado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecalParam {
    /// Largura em mundo (m) do decalque de superfície; escala U no decalque UV.
    Width,
    /// Rotação em graus.
    Rotation,
}

/// Gesto em andamento.
#[derive(Debug, Clone)]
pub(crate) struct DecalGesture {
    pub mode: DecalMode,
    pub param: DecalParam,
    pub anchor_px: [f32; 2],
    /// Centro do decalque na tela quando o gesto começou.
    pub center_px: Option<[f32; 2]>,
    /// Decalque antes do gesto (base para escalar, girar e digitar).
    pub start: DecalLayer,
}

/// Gesto confirmado, ajustável enquanto for o topo do histórico.
#[derive(Debug, Clone, PartialEq)]
pub struct DecalLastOperation {
    pub layer_id: uuid::Uuid,
    pub param: DecalParam,
    pub value: f32,
    history_depth: (usize, usize),
    revision_clock: [u64; 11],
}

/// Alças do decalque ativo em px da viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecalHandles {
    pub center: [f32; 2],
    /// Superior-esquerdo, superior-direito, inferior-direito, inferior-esquerdo.
    pub corners: [[f32; 2]; 4],
    /// Meio da borda de cima.
    pub top: [f32; 2],
    pub rotate: [f32; 2],
}

/// Imagem lida do disco para um decalque ou variante.
#[derive(Debug, Clone)]
pub(crate) struct DecalSource {
    pub name: String,
    pub image: Canvas,
    pub source_svg: Option<String>,
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Valor do parâmetro no decalque.
pub fn decal_value(decal: &DecalLayer, param: DecalParam) -> f32 {
    match param {
        DecalParam::Width => decal
            .anchor
            .map_or(decal.scale_uv[0], |anchor| anchor.width),
        DecalParam::Rotation => decal.rotation_rad.to_degrees(),
    }
}

/// `start` com o parâmetro em `value`; `None` se o valor é inválido (não
/// altera a prévia nem entra no histórico, constituição 11).
pub fn decal_with_value(start: &DecalLayer, param: DecalParam, value: f32) -> Option<DecalLayer> {
    if !value.is_finite() {
        return None;
    }
    let mut next = start.clone();
    match param {
        DecalParam::Width => {
            if value <= 0.0 {
                return None;
            }
            if let Some(anchor) = next.anchor.as_mut() {
                let width = value.min(MAX_SURFACE_WIDTH);
                anchor.width = width;
                anchor.depth = width * 0.5;
            } else {
                let ratio = start.scale_uv[1] / start.scale_uv[0].max(1.0e-5);
                let u = value.clamp(0.01, 5.0);
                next.scale_uv = [u, (u * ratio).clamp(0.01, 5.0)];
            }
        }
        DecalParam::Rotation => next.rotation_rad = value.to_radians(),
    }
    Some(next)
}

/// Ângulo (graus) em `(-180, 180]`.
fn wrap_degrees(degrees: f32) -> f32 {
    let wrapped = (degrees + 180.0).rem_euclid(360.0) - 180.0;
    if wrapped == -180.0 { 180.0 } else { wrapped }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Ponto do mundo para as coordenadas locais `(lx, ly)` do decalque
    /// (`-0.5..0.5`, `ly` negativo = em cima). Decalque UV: pelo mapa UV.
    pub(crate) fn decal_local_to_world(
        &self,
        decal: &DecalLayer,
        lx: f32,
        ly: f32,
    ) -> Option<Vec3> {
        if let Some(anchor) = decal.anchor {
            let (right, up, _) = anchor.frame(decal.rotation_rad);
            let height = decal.surface_height(anchor.width);
            return Some(
                Vec3::from(anchor.point) + right * (lx * anchor.width) - up * (ly * height),
            );
        }
        let (sin, cos) = decal.rotation_rad.sin_cos();
        let du = lx * decal.scale_uv[0] * cos - ly * decal.scale_uv[1] * sin;
        let dv = lx * decal.scale_uv[0] * sin + ly * decal.scale_uv[1] * cos;
        let uv = [
            (decal.center_uv[0] + du).clamp(0.0, 1.0),
            (decal.center_uv[1] + dv).clamp(0.0, 1.0),
        ];
        self.state
            .project
            .active_mesh()?
            .uv_to_world(uv)
            .map(|(p, _)| p)
    }

    fn project_px(&self, point: Vec3) -> Option<[f32; 2]> {
        let viewport = self.viewport_size;
        if viewport[0] <= 1.0 || viewport[1] <= 1.0 {
            return None;
        }
        project_world_point(&self.state.session.camera, viewport, point)
    }

    /// A ferramenta de decalque está em uso: PAINT, camada de decalque ativa
    /// e uma ferramenta que não seja Select.
    pub fn decal_tool_active(&self) -> bool {
        self.state.workspace == Workspace::Paint
            && self.state.session.tools.active_tool != "select"
            && self.active_layer_is_decal()
    }

    /// Alças do decalque ativo na tela.
    pub fn decal_handles(&self) -> Option<DecalHandles> {
        if self.state.workspace != Workspace::Paint {
            return None;
        }
        let decal = self.active_decal()?;
        let screen = |lx: f32, ly: f32| {
            self.decal_local_to_world(&decal, lx, ly)
                .and_then(|p| self.project_px(p))
        };
        let center = screen(0.0, 0.0)?;
        let corners = [
            screen(-0.5, -0.5)?,
            screen(0.5, -0.5)?,
            screen(0.5, 0.5)?,
            screen(-0.5, 0.5)?,
        ];
        let top = screen(0.0, -0.5)?;
        let outward = glam::Vec2::from(top) - glam::Vec2::from(center);
        let direction = if outward.length() > 1.0 {
            outward.normalize()
        } else {
            glam::Vec2::NEG_Y
        };
        let rotate = (glam::Vec2::from(top) + direction * DECAL_ROTATE_OFFSET_PX).to_array();
        Some(DecalHandles {
            center,
            corners,
            top,
            rotate,
        })
    }

    /// Alça sob o ponteiro (girar > cantos > centro).
    pub(crate) fn decal_handle_at(&self, x: f32, y: f32) -> Option<u8> {
        let handles = self.decal_handles()?;
        let near = |point: [f32; 2]| distance(point, [x, y]) <= DECAL_HANDLE_RADIUS_PX;
        if near(handles.rotate) {
            Some(DECAL_HANDLE_ROTATE)
        } else if handles.corners.iter().any(|corner| near(*corner)) {
            Some(DECAL_HANDLE_SCALE)
        } else if near(handles.center) {
            Some(DECAL_HANDLE_MOVE)
        } else {
            None
        }
    }

    /// Alvo do botão principal para a gramática única.
    pub(crate) fn decal_press_target(&self, x: f32, y: f32) -> PressTarget {
        self.decal_handle_at(x, y)
            .map_or(PressTarget::Surface, PressTarget::Handle)
    }

    /// Grava `next` no decalque ativo e recompõe só a região afetada (D5).
    pub(crate) fn apply_decal_state(&mut self, next: DecalLayer) -> bool {
        let before = petunia_module_paint::PaintModule::active_decal_bounds(&self.state);
        let active = self.state.project.active;
        let Some(layer) = self
            .state
            .project
            .assets
            .get_mut(active)
            .and_then(|asset| asset.paint_stack.as_mut())
            .and_then(|stack| stack.active_mut())
        else {
            return false;
        };
        let LayerKind::Decal(decal) = &mut layer.kind else {
            return false;
        };
        if *decal == next {
            return false;
        }
        *decal = next;
        petunia_module_paint::PaintModule::composite_decal_region(&mut self.state, before);
        self.state.mark_dirty();
        true
    }

    /// Abre um gesto de decalque (transação de pintura: Esc restaura).
    pub(crate) fn begin_decal_gesture(&mut self, anchor_px: [f32; 2], mode: DecalMode) -> bool {
        let Some(start) = self.active_decal() else {
            return false;
        };
        let locked = self.state.project.active().is_none_or(|asset| asset.locked)
            || self
                .state
                .project
                .active()
                .and_then(|asset| asset.paint_stack.as_ref())
                .and_then(|stack| stack.active())
                .is_none_or(|layer| layer.locked);
        if locked {
            return false;
        }
        self.state.begin_paint_stroke();
        if self.state.session.tools.paint_stroke.is_none() {
            return false;
        }
        self.tool_session.clear_numeric();
        let center_px = self.decal_handles().map(|handles| handles.center);
        self.decal_gesture = Some(crate::decal_tool::DecalGesture {
            mode,
            param: if mode == DecalMode::Rotate {
                DecalParam::Rotation
            } else {
                DecalParam::Width
            },
            anchor_px,
            center_px,
            start,
        });
        self.tool_gesture = Some(ToolGesture::Decal);
        self.state.mark_dirty();
        true
    }

    /// Novo ponto do ponteiro no gesto. `fine` (Shift) reduz a taxa; `snap`
    /// (Ctrl) encaixa a largura em 5 cm (0,01 no UV) e a rotação em 15°.
    pub(crate) fn update_decal_gesture(
        &mut self,
        current: [f32; 2],
        fine: bool,
        snap: bool,
    ) -> bool {
        let Some(gesture) = self.decal_gesture.clone() else {
            return false;
        };
        if !self.tool_session.numeric_text().is_empty() {
            // O valor digitado vence o ponteiro (constituição 11).
            return false;
        }
        let start = &gesture.start;
        let radial = gesture
            .center_px
            .filter(|center| distance(gesture.anchor_px, *center) >= RADIAL_MIN_PX);
        let next = match gesture.mode {
            DecalMode::Move => {
                if let Some(anchor) = start.anchor {
                    match self.decal_anchor_at(current[0], current[1], anchor.width) {
                        Some(next_anchor) => {
                            let mut next = start.clone();
                            next.anchor = Some(next_anchor);
                            Some(next)
                        }
                        // Fora do objeto o decalque fica onde estava.
                        None => return true,
                    }
                } else {
                    let uv = self.uv_under_pointer(current);
                    match uv {
                        Some(uv) => {
                            let mut next = start.clone();
                            next.center_uv = uv;
                            Some(next)
                        }
                        None => return true,
                    }
                }
            }
            DecalMode::Scale => {
                let mut factor = match radial {
                    Some(center) => distance(current, center) / distance(gesture.anchor_px, center),
                    None => 1.0 + (gesture.anchor_px[1] - current[1]) * 0.01,
                };
                if fine {
                    factor = 1.0 + (factor - 1.0) * 0.1;
                }
                let mut value = decal_value(start, DecalParam::Width) * factor.max(0.02);
                if snap {
                    let step = if start.anchor.is_some() { 0.05 } else { 0.01 };
                    value = ((value / step).round() * step).max(step);
                }
                decal_with_value(start, DecalParam::Width, value)
            }
            DecalMode::Rotate => {
                let mut delta = match radial {
                    Some(center) => {
                        let angle = |p: [f32; 2]| (p[1] - center[1]).atan2(p[0] - center[0]);
                        // Tela com y para baixo: girar no sentido anti-horário
                        // visível diminui o ângulo de tela.
                        -wrap_degrees((angle(current) - angle(gesture.anchor_px)).to_degrees())
                    }
                    None => (current[0] - gesture.anchor_px[0]) * 0.5,
                };
                if fine {
                    delta *= 0.1;
                }
                let mut value = decal_value(start, DecalParam::Rotation) + delta;
                if snap {
                    value = (value / 15.0).round() * 15.0;
                }
                decal_with_value(start, DecalParam::Rotation, value)
            }
        };
        match next {
            Some(next) => {
                self.apply_decal_state(next);
                true
            }
            None => false,
        }
    }

    fn uv_under_pointer(&self, at: [f32; 2]) -> Option<[f32; 2]> {
        let [width, height] = self.viewport_size;
        if width <= 1.0
            || height <= 1.0
            || !(0.0..width).contains(&at[0])
            || !(0.0..height).contains(&at[1])
        {
            return None;
        }
        let (origin, direction) = self
            .state
            .session
            .camera
            .ray(at[0] / width * 2.0 - 1.0, 1.0 - at[1] / height * 2.0);
        let (face, hit) = pick_face_hit(&self.state, origin, direction)?;
        petunia_module_paint::PaintModule::face_hit_uv(&self.state, face, hit, false)
    }

    /// Aplica o texto digitado ao parâmetro do gesto. Buffer vazio devolve o
    /// controle ao ponteiro.
    pub(crate) fn decal_typed_input(&mut self) -> bool {
        let Some(gesture) = self.decal_gesture.clone() else {
            return false;
        };
        let text = self.tool_session.numeric_text().to_owned();
        if text.is_empty() {
            let pointer = self.pointer_position;
            return self.update_decal_gesture(pointer, false, false);
        }
        let base = decal_value(&gesture.start, gesture.param);
        let Ok(value) = crate::numeric::parse_numeric_with_base(&text, base) else {
            return false;
        };
        match decal_with_value(&gesture.start, gesture.param, value) {
            Some(next) => {
                // Mover + valor digitado: a posição já arrastada é mantida.
                let mut next = next;
                if gesture.mode == DecalMode::Move
                    && let Some(current) = self.active_decal()
                {
                    next.anchor = match (next.anchor, current.anchor) {
                        (Some(typed), Some(moved)) => {
                            Some(petunia_project::paint_layers::DecalAnchor {
                                width: typed.width,
                                depth: typed.depth,
                                ..moved
                            })
                        }
                        (typed, _) => typed,
                    };
                    if next.anchor.is_none() {
                        next.center_uv = current.center_uv;
                    }
                }
                self.apply_decal_state(next);
                true
            }
            None => false,
        }
    }

    /// `Tab` no gesto: troca o parâmetro que recebe o texto.
    pub(crate) fn decal_toggle_param(&mut self) -> bool {
        // O valor já digitado fica: o outro parâmetro parte do estado atual.
        let current = self.active_decal();
        let Some(gesture) = self.decal_gesture.as_mut() else {
            return false;
        };
        if let Some(current) = current {
            gesture.start = current;
        }
        gesture.param = match gesture.param {
            DecalParam::Width => DecalParam::Rotation,
            DecalParam::Rotation => DecalParam::Width,
        };
        self.tool_session.clear_numeric();
        self.state.mark_dirty();
        true
    }

    /// Confirma o gesto: uma entrada de Undo, com o SVG já na resolução nova.
    pub(crate) fn commit_decal_gesture(&mut self) -> bool {
        let Some(gesture) = self.decal_gesture.take() else {
            return false;
        };
        if matches!(self.tool_gesture, Some(ToolGesture::Decal)) {
            self.tool_gesture = None;
        }
        self.tool_session.clear_numeric();
        let layer_id = self.fit_active_decal_svg();
        let depth_before = self.state.project.undo.depth();
        self.state.finish_paint_stroke(false);
        let committed = self.state.project.undo.depth() != depth_before;
        self.decal_last_operation = match (committed, layer_id, self.active_decal()) {
            (true, Some(layer_id), Some(decal)) => Some(DecalLastOperation {
                layer_id,
                param: gesture.param,
                value: decal_value(&decal, gesture.param),
                history_depth: self.state.project.undo.depth(),
                revision_clock: self.state.project.project.revision_clock(),
            }),
            _ => None,
        };
        if committed {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_DECAL_TRANSFORM_COMMITTED),
            );
        }
        self.state.mark_dirty();
        true
    }

    /// Cancela o gesto: o documento volta exatamente ao estado anterior.
    pub(crate) fn cancel_decal_gesture(&mut self) -> bool {
        if self.decal_gesture.take().is_none() {
            return false;
        }
        if matches!(self.tool_gesture, Some(ToolGesture::Decal)) {
            self.tool_gesture = None;
        }
        self.tool_session.clear_numeric();
        self.state.finish_paint_stroke(true);
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_DECAL_TRANSFORM_CANCELLED),
        );
        self.state.mark_dirty();
        true
    }

    /// Clique sem arrasto: posiciona o decalque no ponto clicado (um gesto).
    pub(crate) fn decal_click(&mut self, at: [f32; 2]) -> bool {
        if !self.begin_decal_gesture(at, DecalMode::Move) {
            return false;
        }
        self.update_decal_gesture(at, false, false);
        self.commit_decal_gesture()
    }

    /// Re-rasteriza as variantes SVG do decalque ativo na resolução que a
    /// área ocupada pede (D6). Devolve o id da camada ativa.
    fn fit_active_decal_svg(&mut self) -> Option<uuid::Uuid> {
        let asset = self.state.project.active_mut()?;
        let (w, h) = asset.texture.as_ref().map(|c| (c.w, c.h))?;
        let mesh = asset.mesh.clone();
        let stack = asset.paint_stack.as_mut()?;
        let id = stack.active()?.id;
        if matches!(stack.fit_decal_svg(id, Some(&mesh), w, h), Ok(true)) {
            petunia_module_paint::PaintModule::composite_active(&mut self.state);
        }
        Some(id)
    }

    /// A "Última operação" do decalque ainda pode ser ajustada.
    pub fn decal_last_operation_adjustable(&self) -> bool {
        self.decal_gesture.is_none()
            && self.decal_last_operation.as_ref().is_some_and(|last| {
                self.state.project.undo.depth() == last.history_depth
                    && self.state.project.project.revision_clock() == last.revision_clock
            })
    }

    /// Reaplica o último gesto de decalque com `value`, na mesma entrada de
    /// Undo (a profundidade do histórico não muda).
    pub fn adjust_decal_last_operation(&mut self, value: f32) -> bool {
        if !self.decal_last_operation_adjustable() {
            self.decal_last_operation = None;
            return false;
        }
        let Some(last) = self.decal_last_operation.clone() else {
            return false;
        };
        let Some(current) = self.active_decal() else {
            return false;
        };
        let Some(next) = decal_with_value(&current, last.param, value) else {
            return false;
        };
        if !self.state.undo() {
            return false;
        }
        self.state.begin_paint_stroke();
        let applied = self.set_layer_decal(last.layer_id, next);
        let _ = self.fit_active_decal_svg();
        self.state.finish_paint_stroke(!applied);
        if !applied {
            self.state.redo();
            return false;
        }
        self.decal_last_operation = Some(DecalLastOperation {
            value,
            history_depth: self.state.project.undo.depth(),
            revision_clock: self.state.project.project.revision_clock(),
            ..last
        });
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::TOOL_GRAMMAR_ADJUSTED),
        );
        self.state.mark_dirty();
        true
    }

    fn set_layer_decal(&mut self, layer_id: uuid::Uuid, next: DecalLayer) -> bool {
        let active = self.state.project.active;
        let Some(layer) = self
            .state
            .project
            .assets
            .get_mut(active)
            .and_then(|asset| asset.paint_stack.as_mut())
            .and_then(|stack| stack.layers.iter_mut().find(|layer| layer.id == layer_id))
        else {
            return false;
        };
        let LayerKind::Decal(decal) = &mut layer.kind else {
            return false;
        };
        *decal = next;
        petunia_module_paint::PaintModule::composite_active(&mut self.state);
        true
    }

    // ---------------------------------------------------------------
    // Decal Set: variantes e trilha em degraus (D4)
    // ---------------------------------------------------------------

    /// Lê uma imagem (PNG/JPEG/… ou SVG) para decalque ou variante. Erros
    /// viram status e o documento não muda.
    pub(crate) fn read_decal_source(&mut self, path: &std::path::Path) -> Option<DecalSource> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Decal")
            .to_owned();
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
        {
            let svg = match crate::files::load_svg(path) {
                Ok(svg) => svg,
                Err(error) => {
                    self.state.set_status(error);
                    return None;
                }
            };
            return match petunia_project::svg::rasterize_svg(&svg, MAX_DECAL_IMAGE_PX) {
                Ok(image) => Some(DecalSource {
                    name,
                    image,
                    source_svg: Some(svg),
                }),
                Err(error) => {
                    self.state.set_status(error.to_string());
                    None
                }
            };
        }
        let invalid = |bridge: &mut Self| {
            bridge.state.set_status(
                bridge
                    .state
                    .t_id(petunia_config::text_id::STATUS_DECAL_INVALID_IMAGE_DATA),
            );
            None
        };
        let (w, h, rgba) = match crate::files::load_image_rgba(path) {
            Ok(decoded) => decoded,
            Err(error) => {
                self.state.set_status(crate::tr::fill(
                    &self
                        .state
                        .t_id(petunia_config::text_id::STATUS_DECAL_COULD_NOT_READ_THE_IMAGE),
                    &[("error", error.to_string())],
                ));
                return None;
            }
        };
        let (w, h, rgba) = if w.max(h) > MAX_DECAL_IMAGE_PX {
            let scale = MAX_DECAL_IMAGE_PX as f32 / w.max(h) as f32;
            let (nw, nh) = (
                ((w as f32 * scale).round() as u32).max(1),
                ((h as f32 * scale).round() as u32).max(1),
            );
            match image::RgbaImage::from_raw(w, h, rgba) {
                Some(image) => (
                    nw,
                    nh,
                    image::imageops::resize(&image, nw, nh, image::imageops::FilterType::Lanczos3)
                        .into_raw(),
                ),
                None => return invalid(self),
            }
        } else {
            (w, h, rgba)
        };
        if w == 0 || h == 0 || rgba.len() != (w * h * 4) as usize {
            return invalid(self);
        }
        Some(DecalSource {
            name,
            image: Canvas {
                w,
                h,
                pixels: rgba.into(),
            },
            source_svg: None,
        })
    }

    /// Muta o decalque ativo numa transação da pilha (uma entrada de Undo).
    fn mutate_active_decal(
        &mut self,
        label: &str,
        mutate: impl FnOnce(&mut DecalLayer, &str) -> bool,
    ) -> bool {
        self.mutate_paint_stack(label, |stack| {
            let Some(layer) = stack.active_mut() else {
                return false;
            };
            let name = layer.name.clone();
            match &mut layer.kind {
                LayerKind::Decal(decal) => mutate(decal, &name),
                _ => false,
            }
        })
    }

    /// Acrescenta a imagem em `path` como variante do decalque ativo e passa
    /// a mostrá-la.
    pub fn add_decal_variant_from_path(&mut self, path: &std::path::Path) -> bool {
        if !self.active_layer_is_decal() {
            return false;
        }
        let Some(source) = self.read_decal_source(path) else {
            return false;
        };
        let name = source.name.clone();
        let added = self.mutate_active_decal("add decal variant", |decal, base| {
            let index = decal.add_variant(
                base,
                DecalVariant {
                    name: source.name,
                    image: source.image,
                    source_svg: source.source_svg,
                },
            );
            decal.set_variant(index)
        });
        if added {
            self.state.session.decal_time = None;
            self.state.set_status(crate::tr::fill(
                &self
                    .state
                    .t_id(petunia_config::text_id::STATUS_DECAL_VARIANT_ADDED),
                &[("name", name)],
            ));
        }
        added
    }

    /// Mostra a variante `index` do decalque ativo (estado parado).
    pub fn set_decal_variant(&mut self, index: usize) -> bool {
        if self
            .active_decal()
            .is_none_or(|decal| decal.variant_index == index || index >= decal.variant_count())
        {
            return false;
        }
        self.decal_playing = false;
        self.state.session.decal_time = None;
        self.mutate_active_decal("set decal variant", |decal, _| decal.set_variant(index))
    }

    /// Remove a variante mostrada (a última não sai).
    pub fn remove_decal_variant(&mut self) -> bool {
        self.mutate_active_decal("remove decal variant", |decal, _| {
            decal.remove_variant(decal.variant_index)
        })
    }

    /// Grava a variante mostrada no quadro atual da trilha.
    pub fn set_decal_key(&mut self) -> bool {
        let frame = self.decal_frame;
        self.mutate_active_decal("set decal key", |decal, _| {
            let variant = decal.variant_index as u32;
            let track = decal.track.get_or_insert_with(Default::default);
            let before = track.clone();
            track.set_key(frame, variant);
            *track != before
        })
    }

    /// Apaga a chave do quadro atual.
    pub fn remove_decal_key(&mut self) -> bool {
        let frame = self.decal_frame;
        self.mutate_active_decal("remove decal key", |decal, _| {
            decal
                .track
                .as_mut()
                .is_some_and(|track| track.remove_key(frame))
        })
    }

    /// Quadros por segundo da trilha (1–60).
    pub fn set_decal_track_fps(&mut self, fps: f32) -> bool {
        if !fps.is_finite() {
            return false;
        }
        let fps = fps.round().clamp(1.0, 60.0);
        self.mutate_active_decal("decal track fps", |decal, _| {
            let track = decal.track.get_or_insert_with(Default::default);
            let changed = track.fps != fps;
            track.fps = fps;
            changed
        })
    }

    /// Duração do ciclo em quadros (1–600), nunca menor que a última chave.
    pub fn set_decal_track_length(&mut self, frames: i32) -> bool {
        let frames = frames.clamp(1, 600) as u32;
        let changed = self.mutate_active_decal("decal track length", |decal, _| {
            let track = decal.track.get_or_insert_with(Default::default);
            let last_key = track.keys.last().map_or(0, |key| key.frame + 1);
            let length = frames.max(last_key);
            let changed = track.length_frames != length;
            track.length_frames = length;
            changed
        });
        let length = self
            .active_decal()
            .and_then(|decal| decal.track.map(|track| track.length_frames))
            .unwrap_or(1);
        self.decal_frame = self.decal_frame.min(length.saturating_sub(1));
        changed
    }

    /// Posiciona o quadro atual e mostra a variante da trilha nele.
    pub fn set_decal_frame(&mut self, frame: i32) -> bool {
        let Some(decal) = self.active_decal() else {
            return false;
        };
        let track = decal.track.unwrap_or_default();
        let frame = (frame.max(0) as u32).min(track.length_frames.max(1) - 1);
        self.decal_frame = frame;
        self.decal_playing = false;
        // Meio do quadro: sem ambiguidade de arredondamento.
        self.show_decal_time(Some((frame as f32 + 0.5) / track.fps.max(1.0)))
    }

    /// Tocar / parar a trilha. Parar volta ao estado parado (variante escolhida).
    pub fn toggle_decal_playback(&mut self) -> bool {
        if self.decal_playing {
            self.decal_playing = false;
            return self.show_decal_time(None);
        }
        let Some(track) = self.active_decal().and_then(|decal| decal.track) else {
            return false;
        };
        if track.keys.is_empty() {
            return false;
        }
        self.decal_playing = true;
        self.show_decal_time(Some((self.decal_frame as f32 + 0.5) / track.fps.max(1.0)))
    }

    /// Avança a reprodução em `dt` segundos. `true` se a textura mudou.
    pub fn decal_playback_tick(&mut self, dt: f32) -> bool {
        if !self.decal_playing || !(dt.is_finite() && dt > 0.0) {
            return false;
        }
        let Some(track) = self.active_decal().and_then(|decal| decal.track) else {
            self.decal_playing = false;
            return false;
        };
        let time = self.state.session.decal_time.unwrap_or(0.0) + dt;
        self.decal_frame = track.frame_at(time);
        self.show_decal_time(Some(time))
    }

    /// Muda o instante mostrado; recompõe só se alguma variante mudou.
    pub(crate) fn show_decal_time(&mut self, time: Option<f32>) -> bool {
        let shown = |state: &petunia_core::AppState, time: Option<f32>| -> Vec<usize> {
            state
                .project
                .active()
                .and_then(|asset| asset.paint_stack.as_ref())
                .map(|stack| {
                    stack
                        .layers
                        .iter()
                        .filter_map(|layer| match &layer.kind {
                            LayerKind::Decal(decal) => Some(decal.variant_at(time)),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let before = shown(&self.state, self.state.session.decal_time);
        self.state.session.decal_time = time;
        if shown(&self.state, time) == before {
            return false;
        }
        petunia_module_paint::PaintModule::composite_active(&mut self.state);
        self.state.mark_dirty();
        true
    }

    /// Exporta a textura composta com cada variante do decalque ativo (PNG)
    /// e a trilha (JSON) em `dir`: o "bake universal" do Decal Set.
    pub fn export_decal_frames(&mut self, dir: &std::path::Path) -> Result<usize, String> {
        let layer = self
            .state
            .project
            .active()
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .filter(|layer| matches!(layer.kind, LayerKind::Decal(_)))
            .map(|layer| (layer.id, layer.name.clone()))
            .ok_or_else(|| "no active decal".to_owned())?;
        let decal = self
            .active_decal()
            .ok_or_else(|| "no active decal".to_owned())?;
        let frames =
            petunia_module_paint::PaintModule::decal_variant_textures(&self.state, layer.0);
        let stem = sanitize_file_stem(&layer.1);
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let mut names = Vec::with_capacity(frames.len());
        for (index, (name, canvas)) in frames.iter().enumerate() {
            let file = format!("{stem}_{index:02}_{}.png", sanitize_file_stem(name));
            let image = image::RgbaImage::from_raw(canvas.w, canvas.h, canvas.pixels.to_vec())
                .ok_or_else(|| "invalid canvas".to_owned())?;
            image.save(dir.join(&file)).map_err(|e| e.to_string())?;
            names.push(serde_json::json!({ "index": index, "name": name, "file": file }));
        }
        let track = decal.track.unwrap_or_default();
        let manifest = serde_json::json!({
            "decal_set": layer.1,
            "variants": names,
            "variant_index": decal.variant_index,
            "fps": track.fps,
            "length_frames": track.length_frames,
            "looping": track.looping,
            "keys": track.keys.iter().map(|k| [k.frame, k.variant]).collect::<Vec<_>>(),
        });
        let text = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(format!("{stem}_track.json")), text).map_err(|e| e.to_string())?;
        Ok(frames.len())
    }
}

/// Nome de arquivo seguro: letras, dígitos, `-` e `_`.
fn sanitize_file_stem(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.trim_matches('_').is_empty() {
        "decal".to_owned()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_width_rejects_invalid_values_and_keeps_the_uv_aspect() {
        let decal = DecalLayer::new(Canvas::new(4, 2, [0; 4]), [0.5; 2], [0.4, 0.2], 0.0);
        assert!(decal_with_value(&decal, DecalParam::Width, 0.0).is_none());
        assert!(decal_with_value(&decal, DecalParam::Width, f32::NAN).is_none());
        let wider = decal_with_value(&decal, DecalParam::Width, 0.8).unwrap();
        assert_eq!(wider.scale_uv, [0.8, 0.4]);
        let turned = decal_with_value(&decal, DecalParam::Rotation, 90.0).unwrap();
        assert!((turned.rotation_rad - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn angles_wrap_into_a_half_open_turn() {
        assert_eq!(wrap_degrees(190.0), -170.0);
        assert_eq!(wrap_degrees(-180.0), 180.0);
        assert_eq!(wrap_degrees(45.0), 45.0);
    }

    #[test]
    fn file_stems_are_sanitized() {
        assert_eq!(sanitize_file_stem("Boca/Sorriso 2"), "Boca_Sorriso_2");
        assert_eq!(sanitize_file_stem("///"), "decal");
    }
}
