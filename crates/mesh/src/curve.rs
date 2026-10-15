//! Petunia3D — Curvas Bézier Cúbicas 2D e Perfis Vetoriais Adaptativos.
//!
//! Implementa nós com alças (Sharp, Smooth, Symmetric), avaliação analítica,
//! tesselação adaptativa de De Casteljau por tolerância de curvatura,
//! e cálculo de espessura de parede (offset paralelo para perfis ocos).

use serde::{Deserialize, Serialize};

/// Tipo de nó Bézier para controle de continuidade e tangentes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BezierNodeKind {
    /// Alças independentes: permite quinas vivas e bicos agudos.
    #[default]
    Sharp,
    /// Alças colineares opostas: garante tangência contínua C1 suave.
    Smooth,
    /// Alças colineares e de mesmo comprimento: simetria perfeita na curva.
    Symmetric,
}

/// Nó de curva Bézier 2D no plano local de trabalho.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BezierNode {
    /// Posição 2D do ponto âncora no plano local.
    pub point: [f32; 2],
    /// Vetor relativo da alça de entrada (opcional, None = canto reto de entrada).
    pub handle_in: Option<[f32; 2]>,
    /// Vetor relativo da alça de saída (opcional, None = canto reto de saída).
    pub handle_out: Option<[f32; 2]>,
    /// Tipo de nó (Sharp, Smooth, Symmetric).
    pub kind: BezierNodeKind,
}

impl BezierNode {
    /// Cria um nó simples reto (sem alças, vértice pontiagudo).
    pub const fn new(point: [f32; 2]) -> Self {
        Self {
            point,
            handle_in: None,
            handle_out: None,
            kind: BezierNodeKind::Sharp,
        }
    }

    /// Cria um nó suave com alças de entrada e saída.
    pub fn smooth(point: [f32; 2], handle_in: [f32; 2], handle_out: [f32; 2]) -> Self {
        Self {
            point,
            handle_in: Some(handle_in),
            handle_out: Some(handle_out),
            kind: BezierNodeKind::Smooth,
        }
    }

    /// Cria um nó simétrico onde a alça de entrada espelha a de saída.
    pub fn symmetric(point: [f32; 2], handle_out: [f32; 2]) -> Self {
        let handle_in = [-handle_out[0], -handle_out[1]];
        Self {
            point,
            handle_in: Some(handle_in),
            handle_out: Some(handle_out),
            kind: BezierNodeKind::Symmetric,
        }
    }

    /// Posição absoluta da alça de entrada no plano local.
    pub fn absolute_handle_in(&self) -> Option<[f32; 2]> {
        self.handle_in
            .map(|h| [self.point[0] + h[0], self.point[1] + h[1]])
    }

    /// Posição absoluta da alça de saída no plano local.
    pub fn absolute_handle_out(&self) -> Option<[f32; 2]> {
        self.handle_out
            .map(|h| [self.point[0] + h[0], self.point[1] + h[1]])
    }
}

/// Caminho vetorial 2D composto por nós Bézier e segmentos retos.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BezierPath {
    pub nodes: Vec<BezierNode>,
    pub closed: bool,
}

impl BezierPath {
    pub const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            closed: false,
        }
    }

    /// Constrói um caminho a partir de uma lista de pontos retos.
    pub fn from_points(points: &[[f32; 2]], closed: bool) -> Self {
        let nodes = points.iter().map(|&p| BezierNode::new(p)).collect();
        Self { nodes, closed }
    }

    pub fn add_point(&mut self, point: [f32; 2]) {
        self.nodes.push(BezierNode::new(point));
    }

    pub fn add_node(&mut self, node: BezierNode) {
        self.nodes.push(node);
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Avalia a curva Bézier e produz um polígono amostrado adaptativamente.
    ///
    /// `tolerance`: distância máxima de tolerância geométrica para subdivisão
    /// (recomenda-se entre 0.01 e 0.05). Valores menores geram mais polígonos.
    pub fn tessellate(&self, tolerance: f32) -> Vec<[f32; 2]> {
        if self.nodes.is_empty() {
            return Vec::new();
        }
        if self.nodes.len() == 1 {
            return vec![self.nodes[0].point];
        }

        let tol = tolerance.max(1e-4);
        let mut result = Vec::new();
        result.push(self.nodes[0].point);

        let count = if self.closed {
            self.nodes.len()
        } else {
            self.nodes.len() - 1
        };

        for i in 0..count {
            let next_idx = (i + 1) % self.nodes.len();
            let n0 = &self.nodes[i];
            let n1 = &self.nodes[next_idx];

            let p0 = n0.point;
            let p1 = n0.absolute_handle_out().unwrap_or(p0);
            let p3 = n1.point;
            let p2 = n1.absolute_handle_in().unwrap_or(p3);

            let is_straight =
                (p0[0] == p1[0] && p0[1] == p1[1]) && (p2[0] == p3[0] && p2[1] == p3[1]);

            if is_straight {
                if next_idx != 0 || !self.closed {
                    result.push(p3);
                }
            } else {
                let mut segment_points = Vec::new();
                tessellate_cubic_segment(p0, p1, p2, p3, tol, 0, &mut segment_points);
                for pt in segment_points {
                    result.push(pt);
                }
            }
        }

        // Se fechado, garante que o último ponto não duplique o primeiro se for idêntico
        if self.closed
            && result.len() > 1
            && let (Some(&first), Some(&last)) = (result.first(), result.last())
            && (first[0] - last[0]).hypot(first[1] - last[1]) < 1e-5
        {
            result.pop();
        }

        result
    }

    /// Converte o caminho em comandos SVG formatados para renderização na viewport.
    pub fn to_svg_path(&self) -> String {
        use std::fmt::Write as _;
        if self.nodes.is_empty() {
            return String::new();
        }

        let mut svg = String::new();
        let _ = write!(
            svg,
            "M {:.2} {:.2} ",
            self.nodes[0].point[0], self.nodes[0].point[1]
        );

        let count = if self.closed {
            self.nodes.len()
        } else {
            self.nodes.len() - 1
        };

        for i in 0..count {
            let next_idx = (i + 1) % self.nodes.len();
            let n0 = &self.nodes[i];
            let n1 = &self.nodes[next_idx];

            let p0 = n0.point;
            let p1 = n0.absolute_handle_out().unwrap_or(p0);
            let p3 = n1.point;
            let p2 = n1.absolute_handle_in().unwrap_or(p3);

            let has_curve = (p0 != p1) || (p2 != p3);
            if has_curve {
                let _ = write!(
                    svg,
                    "C {:.2} {:.2}, {:.2} {:.2}, {:.2} {:.2} ",
                    p1[0], p1[1], p2[0], p2[1], p3[0], p3[1]
                );
            } else {
                let _ = write!(svg, "L {:.2} {:.2} ", p3[0], p3[1]);
            }
        }

        if self.closed {
            svg.push('Z');
        }

        svg
    }

    /// Converte os nós em curvas Bézier suaves contínuas (G1/C1) calculando alças tangentes
    /// baseadas nas distâncias entre nós adjacentes.
    pub fn auto_smooth(&mut self, factor: f32) {
        let n = self.nodes.len();
        if n < 2 {
            return;
        }
        let factor = factor.clamp(0.05, 0.5);
        let mut new_handles = Vec::with_capacity(n);

        for i in 0..n {
            let p_curr = self.nodes[i].point;
            let (p_prev, p_next) = if self.closed {
                let prev_idx = (i + n - 1) % n;
                let next_idx = (i + 1) % n;
                (self.nodes[prev_idx].point, self.nodes[next_idx].point)
            } else {
                let prev_idx = if i == 0 { 0 } else { i - 1 };
                let next_idx = if i + 1 >= n { n - 1 } else { i + 1 };
                (self.nodes[prev_idx].point, self.nodes[next_idx].point)
            };

            let tan_x = p_next[0] - p_prev[0];
            let tan_y = p_next[1] - p_prev[1];
            let tan_len = tan_x.hypot(tan_y);

            if tan_len < 1e-6 {
                new_handles.push((None, None));
                continue;
            }

            let dir_x = tan_x / tan_len;
            let dir_y = tan_y / tan_len;

            let d_prev = (p_curr[0] - p_prev[0]).hypot(p_curr[1] - p_prev[1]);
            let d_next = (p_next[0] - p_curr[0]).hypot(p_next[1] - p_curr[1]);

            let h_in = if i == 0 && !self.closed {
                None
            } else {
                let len = d_prev * factor;
                Some([-dir_x * len, -dir_y * len])
            };

            let h_out = if i + 1 >= n && !self.closed {
                None
            } else {
                let len = d_next * factor;
                Some([dir_x * len, dir_y * len])
            };

            new_handles.push((h_in, h_out));
        }

        for (i, (h_in, h_out)) in new_handles.into_iter().enumerate() {
            self.nodes[i].handle_in = h_in;
            self.nodes[i].handle_out = h_out;
            self.nodes[i].kind = BezierNodeKind::Smooth;
        }
    }

    /// Limpa todas as alças tornando todos os nós cantos retos (Sharp).
    pub fn clear_handles(&mut self) {
        for node in &mut self.nodes {
            node.handle_in = None;
            node.handle_out = None;
            node.kind = BezierNodeKind::Sharp;
        }
    }
}

/// Avalia o valor de um ponto numa curva Bézier cúbica para t em [0, 1].
pub fn eval_cubic_bezier(
    p0: [f32; 2],
    p1: [f32; 2],
    p2: [f32; 2],
    p3: [f32; 2],
    t: f32,
) -> [f32; 2] {
    let it = 1.0 - t;
    let b0 = it * it * it;
    let b1 = 3.0 * it * it * t;
    let b2 = 3.0 * it * t * t;
    let b3 = t * t * t;
    [
        b0 * p0[0] + b1 * p1[0] + b2 * p2[0] + b3 * p3[0],
        b0 * p0[1] + b1 * p1[1] + b2 * p2[1] + b3 * p3[1],
    ]
}

/// Distância euclidiana máxima de p1 e p2 em relação à reta de corda ligando p0 a p3.
fn cubic_flatness(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2]) -> f32 {
    let dx = p3[0] - p0[0];
    let dy = p3[1] - p0[1];
    let len = dx.hypot(dy);
    if len < 1e-6 {
        return (p1[0] - p0[0])
            .hypot(p1[1] - p0[1])
            .max((p2[0] - p0[0]).hypot(p2[1] - p0[1]));
    }
    let dist = |p: [f32; 2]| -> f32 { ((p[0] - p0[0]) * dy - (p[1] - p0[1]) * dx).abs() / len };
    dist(p1).max(dist(p2))
}

/// Subdivisão recursiva de De Casteljau até que o critério de flatness seja atendido.
fn tessellate_cubic_segment(
    p0: [f32; 2],
    p1: [f32; 2],
    p2: [f32; 2],
    p3: [f32; 2],
    tol: f32,
    depth: usize,
    out: &mut Vec<[f32; 2]>,
) {
    if depth >= 8 || cubic_flatness(p0, p1, p2, p3) <= tol {
        out.push(p3);
        return;
    }

    // Algoritmo de De Casteljau (ponto médio t = 0.5)
    let p01 = [(p0[0] + p1[0]) * 0.5, (p0[1] + p1[1]) * 0.5];
    let p12 = [(p1[0] + p2[0]) * 0.5, (p1[1] + p2[1]) * 0.5];
    let p23 = [(p2[0] + p3[0]) * 0.5, (p2[1] + p3[1]) * 0.5];

    let p012 = [(p01[0] + p12[0]) * 0.5, (p01[1] + p12[1]) * 0.5];
    let p123 = [(p12[0] + p23[0]) * 0.5, (p12[1] + p23[1]) * 0.5];

    let p_mid = [(p012[0] + p123[0]) * 0.5, (p012[1] + p123[1]) * 0.5];

    tessellate_cubic_segment(p0, p01, p012, p_mid, tol, depth + 1, out);
    tessellate_cubic_segment(p_mid, p123, p23, p3, tol, depth + 1, out);
}

/// Gera um polígono paralelo interno ou externo (Offset / Wall Thickness) a partir de um polígono simples.
///
/// Para valores de `thickness > 0.0`, desloca as arestas para o interior (ou exterior conforme winding order),
/// unindo os segmentos nos vértices bissetores com clamping para prevenir autointerseções graves.
pub fn offset_polygon(points: &[[f32; 2]], distance: f32) -> Vec<[f32; 2]> {
    let n = points.len();
    if n < 3 || distance.abs() < 1e-5 {
        return points.to_vec();
    }

    // Calcula a área com sinal para determinar a orientação (CCW vs CW)
    let mut area = 0.0;
    for i in 0..n {
        let p = points[i];
        let q = points[(i + 1) % n];
        area += p[0] * q[1] - q[0] * p[1];
    }
    let is_ccw = area > 0.0;
    // distance > 0: expande para fora; distance < 0: encolhe para dentro
    let sign = if is_ccw { -1.0 } else { 1.0 };
    let d = distance * sign;

    // Calcula normais perpendiculares para cada aresta
    let mut edge_normals = Vec::with_capacity(n);
    for i in 0..n {
        let p0 = points[i];
        let p1 = points[(i + 1) % n];
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let len = dx.hypot(dy).max(1e-6);
        // Normal apontando para a esquerda do vetor de deslocamento
        edge_normals.push([-dy / len, dx / len]);
    }

    // Calcula os novos vértices deslocados na interseção das retas paralelas adjacentes
    let mut offset_verts = Vec::with_capacity(n);
    for i in 0..n {
        let prev_idx = (i + n - 1) % n;
        let n0 = edge_normals[prev_idx];
        let n1 = edge_normals[i];

        let bisector = [n0[0] + n1[0], n0[1] + n1[1]];
        let b_len = bisector[0].hypot(bisector[1]);

        if b_len < 1e-4 {
            // Arestas opostas / colineares invertidas
            offset_verts.push([points[i][0] + n1[0] * d, points[i][1] + n1[1] * d]);
        } else {
            let cos_half = (1.0 + n0[0] * n1[0] + n0[1] * n1[1]) * 0.5;
            let miter_scale = (1.0 / cos_half.max(0.2)).sqrt().min(3.0); // clamp do miter a 3x
            let b_norm = [bisector[0] / b_len, bisector[1] / b_len];
            offset_verts.push([
                points[i][0] + b_norm[0] * d * miter_scale,
                points[i][1] + b_norm[1] * d * miter_scale,
            ]);
        }
    }

    offset_verts
}

/// Cria um perfil duplo oco (Hollow Profile com espessura de parede) fechando os dois laços.
///
/// Retorna uma lista de pontos onde o laço externo é seguido pelo laço interno em ordem reversa,
/// pronto para ser extrudado ou revolucionado formando um objeto oco contínuo.
pub fn create_hollow_profile(points: &[[f32; 2]], wall_thickness: f32) -> Vec<[f32; 2]> {
    let n = points.len();
    if n < 3 || wall_thickness <= 0.0 {
        return points.to_vec();
    }

    let inner = offset_polygon(points, -wall_thickness);
    let mut hollow = Vec::with_capacity(n + inner.len());

    // Laço externo
    hollow.extend_from_slice(points);
    // Laço interno em ordem reversa para criar a cavidade
    for pt in inner.iter().rev() {
        hollow.push(*pt);
    }

    hollow
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bezier_straight_line_tessellation() {
        let path = BezierPath::from_points(&[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]], true);
        let pts = path.tessellate(0.05);
        assert_eq!(pts.len(), 4);
        assert_eq!(pts[0], [0.0, 0.0]);
        assert_eq!(pts[1], [2.0, 0.0]);
        assert_eq!(pts[2], [2.0, 2.0]);
        assert_eq!(pts[3], [0.0, 2.0]);
    }

    #[test]
    fn test_bezier_curved_segment_tessellation() {
        let mut path = BezierPath::new();
        path.add_node(BezierNode {
            point: [0.0, 0.0],
            handle_in: None,
            handle_out: Some([1.0, 0.0]),
            kind: BezierNodeKind::Smooth,
        });
        path.add_node(BezierNode {
            point: [2.0, 2.0],
            handle_in: Some([0.0, -1.0]),
            handle_out: None,
            kind: BezierNodeKind::Smooth,
        });
        path.closed = false;

        let pts = path.tessellate(0.02);
        // Deve conter mais pontos que apenas os 2 extremos devido à curvatura
        assert!(pts.len() >= 5);
        assert_eq!(pts[0], [0.0, 0.0]);
        assert_eq!(*pts.last().unwrap(), [2.0, 2.0]);

        // Os pontos intermediários devem estar dentro do bounding box [0..2, 0..2]
        for p in &pts {
            assert!(p[0] >= -1e-5 && p[0] <= 2.0 + 1e-5);
            assert!(p[1] >= -1e-5 && p[1] <= 2.0 + 1e-5);
        }
    }

    #[test]
    fn test_offset_polygon_square() {
        let square = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let inner = offset_polygon(&square, -1.0);
        assert_eq!(inner.len(), 4);
        // Cada vértice deve ter sido recuado em direção ao centro (aprox 1.0, 1.0)
        assert!((inner[0][0] - 1.0).abs() < 0.2);
        assert!((inner[0][1] - 1.0).abs() < 0.2);
    }

    #[test]
    fn test_create_hollow_profile() {
        let square = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let hollow = create_hollow_profile(&square, 1.0);
        // Deve conter 8 vértices (4 externos + 4 internos)
        assert_eq!(hollow.len(), 8);
    }

    #[test]
    fn test_bezier_auto_smooth_and_clear_handles() {
        let mut path =
            BezierPath::from_points(&[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]], true);
        assert_eq!(path.nodes[0].kind, BezierNodeKind::Sharp);
        assert!(path.nodes[0].handle_in.is_none());

        path.auto_smooth(0.25);
        assert_eq!(path.nodes[0].kind, BezierNodeKind::Smooth);
        assert!(path.nodes[0].handle_in.is_some());
        assert!(path.nodes[0].handle_out.is_some());

        // A tesselacao agora gera curva suave com mais vertices
        let pts = path.tessellate(0.02);
        assert!(pts.len() > 4);

        path.clear_handles();
        assert_eq!(path.nodes[0].kind, BezierNodeKind::Sharp);
        assert!(path.nodes[0].handle_in.is_none());
        assert_eq!(path.tessellate(0.02).len(), 4);
    }
}
