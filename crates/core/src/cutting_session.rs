//! Máquina de estado e controlador neutro de sessão para ferramentas de corte (Knife, Slice e Loop Cut).
//!
//! Totalmente desacoplado de bibliotecas gráficas concretas (egui).
//! Armazena malha base, âncora de tela agnóstica (`[f32; 2]`), pontos de corte e anéis de loop.

use glam::Vec3;
use petunia_mesh::Mesh;
use petunia_mesh::knife::{EdgePoint, cut_face};
use petunia_mesh::loop_cut::{LoopCutError, LoopRing};

use crate::camera::Camera;
use crate::viewport::LogicalRect;

/// Sessão de corte ativa (Knife, Slice Plane ou Loop Cut).
#[derive(Debug, Clone)]
pub struct CutSession {
    /// Cópia original da malha antes do início do fluxo de corte.
    pub source: Mesh,
    /// Coordenada 2D de tela inicial do clique/arrasto (pixels lógicos).
    pub anchor: Option<[f32; 2]>,
    /// Ponto de início de aresta para a ferramenta Knife.
    pub edge_start: Option<EdgePoint>,
    /// Anel de faces/arestas descoberto para a ferramenta Loop Cut.
    pub ring: Option<LoopRing>,
    /// Quantidade de cortes paralelos no Loop Cut (1 a 32).
    pub cuts: usize,
    /// Knife segments previewed since the transaction began.
    pub segments: usize,
    /// Flag indicando se a sessão está no estágio de deslizamento interativo (slide).
    pub sliding: bool,
    /// Se verdadeiro, remove a metade negativa do corte e fecha a tampa (Trim).
    pub fill_cap: bool,
    /// Se verdadeiro, insere cortes simétricos equilibrados (Dual Balanced Loop Rings).
    pub balanced: bool,
}

impl CutSession {
    /// Inicializa uma nova sessão de corte a partir da malha ativa.
    pub fn new(source: Mesh) -> Self {
        Self {
            source,
            anchor: None,
            edge_start: None,
            ring: None,
            cuts: 1,
            segments: 0,
            sliding: false,
            fill_cap: false,
            balanced: false,
        }
    }

    /// Ajusta a quantidade de cortes do loop cut dentro dos limites seguros (1..=32).
    pub fn adjust_cuts(&mut self, delta: i32) {
        self.cuts = (self.cuts as i32 + delta).clamp(1, 32) as usize;
    }

    /// Executa um segmento de corte da faca entre dois pontos de aresta.
    pub fn cut_knife_segment(
        &mut self,
        start: EdgePoint,
        end: EdgePoint,
        current_mesh: &Mesh,
    ) -> Result<Mesh, String> {
        cut_face(current_mesh, start, end)
    }

    /// Como [`Self::cut_knife_segment`], mas atravessa as faces entre os dois
    /// pontos seguindo o plano de visão (sem cliques intermediários).
    pub fn cut_knife_path(
        &mut self,
        start: EdgePoint,
        end: EdgePoint,
        current_mesh: &Mesh,
        view_direction: Vec3,
    ) -> Result<Mesh, String> {
        petunia_mesh::knife::cut_path(current_mesh, start, end, view_direction)
    }

    /// Calcula a malha fatiada por um plano gerado pelo arrasto em tela.
    pub fn compute_slice(
        &self,
        camera: &Camera,
        anchor: [f32; 2],
        current: [f32; 2],
        viewport: LogicalRect,
    ) -> Option<Mesh> {
        let delta_x = current[0] - anchor[0];
        let delta_y = current[1] - anchor[1];

        if (delta_x * delta_x + delta_y * delta_y) <= 16.0 {
            return None;
        }

        let normal = (camera.right() * delta_y + camera.up() * delta_x).normalize_or_zero();
        let normal = if !normal.is_finite() || normal.length_squared() <= 1e-6 {
            camera.right()
        } else {
            normal
        };
        let center = Vec3::from_array(self.source.selection_center());
        let anchor_ndc = viewport.screen_to_ndc(anchor);
        let (origin, direction) = camera.ray(anchor_ndc[0], anchor_ndc[1]);
        let forward = camera.forward();
        let denominator = direction.dot(forward);

        if denominator.abs() <= 1e-5 {
            return None;
        }

        let point = origin + direction * ((center - origin).dot(forward) / denominator);
        let mut mesh = self.source.clone();
        mesh.slice_plane(point, normal, self.fill_cap);
        Some(mesh)
    }

    /// Calcula o fator de deslizamento do loop cut no intervalo [-1.0, 1.0].
    pub fn compute_loop_slide(&self, current_x: f32) -> f32 {
        self.anchor
            .map(|anchor| ((current_x - anchor[0]) / 200.0).clamp(-1.0, 1.0))
            .unwrap_or(0.0)
    }

    /// Gera os segmentos de reta 3D correspondentes às linhas de pré-visualização do loop cut.
    pub fn preview_loop_lines(&self, slide: f32) -> Result<Vec<[Vec3; 2]>, LoopCutError> {
        if let Some(ring) = &self.ring {
            if self.balanced {
                ring.preview_balanced(&self.source, self.cuts, slide)
            } else {
                ring.preview(&self.source, self.cuts, slide)
            }
        } else {
            Ok(Vec::new())
        }
    }

    /// Aplica a inserção do anel de loop na malha original com o fator de deslizamento especificado.
    pub fn apply_loop_cut(&self, slide: f32) -> Result<Mesh, LoopCutError> {
        if let Some(ring) = &self.ring {
            if self.balanced {
                ring.apply_balanced(&self.source, self.cuts, slide)
            } else {
                ring.apply(&self.source, self.cuts, slide)
            }
        } else {
            Err(LoopCutError::InvalidEdge)
        }
    }
}
