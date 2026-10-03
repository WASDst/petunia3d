//! Operações poligonais de DRAW com tolerâncias e dimensões explícitas.
use glam::DVec2;

pub fn regular_polygon(radius: f64, sides: usize) -> Vec<[f64; 2]> {
    if !radius.is_finite() || radius <= 0.0 || !(3..=512).contains(&sides) {
        return vec![];
    }
    (0..sides)
        .map(|i| {
            let t = i as f64 * std::f64::consts::TAU / sides as f64;
            [radius * t.cos(), radius * t.sin()]
        })
        .collect()
}
pub fn ellipse(rx: f64, ry: f64, segments: usize) -> Vec<[f64; 2]> {
    if !ry.is_finite() || ry <= 0.0 {
        return vec![];
    }
    regular_polygon(rx, segments)
        .into_iter()
        .map(|p| [p[0], p[1] * ry / rx])
        .collect()
}
/// Arredonda cada canto; a distância é limitada à metade das arestas vizinhas.
pub fn round_corners(points: &[[f64; 2]], radius: f64, segments: usize) -> Vec<[f64; 2]> {
    round_corners_at(points, radius, segments, None)
}
pub fn round_corners_at(
    points: &[[f64; 2]],
    radius: f64,
    segments: usize,
    selected: Option<usize>,
) -> Vec<[f64; 2]> {
    if points.len() < 3
        || !radius.is_finite()
        || radius <= 0.0
        || points.iter().flatten().any(|v| !v.is_finite())
    {
        return points.to_vec();
    }
    let mut out = Vec::new();
    for i in 0..points.len() {
        let p = DVec2::from_array(points[i]);
        if selected.is_some_and(|index| index != i) {
            out.push(p.to_array());
            continue;
        }
        let prev = DVec2::from_array(points[(i + points.len() - 1) % points.len()]);
        let next = DVec2::from_array(points[(i + 1) % points.len()]);
        let (a, b) = (prev - p, next - p);
        if a.length() < 1e-9 || b.length() < 1e-9 {
            continue;
        }
        let angle = a.normalize().dot(b.normalize()).clamp(-1.0, 1.0).acos();
        let tangent = (angle * 0.5).tan();
        if tangent.abs() < 1e-8 || (std::f64::consts::PI - angle).abs() < 1e-8 {
            out.push(p.to_array());
            continue;
        }
        let distance = (radius / tangent)
            .min(a.length() * 0.49)
            .min(b.length() * 0.49);
        let start = p + a.normalize() * distance;
        let end = p + b.normalize() * distance;
        let r = distance * tangent;
        let bisector = (a.normalize() + b.normalize()).normalize();
        let center = p + bisector * (r / (angle * 0.5).sin());
        let start_angle = (start - center).to_array();
        let end_angle = (end - center).to_array();
        let from = start_angle[1].atan2(start_angle[0]);
        let mut delta = end_angle[1].atan2(end_angle[0]) - from;
        while delta > std::f64::consts::PI {
            delta -= std::f64::consts::TAU;
        }
        while delta < -std::f64::consts::PI {
            delta += std::f64::consts::TAU;
        }
        for k in 0..=segments.clamp(1, 64) {
            let t = from + delta * k as f64 / segments.clamp(1, 64) as f64;
            out.push((center + DVec2::new(t.cos(), t.sin()) * r).to_array());
        }
    }
    out
}
pub fn rounded_rectangle(width: f64, height: f64, radius: f64) -> Vec<[f64; 2]> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return vec![];
    }
    round_corners(
        &[
            [-width / 2.0, -height / 2.0],
            [width / 2.0, -height / 2.0],
            [width / 2.0, height / 2.0],
            [-width / 2.0, height / 2.0],
        ],
        radius,
        6,
    )
}
/// Douglas–Peucker, mantendo pelo menos três pontos num contorno fechado.
pub fn simplify(points: &[[f64; 2]], tolerance: f64, closed: bool) -> Vec<[f64; 2]> {
    if points.len() < 3
        || points.len() > 20_000
        || points.iter().flatten().any(|p| !p.is_finite())
        || !tolerance.is_finite()
        || tolerance <= 0.0
    {
        return points.to_vec();
    }
    // Iterative worklist: imported contours must not grow the call stack.
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut pending = vec![(0, points.len() - 1)];
    while let Some((start, end)) = pending.pop() {
        let a = DVec2::from_array(points[start]);
        let ab = DVec2::from_array(points[end]) - a;
        let mut best = (start, 0.0);
        for (i, point) in points.iter().enumerate().take(end).skip(start + 1) {
            let q = DVec2::from_array(*point);
            let u = if ab.length_squared() > 1e-20 {
                (q - a).dot(ab) / ab.length_squared()
            } else {
                0.0
            };
            let distance = q.distance(a + ab * u.clamp(0.0, 1.0));
            if distance > best.1 {
                best = (i, distance);
            }
        }
        if best.1 > tolerance {
            keep[best.0] = true;
            pending.push((start, best.0));
            pending.push((best.0, end));
        }
    }
    let out: Vec<_> = points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect();
    if closed && out.len() < 3 {
        points.to_vec()
    } else {
        out
    }
}
/// Arco por três pontos, amostrado na ordem a → b → c.
pub fn arc_three_points(a: [f64; 2], b: [f64; 2], c: [f64; 2], segments: usize) -> Vec<[f64; 2]> {
    let (a, b, c) = (
        DVec2::from_array(a),
        DVec2::from_array(b),
        DVec2::from_array(c),
    );
    if !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return vec![];
    }
    let u = b - a;
    let v = c - a;
    let det = 2.0 * u.perp_dot(v);
    if det.abs() < 1e-12 {
        return vec![a.to_array(), b.to_array(), c.to_array()];
    }
    let center = a + DVec2::new(
        (v.y * u.length_squared() - u.y * v.length_squared()) / det,
        (u.x * v.length_squared() - v.x * u.length_squared()) / det,
    );
    let angle = |p: DVec2| {
        let d = p - center;
        d.y.atan2(d.x)
    };
    let start = angle(a);
    let middle = (angle(b) - start).rem_euclid(std::f64::consts::TAU);
    let end = (angle(c) - start).rem_euclid(std::f64::consts::TAU);
    let sweep = if middle <= end {
        end
    } else {
        end - std::f64::consts::TAU
    };
    let r = a.distance(center);
    let n = segments.clamp(2, 512);
    (0..=n)
        .map(|i| {
            let t = start + sweep * i as f64 / n as f64;
            (center + DVec2::new(t.cos(), t.sin()) * r).to_array()
        })
        .collect()
}
/// Contornos de células opacas. Segmentos dirigidos deixam cada pixel ocupado à esquerda.
pub fn trace_alpha(
    width: u32,
    height: u32,
    rgba: &[u8],
    threshold: u8,
    tolerance: f64,
) -> Vec<Vec<[f64; 2]>> {
    use std::collections::BTreeMap;
    if width == 0
        || height == 0
        || width as u64 * height as u64 > 1024 * 1024
        || rgba.len() as u64 != width as u64 * height as u64 * 4
    {
        return vec![];
    }
    let on = |x: i32, y: i32| {
        x >= 0
            && y >= 0
            && x < width as i32
            && y < height as i32
            && rgba[((y as u32 * width + x as u32) * 4 + 3) as usize] > threshold
    };
    let mut edges: BTreeMap<(i32, i32), Vec<(i32, i32)>> = BTreeMap::new();
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            if !on(x, y) {
                continue;
            }
            for (empty, a, b) in [
                (!on(x, y - 1), (x, y), (x + 1, y)),
                (!on(x + 1, y), (x + 1, y), (x + 1, y + 1)),
                (!on(x, y + 1), (x + 1, y + 1), (x, y + 1)),
                (!on(x - 1, y), (x, y + 1), (x, y)),
            ] {
                if empty {
                    edges.entry(a).or_default().push(b);
                    if edges.len() > 20_000 {
                        return vec![];
                    }
                }
            }
        }
    }
    let mut loops = vec![];
    while let Some((&start, _)) = edges.first_key_value() {
        let mut ring = vec![];
        let mut at = start;
        loop {
            ring.push([
                at.0 as f64 - width as f64 / 2.0,
                height as f64 / 2.0 - at.1 as f64,
            ]);
            let Some(next) = edges.get_mut(&at).and_then(|v| v.pop()) else {
                edges.remove(&at);
                break;
            };
            if edges.get(&at).is_some_and(|v| v.is_empty()) {
                edges.remove(&at);
            }
            at = next;
            if at == start {
                break;
            }
        }
        if ring.len() >= 3 {
            ring.reverse();
            loops.push(simplify(&ring, tolerance, true));
        }
    }
    loops
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arc_passes_through_endpoints() {
        let p = arc_three_points([1.0, 0.0], [0.0, 1.0], [-1.0, 0.0], 16);
        assert!(DVec2::from_array(p[0]).distance(DVec2::X) < 1e-9);
        assert!(DVec2::from_array(*p.last().unwrap()).distance(-DVec2::X) < 1e-9);
    }
    #[test]
    fn trace_separate_pixels_and_hole() {
        let mut pixels = vec![255; 3 * 3 * 4];
        pixels[(4 * 4) + 3] = 0;
        let loops = trace_alpha(3, 3, &pixels, 128, 0.1);
        assert_eq!(loops.len(), 2);
    }
    #[test]
    fn round_corner_keeps_dimensions() {
        let p = rounded_rectangle(2.0, 1.0, 0.1);
        assert!(p.len() > 4);
        assert!(
            p.iter()
                .all(|p| p[0].abs() <= 1.0 + 1e-9 && p[1].abs() <= 0.5 + 1e-9)
        );
    }
}
