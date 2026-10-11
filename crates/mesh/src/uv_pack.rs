//! Empacotamento de ilhas UV (V02; website/docs/07-uv/architecture-decision.md, "Pack").
//!
//! Um único shelf packer determinístico serve o layout de charts dos geradores
//! ([`Mesh::layout_uv_charts`]) e o Pack do editor ([`Mesh::pack_uv_islands_with`]):
//! escala uniforme (preserva a densidade relativa entre ilhas), padding por eixo
//! e obstáculos (ilhas que não se movem). Nenhuma ilha empacotada sobrepõe outra
//! nem um obstáculo.

use std::collections::HashSet;

use glam::Vec2;

use super::Mesh;

/// Retângulo alinhado aos eixos no espaço UV.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UvRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl UvRect {
    /// `true` quando os dois retângulos ficam a menos de `gap` um do outro.
    fn within(&self, other: &UvRect, gap: Vec2) -> bool {
        self.min.x < other.max.x + gap.x
            && self.max.x + gap.x > other.min.x
            && self.min.y < other.max.y + gap.y
            && self.max.y + gap.y > other.min.y
    }
}

/// Resultado do shelf packer: escala comum e origem de cada retângulo.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShelfLayout {
    pub scale: f32,
    pub origins: Vec<Vec2>,
}

/// Empacota `sizes` (tamanhos não escalados) em `0..1` por prateleiras, ordenados
/// por altura decrescente (empate pelo índice — determinístico), procurando a
/// maior escala comum que cabe. `padding` separa retângulos entre si, das bordas
/// e dos `obstacles`. `None` quando nada cabe.
pub(crate) fn shelf_pack(
    sizes: &[Vec2],
    padding: Vec2,
    obstacles: &[UvRect],
) -> Option<ShelfLayout> {
    if sizes.is_empty() {
        return Some(ShelfLayout {
            scale: 1.0,
            origins: Vec::new(),
        });
    }
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|&a, &b| {
        sizes[b]
            .y
            .partial_cmp(&sizes[a].y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    let try_pack = |scale: f32| -> Option<Vec<Vec2>> {
        let mut origins = vec![Vec2::ZERO; sizes.len()];
        let (mut x, mut y, mut row_h) = (padding.x, padding.y, 0.0f32);
        for &i in &order {
            let size = sizes[i] * scale;
            loop {
                if x + size.x + padding.x > 1.0 {
                    // Prateleira vazia (bloqueada por obstáculo) ainda avança.
                    y += if row_h > 0.0 { row_h } else { size.y } + padding.y;
                    x = padding.x;
                    row_h = 0.0;
                }
                if x + size.x + padding.x > 1.0 || y + size.y + padding.y > 1.0 {
                    return None;
                }
                let candidate = UvRect {
                    min: Vec2::new(x, y),
                    max: Vec2::new(x, y) + size,
                };
                match obstacles.iter().find(|o| candidate.within(o, padding)) {
                    Some(blocker) => x = blocker.max.x + padding.x,
                    None => break,
                }
            }
            origins[i] = Vec2::new(x, y);
            x += size.x + padding.x;
            row_h = row_h.max(size.y);
        }
        Some(origins)
    };
    let total: f32 = sizes.iter().map(|s| s.x * s.y).sum();
    let mut scale = (1.0 / total.max(1e-9)).sqrt();
    for _ in 0..64 {
        if let Some(origins) = try_pack(scale) {
            return Some(ShelfLayout { scale, origins });
        }
        scale *= 0.92;
    }
    None
}

/// Opções do Pack do editor. A UI fala em pixels; a conversão para UV usa a
/// resolução da textura ativa.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvPackOptions {
    /// Distância mínima entre ilhas e até a borda, em pixels.
    pub padding_px: f32,
    /// Resolução `[largura, altura]` da TextureResource/canal ativo.
    pub texture_size: [u32; 2],
    /// Permite girar 90° ilhas mais altas que largas.
    pub allow_rotation: bool,
    /// Ilhas com algum canto fixado (pin) ficam onde estão.
    pub keep_pinned_islands: bool,
}

impl Default for UvPackOptions {
    fn default() -> Self {
        Self {
            padding_px: 4.0,
            texture_size: [1024, 1024],
            allow_rotation: true,
            keep_pinned_islands: false,
        }
    }
}

/// Resultado de um Pack bem-sucedido.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UvPackReport {
    /// Ilhas reposicionadas.
    pub packed: usize,
    /// Ilhas mantidas no lugar (fora da seleção ou fixadas).
    pub fixed: usize,
    /// Ilhas giradas 90°.
    pub rotated: usize,
}

/// Falha do Pack. Em qualquer variante as UVs não foram alteradas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UvPackError {
    #[error("texture size must be non-zero")]
    InvalidTextureSize,
    #[error("padding must be finite and non-negative")]
    InvalidPadding,
    #[error("no space left to pack the islands")]
    NoSpace,
}

impl Mesh {
    /// Pack inicial com padding em UV (compatibilidade): converte para o Pack
    /// por pixels numa textura 1024², sem rotação. Retorna o número de ilhas.
    pub fn pack_uv_islands(&mut self, padding: f32) -> usize {
        let options = UvPackOptions {
            padding_px: padding.clamp(0.0, 0.25) * 1024.0,
            texture_size: [1024, 1024],
            allow_rotation: false,
            keep_pinned_islands: false,
        };
        self.pack_uv_islands_with(&HashSet::new(), &options)
            .map_or(0, |report| report.packed + report.fixed)
    }

    /// Pack determinístico de ilhas em `0..1`.
    ///
    /// Com `selected_faces` vazio todas as ilhas se movem; caso contrário só as
    /// ilhas que tocam a seleção, e as demais viram obstáculos. Todas as ilhas
    /// movidas recebem a mesma escala (densidade relativa preservada).
    pub fn pack_uv_islands_with(
        &mut self,
        selected_faces: &HashSet<usize>,
        options: &UvPackOptions,
    ) -> Result<UvPackReport, UvPackError> {
        let [w, h] = options.texture_size;
        if w == 0 || h == 0 {
            return Err(UvPackError::InvalidTextureSize);
        }
        if !options.padding_px.is_finite() || options.padding_px < 0.0 {
            return Err(UvPackError::InvalidPadding);
        }
        let padding = Vec2::new(options.padding_px / w as f32, options.padding_px / h as f32);

        let islands = self.uv_islands();
        let mut moving = Vec::new();
        let mut obstacles = Vec::new();
        for island in &islands {
            let selected = selected_faces.is_empty()
                || island.faces.iter().any(|f| selected_faces.contains(f));
            let pinned = options.keep_pinned_islands
                && island.faces.iter().any(|&f| {
                    (0..self.faces[f].uv.len()).any(|c| self.uv_pinned.contains(&(f, c)))
                });
            if selected && !pinned {
                moving.push(island);
            } else {
                obstacles.push(UvRect {
                    min: Vec2::from_array(island.min),
                    max: Vec2::from_array(island.max),
                });
            }
        }

        let rotate: Vec<bool> = moving
            .iter()
            .map(|island| {
                let size = Vec2::from_array(island.max) - Vec2::from_array(island.min);
                options.allow_rotation && size.y > size.x * (1.0 + 1e-4)
            })
            .collect();
        let sizes: Vec<Vec2> = moving
            .iter()
            .zip(&rotate)
            .map(|(island, &r)| {
                let size = (Vec2::from_array(island.max) - Vec2::from_array(island.min))
                    .max(Vec2::splat(1e-5));
                if r { Vec2::new(size.y, size.x) } else { size }
            })
            .collect();
        let layout = shelf_pack(&sizes, padding, &obstacles).ok_or(UvPackError::NoSpace)?;

        for ((island, &r), origin) in moving.iter().zip(&rotate).zip(&layout.origins) {
            let min = Vec2::from_array(island.min);
            let height = island.max[1] - island.min[1];
            for &fi in &island.faces {
                for uv in &mut self.faces[fi].uv {
                    let local = Vec2::from_array(*uv) - min;
                    // 90° anti-horário, transladado de volta para o primeiro quadrante.
                    let local = if r {
                        Vec2::new(height - local.y, local.x)
                    } else {
                        local
                    };
                    *uv = (*origin + local * layout.scale).to_array();
                }
            }
        }
        Ok(UvPackReport {
            packed: moving.len(),
            fixed: obstacles.len(),
            rotated: rotate.iter().filter(|r| **r).count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Face, Vertex};

    /// Dois quads soltos, cada um uma ilha.
    fn two_islands() -> Mesh {
        let verts = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [3.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [4.0, 2.0, 0.0],
            [3.0, 2.0, 0.0],
        ]
        .into_iter()
        .map(|[x, y, z]| Vertex::new(x, y, z))
        .collect();
        let faces = vec![
            Face::with_uv(
                vec![0, 1, 2, 3],
                vec![[0.0, 0.0], [0.2, 0.0], [0.2, 0.2], [0.0, 0.2]],
            ),
            Face::with_uv(
                vec![4, 5, 6, 7],
                vec![[0.1, 0.1], [0.3, 0.1], [0.3, 0.5], [0.1, 0.5]],
            ),
        ];
        Mesh {
            verts,
            faces,
            ..Mesh::default()
        }
    }

    fn bounds(m: &Mesh, face: usize) -> UvRect {
        let pts: Vec<Vec2> = m.faces[face]
            .uv
            .iter()
            .map(|uv| Vec2::from_array(*uv))
            .collect();
        UvRect {
            min: pts.iter().copied().reduce(Vec2::min).unwrap(),
            max: pts.iter().copied().reduce(Vec2::max).unwrap(),
        }
    }

    #[test]
    fn shelf_pack_keeps_padding_from_borders_and_obstacles() {
        let pad = Vec2::splat(0.05);
        let obstacle = UvRect {
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(0.4, 0.4),
        };
        let layout = shelf_pack(&[Vec2::ONE, Vec2::ONE], pad, &[obstacle]).unwrap();
        for origin in &layout.origins {
            let rect = UvRect {
                min: *origin,
                max: *origin + Vec2::ONE * layout.scale,
            };
            assert!(!rect.within(&obstacle, pad - Vec2::splat(1e-6)));
            assert!(rect.min.cmpge(pad - 1e-6).all());
            assert!(rect.max.cmple(Vec2::ONE - pad + 1e-6).all());
        }
    }

    #[test]
    fn pack_is_deterministic_and_preserves_relative_density() {
        let mut a = two_islands();
        let mut b = two_islands();
        let options = UvPackOptions::default();
        let report = a.pack_uv_islands_with(&HashSet::new(), &options).unwrap();
        b.pack_uv_islands_with(&HashSet::new(), &options).unwrap();
        assert_eq!(report.packed, 2);
        for f in 0..2 {
            assert_eq!(a.faces[f].uv, b.faces[f].uv);
        }
        // A ilha 1 tinha o dobro da área UV da ilha 0 e continua tendo.
        let area = |r: UvRect| (r.max - r.min).x * (r.max - r.min).y;
        let ratio = area(bounds(&a, 1)) / area(bounds(&a, 0));
        assert!((ratio - 2.0).abs() < 1e-3, "ratio={ratio}");
    }

    #[test]
    fn pack_never_overlaps_islands_and_respects_pixel_padding() {
        let mut m = two_islands();
        let options = UvPackOptions {
            padding_px: 8.0,
            texture_size: [256, 256],
            ..UvPackOptions::default()
        };
        m.pack_uv_islands_with(&HashSet::new(), &options).unwrap();
        let pad = Vec2::splat(8.0 / 256.0) - Vec2::splat(1e-5);
        assert!(!bounds(&m, 0).within(&bounds(&m, 1), pad));
    }

    #[test]
    fn selected_only_pack_leaves_other_islands_untouched() {
        let mut m = two_islands();
        let untouched = m.faces[1].uv.clone();
        let report = m
            .pack_uv_islands_with(&HashSet::from([0]), &UvPackOptions::default())
            .unwrap();
        assert_eq!((report.packed, report.fixed), (1, 1));
        assert_eq!(m.faces[1].uv, untouched);
        assert!(!bounds(&m, 0).within(&bounds(&m, 1), Vec2::ZERO));
    }

    #[test]
    fn tall_islands_rotate_only_when_allowed() {
        let mut m = two_islands();
        let report = m
            .pack_uv_islands_with(&HashSet::new(), &UvPackOptions::default())
            .unwrap();
        assert_eq!(report.rotated, 1);
        let size = bounds(&m, 1).max - bounds(&m, 1).min;
        assert!(size.x > size.y);

        let mut m = two_islands();
        let options = UvPackOptions {
            allow_rotation: false,
            ..UvPackOptions::default()
        };
        assert_eq!(
            m.pack_uv_islands_with(&HashSet::new(), &options)
                .unwrap()
                .rotated,
            0
        );
    }

    #[test]
    fn invalid_options_fail_without_touching_uvs() {
        let mut m = two_islands();
        let before: Vec<_> = m.faces.iter().map(|f| f.uv.clone()).collect();
        let zero = UvPackOptions {
            texture_size: [0, 512],
            ..UvPackOptions::default()
        };
        assert_eq!(
            m.pack_uv_islands_with(&HashSet::new(), &zero),
            Err(UvPackError::InvalidTextureSize)
        );
        let negative = UvPackOptions {
            padding_px: -1.0,
            ..UvPackOptions::default()
        };
        assert_eq!(
            m.pack_uv_islands_with(&HashSet::new(), &negative),
            Err(UvPackError::InvalidPadding)
        );
        assert_eq!(
            m.faces.iter().map(|f| f.uv.clone()).collect::<Vec<_>>(),
            before
        );
    }
}
