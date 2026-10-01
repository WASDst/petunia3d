//! Cleanup seguro do resultado booleano (cap. 04, "Política normativa de Fuse e Cut").
//!
//! O kernel devolve só triângulos: cada face do operando vira vários triângulos
//! (um quad de parede vira 2; uma tampa com furo vira um leque de dezenas). O
//! caderno permite exatamente isto sobre o resultado: **degenerados, weld de
//! pontos coincidentes e dissolução coplanar quando comprovadamente segura** — e
//! proíbe remesh/retopologia automática. Nada aqui cria pontos novos.
//!
//! # Como o cleanup é seguro
//!
//! 1. Cada triângulo do kernel é ligado à **face de origem** (do operando A ou B)
//!    em que ele está coplanar e contido. Só se dissolvem triângulos da **mesma**
//!    face de origem: as arestas que o autor desenhou (grades, loops) ficam.
//! 2. Cada região conexa dissolvida vira:
//!    - **1 contorno** → uma face (triângulo, quad ou n-gon);
//!    - **contorno + 1 furo** (anel) → faixa de quads e triângulos entre os dois
//!      laços, por programação dinâmica (como o Connect), em vez de um leque;
//!    - qualquer outro caso (≥ 2 furos, contorno que se toca) → mantém os
//!      triângulos do kernel.
//! 3. Rede de segurança: o resultado só substitui o do kernel se tiver o mesmo
//!    volume e não introduzir defeitos de topologia; senão devolve o do kernel.
//!
//! UV, material e cor são herdados da origem. Se todas as faces vêm do operando A,
//! a UV dele é preservada por mapeamento afim; senão o resultado ganha um layout
//! de charts novo (as UVs de A e B se sobreporiam).

use std::collections::{HashMap, HashSet};

use glam::{DVec3, Vec3};

use crate::{Face, Mesh, Vertex};

/// Cor de vértice de pontos novos criados pelo kernel.
const NEW_VERTEX_COLOR: [f32; 3] = [0.75, 0.75, 0.78];

/// Tolerância relativa (à diagonal da caixa) para soldar e testar coplanaridade.
const REL_TOLERANCE: f64 = 1.0e-5;

/// Como as UVs do resultado foram decididas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UvOutcome {
    /// Charts novos (o resultado mistura faces de A e B).
    #[default]
    Relaid,
    /// Herdadas por mapeamento afim da face de origem (todas vêm de A).
    Inherited,
}

/// O que o cleanup fez, para a mensagem de status e para os testes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CleanupReport {
    /// Triângulos que o kernel devolveu.
    pub kernel_triangles: usize,
    /// Faces do resultado final.
    pub faces: usize,
    pub triangles: usize,
    pub quads: usize,
    pub ngons: usize,
    /// Regiões de 2+ triângulos fundidas numa face.
    pub merged_regions: usize,
    /// Anéis (contorno + furo) preenchidos com faixa de quads/triângulos.
    pub bridged_annuli: usize,
    /// Regiões que ficaram com a triangulação do kernel.
    pub kept_regions: usize,
    pub uv: UvOutcome,
    /// A rede de segurança recusou o resultado limpo e devolveu o do kernel.
    pub fell_back: bool,
}

/// Face de origem: `side` 0 = operando A, 1 = operando B.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Origin {
    side: u8,
    face: u32,
}

struct SourceFace {
    origin: Origin,
    normal: DVec3,
    d: f64,
    min: DVec3,
    max: DVec3,
    /// Eixo descartado ao projetar o polígono em 2D.
    axis: usize,
    poly: Vec<[f64; 2]>,
}

fn drop_axis(p: DVec3, axis: usize) -> [f64; 2] {
    match axis {
        0 => [p.y, p.z],
        1 => [p.z, p.x],
        _ => [p.x, p.y],
    }
}

fn dvec(v: Vec3) -> DVec3 {
    DVec3::new(f64::from(v.x), f64::from(v.y), f64::from(v.z))
}

fn newell(points: &[DVec3]) -> DVec3 {
    let mut n = DVec3::ZERO;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        n.x += (a.y - b.y) * (a.z + b.z);
        n.y += (a.z - b.z) * (a.x + b.x);
        n.z += (a.x - b.x) * (a.y + b.y);
    }
    n
}

fn point_in_polygon(point: [f64; 2], poly: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (pi, pj) = (poly[i], poly[j]);
        if (pi[1] > point[1]) != (pj[1] > point[1])
            && point[0] < (pj[0] - pi[0]) * (point[1] - pi[1]) / (pj[1] - pi[1]) + pi[0]
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Índice espacial das faces de A e B para achar a origem de cada triângulo.
struct SourceIndex {
    faces: Vec<SourceFace>,
    cell: f64,
    grid: HashMap<(i32, i32, i32), Vec<u32>>,
    big: Vec<u32>,
    eps: f64,
}

impl SourceIndex {
    fn new(a: &Mesh, b: &Mesh, diag: f64, eps: f64) -> Self {
        let mut index = Self {
            faces: Vec::new(),
            cell: (diag / 32.0).max(1.0e-6),
            grid: HashMap::new(),
            big: Vec::new(),
            eps,
        };
        for (side, mesh) in [(0u8, a), (1u8, b)] {
            for (fi, face) in mesh.faces.iter().enumerate() {
                if face.verts.len() < 3
                    || face.verts.iter().any(|&v| v as usize >= mesh.verts.len())
                {
                    continue;
                }
                let points: Vec<DVec3> = face
                    .verts
                    .iter()
                    .map(|&v| dvec(mesh.verts[v as usize].vec()))
                    .collect();
                let raw = newell(&points);
                if raw.length_squared() < 1.0e-30 {
                    continue;
                }
                let normal = raw.normalize();
                let abs = normal.abs();
                let axis = if abs.x >= abs.y && abs.x >= abs.z {
                    0
                } else if abs.y >= abs.z {
                    1
                } else {
                    2
                };
                let (mut min, mut max) = (points[0], points[0]);
                for p in &points {
                    min = min.min(*p);
                    max = max.max(*p);
                }
                let id = index.faces.len() as u32;
                index.faces.push(SourceFace {
                    origin: Origin {
                        side,
                        face: fi as u32,
                    },
                    normal,
                    d: normal.dot(points[0]),
                    min,
                    max,
                    axis,
                    poly: points.iter().map(|p| drop_axis(*p, axis)).collect(),
                });
                index.insert(id);
            }
        }
        index
    }

    fn cell_of(&self, p: DVec3) -> (i32, i32, i32) {
        (
            (p.x / self.cell).floor() as i32,
            (p.y / self.cell).floor() as i32,
            (p.z / self.cell).floor() as i32,
        )
    }

    fn insert(&mut self, id: u32) {
        let face = &self.faces[id as usize];
        let pad = DVec3::splat(self.eps * 2.0);
        let (lo, hi) = (self.cell_of(face.min - pad), self.cell_of(face.max + pad));
        let count =
            i64::from(hi.0 - lo.0 + 1) * i64::from(hi.1 - lo.1 + 1) * i64::from(hi.2 - lo.2 + 1);
        if count > 512 {
            self.big.push(id);
            return;
        }
        for x in lo.0..=hi.0 {
            for y in lo.1..=hi.1 {
                for z in lo.2..=hi.2 {
                    self.grid.entry((x, y, z)).or_default().push(id);
                }
            }
        }
    }

    /// Face de origem coplanar em que o triângulo está contido.
    fn locate(&self, tri: [DVec3; 3]) -> Option<Origin> {
        let cross = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
        if cross.length_squared() < 1.0e-30 {
            return None;
        }
        let normal = cross.normalize();
        let centroid = (tri[0] + tri[1] + tri[2]) / 3.0;
        let mut candidates: Vec<u32> = self
            .grid
            .get(&self.cell_of(centroid))
            .into_iter()
            .flatten()
            .chain(self.big.iter())
            .copied()
            .collect();
        candidates.sort_unstable();
        candidates.dedup();
        let plane_eps = self.eps * 2.0;
        candidates.into_iter().find_map(|id| {
            let face = &self.faces[id as usize];
            if normal.dot(face.normal).abs() < 1.0 - 2.0e-4 {
                return None;
            }
            if tri
                .iter()
                .any(|p| (face.normal.dot(*p) - face.d).abs() > plane_eps)
            {
                return None;
            }
            point_in_polygon(drop_axis(centroid, face.axis), &face.poly).then_some(face.origin)
        })
    }
}

struct UnionFind {
    parent: Vec<u32>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n as u32).collect(),
        }
    }
    fn find(&mut self, mut x: u32) -> u32 {
        while self.parent[x as usize] != x {
            let grand = self.parent[self.parent[x as usize] as usize];
            self.parent[x as usize] = grand;
            x = grand;
        }
        x
    }
    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            // A menor raiz vence: o resultado independe da ordem das uniões.
            let (keep, drop) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent[drop as usize] = keep;
        }
    }
}

/// Aplica o cleanup seguro ao resultado `raw` (só triângulos) do kernel.
///
/// `a` e `b` são os operandos **originais** (com quads/n-gons): suas faces dizem
/// quais triângulos podem ser fundidos de volta.
pub fn cleanup_boolean_result(raw: &Mesh, a: &Mesh, b: &Mesh) -> (Mesh, CleanupReport) {
    let mut report = CleanupReport {
        kernel_triangles: raw.faces.len(),
        ..CleanupReport::default()
    };
    let passthrough = |report: &mut CleanupReport, mesh: &Mesh| {
        report.faces = mesh.faces.len();
        report.triangles = mesh.faces.iter().filter(|f| f.verts.len() == 3).count();
        report.quads = mesh.faces.iter().filter(|f| f.verts.len() == 4).count();
        report.ngons = mesh.faces.iter().filter(|f| f.verts.len() > 4).count();
    };
    if raw.faces.is_empty() || raw.faces.iter().any(|f| f.verts.len() != 3) {
        passthrough(&mut report, raw);
        return (raw.clone(), report);
    }

    let positions: Vec<DVec3> = raw.verts.iter().map(|v| dvec(v.vec())).collect();
    let (mut lo, mut hi) = (positions[0], positions[0]);
    for p in &positions {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let diag = (hi - lo).length().max(1.0e-6);
    let eps = diag * REL_TOLERANCE;

    // 1. Weld por grade e remoção de triângulos degenerados.
    let weld = weld_map(&positions, eps * 0.1);
    let mut tris: Vec<[u32; 3]> = Vec::with_capacity(raw.faces.len());
    for face in &raw.faces {
        let t = [
            weld[face.verts[0] as usize],
            weld[face.verts[1] as usize],
            weld[face.verts[2] as usize],
        ];
        if t[0] != t[1] && t[1] != t[2] && t[0] != t[2] {
            tris.push(t);
        }
    }
    if tris.is_empty() {
        passthrough(&mut report, raw);
        return (raw.clone(), report);
    }

    // 2. Origem de cada triângulo.
    let index = SourceIndex::new(a, b, diag, eps);
    let origins: Vec<Option<Origin>> = tris
        .iter()
        .map(|t| {
            index.locate([
                positions[t[0] as usize],
                positions[t[1] as usize],
                positions[t[2] as usize],
            ])
        })
        .collect();

    // 3. Regiões: mesma origem + aresta compartilhada por exatamente 2 triângulos.
    let mut edge_tris: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    for (ti, t) in tris.iter().enumerate() {
        for k in 0..3 {
            let (u, v) = (t[k], t[(k + 1) % 3]);
            edge_tris
                .entry((u.min(v), u.max(v)))
                .or_default()
                .push(ti as u32);
        }
    }
    let mut uf = UnionFind::new(tris.len());
    for pair in edge_tris.values() {
        if let [x, y] = pair[..]
            && let (Some(ox), Some(oy)) = (origins[x as usize], origins[y as usize])
            && ox == oy
        {
            uf.union(x, y);
        }
    }
    let mut regions: Vec<(u32, Vec<u32>)> = Vec::new();
    let mut region_of_root: HashMap<u32, usize> = HashMap::new();
    for ti in 0..tris.len() as u32 {
        let root = uf.find(ti);
        let slot = *region_of_root.entry(root).or_insert_with(|| {
            regions.push((root, Vec::new()));
            regions.len() - 1
        });
        regions[slot].1.push(ti);
    }

    // 4. Dissolução por região.
    let mut faces: Vec<(Vec<u32>, Option<Origin>)> = Vec::new();
    for (_, members) in &regions {
        let origin = origins[members[0] as usize];
        if members.len() == 1 || origin.is_none() {
            for &ti in members {
                faces.push((tris[ti as usize].to_vec(), origins[ti as usize]));
            }
            continue;
        }
        match dissolve_region(members, &tris, &positions) {
            Some(Dissolved::Polygon(poly)) => {
                report.merged_regions += 1;
                faces.push((poly, origin));
            }
            Some(Dissolved::Strip(strip)) => {
                report.merged_regions += 1;
                report.bridged_annuli += 1;
                faces.extend(strip.into_iter().map(|f| (f, origin)));
            }
            None => {
                report.kept_regions += 1;
                for &ti in members {
                    faces.push((tris[ti as usize].to_vec(), origins[ti as usize]));
                }
            }
        }
    }

    // 5. Malha compacta (só vértices usados), com cor herdada por posição.
    let colors = ColorLookup::new(a, b, eps);
    let mut remap: HashMap<u32, u32> = HashMap::new();
    let mut out = Mesh::default();
    for (verts, _) in &faces {
        for &v in verts {
            remap.entry(v).or_insert_with(|| {
                let p = raw.verts[v as usize].pos;
                out.verts.push(Vertex {
                    pos: p,
                    color: colors.color_at(p).unwrap_or(NEW_VERTEX_COLOR),
                    selected: false,
                });
                (out.verts.len() - 1) as u32
            });
        }
    }
    for (verts, _) in &faces {
        out.faces
            .push(Face::new(verts.iter().map(|v| remap[v]).collect()));
    }

    // 6. UV e material herdados da origem.
    let all_from_a = faces
        .iter()
        .all(|(_, o)| matches!(o, Some(Origin { side: 0, .. })));
    for (face, (_, origin)) in out.faces.iter_mut().zip(&faces) {
        if let Some(origin) = origin {
            let source = if origin.side == 0 { a } else { b };
            face.material_slot = source
                .faces
                .get(origin.face as usize)
                .and_then(|f| f.material_slot);
        }
    }
    if all_from_a {
        for (face, (_, origin)) in out.faces.iter_mut().zip(&faces) {
            let Some(origin) = origin else { continue };
            let Some(source_face) = a.faces.get(origin.face as usize) else {
                continue;
            };
            let uvs: Vec<[f32; 2]> = face
                .verts
                .iter()
                .map(|&v| inherited_uv(a, source_face, out.verts[v as usize].vec()))
                .collect();
            face.uv = uvs;
        }
        report.uv = UvOutcome::Inherited;
    } else {
        out.layout_uv_charts(crate::primitives::DEFAULT_UV_PADDING);
        report.uv = UvOutcome::Relaid;
    }

    // 7. Rede de segurança.
    if !safe_replacement(raw, &out) {
        report.fell_back = true;
        report.merged_regions = 0;
        report.bridged_annuli = 0;
        report.kept_regions = 0;
        passthrough(&mut report, raw);
        return (raw.clone(), report);
    }
    passthrough(&mut report, &out);
    (out, report)
}

/// Mapa vértice → representante, soldando posições a menos de `tolerance`.
fn weld_map(positions: &[DVec3], tolerance: f64) -> Vec<u32> {
    let cell = tolerance.max(1.0e-12);
    let mut buckets: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let mut map = Vec::with_capacity(positions.len());
    for (i, p) in positions.iter().enumerate() {
        let key = (
            (p.x / cell).round() as i64,
            (p.y / cell).round() as i64,
            (p.z / cell).round() as i64,
        );
        let rep = *buckets.entry(key).or_insert(i as u32);
        map.push(rep);
    }
    map
}

/// Cor de vértice por posição (A tem prioridade sobre B).
struct ColorLookup {
    cell: f32,
    colors: HashMap<(i64, i64, i64), [f32; 3]>,
}

impl ColorLookup {
    fn new(a: &Mesh, b: &Mesh, eps: f64) -> Self {
        let cell = (eps as f32).max(1.0e-7);
        let mut colors = HashMap::new();
        for mesh in [a, b] {
            for v in &mesh.verts {
                colors.entry(Self::key(cell, v.pos)).or_insert(v.color);
            }
        }
        Self { cell, colors }
    }
    fn key(cell: f32, p: [f32; 3]) -> (i64, i64, i64) {
        (
            (p[0] / cell).round() as i64,
            (p[1] / cell).round() as i64,
            (p[2] / cell).round() as i64,
        )
    }
    fn color_at(&self, p: [f32; 3]) -> Option<[f32; 3]> {
        self.colors.get(&Self::key(self.cell, p)).copied()
    }
}

/// UV de `point` (no plano de `face`) por mapeamento afim dos 3 cantos mais bem
/// condicionados da face de origem.
fn inherited_uv(mesh: &Mesh, face: &Face, point: Vec3) -> [f32; 2] {
    let n = face.verts.len();
    let pos = |i: usize| mesh.verts[face.verts[i] as usize].vec();
    let uv = |i: usize| face.uv.get(i).copied().unwrap_or([0.0, 0.0]);
    let mut best = (0, 1, 2);
    let mut best_area = -1.0f32;
    for i in 0..n {
        for j in i + 1..n {
            for k in j + 1..n {
                let area = (pos(j) - pos(i)).cross(pos(k) - pos(i)).length_squared();
                if area > best_area {
                    best_area = area;
                    best = (i, j, k);
                }
            }
        }
    }
    let (i, j, k) = best;
    let (p0, e1, e2) = (pos(i), pos(j) - pos(i), pos(k) - pos(i));
    let q = point - p0;
    let (a11, a12, a22) = (e1.dot(e1), e1.dot(e2), e2.dot(e2));
    let det = a11 * a22 - a12 * a12;
    if det.abs() < 1.0e-20 {
        return uv(i);
    }
    let (b1, b2) = (q.dot(e1), q.dot(e2));
    let s = (b1 * a22 - b2 * a12) / det;
    let t = (b2 * a11 - b1 * a12) / det;
    let (u0, u1, u2) = (uv(i), uv(j), uv(k));
    [
        u0[0] + s * (u1[0] - u0[0]) + t * (u2[0] - u0[0]),
        u0[1] + s * (u1[1] - u0[1]) + t * (u2[1] - u0[1]),
    ]
}

/// Volume com sinal (teorema da divergência) da malha triangulada.
fn signed_volume(mesh: &Mesh) -> f64 {
    let mut volume = 0.0;
    for fi in 0..mesh.faces.len() {
        let face = &mesh.faces[fi];
        for tri in mesh.face_triangle_corners(fi) {
            let p = |c: usize| dvec(mesh.verts[face.verts[tri[c]] as usize].vec());
            volume += p(0).dot(p(1).cross(p(2))) / 6.0;
        }
    }
    volume
}

fn topology_defects(mesh: &Mesh) -> usize {
    crate::HalfEdgeMesh::from_mesh(mesh).1.len()
}

/// O resultado limpo só vale se for geometricamente o mesmo sólido e não piorar a topologia.
fn safe_replacement(raw: &Mesh, cleaned: &Mesh) -> bool {
    if cleaned.faces.is_empty() {
        return false;
    }
    let (vr, vc) = (signed_volume(raw), signed_volume(cleaned));
    if !(vr.is_finite() && vc.is_finite()) || (vr - vc).abs() > vr.abs() * 1.0e-4 + 1.0e-9 {
        return false;
    }
    topology_defects(cleaned) <= topology_defects(raw)
}

enum Dissolved {
    Polygon(Vec<u32>),
    Strip(Vec<Vec<u32>>),
}

/// Contornos (externo + furos) de uma região conexa de triângulos coplanares.
fn dissolve_region(members: &[u32], tris: &[[u32; 3]], pos: &[DVec3]) -> Option<Dissolved> {
    let mut directed: HashSet<(u32, u32)> = HashSet::new();
    for &ti in members {
        let t = tris[ti as usize];
        for k in 0..3 {
            if !directed.insert((t[k], t[(k + 1) % 3])) {
                return None; // aresta duplicada: não manifold
            }
        }
    }
    let mut next: HashMap<u32, u32> = HashMap::new();
    for &(u, v) in &directed {
        if !directed.contains(&(v, u)) && next.insert(u, v).is_some() {
            return None; // vértice de contato: contorno ambíguo
        }
    }
    if next.is_empty() {
        return None;
    }
    // Laços na ordem do menor índice inicial: determinístico.
    let mut starts: Vec<u32> = next.keys().copied().collect();
    starts.sort_unstable();
    let mut visited: HashSet<u32> = HashSet::new();
    let mut loops: Vec<Vec<u32>> = Vec::new();
    for start in starts {
        if visited.contains(&start) {
            continue;
        }
        let mut ring = Vec::new();
        let mut current = start;
        loop {
            if !visited.insert(current) {
                return None;
            }
            ring.push(current);
            current = *next.get(&current)?;
            if current == start {
                break;
            }
        }
        if ring.len() < 3 {
            return None;
        }
        loops.push(ring);
    }

    let mut normal = DVec3::ZERO;
    for &ti in members {
        let t = tris[ti as usize];
        normal += (pos[t[1] as usize] - pos[t[0] as usize])
            .cross(pos[t[2] as usize] - pos[t[0] as usize]);
    }
    if normal.length_squared() < 1.0e-30 {
        return None;
    }
    let normal = normal.normalize();
    let area_of = |ring: &[u32]| -> f64 {
        let pts: Vec<DVec3> = ring.iter().map(|&v| pos[v as usize]).collect();
        newell(&pts).dot(normal) * 0.5
    };
    let mut outer: Option<usize> = None;
    let mut holes: Vec<usize> = Vec::new();
    for (i, ring) in loops.iter().enumerate() {
        if area_of(ring) > 0.0 {
            if outer.replace(i).is_some() {
                return None;
            }
        } else {
            holes.push(i);
        }
    }
    let outer = outer?;
    match holes.len() {
        0 => Some(Dissolved::Polygon(loops.swap_remove(outer))),
        1 => {
            let hole = holes[0];
            let strip = bridge_annulus(&loops[outer], &loops[hole], pos, normal)?;
            Some(Dissolved::Strip(strip))
        }
        _ => None,
    }
}

/// Custo de um triângulo na faixa (0 = equilátero; cresce com o achatamento).
fn triangle_cost(p: [[f64; 2]; 3]) -> f64 {
    let cross =
        (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]);
    if cross <= 1.0e-14 {
        return f64::INFINITY;
    }
    let sq = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2);
    let edges = sq(p[0], p[1]) + sq(p[1], p[2]) + sq(p[2], p[0]);
    0.1 + (edges / (2.0 * 3.0f64.sqrt() * cross) - 1.0).max(0.0)
}

/// Custo de um quad na faixa (0 = retângulo); só quads convexos.
fn quad_cost(p: [[f64; 2]; 4]) -> f64 {
    let mut cos_sum = 0.0;
    let (mut shortest, mut longest) = (f64::INFINITY, 0.0f64);
    for k in 0..4 {
        let (a, b, c) = (p[k], p[(k + 1) % 4], p[(k + 2) % 4]);
        let (u, v) = ([a[0] - b[0], a[1] - b[1]], [c[0] - b[0], c[1] - b[1]]);
        let cross = u[0] * v[1] - u[1] * v[0];
        let (lu, lv) = (u[0].hypot(u[1]), v[0].hypot(v[1]));
        if lu < 1.0e-12 || lv < 1.0e-12 || cross >= 0.0 {
            return f64::INFINITY; // sentido horário ou reflexo
        }
        cos_sum += ((u[0] * v[0] + u[1] * v[1]) / (lu * lv)).abs();
        shortest = shortest.min(lu);
        longest = longest.max(lu);
    }
    0.1 + 0.5 * cos_sum / 4.0 + 0.15 * (longest / shortest - 1.0).min(10.0)
}

/// Preenche o anel entre `outer` (anti-horário em torno de `normal`) e `hole`
/// (horário) com uma faixa de quads e triângulos de custo mínimo.
fn bridge_annulus(
    outer: &[u32],
    hole: &[u32],
    pos: &[DVec3],
    normal: DVec3,
) -> Option<Vec<Vec<u32>>> {
    let helper = if normal.x.abs() < 0.9 {
        DVec3::X
    } else {
        DVec3::Y
    };
    let u = normal.cross(helper).normalize();
    let v = normal.cross(u);
    let flat = |i: u32| -> [f64; 2] {
        let p = pos[i as usize];
        [p.dot(u), p.dot(v)]
    };
    let p_ring: Vec<u32> = outer.to_vec();
    let q_ring: Vec<u32> = hole.iter().rev().copied().collect();
    let (n, m) = (p_ring.len(), q_ring.len());

    // Deslocamentos candidatos de Q: todos se o custo cabe no orçamento, senão os
    // mais próximos do primeiro ponto de P.
    let mut offsets: Vec<usize> = (0..m).collect();
    if n * m * m > 4_000_000 {
        let origin = flat(p_ring[0]);
        offsets.sort_by(|&x, &y| {
            let d = |s: usize| {
                let q = flat(q_ring[s]);
                (q[0] - origin[0]).hypot(q[1] - origin[1])
            };
            d(x).total_cmp(&d(y))
        });
        offsets.truncate(8);
    }

    let mut best: Option<(f64, usize, Vec<Vec<u32>>)> = None;
    for &shift in &offsets {
        let q: Vec<u32> = (0..m).map(|j| q_ring[(j + shift) % m]).collect();
        if let Some((cost, faces)) = strip_dp(&p_ring, &q, &flat) {
            let better = best.as_ref().is_none_or(|(c, s, _)| {
                cost < *c - 1.0e-12 || (cost <= *c + 1.0e-12 && shift < *s)
            });
            if better {
                best = Some((cost, shift, faces));
            }
        }
    }
    let (_, _, faces) = best?;

    // Conferência: a faixa cobre exatamente a área do anel.
    let area = |ring: &[u32]| -> f64 {
        let pts: Vec<[f64; 2]> = ring.iter().map(|&i| flat(i)).collect();
        (0..pts.len())
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                a[0] * b[1] - a[1] * b[0]
            })
            .sum::<f64>()
            * 0.5
    };
    let expected = area(outer) + area(hole);
    let covered: f64 = faces.iter().map(|f| area(f)).sum();
    ((covered - expected).abs() <= expected.abs() * 1.0e-4 + 1.0e-12).then_some(faces)
}

/// Programação dinâmica da faixa entre os laços `p` e `q` (ambos anti-horários),
/// com `q[0]` casado a `p[0]` e fechamento cíclico.
fn strip_dp(p: &[u32], q: &[u32], flat: &impl Fn(u32) -> [f64; 2]) -> Option<(f64, Vec<Vec<u32>>)> {
    let (n, m) = (p.len(), q.len());
    let width = m + 1;
    let mut cost = vec![f64::INFINITY; (n + 1) * width];
    // 0 = vazio, 1 = triângulo sobre P, 2 = triângulo sobre Q, 3 = quad.
    let mut step = vec![0u8; (n + 1) * width];
    cost[0] = 0.0;
    let pt = |i: usize| p[i % n];
    let qt = |j: usize| q[j % m];
    for i in 0..=n {
        for j in 0..=m {
            let here = cost[i * width + j];
            if !here.is_finite() {
                continue;
            }
            let mut relax = |ni: usize, nj: usize, extra: f64, kind: u8| {
                let total = here + extra;
                let slot = ni * width + nj;
                if total < cost[slot] {
                    cost[slot] = total;
                    step[slot] = kind;
                }
            };
            if i < n {
                let c = triangle_cost([flat(pt(i)), flat(pt(i + 1)), flat(qt(j))]);
                relax(i + 1, j, c, 1);
            }
            if j < m {
                let c = triangle_cost([flat(qt(j + 1)), flat(qt(j)), flat(pt(i))]);
                relax(i, j + 1, c, 2);
            }
            if i < n && j < m {
                let c = quad_cost([flat(pt(i)), flat(pt(i + 1)), flat(qt(j + 1)), flat(qt(j))]);
                relax(i + 1, j + 1, c, 3);
            }
        }
    }
    let total = cost[n * width + m];
    if !total.is_finite() {
        return None;
    }
    let (mut i, mut j) = (n, m);
    let mut faces = Vec::new();
    while i > 0 || j > 0 {
        match step[i * width + j] {
            1 => {
                faces.push(vec![pt(i - 1), pt(i), qt(j)]);
                i -= 1;
            }
            2 => {
                faces.push(vec![qt(j), qt(j - 1), pt(i)]);
                j -= 1;
            }
            3 => {
                faces.push(vec![pt(i - 1), pt(i), qt(j), qt(j - 1)]);
                i -= 1;
                j -= 1;
            }
            _ => return None,
        }
    }
    faces.reverse();
    Some((total, faces))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boolean::{BooleanOp, boolean_meshes_clean};

    fn plate_and_cylinder(sides: u32) -> (Mesh, Mesh) {
        let plate = Mesh::box_dim(4.0, 1.0, 4.0);
        let cylinder = Mesh::cylinder(sides, 1.0, 3.0);
        (plate, cylinder)
    }

    fn counts(mesh: &Mesh) -> (usize, usize, usize) {
        let tris = mesh.faces.iter().filter(|f| f.verts.len() == 3).count();
        let quads = mesh.faces.iter().filter(|f| f.verts.len() == 4).count();
        let ngons = mesh.faces.iter().filter(|f| f.verts.len() > 4).count();
        (tris, quads, ngons)
    }

    #[test]
    fn plate_minus_cylinder_recovers_quads_and_bridges_the_caps() {
        let (plate, cylinder) = plate_and_cylinder(24);
        let (mesh, report) =
            boolean_meshes_clean(&plate, &cylinder, BooleanOp::Difference).unwrap();
        let (tris, quads, ngons) = counts(&mesh);
        assert!(!report.fell_back, "{report:?}");
        assert_eq!(report.bridged_annuli, 2, "uma faixa por tampa: {report:?}");
        // 24 quads de parede do furo + 4 laterais = 28 quads, mais os da faixa.
        assert!(quads >= 28, "quads={quads} {report:?}");
        assert!(
            mesh.faces.len() < report.kernel_triangles / 2 + 40,
            "bem menos faces que os {} triângulos do kernel (agora {})",
            report.kernel_triangles,
            mesh.faces.len()
        );
        assert_eq!(ngons, 0);
        // Sem os leques longos do kernel: só os triângulos que a faixa exige.
        assert!(tris <= 2 * 24 + 8, "tris={tris} quads={quads}");
        assert_eq!(topology_defects(&mesh), 0);
    }

    #[test]
    fn cleanup_preserves_volume_exactly() {
        let (plate, cylinder) = plate_and_cylinder(16);
        let mut ta = plate.clone();
        let mut tb = cylinder.clone();
        ta.triangulate();
        tb.triangulate();
        let raw = crate::boolean::boolean_meshes(&ta, &tb, BooleanOp::Difference).unwrap();
        let (clean, report) = cleanup_boolean_result(&raw, &plate, &cylinder);
        assert!(!report.fell_back);
        let (vr, vc) = (signed_volume(&raw), signed_volume(&clean));
        assert!((vr - vc).abs() < vr.abs() * 1.0e-5, "{vr} vs {vc}");
        // 4 × 4 × 1 menos o cilindro de raio 1 (polígono de 16 lados) dentro da placa.
        let polygon_area = 0.5 * 16.0 * 1.0 * (std::f64::consts::TAU / 16.0).sin();
        assert!((vc - (16.0 - polygon_area)).abs() < 1.0e-3, "{vc}");
    }

    #[test]
    fn union_of_overlapping_boxes_merges_the_split_faces() {
        let a = Mesh::box_dim(2.0, 2.0, 2.0);
        let mut b = Mesh::box_dim(2.0, 2.0, 2.0);
        for v in &mut b.verts {
            v.pos[0] += 1.0;
            v.pos[1] += 1.0;
        }
        let (mesh, report) = boolean_meshes_clean(&a, &b, BooleanOp::Union).unwrap();
        assert!(!report.fell_back, "{report:?}");
        let (tris, _, _) = counts(&mesh);
        assert!(
            mesh.faces.len() < report.kernel_triangles,
            "fundiu regiões coplanares: {} < {}",
            mesh.faces.len(),
            report.kernel_triangles
        );
        assert!(tris < report.kernel_triangles / 2, "tris={tris}");
        assert_eq!(topology_defects(&mesh), 0);
    }

    /// Laje 2×1×1 com a face de cima, a de baixo e as da frente/trás divididas em 2
    /// quads (a aresta do meio é "desenhada pelo autor").
    fn two_cell_slab() -> Mesh {
        let mut mesh = Mesh::default();
        for ix in 0..3 {
            for iy in 0..2 {
                for iz in 0..2 {
                    mesh.verts.push(Vertex::new(
                        -2.0 + 2.0 * ix as f32,
                        -0.5 + iy as f32,
                        -1.0 + 2.0 * iz as f32,
                    ));
                }
            }
        }
        let id = |ix: u32, iy: u32, iz: u32| ix * 4 + iy * 2 + iz;
        for ix in 0..2 {
            // topo (y=1) e fundo (y=0)
            mesh.faces.push(Face::new(vec![
                id(ix, 1, 0),
                id(ix, 1, 1),
                id(ix + 1, 1, 1),
                id(ix + 1, 1, 0),
            ]));
            mesh.faces.push(Face::new(vec![
                id(ix, 0, 0),
                id(ix + 1, 0, 0),
                id(ix + 1, 0, 1),
                id(ix, 0, 1),
            ]));
            // frente (z=1) e trás (z=0)
            mesh.faces.push(Face::new(vec![
                id(ix, 0, 1),
                id(ix + 1, 0, 1),
                id(ix + 1, 1, 1),
                id(ix, 1, 1),
            ]));
            mesh.faces.push(Face::new(vec![
                id(ix, 0, 0),
                id(ix, 1, 0),
                id(ix + 1, 1, 0),
                id(ix + 1, 0, 0),
            ]));
        }
        mesh.faces.push(Face::new(vec![
            id(0, 0, 0),
            id(0, 0, 1),
            id(0, 1, 1),
            id(0, 1, 0),
        ]));
        mesh.faces.push(Face::new(vec![
            id(2, 0, 0),
            id(2, 1, 0),
            id(2, 1, 1),
            id(2, 0, 1),
        ]));
        mesh.recalculate_normals();
        mesh
    }

    #[test]
    fn cleanup_keeps_the_authored_edges_of_the_operand() {
        // Cortar a laje com um cubo pequeno no meio: as duas metades da face de cima
        // continuam faces separadas (a aresta do autor fica), não viram uma só.
        let slab = two_cell_slab();
        let cutter = Mesh::box_dim(0.5, 3.0, 0.5);
        let (mesh, report) = boolean_meshes_clean(&slab, &cutter, BooleanOp::Difference).unwrap();
        assert!(!report.fell_back, "{report:?}");
        let center_x = |f: &Face| {
            f.verts
                .iter()
                .map(|&v| mesh.verts[v as usize].pos[0])
                .sum::<f32>()
                / f.verts.len() as f32
        };
        let top_faces: Vec<_> = mesh
            .faces
            .iter()
            .filter(|f| {
                f.verts
                    .iter()
                    .all(|&v| (mesh.verts[v as usize].pos[1] - 0.5).abs() < 1e-4)
            })
            .collect();
        assert!(
            top_faces.iter().any(|f| center_x(f) < -0.4),
            "metade esquerda"
        );
        assert!(
            top_faces.iter().any(|f| center_x(f) > 0.4),
            "metade direita"
        );
        assert!(
            !top_faces.iter().any(|f| {
                let xs: Vec<f32> = f
                    .verts
                    .iter()
                    .map(|&v| mesh.verts[v as usize].pos[0])
                    .collect();
                xs.iter().any(|&x| x < -0.4) && xs.iter().any(|&x| x > 0.4)
            }),
            "nenhuma face de cima cruza a aresta do meio de lado a lado"
        );
        assert_eq!(topology_defects(&mesh), 0);
    }

    #[test]
    fn inputs_without_origin_match_fall_back_to_kernel_triangles() {
        // Malha crua de kernel sem relação com A/B: nada é fundido, nada quebra.
        let (plate, cylinder) = plate_and_cylinder(8);
        let mut ta = plate.clone();
        ta.triangulate();
        let mut tb = cylinder.clone();
        tb.triangulate();
        let raw = crate::boolean::boolean_meshes(&ta, &tb, BooleanOp::Difference).unwrap();
        let unrelated = Mesh::cube(0.1);
        let (mesh, report) = cleanup_boolean_result(&raw, &unrelated, &unrelated);
        assert_eq!(mesh.faces.len(), raw.faces.len());
        assert_eq!(report.merged_regions, 0);
    }

    #[test]
    fn inherits_uv_when_every_face_comes_from_a() {
        // B só "morde" um canto interno de A e A continua dono de todas as faces
        // externas; o resultado de interseção com um cubo maior é o próprio A.
        let a = Mesh::box_dim(2.0, 2.0, 2.0);
        let b = Mesh::box_dim(6.0, 6.0, 6.0);
        let (mesh, report) = boolean_meshes_clean(&a, &b, BooleanOp::Intersection).unwrap();
        assert_eq!(report.uv, UvOutcome::Inherited, "{report:?}");
        assert_eq!(mesh.faces.len(), 6, "o cubo inteiro, uma face por lado");
        for (face, fi) in mesh.faces.iter().zip(0..) {
            assert!(
                face.uv.iter().any(|uv| uv != &[0.0, 0.0]),
                "face {fi} ficou sem UV"
            );
        }
    }

    #[test]
    fn mixed_origin_results_get_a_fresh_non_overlapping_layout() {
        let (plate, cylinder) = plate_and_cylinder(12);
        let (mesh, report) =
            boolean_meshes_clean(&plate, &cylinder, BooleanOp::Difference).unwrap();
        assert_eq!(report.uv, UvOutcome::Relaid);
        assert!(
            mesh.faces
                .iter()
                .flat_map(|f| f.uv.iter())
                .all(|uv| (-0.001..=1.001).contains(&uv[0]) && (-0.001..=1.001).contains(&uv[1])),
            "UVs dentro de 0..1"
        );
        assert!(
            mesh.faces
                .iter()
                .any(|f| f.uv.iter().any(|uv| uv != &[0.0, 0.0]))
        );
    }

    #[test]
    fn material_slots_follow_the_origin_faces() {
        let mut plate = Mesh::box_dim(4.0, 1.0, 4.0);
        for f in &mut plate.faces {
            f.material_slot = Some(3);
        }
        let cylinder = Mesh::cylinder(12, 1.0, 3.0);
        let (mesh, _) = boolean_meshes_clean(&plate, &cylinder, BooleanOp::Difference).unwrap();
        assert!(mesh.faces.iter().any(|f| f.material_slot == Some(3)));
    }

    #[test]
    fn vertex_colors_are_inherited_by_position() {
        let mut plate = Mesh::box_dim(4.0, 1.0, 4.0);
        for v in &mut plate.verts {
            v.color = [1.0, 0.0, 0.0];
        }
        let cylinder = Mesh::cylinder(12, 1.0, 3.0);
        let (mesh, _) = boolean_meshes_clean(&plate, &cylinder, BooleanOp::Difference).unwrap();
        assert!(mesh.verts.iter().any(|v| v.color == [1.0, 0.0, 0.0]));
    }

    #[test]
    fn cleanup_is_deterministic() {
        let (plate, cylinder) = plate_and_cylinder(20);
        let (m1, r1) = boolean_meshes_clean(&plate, &cylinder, BooleanOp::Difference).unwrap();
        let (m2, r2) = boolean_meshes_clean(&plate, &cylinder, BooleanOp::Difference).unwrap();
        assert_eq!(r1, r2);
        assert_eq!(m1.topology_fingerprint(), m2.topology_fingerprint());
    }

    #[test]
    fn annulus_strip_uses_quads_when_the_counts_match() {
        // Quadrado 8 pontos × círculo 8 pontos: a faixa é toda de quads.
        let mut pos: Vec<DVec3> = Vec::new();
        for k in 0..8 {
            let a = k as f64 / 8.0 * std::f64::consts::TAU;
            pos.push(DVec3::new(2.0 * a.cos(), 0.0, 2.0 * a.sin()));
        }
        for k in 0..8 {
            let a = k as f64 / 8.0 * std::f64::consts::TAU;
            pos.push(DVec3::new(1.0 * a.cos(), 0.0, 1.0 * a.sin()));
        }
        // Normal -Y: (cos, 0, sin) cresce no sentido horário visto de +Y.
        let normal = DVec3::NEG_Y;
        let outer: Vec<u32> = (0..8).collect();
        let hole: Vec<u32> = (8..16).rev().collect();
        let strip = bridge_annulus(&outer, &hole, &pos, normal).expect("faixa válida");
        assert_eq!(strip.len(), 8);
        assert!(strip.iter().all(|f| f.len() == 4), "{strip:?}");
    }
}
