//! Arranjo planar de perfis (ADR 007, Onda 4): regiões fechadas formadas por
//! polilinhas que se cruzam no mesmo plano, como no Plasticity.
//!
//! Algoritmo clássico de arranjo em DCEL simplificada:
//! 1. divide todos os segmentos em todas as interseções (O(n²), suficiente
//!    para perfis low-poly);
//! 2. remove pontas soltas (arestas que não fecham nada);
//! 3. percorre as faces com a regra "próxima aresta no sentido horário" —
//!    ciclos anti-horários (área positiva) são regiões limitadas, ciclos
//!    horários são contornos externos de componentes;
//! 4. cada contorno externo vira furo da menor região que o contém.
//!
//! Trabalha só com `[f64; 2]` no espaço do plano; nada de tipos externos.

use std::collections::HashMap;

/// Polilinha de entrada no espaço 2D do plano.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline2 {
    pub points: Vec<[f64; 2]>,
    pub closed: bool,
}

/// Região limitada: contorno anti-horário e furos horários.
#[derive(Debug, Clone, PartialEq)]
pub struct Region2 {
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}

impl Region2 {
    /// Área com os furos descontados.
    pub fn area(&self) -> f64 {
        signed_area(&self.outer) + self.holes.iter().map(|h| signed_area(h)).sum::<f64>()
    }

    /// Ponto estritamente dentro do contorno e fora de todos os furos.
    pub fn contains(&self, point: [f64; 2]) -> bool {
        point_in_polygon(&self.outer, point)
            && !self.holes.iter().any(|hole| point_in_polygon(hole, point))
    }
}

/// Todas as regiões limitadas do arranjo das polilinhas.
pub fn planar_regions(polylines: &[Polyline2]) -> Vec<Region2> {
    let segments = collect_segments(polylines);
    if segments.len() < 3 {
        return Vec::new();
    }
    let scale = segments
        .iter()
        .flat_map(|(a, b)| [a, b])
        .fold(0.0_f64, |acc, p| acc.max(p[0].abs()).max(p[1].abs()))
        .max(1.0);
    let eps = scale * 1.0e-9;

    let split = split_segments(&segments, eps);
    let mut graph = Graph::new(eps * 10.0);
    for (a, b) in split {
        graph.add_edge(a, b);
    }
    graph.prune_dangling();
    graph.regions(eps)
}

/// Índice da região que contém `point`, se houver.
pub fn region_at(regions: &[Region2], point: [f64; 2]) -> Option<usize> {
    regions.iter().position(|region| region.contains(point))
}

/// Área com sinal (positiva = anti-horário).
pub fn signed_area(points: &[[f64; 2]]) -> f64 {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        * 0.5
}

/// Ponto dentro do polígono (regra par-ímpar); pontos na borda são ambíguos.
pub fn point_in_polygon(polygon: &[[f64; 2]], point: [f64; 2]) -> bool {
    let n = polygon.len();
    let mut inside = false;
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[j]);
        if (a[1] > point[1]) != (b[1] > point[1]) {
            let x = (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0];
            if point[0] < x {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn collect_segments(polylines: &[Polyline2]) -> Vec<([f64; 2], [f64; 2])> {
    let mut segments = Vec::new();
    for line in polylines {
        let points: Vec<[f64; 2]> = line
            .points
            .iter()
            .copied()
            .filter(|p| p[0].is_finite() && p[1].is_finite())
            .collect();
        if points.len() < 2 {
            continue;
        }
        let count = if line.closed && points.len() >= 3 {
            points.len()
        } else {
            points.len() - 1
        };
        for i in 0..count {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            if (a[0] - b[0]).hypot(a[1] - b[1]) > 0.0 {
                segments.push((a, b));
            }
        }
    }
    segments
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn lerp(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// Divide cada segmento em todos os pontos em que toca outro (inclusive
/// sobreposições colineares).
fn split_segments(segments: &[([f64; 2], [f64; 2])], eps: f64) -> Vec<([f64; 2], [f64; 2])> {
    let mut params: Vec<Vec<f64>> = vec![vec![0.0, 1.0]; segments.len()];
    for i in 0..segments.len() {
        for j in (i + 1)..segments.len() {
            let (p, p2) = segments[i];
            let (q, q2) = segments[j];
            let r = sub(p2, p);
            let s = sub(q2, q);
            let denom = cross(r, s);
            let qp = sub(q, p);
            let (rr, ss) = (dot(r, r), dot(s, s));
            if denom.abs() > eps * (rr.sqrt() * ss.sqrt()).max(eps) {
                let t = cross(qp, s) / denom;
                let u = cross(qp, r) / denom;
                let tol = 1.0e-9;
                if (-tol..=1.0 + tol).contains(&t) && (-tol..=1.0 + tol).contains(&u) {
                    params[i].push(t.clamp(0.0, 1.0));
                    params[j].push(u.clamp(0.0, 1.0));
                }
            } else if cross(qp, r).abs() <= eps * rr.sqrt().max(eps) {
                // Colineares: projeta as pontas de um no outro.
                for (point, target, tr, list) in [(q, p, r, i), (q2, p, r, i)] {
                    let t = dot(sub(point, target), tr) / rr;
                    if (0.0..=1.0).contains(&t) {
                        params[list].push(t);
                    }
                }
                for point in [p, p2] {
                    let u = dot(sub(point, q), s) / ss;
                    if (0.0..=1.0).contains(&u) {
                        params[j].push(u);
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    for (index, (a, b)) in segments.iter().enumerate() {
        let list = &mut params[index];
        list.sort_by(f64::total_cmp);
        list.dedup_by(|x, y| (*x - *y).abs() < 1.0e-12);
        for pair in list.windows(2) {
            let (start, end) = (lerp(*a, *b, pair[0]), lerp(*a, *b, pair[1]));
            if (start[0] - end[0]).hypot(start[1] - end[1]) > eps {
                out.push((start, end));
            }
        }
    }
    out
}

struct Graph {
    points: Vec<[f64; 2]>,
    lookup: HashMap<(i64, i64), usize>,
    cell: f64,
    neighbors: Vec<Vec<usize>>,
}

impl Graph {
    fn new(merge: f64) -> Self {
        Self {
            points: Vec::new(),
            lookup: HashMap::new(),
            cell: merge.max(1.0e-12),
            neighbors: Vec::new(),
        }
    }

    /// Vértices a menos de `cell` são o mesmo (procura nas células vizinhas).
    fn vertex(&mut self, p: [f64; 2]) -> usize {
        let key = (
            (p[0] / self.cell).round() as i64,
            (p[1] / self.cell).round() as i64,
        );
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(&index) = self.lookup.get(&(key.0 + dx, key.1 + dy)) {
                    let q = self.points[index];
                    if (q[0] - p[0]).hypot(q[1] - p[1]) <= self.cell {
                        return index;
                    }
                }
            }
        }
        let index = self.points.len();
        self.points.push(p);
        self.neighbors.push(Vec::new());
        self.lookup.insert(key, index);
        index
    }

    fn add_edge(&mut self, a: [f64; 2], b: [f64; 2]) {
        let (ia, ib) = (self.vertex(a), self.vertex(b));
        if ia == ib || self.neighbors[ia].contains(&ib) {
            return;
        }
        self.neighbors[ia].push(ib);
        self.neighbors[ib].push(ia);
    }

    fn prune_dangling(&mut self) {
        let mut stack: Vec<usize> = (0..self.points.len())
            .filter(|&v| self.neighbors[v].len() == 1)
            .collect();
        while let Some(v) = stack.pop() {
            if self.neighbors[v].len() != 1 {
                continue;
            }
            let u = self.neighbors[v][0];
            self.neighbors[v].clear();
            self.neighbors[u].retain(|&w| w != v);
            if self.neighbors[u].len() == 1 {
                stack.push(u);
            }
        }
    }

    fn regions(&mut self, eps: f64) -> Vec<Region2> {
        // Vizinhos em ordem anti-horária de ângulo.
        for v in 0..self.points.len() {
            let origin = self.points[v];
            let points = &self.points;
            self.neighbors[v].sort_by(|&a, &b| {
                let (da, db) = (sub(points[a], origin), sub(points[b], origin));
                da[1].atan2(da[0]).total_cmp(&db[1].atan2(db[0]))
            });
        }
        let mut visited: HashMap<(usize, usize), bool> = HashMap::new();
        let mut positive = Vec::new();
        let mut negative = Vec::new();
        for u in 0..self.points.len() {
            for &v in &self.neighbors[u] {
                if visited.contains_key(&(u, v)) {
                    continue;
                }
                let mut cycle = Vec::new();
                let (mut a, mut b) = (u, v);
                let limit = self.points.len() * 4 + 8;
                loop {
                    visited.insert((a, b), true);
                    cycle.push(a);
                    // Próxima: em `b`, o vizinho imediatamente anterior a `a`
                    // na ordem anti-horária (interior à esquerda).
                    let around = &self.neighbors[b];
                    let at = around.iter().position(|&w| w == a).unwrap_or(0);
                    let next = around[(at + around.len() - 1) % around.len()];
                    a = b;
                    b = next;
                    if (a, b) == (u, v) || cycle.len() > limit {
                        break;
                    }
                }
                let polygon: Vec<[f64; 2]> = cycle.iter().map(|&i| self.points[i]).collect();
                let area = signed_area(&polygon);
                if area > eps {
                    positive.push(polygon);
                } else if area < -eps {
                    negative.push(polygon);
                }
            }
        }
        let mut regions: Vec<Region2> = positive
            .into_iter()
            .map(|outer| Region2 {
                outer,
                holes: Vec::new(),
            })
            .collect();
        for hole in negative {
            // Amostra do lado externo do componente: à esquerda da 1ª aresta.
            let (a, b) = (hole[0], hole[1 % hole.len()]);
            let direction = sub(b, a);
            let length = direction[0].hypot(direction[1]).max(eps);
            let offset = (length * 1.0e-4).max(eps * 100.0);
            let sample = [
                (a[0] + b[0]) * 0.5 - direction[1] / length * offset,
                (a[1] + b[1]) * 0.5 + direction[0] / length * offset,
            ];
            let host = regions
                .iter()
                .enumerate()
                .filter(|(_, region)| point_in_polygon(&region.outer, sample))
                .min_by(|a, b| signed_area(&a.1.outer).total_cmp(&signed_area(&b.1.outer)))
                .map(|(index, _)| index);
            if let Some(host) = host {
                regions[host].holes.push(hole);
            }
        }
        regions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, size: f64) -> Polyline2 {
        Polyline2 {
            points: vec![[x, y], [x + size, y], [x + size, y + size], [x, y + size]],
            closed: true,
        }
    }

    fn areas(regions: &[Region2]) -> Vec<f64> {
        let mut areas: Vec<f64> = regions.iter().map(Region2::area).collect();
        areas.sort_by(f64::total_cmp);
        areas
    }

    #[test]
    fn single_closed_profile_is_one_region() {
        let regions = planar_regions(&[square(0.0, 0.0, 2.0)]);
        assert_eq!(areas(&regions), vec![4.0]);
        assert!(
            signed_area(&regions[0].outer) > 0.0,
            "contorno anti-horário"
        );
    }

    #[test]
    fn overlapping_profiles_make_three_regions() {
        let regions = planar_regions(&[square(0.0, 0.0, 2.0), square(1.0, 1.0, 2.0)]);
        let found = areas(&regions);
        assert_eq!(found.len(), 3, "{found:?}");
        for (area, expected) in found.iter().zip([1.0, 3.0, 3.0]) {
            assert!((area - expected).abs() < 1e-9, "{found:?}");
        }
        assert_eq!(
            region_at(&regions, [1.5, 1.5]).map(|i| regions[i].area()),
            Some(1.0)
        );
        assert!(region_at(&regions, [5.0, 5.0]).is_none());
    }

    #[test]
    fn nested_profile_becomes_a_hole_of_the_outer_region() {
        let regions = planar_regions(&[square(0.0, 0.0, 4.0), square(1.0, 1.0, 1.0)]);
        assert_eq!(regions.len(), 2);
        let ring = regions.iter().find(|r| !r.holes.is_empty()).expect("anel");
        assert!((ring.area() - 15.0).abs() < 1e-9);
        assert!(signed_area(&ring.holes[0]) < 0.0, "furo horário");
        // O ponto no furo pertence à região interna, não ao anel.
        let inner = region_at(&regions, [1.5, 1.5]).unwrap();
        assert!((regions[inner].area() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn open_line_across_a_profile_splits_it_and_tails_are_ignored() {
        let cut = Polyline2 {
            points: vec![[-1.0, 1.0], [3.0, 1.0]],
            closed: false,
        };
        let regions = planar_regions(&[square(0.0, 0.0, 2.0), cut]);
        assert_eq!(areas(&regions), vec![2.0, 2.0]);

        let dangling = Polyline2 {
            points: vec![[0.5, 0.5], [0.8, 0.9]],
            closed: false,
        };
        assert_eq!(planar_regions(&[dangling]).len(), 0);
    }

    #[test]
    fn shared_edge_and_touching_corners_are_merged() {
        // Dois quadrados lado a lado compartilham uma aresta inteira.
        let regions = planar_regions(&[square(0.0, 0.0, 1.0), square(1.0, 0.0, 1.0)]);
        assert_eq!(areas(&regions), vec![1.0, 1.0]);
    }

    #[test]
    fn invalid_input_is_ignored() {
        let bad = Polyline2 {
            points: vec![[f64::NAN, 0.0], [1.0, 1.0]],
            closed: true,
        };
        assert!(planar_regions(&[bad]).is_empty());
        assert!(planar_regions(&[]).is_empty());
    }
}
