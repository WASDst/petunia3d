//! Petunia3D mesh — primitivas procedurais.

use super::{Face, Mesh, Vertex, triangulate};
use glam::Vec3;
use std::collections::{HashMap, HashSet};

/// Ponto médio soldado de uma aresta (cache por par ordenado).
fn midpoint_vertex(
    a: usize,
    b: usize,
    verts: &mut Vec<[f32; 3]>,
    cache: &mut HashMap<(usize, usize), usize>,
) -> usize {
    let key = (a.min(b), a.max(b));
    if let Some(&idx) = cache.get(&key) {
        return idx;
    }
    let mid = [
        (verts[a][0] + verts[b][0]) * 0.5,
        (verts[a][1] + verts[b][1]) * 0.5,
        (verts[a][2] + verts[b][2]) * 0.5,
    ];
    verts.push(mid);
    let idx = verts.len() - 1;
    cache.insert(key, idx);
    idx
}

/// Auditoria geométrica de uma primitiva gerada (Wave P1 — §28).
#[derive(Debug, Clone, PartialEq)]
pub struct PrimitiveAudit {
    pub verts: usize,
    pub faces: usize,
    pub tris: usize,
    pub all_finite: bool,
    pub indices_valid: bool,
    pub degenerate_faces: usize,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    /// Faces com normal apontando para dentro (só vale para formas estrela).
    pub outward_violations: usize,
    /// Pares de vértices com posição bit-idêntica.
    pub coincident_verts: usize,
}

/// Inspeciona a malha sem reparar: posições finitas, índices válidos, áreas,
///
/// bordas finitas, orientação para fora (formas estrela) e duplicatas exatas.
pub fn primitive_audit(mesh: &Mesh) -> PrimitiveAudit {
    let mut all_finite = true;
    for v in &mesh.verts {
        if !v.pos.iter().all(|x| x.is_finite()) || !v.color.iter().all(|x| x.is_finite()) {
            all_finite = false;
        }
    }
    let mut indices_valid = true;
    for f in &mesh.faces {
        if f.verts.len() < 3
            || f.uv.len() != f.verts.len()
            || f.verts.iter().any(|&i| (i as usize) >= mesh.verts.len())
        {
            indices_valid = false;
        }
    }
    let center = if mesh.verts.is_empty() {
        Vec3::ZERO
    } else {
        mesh.verts.iter().map(|v| v.vec()).sum::<Vec3>() / mesh.verts.len() as f32
    };
    let mut tris = 0usize;
    let mut degenerate_faces = 0usize;
    let mut outward_violations = 0usize;
    for (fi, f) in mesh.faces.iter().enumerate() {
        let n = f.verts.len();
        if n < 3 {
            continue;
        }
        let mut face_degenerate = false;
        for i in 1..(n - 1) {
            let a = mesh.verts.get(f.verts[0] as usize);
            let b = mesh.verts.get(f.verts[i] as usize);
            let c = mesh.verts.get(f.verts[i + 1] as usize);
            let (Some(a), Some(b), Some(c)) = (a, b, c) else {
                continue;
            };
            tris += 1;
            // Tolerância exata (não relativa): malhas minúsculas válidas têm
            // área pequena; degenerada é área nula (vértices duplicados).
            let area2 = (b.vec() - a.vec()).cross(c.vec() - a.vec()).length();
            if area2 <= 0.0 {
                face_degenerate = true;
            }
        }
        if face_degenerate {
            degenerate_faces += 1;
        }
        // Orientação: normal da face deve apontar para fora do centroide.
        let normal = mesh.face_normal(fi);
        if normal.length() > 1e-9 {
            let centroid = f.verts.iter().fold(Vec3::ZERO, |acc, &vi| {
                acc + mesh
                    .verts
                    .get(vi as usize)
                    .map(|v| v.vec())
                    .unwrap_or(Vec3::ZERO)
            }) / n.max(1) as f32;
            if normal.dot(centroid - center) < -1e-6 {
                outward_violations += 1;
            }
        }
    }
    let mut bounds_min = [f32::MAX; 3];
    let mut bounds_max = [f32::MIN; 3];
    for v in &mesh.verts {
        for k in 0..3 {
            bounds_min[k] = bounds_min[k].min(v.pos[k]);
            bounds_max[k] = bounds_max[k].max(v.pos[k]);
        }
    }
    let mut coincident_verts = 0usize;
    for (i, a) in mesh.verts.iter().enumerate() {
        for b in mesh.verts.iter().skip(i + 1) {
            if a.pos == b.pos {
                coincident_verts += 1;
            }
        }
    }
    PrimitiveAudit {
        verts: mesh.verts.len(),
        faces: mesh.faces.len(),
        tris,
        all_finite,
        indices_valid,
        degenerate_faces,
        bounds_min,
        bounds_max,
        outward_violations,
        coincident_verts,
    }
}

/// Preenchimento do círculo (§16): anel aberto ou leque soldado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CircleFill {
    None,
    Disc,
}

/// Parâmetros de criação por primitiva (regeneração determinística).
///
/// Defaults low-poly (§5). Limites (§44): lados 3–32, anéis 2–24, subdivisão ico 0–3,
/// torus 3–64/3–32, raios e alturas > 0 (clamp).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PrimitiveDescriptor {
    Box {
        width: f32,
        height: f32,
        depth: f32,
    },
    Plane {
        width: f32,
        height: f32,
    },
    Wedge {
        width: f32,
        height: f32,
        depth: f32,
    },
    Cylinder {
        radius: f32,
        height: f32,
        sides: u32,
        cap_top: bool,
        cap_bottom: bool,
    },
    Cone {
        bottom_radius: f32,
        top_radius: f32,
        height: f32,
        sides: u32,
        cap_bottom: bool,
        cap_top: bool,
    },
    Circle {
        radius: f32,
        vertices: u32,
        fill: CircleFill,
    },
    Torus {
        major_radius: f32,
        minor_radius: f32,
        major_segments: u32,
        minor_segments: u32,
    },
    LowSphere {
        radius: f32,
        segments: u32,
        rings: u32,
    },
    Icosphere {
        radius: f32,
        subdiv: u32,
    },
    Capsule {
        radius: f32,
        height: f32,
        radial_segments: u32,
        cap_segments: u32,
    },
}

impl PrimitiveDescriptor {
    /// Identificador canônico da espécie ("box", "cylinder", "sphere", etc.).
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Box { .. } => "box",
            Self::Plane { .. } => "plane",
            Self::Wedge { .. } => "wedge",
            Self::Cylinder { .. } => "cylinder",
            Self::Cone { .. } => "cone",
            Self::Circle { .. } => "circle",
            Self::Torus { .. } => "torus",
            Self::LowSphere { .. } => "sphere",
            Self::Icosphere { .. } => "icosphere",
            Self::Capsule { .. } => "capsule",
        }
    }

    /// Constrói a malha local (sem offset) a partir dos parâmetros.
    pub fn build(self) -> Mesh {
        match self {
            Self::Box {
                width,
                height,
                depth,
            } => Mesh::box_dim(width, height, depth),
            Self::Plane { width, height } => {
                let mut mesh = Mesh::plane(1.0);
                for v in &mut mesh.verts {
                    v.pos[0] *= width.clamp(0.05, 100.0);
                    v.pos[2] *= height.clamp(0.05, 100.0);
                }
                mesh
            }
            Self::Wedge {
                width,
                height,
                depth,
            } => Mesh::wedge(width, height, depth),
            Self::Cylinder {
                radius,
                height,
                sides,
                cap_top,
                cap_bottom,
            } => Mesh::radial_frustum(
                radius.clamp(0.05, 100.0),
                radius.clamp(0.05, 100.0),
                height.clamp(0.05, 100.0),
                sides.clamp(3, 32),
                cap_bottom,
                cap_top,
            ),
            Self::Cone {
                bottom_radius,
                top_radius,
                height,
                sides,
                cap_bottom,
                cap_top,
            } => Mesh::radial_frustum(
                bottom_radius.clamp(0.0, 100.0),
                top_radius.clamp(0.0, 100.0),
                height.clamp(0.05, 100.0),
                sides.clamp(3, 32),
                cap_bottom,
                cap_top,
            ),
            Self::Circle {
                radius,
                vertices,
                fill,
            } => Mesh::circle(
                radius.clamp(0.05, 100.0),
                vertices.clamp(3, 64),
                fill == CircleFill::Disc,
            ),
            Self::Torus {
                major_radius,
                minor_radius,
                major_segments,
                minor_segments,
            } => Mesh::torus(
                major_radius.clamp(0.05, 100.0),
                minor_radius.clamp(0.01, 100.0),
                major_segments.clamp(3, 64),
                minor_segments.clamp(3, 32),
            ),
            Self::LowSphere {
                radius,
                segments,
                rings,
            } => Mesh::sphere_low(
                segments.clamp(3, 32),
                rings.clamp(2, 24),
                radius.clamp(0.05, 100.0),
            ),
            Self::Icosphere { radius, subdiv } => {
                Mesh::icosphere(radius.clamp(0.05, 100.0), subdiv.min(3))
            }
            Self::Capsule {
                radius,
                height,
                radial_segments,
                cap_segments,
            } => Mesh::capsule_profile(
                radial_segments.clamp(3, 32),
                radius.clamp(0.05, 100.0),
                height.clamp(0.05, 100.0),
                cap_segments.clamp(1, 8),
            ),
        }
    }
}

impl Mesh {
    // ---------------- primitivas ----------------

    pub fn cube(size: f32) -> Self {
        let h = size.max(0.05) * 0.5;
        let mut m = Self {
            verts: vec![
                Vertex::new(-h, -h, -h),
                Vertex::new(h, -h, -h),
                Vertex::new(h, h, -h),
                Vertex::new(-h, h, -h),
                Vertex::new(-h, -h, h),
                Vertex::new(h, -h, h),
                Vertex::new(h, h, h),
                Vertex::new(-h, h, h),
            ],
            faces: vec![
                Face::new(vec![0, 3, 2, 1]),
                Face::new(vec![4, 5, 6, 7]),
                Face::new(vec![0, 1, 5, 4]),
                Face::new(vec![2, 3, 7, 6]),
                Face::new(vec![0, 4, 7, 3]),
                Face::new(vec![1, 2, 6, 5]),
            ],
            selected_edges: HashSet::new(),
            uv_seams: HashSet::new(),
            uv_pinned: HashSet::new(),
        };
        m.project_planar();
        m
    }

    pub fn plane(size: f32) -> Self {
        let h = size.max(0.05) * 0.5;
        let mut m = Self {
            verts: vec![
                Vertex::new(-h, 0.0, -h),
                Vertex::new(h, 0.0, -h),
                Vertex::new(h, 0.0, h),
                Vertex::new(-h, 0.0, h),
            ],
            faces: vec![Face::with_uv(
                vec![0, 3, 2, 1],
                vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            )],
            selected_edges: HashSet::new(),
            uv_seams: HashSet::new(),
            uv_pinned: HashSet::new(),
        };
        m.project_planar();
        m
    }

    /// Gerador radial/frustum comum (Wave P1 — §9): um tronco de cone com raios
    /// independentes e tampas opcionais. Cylinder = raios iguais; Cone = topo 0;
    /// frustum = topo intermediário. Topologia soldada, sem vértices duplicados.
    pub fn radial_frustum(
        bottom_radius: f32,
        top_radius: f32,
        height: f32,
        sides: u32,
        cap_bottom: bool,
        cap_top: bool,
    ) -> Self {
        let seg = (sides as usize).clamp(3, 32);
        let br = bottom_radius.max(0.0);
        let tr = top_radius.max(0.0);
        let h = (height * 0.5).max(1e-6);
        let mut m = Mesh::default();
        // Anel inferior (sempre com área; raio 0 vira ápice soldado).
        let bottom_apex = br < 1e-6;
        let top_apex = tr < 1e-6;
        let ring = |r: f32, y: f32, out: &mut Mesh| -> Vec<u32> {
            if r < 1e-6 {
                out.verts.push(Vertex::new(0.0, y, 0.0));
                vec![(out.verts.len() - 1) as u32]
            } else {
                let base = out.verts.len() as u32;
                for i in 0..seg {
                    let a = (i as f32 / seg as f32) * std::f32::consts::TAU;
                    out.verts.push(Vertex::new(a.cos() * r, y, a.sin() * r));
                }
                (base..base + seg as u32).collect()
            }
        };
        let lower = ring(br, -h, &mut m);
        let upper = ring(tr, h, &mut m);
        // Laterais (mesma convenção de winding do `cylinder` original: para fora).
        if lower.len() == 1 && upper.len() == 1 {
            // Segmento degenerado no eixo: nada a costurar.
        } else if lower.len() == 1 {
            for i in 0..seg {
                let j = (i + 1) % seg;
                m.push_face(Face::new(vec![lower[0], upper[i], upper[j]]));
            }
        } else if upper.len() == 1 {
            for i in 0..seg {
                let j = (i + 1) % seg;
                m.push_face(Face::new(vec![lower[i], upper[0], lower[j]]));
            }
        } else {
            for i in 0..seg {
                let j = (i + 1) % seg;
                m.push_face(Face::new(vec![lower[i], upper[i], upper[j], lower[j]]));
            }
        }
        // Tampas com centro único soldado (fundo −Y, topo +Y).
        if cap_bottom && !bottom_apex {
            let cb = m.verts.len() as u32;
            m.verts.push(Vertex::new(0.0, -h, 0.0));
            for i in 0..seg {
                let j = (i + 1) % seg;
                m.push_face(Face::new(vec![cb, lower[i], lower[j]]));
            }
        }
        if cap_top && !top_apex {
            let ct = m.verts.len() as u32;
            m.verts.push(Vertex::new(0.0, h, 0.0));
            for i in 0..seg {
                let j = (i + 1) % seg;
                m.push_face(Face::new(vec![ct, upper[j], upper[i]]));
            }
        }
        m.project_planar();
        m
    }

    pub fn cylinder(segments: u32, radius: f32, height: f32) -> Self {
        Self::radial_frustum(radius, radius, height, segments, true, true)
    }

    pub fn cone(segments: u32, radius: f32, height: f32) -> Self {
        Self::radial_frustum(radius, 0.0, height, segments, true, false)
    }

    /// Caixa com dimensões independentes, centrada na origem.
    pub fn box_dim(width: f32, height: f32, depth: f32) -> Self {
        let (w, h, d) = (
            (width * 0.5).max(1e-6),
            (height * 0.5).max(1e-6),
            (depth * 0.5).max(1e-6),
        );
        let mut m = Self {
            verts: vec![
                Vertex::new(-w, -h, -d),
                Vertex::new(w, -h, -d),
                Vertex::new(w, h, -d),
                Vertex::new(-w, h, -d),
                Vertex::new(-w, -h, d),
                Vertex::new(w, -h, d),
                Vertex::new(w, h, d),
                Vertex::new(-w, h, d),
            ],
            faces: vec![
                Face::new(vec![0, 3, 2, 1]),
                Face::new(vec![4, 5, 6, 7]),
                Face::new(vec![0, 1, 5, 4]),
                Face::new(vec![2, 3, 7, 6]),
                Face::new(vec![0, 4, 7, 3]),
                Face::new(vec![1, 2, 6, 5]),
            ],
            selected_edges: HashSet::new(),
            uv_seams: HashSet::new(),
            uv_pinned: HashSet::new(),
        };
        m.project_planar();
        m
    }

    /// Cunha/rampa: caixa com o topo inclinado (alto atrás, zero na frente).
    /// 6 vértices, 5 faces (fundo, trás, rampa, 2 laterais triangulares).
    pub fn wedge(width: f32, height: f32, depth: f32) -> Self {
        let (w, h, d) = (
            (width * 0.5).max(1e-6),
            (height * 0.5).max(1e-6),
            (depth * 0.5).max(1e-6),
        );
        let mut m = Self {
            verts: vec![
                Vertex::new(-w, -h, -d), // 0 fundo-trás-esq
                Vertex::new(w, -h, -d),  // 1 fundo-trás-dir
                Vertex::new(-w, -h, d),  // 2 fundo-frente-esq
                Vertex::new(w, -h, d),   // 3 fundo-frente-dir
                Vertex::new(-w, h, -d),  // 4 topo-trás-esq
                Vertex::new(w, h, -d),   // 5 topo-trás-dir
            ],
            faces: vec![
                Face::new(vec![0, 1, 3, 2]), // fundo (−Y)
                Face::new(vec![0, 4, 5, 1]), // trás (−Z)
                Face::new(vec![4, 2, 3, 5]), // rampa
                Face::new(vec![0, 2, 4]),    // lateral esq
                Face::new(vec![1, 5, 3]),    // lateral dir
            ],
            selected_edges: HashSet::new(),
            uv_seams: HashSet::new(),
            uv_pinned: HashSet::new(),
        };
        m.project_planar();
        m
    }

    /// Círculo/disco no plano XZ voltado a +Y. `fill=false` = ngono plano;
    /// `fill=true` = leque de triângulos com centro soldado.
    pub fn circle(radius: f32, vertices: u32, fill: bool) -> Self {
        let n = (vertices as usize).clamp(3, 64);
        let r = radius.max(1e-6);
        let mut m = Mesh::default();
        if fill {
            let center = m.verts.len() as u32;
            m.verts.push(Vertex::new(0.0, 0.0, 0.0));
            let base = m.verts.len() as u32;
            for i in 0..n {
                let a = (i as f32 / n as f32) * std::f32::consts::TAU;
                m.verts.push(Vertex::new(a.cos() * r, 0.0, a.sin() * r));
            }
            for i in 0..n {
                let j = (i + 1) % n;
                m.push_face(Face::new(vec![center, base + j as u32, base + i as u32]));
            }
        } else {
            let base = m.verts.len() as u32;
            for i in 0..n {
                let a = (i as f32 / n as f32) * std::f32::consts::TAU;
                m.verts.push(Vertex::new(a.cos() * r, 0.0, a.sin() * r));
            }
            m.push_face(Face::new((base..base + n as u32).collect()));
        }
        m.project_planar();
        m
    }

    /// Toro centrado na origem, eixo Y. Malha fechada sem costura duplicada
    /// (índices com wrap-around).
    pub fn torus(
        major_radius: f32,
        minor_radius: f32,
        major_segments: u32,
        minor_segments: u32,
    ) -> Self {
        let major = major_radius.max(1e-6);
        let minor = minor_radius.clamp(1e-9, (major * 0.99).max(1e-9));
        let big = (major_segments as usize).clamp(3, 64);
        let small = (minor_segments as usize).clamp(3, 32);
        let mut m = Mesh::default();
        for i in 0..big {
            let u = (i as f32 / big as f32) * std::f32::consts::TAU;
            for j in 0..small {
                let v = (j as f32 / small as f32) * std::f32::consts::TAU;
                m.verts.push(Vertex::new(
                    (major + minor * v.cos()) * u.cos(),
                    minor * v.sin(),
                    (major + minor * v.cos()) * u.sin(),
                ));
            }
        }
        let at = |i: usize, j: usize| ((i % big) * small + (j % small)) as u32;
        for i in 0..big {
            for j in 0..small {
                m.push_face(Face::new(vec![
                    at(i, j),
                    at(i, j + 1),
                    at(i + 1, j + 1),
                    at(i + 1, j),
                ]));
            }
        }
        m.project_planar();
        m
    }

    /// Icosfera por subdivisão de icosaedro (vértices soldados via cache de
    /// aresta). `subdiv` 0..=3 (3 = 1280 tris; acima disso, escolha deliberada).
    pub fn icosphere(radius: f32, subdiv: u32) -> Self {
        let radius = radius.max(1e-6);
        let levels = subdiv.min(3) as usize;
        let t = (1.0 + 5.0_f32.sqrt()) / 2.0;
        let mut verts = vec![
            [-1.0, t, 0.0],
            [1.0, t, 0.0],
            [-1.0, -t, 0.0],
            [1.0, -t, 0.0],
            [0.0, -1.0, t],
            [0.0, 1.0, t],
            [0.0, -1.0, -t],
            [0.0, 1.0, -t],
            [t, 0.0, -1.0],
            [t, 0.0, 1.0],
            [-t, 0.0, -1.0],
            [-t, 0.0, 1.0],
        ];
        let mut faces = vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];
        let mut edge_cache: std::collections::HashMap<(usize, usize), usize> =
            std::collections::HashMap::new();
        for _ in 0..levels {
            edge_cache.clear();
            let mut next = Vec::with_capacity(faces.len() * 4);
            for [a, b, c] in faces.drain(..) {
                let ab = midpoint_vertex(a, b, &mut verts, &mut edge_cache);
                let bc = midpoint_vertex(b, c, &mut verts, &mut edge_cache);
                let ca = midpoint_vertex(c, a, &mut verts, &mut edge_cache);
                next.push([a, ab, ca]);
                next.push([b, bc, ab]);
                next.push([c, ca, bc]);
                next.push([ab, bc, ca]);
            }
            faces = next;
        }
        let mut m = Mesh::default();
        for v in &verts {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
            m.verts.push(Vertex::new(
                v[0] / len * radius,
                v[1] / len * radius,
                v[2] / len * radius,
            ));
        }
        for [a, b, c] in faces {
            m.push_face(Face::new(vec![a as u32, b as u32, c as u32]));
        }
        m.project_planar();
        m
    }

    pub fn sphere_low(segments: u32, rings: u32, radius: f32) -> Self {
        let seg = segments.clamp(3, 32) as usize;
        let rg = rings.clamp(2, 24) as usize;
        let radius = radius.max(1e-6);
        let mut m = Mesh::default();
        // Polos soldados (vértice único): sem coincidentes, leque limpo.
        let south = m.verts.len() as u32;
        m.verts.push(Vertex::new(0.0, -radius, 0.0));
        for r in 1..rg {
            let phi = (r as f32 / rg as f32) * std::f32::consts::PI;
            for s in 0..seg {
                let theta = (s as f32 / seg as f32) * std::f32::consts::TAU;
                m.verts.push(Vertex::new(
                    radius * phi.sin() * theta.cos(),
                    radius * phi.cos(),
                    radius * phi.sin() * theta.sin(),
                ));
            }
        }
        let north = m.verts.len() as u32;
        m.verts.push(Vertex::new(0.0, radius, 0.0));
        // Anéis: ring(1) junto ao norte … ring(rg-1) junto ao sul.
        let ring = |r: usize| 1 + (r - 1) * seg;
        for s in 0..seg {
            let j = (s + 1) % seg;
            m.push_face(Face::new(vec![
                south,
                (ring(rg - 1) + s) as u32,
                (ring(rg - 1) + j) as u32,
            ]));
        }
        for r in 1..rg - 1 {
            for s in 0..seg {
                let j = (s + 1) % seg;
                m.push_face(Face::new(vec![
                    (ring(r + 1) + s) as u32,
                    (ring(r) + s) as u32,
                    (ring(r) + j) as u32,
                    (ring(r + 1) + j) as u32,
                ]));
            }
        }
        for s in 0..seg {
            let j = (s + 1) % seg;
            m.push_face(Face::new(vec![
                (ring(1) + s) as u32,
                north,
                (ring(1) + j) as u32,
            ]));
        }
        m.project_planar();
        m
    }

    /// Cápsula low-poly: corpo cilíndrico + calotas hemisféricas com polos
    /// soldados e anéis de calota parametrizáveis (perfil cosseno).
    pub fn capsule_profile(segments: u32, radius: f32, height: f32, cap_rings: u32) -> Self {
        let seg = (segments as usize).clamp(3, 32);
        let radius = radius.max(1e-6);
        let caps = (cap_rings as usize).clamp(1, 8);
        let body = (height * 0.5 - radius).max(0.0);
        let mut m = Mesh::default();
        let south = m.verts.len() as u32;
        m.verts.push(Vertex::new(0.0, -body - radius, 0.0));
        // Anéis da calota sul (do polo ao equador), corpo, calota norte.
        let mut ring_of: Vec<Vec<u32>> = Vec::new();
        for c in 0..caps {
            let t = ((c + 1) as f32) / ((caps + 1) as f32);
            let ang = -std::f32::consts::FRAC_PI_2 + t * std::f32::consts::FRAC_PI_2;
            let y = -body + radius * ang.sin();
            let r = (radius * ang.cos()).max(0.0);
            let base = m.verts.len() as u32;
            for s in 0..seg {
                let a = (s as f32 / seg as f32) * std::f32::consts::TAU;
                m.verts.push(Vertex::new(a.cos() * r, y, a.sin() * r));
            }
            ring_of.push((base..base + seg as u32).collect());
        }
        let body_lo = m.verts.len() as u32;
        for s in 0..seg {
            let a = (s as f32 / seg as f32) * std::f32::consts::TAU;
            m.verts
                .push(Vertex::new(a.cos() * radius, -body, a.sin() * radius));
        }
        // Corpo zero: não aloca o segundo anel (evita duplicatas exatas).
        let body_hi = if body > 1e-9 {
            let base = m.verts.len() as u32;
            for s in 0..seg {
                let a = (s as f32 / seg as f32) * std::f32::consts::TAU;
                m.verts
                    .push(Vertex::new(a.cos() * radius, body, a.sin() * radius));
            }
            Some(base)
        } else {
            None
        };
        for c in 0..caps {
            let t = ((c + 1) as f32) / ((caps + 1) as f32);
            let ang = t * std::f32::consts::FRAC_PI_2;
            let y = body + radius * ang.sin();
            let r = (radius * ang.cos()).max(0.0);
            let base = m.verts.len() as u32;
            for s in 0..seg {
                let a = (s as f32 / seg as f32) * std::f32::consts::TAU;
                m.verts.push(Vertex::new(a.cos() * r, y, a.sin() * r));
            }
            ring_of.push((base..base + seg as u32).collect());
        }
        let north = m.verts.len() as u32;
        m.verts.push(Vertex::new(0.0, body + radius, 0.0));

        // Costura de baixo para cima: polo sul, calotas, corpo, polo norte.
        // Corpo zero (altura == diâmetro) funde os dois anéis do corpo.
        let mut bands: Vec<Vec<u32>> = Vec::new();
        bands.push(vec![south]);
        bands.extend(ring_of.iter().take(caps).cloned());
        bands.push((body_lo..body_lo + seg as u32).collect());
        if let Some(base) = body_hi {
            bands.push((base..base + seg as u32).collect());
        }
        bands.extend(ring_of.iter().skip(caps).cloned());
        bands.push(vec![north]);
        for w in 0..bands.len() - 1 {
            let (lo, hi) = (&bands[w], &bands[w + 1]);
            if lo.len() == 1 && hi.len() == 1 {
                continue;
            } else if lo.len() == 1 {
                for s in 0..seg {
                    let j = (s + 1) % seg;
                    m.push_face(Face::new(vec![lo[0], hi[s], hi[j]]));
                }
            } else if hi.len() == 1 {
                for s in 0..seg {
                    let j = (s + 1) % seg;
                    m.push_face(Face::new(vec![lo[s], hi[0], lo[j]]));
                }
            } else {
                for s in 0..seg {
                    let j = (s + 1) % seg;
                    m.push_face(Face::new(vec![lo[s], hi[s], hi[j], lo[j]]));
                }
            }
        }
        m.project_planar();
        m
    }

    /// Cápsula com perfil padrão (2 anéis de calota — forma clássica).
    pub fn capsule(segments: u32, radius: f32, height: f32) -> Self {
        Self::capsule_profile(segments, radius, height, 2)
    }

    /// Revolve (torno): perfil `[[raio, altura], ..]` girado em torno do eixo Y.
    /// Raio é clampado em >= 0. Fecha polos quando o perfil encosta no eixo.
    pub fn revolve(profile: &[[f32; 2]], segments: u32) -> Result<Self, String> {
        Self::revolve_angle(profile, segments, 360.0)
    }

    /// Revolução em torno do eixo Y por um ângulo configurável em graus (ex.: 90°, 180°, 360°).
    pub fn revolve_angle(
        profile: &[[f32; 2]],
        segments: u32,
        angle_degrees: f32,
    ) -> Result<Self, String> {
        if profile.len() < 2 {
            return Err("perfil precisa de ao menos 2 pontos".to_string());
        }
        let seg = segments.clamp(3, 64) as usize;
        let angle_deg = if angle_degrees.is_finite() && angle_degrees > 0.0 {
            angle_degrees.min(360.0)
        } else {
            360.0
        };
        let is_full_circle = (angle_deg - 360.0).abs() < 1e-3;
        let angle_rad = angle_deg.to_radians();

        let mut m = Mesh::default();
        let num_steps = if is_full_circle { seg } else { seg + 1 };

        // anel por ponto do perfil
        for &[r, y] in profile {
            let r = r.max(0.0);
            for s in 0..num_steps {
                let a = if is_full_circle {
                    (s as f32 / seg as f32) * std::f32::consts::TAU
                } else {
                    (s as f32 / seg as f32) * angle_rad
                };
                m.verts.push(Vertex::new(a.cos() * r, y, a.sin() * r));
            }
        }
        let axis = |k: usize| profile[k][0].max(0.0) < 1e-6;
        for k in 0..profile.len() - 1 {
            if axis(k) && axis(k + 1) {
                continue; // segmento degenerado no eixo
            }
            for s in 0..seg {
                let a = (k * num_steps + s) as u32;
                let b = if is_full_circle {
                    (k * num_steps + (s + 1) % seg) as u32
                } else {
                    (k * num_steps + s + 1) as u32
                };
                let c = if is_full_circle {
                    ((k + 1) * num_steps + (s + 1) % seg) as u32
                } else {
                    ((k + 1) * num_steps + s + 1) as u32
                };
                let d = ((k + 1) * num_steps + s) as u32;
                if axis(k) {
                    m.push_face(Face::new(vec![a, d, c]));
                } else if axis(k + 1) {
                    m.push_face(Face::new(vec![a, d, b]));
                } else {
                    m.push_face(Face::new(vec![a, d, c, b]));
                }
            }
        }
        // Se a revolução for parcial (setor angular aberto), adiciona tampas nas extremidades
        if !is_full_circle && profile.len() >= 3 {
            let start_verts: Vec<u32> =
                (0..profile.len()).map(|k| (k * num_steps) as u32).collect();
            let end_verts: Vec<u32> = (0..profile.len())
                .map(|k| (k * num_steps + seg) as u32)
                .rev()
                .collect();
            m.push_face(Face::new(start_verts));
            m.push_face(Face::new(end_verts));
        }
        m.project_planar();
        Ok(m)
    }

    /// Malha a partir de polígono 2D extrudado em +Z.
    /// Gera quads quando o polígono tem 4 pontos (ex.: retângulos), triângulos para 3 pontos,
    /// e n-gons para > 4 pontos no authoring, sem granular em triângulos desnecessários.
    pub fn from_polygon(points: &[[f32; 2]], depth: f32) -> Result<Self, String> {
        let n = points.len();
        if n < 3 {
            return Err("polígono precisa de ao menos 3 pontos".to_string());
        }
        let area = triangulate::polygon_area(points);
        if area.abs() < 1e-9 {
            return Err("área do polígono degenerada".to_string());
        }
        // Garante orientação CCW para o perfil no plano XY
        let mut pts = points.to_vec();
        if area < 0.0 {
            pts.reverse();
        }
        let mut m = Mesh::default();
        for &[x, y] in &pts {
            m.verts.push(Vertex::new(x, y, 0.0));
        }
        let d = depth.max(0.01);
        for &[x, y] in &pts {
            m.verts.push(Vertex::new(x, y, d));
        }
        let off = n as u32;

        if n == 4 {
            // base: normal -Z (ordem horária: 3, 2, 1, 0) -> QUAD
            m.push_face(Face::new(vec![3, 2, 1, 0]));
            // tampa: normal +Z (ordem anti-horária: off, off+1, off+2, off+3) -> QUAD
            m.push_face(Face::new(vec![off, off + 1, off + 2, off + 3]));
        } else if n == 3 {
            m.push_face(Face::new(vec![2, 1, 0]));
            m.push_face(Face::new(vec![off, off + 1, off + 2]));
        } else {
            // N-gon caps no authoring (conforme bible 03)
            let base_verts: Vec<u32> = (0..n as u32).rev().collect();
            m.push_face(Face::new(base_verts));
            let top_verts: Vec<u32> = (off..off + n as u32).collect();
            m.push_face(Face::new(top_verts));
        }

        // laterais: segue a ordem do polígono (todos quads)
        for k in 0..n {
            let k2 = (k + 1) % n;
            m.push_face(Face::new(vec![
                k as u32,
                k2 as u32,
                off + k2 as u32,
                off + k as u32,
            ]));
        }
        m.project_planar();
        Ok(m)
    }
}

#[cfg(test)]
mod primitive_tests {
    use super::{Mesh, primitive_audit};

    fn assert_clean(mesh: &Mesh, name: &str) {
        let a = primitive_audit(mesh);
        assert!(a.all_finite, "{name}: posições não finitas");
        assert!(a.indices_valid, "{name}: índices/UV inválidos");
        assert_eq!(a.degenerate_faces, 0, "{name}: faces degeneradas");
        assert_eq!(a.coincident_verts, 0, "{name}: vértices coincidentes");
    }

    fn assert_outward(mesh: &Mesh, name: &str) {
        let a = primitive_audit(mesh);
        assert_eq!(a.outward_violations, 0, "{name}: normais para dentro");
    }

    #[test]
    fn cube_bounds_and_topology() {
        let m = Mesh::cube(2.0);
        let a = primitive_audit(&m);
        assert_eq!((a.verts, a.faces), (8, 6));
        assert_eq!(a.bounds_min, [-1.0, -1.0, -1.0]);
        assert_eq!(a.bounds_max, [1.0, 1.0, 1.0]);
        assert_clean(&m, "cube");
        assert_outward(&m, "cube");
    }

    #[test]
    fn box_dimensions() {
        let m = Mesh::box_dim(1.0, 2.0, 4.0);
        let a = primitive_audit(&m);
        assert_eq!((a.verts, a.faces), (8, 6));
        assert_eq!(a.bounds_min, [-0.5, -1.0, -2.0]);
        assert_eq!(a.bounds_max, [0.5, 1.0, 2.0]);
        assert_clean(&m, "box");
        assert_outward(&m, "box");
    }

    #[test]
    fn plane_orientation_and_topology() {
        let m = Mesh::plane(2.0);
        let a = primitive_audit(&m);
        assert_eq!((a.verts, a.faces, a.tris), (4, 1, 2));
        assert!(m.verts.iter().all(|v| v.pos[1] == 0.0));
        assert_clean(&m, "plane");
    }

    #[test]
    fn wedge_topology_and_slope() {
        let m = Mesh::wedge(2.0, 2.0, 2.0);
        let a = primitive_audit(&m);
        assert_eq!((a.verts, a.faces), (6, 5));
        assert_eq!(a.bounds_min, [-1.0, -1.0, -1.0]);
        assert_eq!(a.bounds_max, [1.0, 1.0, 1.0]);
        assert_clean(&m, "wedge");
        assert_outward(&m, "wedge");
    }

    #[test]
    fn cylinder_counts_and_caps() {
        for sides in [3, 6, 8, 32] {
            let m = Mesh::cylinder(sides, 1.0, 2.0);
            let a = primitive_audit(&m);
            let s = sides as usize;
            assert_eq!((a.verts, a.faces), (2 * s + 2, 3 * s), "sides {sides}");
            assert_clean(&m, "cylinder");
            assert_outward(&m, "cylinder");
        }
        // Sem tampas: só laterais.
        let open = Mesh::radial_frustum(1.0, 1.0, 2.0, 8, false, false);
        let a = primitive_audit(&open);
        assert_eq!((a.verts, a.faces), (16, 8));
        assert_clean(&open, "cylinder aberto");
        // Só topo.
        let top_only = Mesh::radial_frustum(1.0, 1.0, 2.0, 8, false, true);
        assert_eq!(primitive_audit(&top_only).faces, 16);
    }

    #[test]
    fn cone_and_frustum_share_generator() {
        let cone = Mesh::cone(8, 1.0, 2.0);
        let frustum_cone = Mesh::radial_frustum(1.0, 0.0, 2.0, 8, true, false);
        assert_eq!(
            primitive_audit(&cone).faces,
            primitive_audit(&frustum_cone).faces
        );
        assert_eq!(
            primitive_audit(&cone).verts,
            primitive_audit(&frustum_cone).verts
        );
        assert_clean(&cone, "cone");
        assert_outward(&cone, "cone");
        // Frustum de verdade (topo > 0): sem ápice, duas tampas.
        let frustum = Mesh::radial_frustum(1.0, 0.5, 2.0, 8, true, true);
        let a = primitive_audit(&frustum);
        assert_eq!((a.verts, a.faces), (18, 24));
        assert_clean(&frustum, "frustum");
        assert_outward(&frustum, "frustum");
        // Cilindro = frustum degenerado equivalente.
        let cyl = Mesh::cylinder(8, 1.0, 2.0);
        let cyl_as_frustum = Mesh::radial_frustum(1.0, 1.0, 2.0, 8, true, true);
        assert_eq!(
            primitive_audit(&cyl).faces,
            primitive_audit(&cyl_as_frustum).faces
        );
    }

    #[test]
    fn circle_fill_modes() {
        let filled = Mesh::circle(1.0, 8, true);
        let a = primitive_audit(&filled);
        assert_eq!((a.verts, a.faces, a.tris), (9, 8, 8));
        assert_clean(&filled, "circle filled");
        let ring = Mesh::circle(1.0, 8, false);
        let a = primitive_audit(&ring);
        assert_eq!((a.verts, a.faces), (8, 1));
        assert_clean(&ring, "circle ring");
        // Mínimo: triângulo.
        let tri = Mesh::circle(1.0, 3, true);
        assert_eq!(primitive_audit(&tri).faces, 3);
        // Default: 12 vértices.
        assert_eq!(primitive_audit(&Mesh::circle(1.0, 12, true)).faces, 12);
    }

    #[test]
    fn torus_closure_and_defaults() {
        let m = Mesh::torus(2.0, 0.5, 12, 6);
        let a = primitive_audit(&m);
        assert_eq!((a.verts, a.faces), (72, 72));
        assert_clean(&m, "torus");
        // Fechado: anel major completo (primeiro == último implicitamente).
        assert!((a.bounds_max[0] - 2.5).abs() < 1e-4);
        // Mínimos.
        let min = Mesh::torus(1.0, 0.3, 3, 3);
        assert_eq!(primitive_audit(&min).faces, 9);
        assert_clean(&min, "torus mínimo");
    }

    #[test]
    fn uv_sphere_defaults_and_minima() {
        // Default 12/6: 62 verts, 72 faces.
        let m = Mesh::sphere_low(12, 6, 1.0);
        let a = primitive_audit(&m);
        assert_eq!((a.verts, a.faces), (62, 72));
        assert_clean(&m, "uv sphere");
        assert_outward(&m, "uv sphere");
        // Mínimo 3/2.
        let min = Mesh::sphere_low(3, 2, 1.0);
        let a = primitive_audit(&min);
        assert_eq!((a.verts, a.faces), (5, 6));
        assert_clean(&min, "uv sphere mínima");
    }

    #[test]
    fn icosphere_subdivision_growth() {
        // 20·4^n tris; 10·4^n+2 verts.
        for (level, verts, faces) in [(0, 12, 20), (1, 42, 80), (2, 162, 320)] {
            let m = Mesh::icosphere(1.0, level);
            let a = primitive_audit(&m);
            assert_eq!((a.verts, a.faces), (verts, faces), "nível {level}");
            assert_clean(&m, "icosphere");
            assert_outward(&m, "icosphere");
            // Projeção no raio.
            for v in &m.verts {
                let len = (v.pos[0] * v.pos[0] + v.pos[1] * v.pos[1] + v.pos[2] * v.pos[2]).sqrt();
                assert!((len - 1.0).abs() < 1e-4, "nível {level}");
            }
        }
        // Teto: nível 9 vira 3.
        assert_eq!(primitive_audit(&Mesh::icosphere(1.0, 9)).faces, 1280);
    }

    #[test]
    fn capsule_seams_and_minima() {
        let m = Mesh::capsule_profile(8, 0.5, 2.0, 2);
        let a = primitive_audit(&m);
        // 2 polos + (2+2+2) anéis de 8 + corpo implícito: verts = 2 + 6*8.
        assert_eq!(a.verts, 2 + 6 * 8);
        assert_clean(&m, "capsule");
        assert_outward(&m, "capsule");
        // Corpo zero (esfera facetada) e 1 anel de calota.
        let short = Mesh::capsule_profile(6, 0.5, 0.5, 1);
        assert_clean(&short, "capsule curta");
        assert_outward(&short, "capsule curta");
        // Altura total respeita 2r + corpo.
        assert!((a.bounds_max[1] - 1.0).abs() < 1e-4);
        assert!((a.bounds_min[1] + 1.0).abs() < 1e-4);
    }

    #[test]
    fn invalid_params_clamp_safely() {
        for m in [
            Mesh::cube(0.0),
            Mesh::box_dim(-1.0, 0.0, -5.0),
            Mesh::wedge(0.0, -1.0, 0.0),
            Mesh::circle(0.0, 2, true),
            Mesh::torus(0.0, 0.0, 0, 0),
            Mesh::sphere_low(0, 0, -1.0),
            Mesh::icosphere(0.0, 99),
            Mesh::capsule_profile(0, 0.0, 0.0, 0),
            Mesh::plane(0.0),
        ] {
            assert_clean(&m, "clamp");
            assert!(!m.faces.is_empty(), "topologia mínima");
        }
        // Raios nulos nos dois anéis: malha vazia porém válida (sem ápice duplo).
        for m in [
            Mesh::cylinder(0, 0.0, 0.0),
            Mesh::cylinder(100, -2.0, -3.0),
            Mesh::radial_frustum(0.0, 0.0, 1.0, 8, true, true),
        ] {
            let a = primitive_audit(&m);
            assert!(a.all_finite && a.indices_valid);
            assert_eq!(a.degenerate_faces, 0);
        }
    }

    #[test]
    fn primitives_are_deterministic() {
        fn rebuild(name: &str) -> Mesh {
            match name {
                "cube" => Mesh::cube(2.0),
                "box" => Mesh::box_dim(1.0, 2.0, 3.0),
                "plane" => Mesh::plane(2.0),
                "wedge" => Mesh::wedge(2.0, 2.0, 2.0),
                "cylinder" => Mesh::cylinder(8, 1.0, 2.0),
                "cone" => Mesh::cone(8, 1.0, 2.0),
                "circle" => Mesh::circle(1.0, 12, true),
                "torus" => Mesh::torus(2.0, 0.5, 12, 6),
                "uvsphere" => Mesh::sphere_low(12, 6, 1.0),
                "icosphere" => Mesh::icosphere(1.0, 1),
                _ => Mesh::capsule_profile(8, 0.5, 2.0, 2),
            }
        }
        for name in [
            "cube",
            "box",
            "plane",
            "wedge",
            "cylinder",
            "cone",
            "circle",
            "torus",
            "uvsphere",
            "icosphere",
            "capsule",
        ] {
            let a = rebuild(name);
            let b = rebuild(name);
            assert_eq!(a.verts.len(), b.verts.len(), "{name}");
            assert_eq!(a.faces.len(), b.faces.len(), "{name}");
            assert_eq!(a.verts[0].pos, b.verts[0].pos, "{name}");
            assert_eq!(primitive_audit(&a).tris, primitive_audit(&b).tris, "{name}");
        }
    }
}
