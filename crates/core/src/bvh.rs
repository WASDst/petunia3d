//! BVH de triângulos para os raios da viewport (picking, oclusão e snap).
//!
//! Antes, cada consulta percorria todos os triângulos: a seleção por caixa
//! custava O(vértices × triângulos) e o snap O(triângulos) por candidato. A
//! árvore é construída uma vez por revisão de geometria (divisão pela mediana
//! do eixo mais longo dos centróides) e cada raio passa a custar
//! O(log triângulos) no caso comum.

#![forbid(unsafe_code)]

use glam::Vec3;
use petunia_mesh::triangulate;

/// Triângulos por folha: abaixo disso testar direto é mais barato que descer.
const LEAF_SIZE: usize = 6;

#[derive(Debug, Clone, Copy)]
struct Node {
    min: Vec3,
    max: Vec3,
    /// Folha: primeiro triângulo. Interno: índice do filho esquerdo (o
    /// direito é `first + 1`).
    first: u32,
    /// Triângulos da folha; `0` marca nó interno.
    count: u32,
}

/// Árvore de volumes sobre triângulos com um identificador por triângulo
/// (índice de face, por exemplo).
#[derive(Debug, Clone, Default)]
pub struct TriangleBvh {
    nodes: Vec<Node>,
    triangles: Vec<[Vec3; 3]>,
    ids: Vec<u32>,
}

impl TriangleBvh {
    /// Constrói a árvore; triângulos com coordenadas não finitas são ignorados.
    pub fn build(items: Vec<([Vec3; 3], u32)>) -> Self {
        let items: Vec<_> = items
            .into_iter()
            .filter(|(points, _)| points.iter().all(|p| p.is_finite()))
            .collect();
        if items.is_empty() {
            return Self::default();
        }
        let centroids: Vec<Vec3> = items
            .iter()
            .map(|(points, _)| (points[0] + points[1] + points[2]) / 3.0)
            .collect();
        let mut order: Vec<u32> = (0..items.len() as u32).collect();
        let mut nodes = vec![Node {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
            first: 0,
            count: 0,
        }];
        let mut stack = vec![(0usize, 0usize, items.len())];
        while let Some((node, start, end)) = stack.pop() {
            let slice = &mut order[start..end];
            let (mut min, mut max) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
            let (mut cmin, mut cmax) = (min, max);
            for &index in slice.iter() {
                for point in items[index as usize].0 {
                    min = min.min(point);
                    max = max.max(point);
                }
                let centroid = centroids[index as usize];
                cmin = cmin.min(centroid);
                cmax = cmax.max(centroid);
            }
            let extent = cmax - cmin;
            let axis = if extent.x >= extent.y && extent.x >= extent.z {
                0
            } else if extent.y >= extent.z {
                1
            } else {
                2
            };
            let count = end - start;
            if count <= LEAF_SIZE || extent[axis] <= f32::EPSILON {
                nodes[node] = Node {
                    min,
                    max,
                    first: start as u32,
                    count: count as u32,
                };
                continue;
            }
            let middle = count / 2;
            slice.select_nth_unstable_by(middle, |a, b| {
                centroids[*a as usize][axis].total_cmp(&centroids[*b as usize][axis])
            });
            let left = nodes.len();
            let placeholder = Node {
                min,
                max,
                first: 0,
                count: 0,
            };
            nodes.push(placeholder);
            nodes.push(placeholder);
            nodes[node] = Node {
                min,
                max,
                first: left as u32,
                count: 0,
            };
            stack.push((left, start, start + middle));
            stack.push((left + 1, start + middle, end));
        }
        let triangles = order.iter().map(|&i| items[i as usize].0).collect();
        let ids = order.iter().map(|&i| items[i as usize].1).collect();
        Self {
            nodes,
            triangles,
            ids,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    pub fn len(&self) -> usize {
        self.triangles.len()
    }

    /// Caixa envolvente de todos os triângulos.
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        self.nodes
            .first()
            .filter(|_| !self.is_empty())
            .map(|n| (n.min, n.max))
    }

    /// Distância de entrada do raio na caixa, se a atravessa antes de `limit`.
    fn enter(node: &Node, origin: Vec3, direction: Vec3, limit: f32) -> Option<f32> {
        let (mut near, mut far) = (0.0_f32, limit);
        for axis in 0..3 {
            let lo = node.min[axis] - 1.0e-5;
            let hi = node.max[axis] + 1.0e-5;
            if direction[axis].abs() < 1.0e-10 {
                if origin[axis] < lo || origin[axis] > hi {
                    return None;
                }
            } else {
                let a = (lo - origin[axis]) / direction[axis];
                let b = (hi - origin[axis]) / direction[axis];
                near = near.max(a.min(b));
                far = far.min(a.max(b));
                if near > far {
                    return None;
                }
            }
        }
        Some(near)
    }

    /// Acerto mais próximo antes de `limit` que `accept` aprova, com o id do
    /// triângulo. `accept` recebe a distância ao longo do raio.
    pub fn nearest(
        &self,
        origin: Vec3,
        direction: Vec3,
        limit: f32,
        mut accept: impl FnMut(f32) -> bool,
    ) -> Option<(f32, u32)> {
        if self.is_empty() || !origin.is_finite() || !direction.is_finite() {
            return None;
        }
        let mut best: Option<(f32, u32)> = None;
        let mut stack = vec![0usize];
        while let Some(index) = stack.pop() {
            let node = &self.nodes[index];
            let bound = best.map_or(limit, |(d, _)| d);
            if Self::enter(node, origin, direction, bound).is_none() {
                continue;
            }
            if node.count == 0 {
                let (left, right) = (node.first as usize, node.first as usize + 1);
                let dl = Self::enter(&self.nodes[left], origin, direction, bound);
                let dr = Self::enter(&self.nodes[right], origin, direction, bound);
                // O filho mais próximo sai primeiro da pilha.
                match (dl, dr) {
                    (Some(a), Some(b)) if a <= b => stack.extend([right, left]),
                    (Some(_), Some(_)) => stack.extend([left, right]),
                    (Some(_), None) => stack.push(left),
                    (None, Some(_)) => stack.push(right),
                    (None, None) => {}
                }
                continue;
            }
            let start = node.first as usize;
            for offset in 0..node.count as usize {
                let [a, b, c] = self.triangles[start + offset];
                let Some(distance) = triangulate::ray_tri(origin, direction, a, b, c) else {
                    continue;
                };
                if distance.is_finite()
                    && distance < best.map_or(limit, |(d, _)| d)
                    && accept(distance)
                {
                    best = Some((distance, self.ids[start + offset]));
                }
            }
        }
        best
    }

    /// Existe algum acerto antes de `limit` que `accept` aprova?
    pub fn any(
        &self,
        origin: Vec3,
        direction: Vec3,
        limit: f32,
        mut accept: impl FnMut(f32) -> bool,
    ) -> bool {
        if self.is_empty() || !origin.is_finite() || !direction.is_finite() {
            return false;
        }
        let mut stack = vec![0usize];
        while let Some(index) = stack.pop() {
            let node = &self.nodes[index];
            if Self::enter(node, origin, direction, limit).is_none() {
                continue;
            }
            if node.count == 0 {
                stack.push(node.first as usize);
                stack.push(node.first as usize + 1);
                continue;
            }
            let start = node.first as usize;
            for offset in 0..node.count as usize {
                let [a, b, c] = self.triangles[start + offset];
                if let Some(distance) = triangulate::ray_tri(origin, direction, a, b, c)
                    && distance.is_finite()
                    && distance < limit
                    && accept(distance)
                {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(n: u32) -> Vec<([Vec3; 3], u32)> {
        let mut items = Vec::new();
        for y in 0..n {
            for x in 0..n {
                let (x0, y0) = (x as f32, y as f32);
                let id = y * n + x;
                items.push((
                    [
                        Vec3::new(x0, y0, 0.0),
                        Vec3::new(x0 + 1.0, y0, 0.0),
                        Vec3::new(x0 + 1.0, y0 + 1.0, 0.0),
                    ],
                    id,
                ));
                items.push((
                    [
                        Vec3::new(x0, y0, 0.0),
                        Vec3::new(x0 + 1.0, y0 + 1.0, 0.0),
                        Vec3::new(x0, y0 + 1.0, 0.0),
                    ],
                    id,
                ));
            }
        }
        items
    }

    fn brute_nearest(items: &[([Vec3; 3], u32)], origin: Vec3, dir: Vec3) -> Option<(f32, u32)> {
        items
            .iter()
            .filter_map(|([a, b, c], id)| {
                triangulate::ray_tri(origin, dir, *a, *b, *c).map(|d| (d, *id))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
    }

    #[test]
    fn nearest_matches_brute_force_on_a_grid() {
        let items = grid(24);
        let bvh = TriangleBvh::build(items.clone());
        for (x, y) in [(0.3, 0.2), (5.7, 9.1), (23.9, 23.9), (12.0, 12.5)] {
            let origin = Vec3::new(x, y, 5.0);
            let dir = Vec3::NEG_Z;
            let expected = brute_nearest(&items, origin, dir);
            let got = bvh.nearest(origin, dir, f32::INFINITY, |_| true);
            assert_eq!(got.map(|g| g.1), expected.map(|e| e.1));
            assert!((got.unwrap().0 - 5.0).abs() < 1e-4);
        }
        assert!(
            bvh.nearest(
                Vec3::new(-3.0, -3.0, 5.0),
                Vec3::NEG_Z,
                f32::INFINITY,
                |_| true
            )
            .is_none()
        );
    }

    #[test]
    fn any_respects_the_limit_and_the_filter() {
        let bvh = TriangleBvh::build(grid(8));
        let origin = Vec3::new(2.5, 2.5, 5.0);
        assert!(bvh.any(origin, Vec3::NEG_Z, 6.0, |_| true));
        assert!(!bvh.any(origin, Vec3::NEG_Z, 4.0, |_| true));
        assert!(!bvh.any(origin, Vec3::NEG_Z, 6.0, |_| false));
    }

    #[test]
    fn empty_and_non_finite_inputs_are_safe() {
        let bvh = TriangleBvh::build(vec![([Vec3::NAN, Vec3::ZERO, Vec3::X], 0)]);
        assert!(bvh.is_empty());
        assert!(bvh.bounds().is_none());
        assert!(bvh.nearest(Vec3::ZERO, Vec3::Z, 1.0, |_| true).is_none());
        assert!(!bvh.any(Vec3::ZERO, Vec3::Z, 1.0, |_| true));
    }
}
