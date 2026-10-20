//! Petunia3D mesh — triangulação ear clipping.

use super::Vertex;
use glam::Vec3;

pub fn face_normal_of(verts: &[Vertex], idx: &[u32]) -> Vec3 {
    let n = idx.len();
    if n < 3 {
        return Vec3::Y;
    }
    // Newell's method: acumula o produto vetorial em torno do perímetro do polígono
    let mut normal = Vec3::ZERO;
    for i in 0..n {
        let curr_idx = idx[i] as usize;
        let next_idx = idx[(i + 1) % n] as usize;
        if curr_idx >= verts.len() || next_idx >= verts.len() {
            continue;
        }
        let v_curr = verts[curr_idx].vec();
        let v_next = verts[next_idx].vec();
        normal.x += (v_curr.y - v_next.y) * (v_curr.z + v_next.z);
        normal.y += (v_curr.z - v_next.z) * (v_curr.x + v_next.x);
        normal.z += (v_curr.x - v_next.x) * (v_curr.y + v_next.y);
    }
    let norm = normal.normalize_or_zero();
    if norm.length_squared() > 0.5 {
        norm
    } else {
        // Fallback para triângulos ou casos degenerados
        let a = verts[idx[0] as usize].vec();
        let b = verts[idx[1] as usize].vec();
        let c = verts[idx[2] as usize].vec();
        (b - a).cross(c - a).normalize_or_zero()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayTriangleHit {
    pub distance: f32,
    pub barycentric: [f32; 3],
}

/// Folga baricêntrica (adimensional) nas arestas. Sem ela, um raio exatamente
/// sobre a aresta compartilhada por dois triângulos (a diagonal de um quad, onde
/// fica o centro da face) podia ser recusado pelos dois por arredondamento e
/// "vazar" para a geometria de trás (B10). 1e-5 de uma aresta de 2 m é 0,02 mm.
const EDGE_TOLERANCE: f32 = 1.0e-5;

/// Möller–Trumbore com distância e coordenadas baricêntricas.
pub fn ray_tri_hit(origin: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<RayTriangleHit> {
    if !origin.is_finite() || !dir.is_finite() || !a.is_finite() || !b.is_finite() || !c.is_finite()
    {
        return None;
    }
    let e1 = b - a;
    let e2 = c - a;
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-9 {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = origin - a;
    let u = tvec.dot(p) * inv;
    if !(-EDGE_TOLERANCE..=1.0 + EDGE_TOLERANCE).contains(&u) {
        return None;
    }
    let q = tvec.cross(e1);
    let v = dir.dot(q) * inv;
    if v < -EDGE_TOLERANCE || u + v > 1.0 + EDGE_TOLERANCE {
        return None;
    }
    let t = e2.dot(q) * inv;
    // Dentro da folga, as baricêntricas voltam ao triângulo (somam 1, ≥ 0).
    let (u, v) = (u.max(0.0), v.max(0.0));
    let sum = u + v;
    let (u, v) = if sum > 1.0 {
        (u / sum, v / sum)
    } else {
        (u, v)
    };
    (t > 1e-6 && t.is_finite()).then_some(RayTriangleHit {
        distance: t,
        barycentric: [1.0 - u - v, u, v],
    })
}

/// Compatibilidade para callers que precisam apenas da distância do raio.
pub fn ray_tri(origin: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    ray_tri_hit(origin, dir, a, b, c).map(|hit| hit.distance)
}

// ---------------------------------------------------------------------------
// Ear clipping (triangulação de polígono simples p/ Draw Profile)
// ---------------------------------------------------------------------------

pub fn polygon_area(pts: &[[f32; 2]]) -> f32 {
    let mut a = 0.0;
    for i in 0..pts.len() {
        let p = pts[i];
        let q = pts[(i + 1) % pts.len()];
        a += p[0] * q[1] - q[0] * p[1];
    }
    a * 0.5
}

fn point_in_tri(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let v0 = [c[0] - a[0], c[1] - a[1]];
    let v1 = [b[0] - a[0], b[1] - a[1]];
    let v2 = [p[0] - a[0], p[1] - a[1]];
    let d00 = v0[0] * v0[0] + v0[1] * v0[1];
    let d01 = v0[0] * v1[0] + v0[1] * v1[1];
    let d11 = v1[0] * v1[0] + v1[1] * v1[1];
    let d20 = v2[0] * v0[0] + v2[1] * v0[1];
    let d21 = v2[0] * v1[0] + v2[1] * v1[1];
    let den = d00 * d11 - d01 * d01;
    if den.abs() < 1e-12 {
        return false;
    }
    let v = (d11 * d20 - d01 * d21) / den;
    let w = (d00 * d21 - d01 * d20) / den;
    v >= -1e-6 && w >= -1e-6 && v + w <= 1.0 + 1e-6
}

/// Triangula polígono simples (CCW ou CW). Erro se degenerado/auto-intersectante.
pub fn ear_clip(points: &[[f32; 2]]) -> Result<Vec<[usize; 3]>, String> {
    if points.len() < 3 {
        return Err("polígono precisa de ao menos 3 pontos".to_string());
    }
    if points.len() > 512 {
        return Err("polígono grande demais (máx 512 pontos)".to_string());
    }
    // remove duplicados consecutivos, lembrando os índices originais
    let mut pts: Vec<[f32; 2]> = Vec::new();
    let mut kept: Vec<usize> = Vec::new();
    for (n, &p) in points.iter().enumerate() {
        if pts
            .last()
            .map(|&q| (q[0] - p[0]).abs() + (q[1] - p[1]).abs() > 1e-7)
            .unwrap_or(true)
        {
            pts.push(p);
            kept.push(n);
        }
    }
    if pts.len() < 3 {
        return Err("pontos degenerados".to_string());
    }
    let mut order: Vec<usize> = (0..pts.len()).collect();
    if polygon_area(&pts) < 0.0 {
        order.reverse();
    }
    let at = |i: usize| pts[order[i % order.len()]];
    let orig = |i: usize| kept[order[i % order.len()]];
    let mut v: Vec<usize> = (0..order.len()).collect();
    let mut tris = Vec::new();
    let mut guard = 0usize;
    while v.len() > 3 && guard < 4096 {
        guard += 1;
        let n = v.len();
        let mut clipped = false;
        for i in 0..n {
            let a = v[(i + n - 1) % n];
            let b = v[i];
            let c = v[(i + 1) % n];
            let (pa, pb, pc) = (at(a), at(b), at(c));
            // convexo? (CCW => cross > 0)
            let cross = (pb[0] - pa[0]) * (pc[1] - pa[1]) - (pb[1] - pa[1]) * (pc[0] - pa[0]);
            if cross <= 1e-9 {
                continue;
            }
            if (0..n).any(|j| {
                let k = v[j];
                k != a && k != b && k != c && point_in_tri(at(k), pa, pb, pc)
            }) {
                continue;
            }
            tris.push([orig(a), orig(b), orig(c)]);
            v.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            return Err("polígono auto-intersectante ou degenerado".to_string());
        }
    }
    if v.len() == 3 {
        tris.push([orig(v[0]), orig(v[1]), orig(v[2])]);
    }
    Ok(tris)
}

#[cfg(test)]
mod seam_tests {
    use super::ray_tri;
    use glam::{Quat, Vec3};

    /// B10: um raio exatamente sobre a diagonal compartilhada de um quad (o centro
    /// da face fica nela) precisa acertar algum dos dois triângulos. Sem tolerância,
    /// o arredondamento recusava os dois e o raio vazava para a face de trás.
    #[test]
    fn a_ray_on_the_shared_diagonal_of_a_quad_never_slips_through() {
        let mut misses = Vec::new();
        for step in 0..400 {
            let angle = step as f32 * 0.137;
            let rotation = Quat::from_euler(glam::EulerRot::XYZ, angle, angle * 0.7, -angle * 0.3);
            let scale = Vec3::new(1.2, 0.8 + (step % 7) as f32 * 0.05, 1.0);
            let offset = Vec3::new(1.5, 0.3, -0.7);
            let place = |p: Vec3| rotation * (p * scale) + offset;
            let (a, b, c, d) = (
                place(Vec3::new(-1.0, -1.0, 1.0)),
                place(Vec3::new(1.0, -1.0, 1.0)),
                place(Vec3::new(1.0, 1.0, 1.0)),
                place(Vec3::new(-1.0, 1.0, 1.0)),
            );
            let normal = (b - a).cross(c - a).normalize();
            for t in [0.5_f32, 0.25, 0.731, 0.1] {
                let on_diagonal = a.lerp(c, t);
                let origin = on_diagonal + normal * 5.0 + (b - a) * 0.0;
                let dir = (on_diagonal - origin).normalize();
                let hit = ray_tri(origin, dir, a, b, c).or_else(|| ray_tri(origin, dir, a, c, d));
                if hit.is_none() {
                    misses.push((step, t));
                }
            }
        }
        assert!(
            misses.is_empty(),
            "{} raios vazaram pela diagonal: {:?}",
            misses.len(),
            &misses[..misses.len().min(8)]
        );
    }
}
