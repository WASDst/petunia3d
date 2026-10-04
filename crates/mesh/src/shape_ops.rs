//! Operações booleanas 2D entre regiões planares (Shape Builder e Pathfinder do DRAW).
//!
//! Trabalha só com tipos do Petunia ([`Region2`]: contorno anti-horário + furos
//! horários); `geo` fica atrás desta fronteira (P0-03). Tudo é **poligonal**: as
//! curvas já chegam tesseladas pelo arranjo planar, e o resultado volta como
//! polilinhas (fidelidade de curva é trabalho de uma versão futura).

use std::collections::HashSet;

use geo::{BooleanOps, Coord, LineString, MultiLineString, MultiPolygon, Polygon};

use crate::arrangement::{Region2, signed_area};

/// Área mínima (unidades²) para uma peça sobreviver a uma operação: abaixo disso
/// é resíduo numérico da interseção, não geometria que o usuário desenhou.
const MIN_AREA: f64 = 1.0e-10;

/// Operação do Pathfinder sobre uma pilha de formas (da mais ao fundo à da frente).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathfinderOp {
    /// Une todas as formas.
    Unite,
    /// A forma mais ao fundo menos todas as da frente (Minus Front).
    Subtract,
    /// Só o que todas as formas têm em comum.
    Intersect,
    /// Remove as sobreposições (ou-exclusivo encadeado).
    Exclude,
}

fn ring(points: &[[f64; 2]]) -> LineString<f64> {
    let mut coords: Vec<Coord<f64>> = points.iter().map(|p| Coord { x: p[0], y: p[1] }).collect();
    if coords.first() != coords.last()
        && let Some(first) = coords.first().copied()
    {
        coords.push(first);
    }
    LineString::new(coords)
}

fn polygon_of(region: &Region2) -> Polygon<f64> {
    Polygon::new(
        ring(&region.outer),
        region.holes.iter().map(|h| ring(h)).collect(),
    )
}

/// Anel aberto (sem repetir o primeiro ponto) sem pontos duplicados consecutivos.
fn open_ring(line: &LineString<f64>) -> Vec<[f64; 2]> {
    let mut points: Vec<[f64; 2]> = line.coords().map(|c| [c.x, c.y]).collect();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points.dedup_by(|a, b| (a[0] - b[0]).hypot(a[1] - b[1]) < 1.0e-12);
    if points.len() > 1 {
        let (first, last) = (points[0], points[points.len() - 1]);
        if (first[0] - last[0]).hypot(first[1] - last[1]) < 1.0e-12 {
            points.pop();
        }
    }
    points
}

/// Converte o resultado do `geo` em regiões normalizadas (externo anti-horário,
/// furos horários) e descarta resíduos.
fn regions_of(multi: &MultiPolygon<f64>) -> Vec<Region2> {
    let mut out = Vec::new();
    for polygon in &multi.0 {
        let mut outer = open_ring(polygon.exterior());
        if outer.len() < 3 {
            continue;
        }
        if signed_area(&outer) < 0.0 {
            outer.reverse();
        }
        let mut holes = Vec::new();
        for interior in polygon.interiors() {
            let mut hole = open_ring(interior);
            if hole.len() < 3 {
                continue;
            }
            if signed_area(&hole) > 0.0 {
                hole.reverse();
            }
            holes.push(hole);
        }
        let region = Region2 { outer, holes };
        if region.area() > MIN_AREA {
            out.push(region);
        }
    }
    out
}

/// União de regiões. Regiões que se tocam por arestas viram uma só.
pub fn regions_union(regions: &[Region2]) -> Vec<Region2> {
    let mut iter = regions.iter();
    let Some(first) = iter.next() else {
        return Vec::new();
    };
    let mut acc = MultiPolygon::new(vec![polygon_of(first)]);
    for region in iter {
        acc = acc.union(&MultiPolygon::new(vec![polygon_of(region)]));
    }
    regions_of(&acc)
}

/// Recorta as células do arranjo ao preenchimento real das formas compostas.
pub fn occupied_regions(cells: &[Region2], shapes: &[Region2]) -> Vec<Region2> {
    let mut cover = MultiPolygon::new(vec![]);
    for shape in shapes {
        cover = cover.union(&MultiPolygon::new(vec![polygon_of(shape)]));
    }
    cells
        .iter()
        .flat_map(|cell| {
            regions_of(&MultiPolygon::new(vec![polygon_of(cell)]).intersection(&cover))
        })
        .collect()
}

/// Pathfinder preservando os furos de cada forma.
pub fn pathfinder_regions(shapes: &[Region2], op: PathfinderOp) -> Vec<Region2> {
    let Some((first, rest)) = shapes.split_first() else {
        return vec![];
    };
    let mut acc = MultiPolygon::new(vec![polygon_of(first)]);
    for shape in rest {
        let other = MultiPolygon::new(vec![polygon_of(shape)]);
        acc = match op {
            PathfinderOp::Unite => acc.union(&other),
            PathfinderOp::Subtract => acc.difference(&other),
            PathfinderOp::Intersect => acc.intersection(&other),
            PathfinderOp::Exclude => acc.xor(&other),
        };
    }
    regions_of(&acc)
}

/// Aplica uma operação do Pathfinder à pilha de formas (cada uma um anel simples).
pub fn pathfinder(shapes: &[Vec<[f64; 2]>], op: PathfinderOp) -> Vec<Region2> {
    let polygons: Vec<MultiPolygon<f64>> = shapes
        .iter()
        .filter(|s| s.len() >= 3)
        .map(|s| MultiPolygon::new(vec![Polygon::new(ring(s), vec![])]))
        .collect();
    let Some((first, rest)) = polygons.split_first() else {
        return Vec::new();
    };
    let mut acc = first.clone();
    match op {
        PathfinderOp::Unite => {
            for p in rest {
                acc = acc.union(p);
            }
        }
        PathfinderOp::Subtract => {
            for p in rest {
                acc = acc.difference(p);
            }
        }
        PathfinderOp::Intersect => {
            for p in rest {
                acc = acc.intersection(p);
            }
        }
        PathfinderOp::Exclude => {
            for p in rest {
                acc = acc.xor(p);
            }
        }
    }
    regions_of(&acc)
}

/// Laços únicos (todos anti-horários) que descrevem um conjunto de faces:
/// o contorno externo de cada uma e o laço de cada furo. Laços idênticos (o furo
/// de uma face é o contorno de outra) aparecem uma vez só.
pub fn face_loops(faces: &[Region2]) -> Vec<Vec<[f64; 2]>> {
    let mut seen: HashSet<Vec<(i64, i64)>> = HashSet::new();
    let mut loops = Vec::new();
    let mut push = |mut points: Vec<[f64; 2]>| {
        if signed_area(&points) < 0.0 {
            points.reverse();
        }
        let mut key: Vec<(i64, i64)> = points
            .iter()
            .map(|p| ((p[0] * 1.0e7).round() as i64, (p[1] * 1.0e7).round() as i64))
            .collect();
        key.sort_unstable();
        if seen.insert(key) {
            loops.push(points);
        }
    };
    for face in faces {
        push(face.outer.clone());
        for hole in &face.holes {
            push(hole.clone());
        }
    }
    loops
}

/// Keeps the pieces of an open construction path outside consumed regions.
pub fn clip_open_outside(points: &[[f64; 2]], regions: &[Region2]) -> Vec<Vec<[f64; 2]>> {
    if points.len() < 2 {
        return vec![];
    }
    let polygons = MultiPolygon(regions.iter().map(polygon_of).collect());
    let line = LineString::new(points.iter().map(|p| Coord { x: p[0], y: p[1] }).collect());
    polygons
        .clip(&MultiLineString(vec![line]), true)
        .0
        .into_iter()
        .map(|line| line.coords().map(|c| [c.x, c.y]).collect::<Vec<_>>())
        .filter(|p| p.len() >= 2)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrangement::{Polyline2, planar_regions};

    fn square(x: f64, y: f64, size: f64) -> Vec<[f64; 2]> {
        vec![[x, y], [x + size, y], [x + size, y + size], [x, y + size]]
    }

    fn closed(points: Vec<[f64; 2]>) -> Polyline2 {
        Polyline2 {
            points,
            closed: true,
        }
    }

    fn total_area(regions: &[Region2]) -> f64 {
        regions.iter().map(Region2::area).sum()
    }

    #[test]
    fn union_of_two_overlapping_squares() {
        let regions =
            planar_regions(&[closed(square(0.0, 0.0, 2.0)), closed(square(1.0, 1.0, 2.0))]);
        assert_eq!(regions.len(), 3);
        let merged = regions_union(&regions);
        assert_eq!(merged.len(), 1, "as 3 faces viram uma");
        assert!(
            (total_area(&merged) - 7.0).abs() < 1e-9,
            "{}",
            total_area(&merged)
        );
    }

    #[test]
    fn pathfinder_matches_the_set_algebra() {
        let shapes = vec![square(0.0, 0.0, 2.0), square(1.0, 1.0, 2.0)];
        let area = |op| total_area(&pathfinder(&shapes, op));
        assert!((area(PathfinderOp::Unite) - 7.0).abs() < 1e-9);
        assert!((area(PathfinderOp::Subtract) - 3.0).abs() < 1e-9);
        assert!((area(PathfinderOp::Intersect) - 1.0).abs() < 1e-9);
        assert!((area(PathfinderOp::Exclude) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn subtract_a_centered_square_leaves_a_ring_with_a_hole() {
        let shapes = vec![square(0.0, 0.0, 4.0), square(1.0, 1.0, 2.0)];
        let result = pathfinder(&shapes, PathfinderOp::Subtract);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].holes.len(), 1);
        assert!((result[0].area() - 12.0).abs() < 1e-9);
        assert!(signed_area(&result[0].outer) > 0.0, "externo anti-horário");
        assert!(signed_area(&result[0].holes[0]) < 0.0, "furo horário");
    }

    #[test]
    fn face_loops_dedupe_the_shared_hole_boundary() {
        let ring_region = Region2 {
            outer: square(0.0, 0.0, 4.0),
            holes: vec![{
                let mut h = square(1.0, 1.0, 2.0);
                h.reverse();
                h
            }],
        };
        let inner = Region2 {
            outer: square(1.0, 1.0, 2.0),
            holes: Vec::new(),
        };
        let loops = face_loops(&[ring_region, inner]);
        assert_eq!(loops.len(), 2, "o furo do anel é o contorno do miolo");
        assert!(loops.iter().all(|l| signed_area(l) > 0.0));
    }

    #[test]
    fn tiling_the_faces_reproduces_the_same_arrangement() {
        // Idempotência: re-arranjar os laços das faces devolve as mesmas faces.
        let source =
            planar_regions(&[closed(square(0.0, 0.0, 2.0)), closed(square(1.0, 1.0, 2.0))]);
        let again = planar_regions(
            &face_loops(&source)
                .into_iter()
                .map(closed)
                .collect::<Vec<_>>(),
        );
        assert_eq!(again.len(), source.len());
        assert!((total_area(&again) - total_area(&source)).abs() < 1e-9);
    }

    #[test]
    fn merging_two_faces_and_tiling_the_rest_keeps_the_union_intact() {
        let source =
            planar_regions(&[closed(square(0.0, 0.0, 2.0)), closed(square(1.0, 1.0, 2.0))]);
        // Funde as duas faces que contêm o canto (0,0) e a sobreposição; mantém a outra.
        let touched: Vec<Region2> = source
            .iter()
            .filter(|r| r.contains([0.5, 0.5]) || r.contains([1.5, 1.5]))
            .cloned()
            .collect();
        assert_eq!(touched.len(), 2);
        let merged = regions_union(&touched);
        assert_eq!(merged.len(), 1);
        let rest: Vec<Region2> = source
            .iter()
            .filter(|r| !(r.contains([0.5, 0.5]) || r.contains([1.5, 1.5])))
            .cloned()
            .collect();
        let mut faces = merged;
        faces.extend(rest);
        let rearranged = planar_regions(
            &face_loops(&faces)
                .into_iter()
                .map(closed)
                .collect::<Vec<_>>(),
        );
        assert_eq!(rearranged.len(), 2, "o quadrado fundido e a sobra");
        assert!((total_area(&rearranged) - 7.0).abs() < 1e-9);
    }

    #[test]
    fn empty_input_is_harmless() {
        assert!(regions_union(&[]).is_empty());
        assert!(pathfinder(&[], PathfinderOp::Unite).is_empty());
        assert!(face_loops(&[]).is_empty());
    }
}
