//! Algoritmo de Varredura 3D (Sweep) com Rotation Minimizing Frames (RMF).
//!
//! Permite varrer um perfil 2D ao longo de uma curva guia 3D arbitrária (spine),
//! com minimização de torção (Parallel Transport / Double Reflection RMF),
//! mitering com contenção em cantos vivos e tampas (end caps) trianguladas.

use glam::{DVec3, Vec3};

use super::{Face, Mesh, Vertex, compute_parallel_transport_frames};

/// Opções de configuração para a operação de Sweep.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepOptions {
    /// Se o caminho forma um laço fechado contínuo.
    pub closed_path: bool,
    /// Se o perfil de seção transversal é um laço fechado (ex: tubo vs fita/fita aberta).
    pub closed_profile: bool,
    /// Se deve gerar tampa no início (se o caminho for aberto e o perfil fechado).
    pub cap_start: bool,
    /// Se deve gerar tampa no fim (se o caminho for aberto e o perfil fechado).
    pub cap_end: bool,
    /// Se deve alinhar os anéis no plano bissetor em cantos vivos (mitering).
    pub miter: bool,
    /// Limite máximo de extensão do miter (previne pontas infinitas em cantos agudos).
    pub miter_limit: f32,
}

impl Default for SweepOptions {
    fn default() -> Self {
        Self {
            closed_path: false,
            closed_profile: true,
            cap_start: true,
            cap_end: true,
            miter: true,
            miter_limit: 3.0,
        }
    }
}

/// Sistema de coordenadas ortonormais local (Frame RMF) ao longo do caminho.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepFrame {
    pub origin: Vec3,
    pub tangent: Vec3,
    pub normal: Vec3,   // Eixo local X do perfil
    pub binormal: Vec3, // Eixo local Y do perfil
}

impl SweepFrame {
    /// Projeta um ponto 2D do perfil no espaço 3D usando este frame.
    #[inline]
    pub fn to_world(&self, pt: [f32; 2]) -> Vec3 {
        self.origin + self.normal * pt[0] + self.binormal * pt[1]
    }
}

/// Calcula Rotation Minimizing Frames (RMF) usando o método de Double Reflection (Wang et al., 2008).
///
/// Este método elimina o gimbal lock e torções indesejadas (flipping) em curvas 3D,
/// garantindo transporte paralelo de 4ª ordem globalmente sem singularidades.
pub fn compute_rmf_frames(path: &[Vec3], closed: bool) -> Result<Vec<SweepFrame>, String> {
    let path: Vec<_> = path
        .iter()
        .map(|point| DVec3::new(point.x as f64, point.y as f64, point.z as f64))
        .collect();
    compute_parallel_transport_frames(&path, closed)
        .map(|frames| {
            frames
                .into_iter()
                .map(|frame| SweepFrame {
                    origin: frame.origin.as_vec3(),
                    tangent: frame.tangent.as_vec3(),
                    normal: frame.normal.as_vec3(),
                    binormal: frame.binormal.as_vec3(),
                })
                .collect()
        })
        .map_err(|error| format!("Caminho de Sweep inválido: {error}"))
}

/// Aplica o alinhamento de bissetriz (mitering) em vértices de canto.
fn apply_mitering(
    frame: &SweepFrame,
    prev_tangent: Option<Vec3>,
    next_tangent: Option<Vec3>,
    pt: [f32; 2],
    miter_limit: f32,
) -> Vec3 {
    let p_local = frame.normal * pt[0] + frame.binormal * pt[1];

    let (Some(t_in), Some(t_out)) = (prev_tangent, next_tangent) else {
        return frame.origin + p_local;
    };

    let bisector = (t_in + t_out).normalize_or_zero();
    let cos_half = t_in.dot(bisector);

    if cos_half <= 0.1 || bisector.length_squared() < 0.5 {
        return frame.origin + p_local;
    }

    // Fator de escala do miter na direção de curvatura
    let miter_scale = (1.0 / cos_half).min(miter_limit.max(1.0));

    // Vetor de dobra no plano (tangente de entrada x saída)
    let bend_dir = t_in.cross(t_out);
    if bend_dir.length_squared() < 1e-6 {
        return frame.origin + p_local;
    }

    let bend_axis = bend_dir.normalize();
    let p_proj_bend = p_local.dot(bend_axis) * bend_axis;
    let p_perp_bend = p_local - p_proj_bend;

    frame.origin + p_proj_bend + p_perp_bend * miter_scale
}

/// Executa a varredura (Sweep) de um perfil 2D ao longo de um caminho 3D.
pub fn generate_sweep(
    profile: &[[f32; 2]],
    path: &[Vec3],
    options: SweepOptions,
) -> Result<Mesh, String> {
    let num_profile = profile.len();
    if num_profile < 2 {
        return Err("O perfil para Sweep precisa ter pelo menos 2 pontos".to_string());
    }
    let num_path = path.len();
    if num_path < 2 {
        return Err("O caminho para Sweep precisa ter pelo menos 2 pontos".to_string());
    }

    let frames = compute_rmf_frames(path, options.closed_path)?;

    // Calcula parâmetros UV ao longo do perfil (eixo U)
    let mut profile_u = Vec::with_capacity(num_profile);
    profile_u.push(0.0);
    let mut total_profile_len = 0.0;
    for i in 0..(num_profile - 1) {
        let d = (profile[i + 1][0] - profile[i][0]).hypot(profile[i + 1][1] - profile[i][1]);
        total_profile_len += d;
        profile_u.push(total_profile_len);
    }
    if total_profile_len > 1e-6 {
        for u in &mut profile_u {
            *u /= total_profile_len;
        }
    }

    // Calcula parâmetros UV ao longo do caminho (eixo V)
    let mut path_v = Vec::with_capacity(num_path);
    path_v.push(0.0);
    let mut total_path_len = 0.0;
    for i in 0..(num_path - 1) {
        let d = (path[i + 1] - path[i]).length();
        total_path_len += d;
        path_v.push(total_path_len);
    }
    if options.closed_path {
        total_path_len += (path[0] - path[num_path - 1]).length();
    }
    if total_path_len > 1e-6 {
        for v in &mut path_v {
            *v /= total_path_len;
        }
    }

    let mut mesh = Mesh::default();

    // 1. Gera todos os anéis de vértices
    for i in 0..num_path {
        let frame = &frames[i];
        let prev_tangent = if i > 0 {
            Some(frames[i - 1].tangent)
        } else if options.closed_path {
            Some(frames[num_path - 1].tangent)
        } else {
            None
        };
        let next_tangent = if i + 1 < num_path {
            Some(frames[i + 1].tangent)
        } else if options.closed_path {
            Some(frames[0].tangent)
        } else {
            None
        };

        for &pt in profile {
            let pt_world = if options.miter {
                apply_mitering(frame, prev_tangent, next_tangent, pt, options.miter_limit)
            } else {
                frame.to_world(pt)
            };
            mesh.verts
                .push(Vertex::new(pt_world.x, pt_world.y, pt_world.z));
        }
    }

    // 2. Conecta os anéis com faces quads
    let ring_steps = if options.closed_path {
        num_path
    } else {
        num_path - 1
    };

    for i in 0..ring_steps {
        let ring_curr = i;
        let ring_next = (i + 1) % num_path;

        let v_curr = path_v[ring_curr];
        let v_next = if options.closed_path && ring_next == 0 {
            1.0
        } else {
            path_v[ring_next]
        };

        for j in 0..(num_profile - 1) {
            let j_next = j + 1;
            let u_curr = profile_u[j];
            let u_next = profile_u[j_next];

            let idx_0 = (ring_curr * num_profile + j) as u32;
            let idx_1 = (ring_curr * num_profile + j_next) as u32;
            let idx_2 = (ring_next * num_profile + j_next) as u32;
            let idx_3 = (ring_next * num_profile + j) as u32;

            mesh.push_face(Face::with_uv(
                vec![idx_0, idx_1, idx_2, idx_3],
                vec![
                    [u_curr, v_curr],
                    [u_next, v_curr],
                    [u_next, v_next],
                    [u_curr, v_next],
                ],
            ));
        }

        // Se o perfil for fechado, fecha a lateral conectando o último ponto ao primeiro
        let is_profile_closed = options.closed_profile || {
            let p_first = profile[0];
            let p_last = profile[num_profile - 1];
            (p_first[0] - p_last[0]).hypot(p_first[1] - p_last[1]) < 1e-4
        };

        if is_profile_closed {
            let j = num_profile - 1;
            let j_next = 0;
            let idx_0 = (ring_curr * num_profile + j) as u32;
            let idx_1 = (ring_curr * num_profile + j_next) as u32;
            let idx_2 = (ring_next * num_profile + j_next) as u32;
            let idx_3 = (ring_next * num_profile + j) as u32;

            mesh.push_face(Face::with_uv(
                vec![idx_0, idx_1, idx_2, idx_3],
                vec![[1.0, v_curr], [0.0, v_curr], [0.0, v_next], [1.0, v_next]],
            ));
        }
    }

    // 3. Gera tampas de início e fim se aplicável (quads para 4 pontos, n-gons para > 4)
    if !options.closed_path && num_profile >= 3 && (options.cap_start || options.cap_end) {
        if num_profile == 4 {
            if options.cap_start {
                mesh.push_face(Face::with_uv(
                    vec![3, 2, 1, 0],
                    vec![profile[3], profile[2], profile[1], profile[0]],
                ));
            }
            if options.cap_end {
                let off = ((num_path - 1) * num_profile) as u32;
                mesh.push_face(Face::with_uv(
                    vec![off, off + 1, off + 2, off + 3],
                    vec![profile[0], profile[1], profile[2], profile[3]],
                ));
            }
        } else if num_profile == 3 {
            if options.cap_start {
                mesh.push_face(Face::with_uv(
                    vec![2, 1, 0],
                    vec![profile[2], profile[1], profile[0]],
                ));
            }
            if options.cap_end {
                let off = ((num_path - 1) * num_profile) as u32;
                mesh.push_face(Face::with_uv(
                    vec![off, off + 1, off + 2],
                    vec![profile[0], profile[1], profile[2]],
                ));
            }
        } else {
            if options.cap_start {
                let start_verts: Vec<u32> = (0..num_profile as u32).rev().collect();
                let start_uvs: Vec<[f32; 2]> = profile.iter().rev().copied().collect();
                mesh.push_face(Face::with_uv(start_verts, start_uvs));
            }
            if options.cap_end {
                let off = ((num_path - 1) * num_profile) as u32;
                let end_verts: Vec<u32> = (off..off + num_profile as u32).collect();
                let end_uvs: Vec<[f32; 2]> = profile.to_vec();
                mesh.push_face(Face::with_uv(end_verts, end_uvs));
            }
        }
    }

    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rmf_straight_line() {
        let path = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 10.0),
        ];
        let frames = compute_rmf_frames(&path, false).unwrap();
        assert_eq!(frames.len(), 3);
        for f in &frames {
            assert!((f.tangent.dot(Vec3::Z) - 1.0).abs() < 1e-4);
            assert!(f.normal.is_normalized());
            assert!(f.binormal.is_normalized());
            assert!(f.normal.dot(f.tangent).abs() < 1e-4);
        }
    }

    #[test]
    fn test_rmf_90_degree_bend() {
        let path = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(5.0, 5.0, 0.0),
        ];
        let frames = compute_rmf_frames(&path, false).unwrap();
        assert_eq!(frames.len(), 3);
        // Cada frame deve ser estritamente ortonormal
        for f in &frames {
            assert!(f.tangent.is_normalized());
            assert!(f.normal.is_normalized());
            assert!(f.binormal.is_normalized());
            assert!(f.normal.dot(f.tangent).abs() < 1e-4);
            assert!(f.binormal.dot(f.tangent).abs() < 1e-4);
            assert!(f.normal.dot(f.binormal).abs() < 1e-4);
        }
    }

    #[test]
    fn test_sweep_straight_pipe() {
        let profile = vec![[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        let path = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
        ];
        let mesh = generate_sweep(
            &profile,
            &path,
            SweepOptions {
                closed_path: false,
                closed_profile: true,
                cap_start: true,
                cap_end: true,
                miter: true,
                miter_limit: 3.0,
            },
        )
        .unwrap();

        // 3 anéis de 4 vértices = 12 vértices
        assert_eq!(mesh.verts.len(), 12);
        // Deve conter faces laterais e tampas
        assert!(!mesh.faces.is_empty());
        for v in &mesh.verts {
            assert!(v.pos[0].is_finite());
            assert!(v.pos[1].is_finite());
            assert!(v.pos[2].is_finite());
        }
    }

    #[test]
    fn test_sweep_closed_loop() {
        // Caminho quadrado fechado
        let path = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
        ];
        let profile = vec![[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]];
        let mesh = generate_sweep(
            &profile,
            &path,
            SweepOptions {
                closed_path: true,
                closed_profile: true,
                cap_start: false,
                cap_end: false,
                miter: true,
                miter_limit: 3.0,
            },
        )
        .unwrap();

        // 4 anéis de 4 vértices = 16 vértices
        assert_eq!(mesh.verts.len(), 16);
        assert!(!mesh.faces.is_empty());
    }
}
