//! Morph targets (blend shapes) — deformação linear de malha.
//!
//! Um **alvo** guarda apenas as *posições* dos vértices, na mesma ordem e com
//! a mesma contagem da malha base: a topologia (faces, arestas, UV) é
//! preservada durante o morph — só as posições mudam.
//!
//! A **mistura** (`MorphBlend`) combina vários alvos com pesos. A posição
//! final de cada vértice é
//!
//! ```text
//! p = p_base + Σ (peso_i * (p_alvo_i − p_base))
//! ```
//!
//! `Mesh::morph_blend` devolve uma **cópia** da malha: a malha original nunca
//! é mutada, o que mantém a operação segura para edição, undo e pré-visualização.
//!
//! ```
//! use petunia_mesh::{Mesh, MorphBlend, MorphTarget};
//!
//! let base = Mesh::cube(2.0);
//! let target = MorphTarget::from_mesh("forma-alvo", &base);
//! let blend = MorphBlend {
//!     targets: vec![target],
//!     weights: vec![0.5],
//! };
//! let misturada = base.morph_blend(&blend).expect("mistura válida");
//! assert_eq!(misturada.verts.len(), base.verts.len());
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Mesh;

/// Alvo de morph (blend shape): guarda só as posições dos vértices.
///
/// A lista `positions` tem exatamente o mesmo número de vértices da malha
/// base — o alvo não carrega conectividade, cores nem UV.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MorphTarget {
    /// Nome legível do alvo (exibido na UI, exportado em glTF).
    pub name: String,
    /// Posição de cada vértice, na mesma ordem dos vértices da malha base.
    pub positions: Vec<[f32; 3]>,
}

impl MorphTarget {
    /// Cria um alvo a partir de uma lista de posições (uma por vértice da base).
    ///
    /// A lista é aceita como veio: a validação de contagem acontece na
    /// mistura (`Mesh::morph_blend`), onde o alvo é confrontado com a malha.
    pub fn new(name: impl Into<String>, positions: Vec<[f32; 3]>) -> Self {
        Self {
            name: name.into(),
            positions,
        }
    }

    /// Cria um alvo com as posições atuais da malha (forma neutra).
    ///
    /// Serve de ponto de partida para "esculpir" o alvo: edite a malha e
    /// capture o novo estado com `MorphTarget::from_mesh`.
    pub fn from_mesh(name: impl Into<String>, mesh: &Mesh) -> Self {
        Self {
            name: name.into(),
            positions: mesh.verts.iter().map(|vertex| vertex.pos).collect(),
        }
    }
}

/// Mistura de alvos: cada alvo com o seu peso.
///
/// Peso `0.0` mantém a forma base, peso `1.0` leva exatamente à forma do
/// alvo; pesos acima de `1.0` extrapolam (a forma segue além do alvo).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MorphBlend {
    /// Alvos combinados (todos com a mesma topologia da malha base).
    pub targets: Vec<MorphTarget>,
    /// Um peso por alvo, na mesma ordem de `targets`.
    pub weights: Vec<f32>,
}

/// Por que a mistura de morph foi recusada.
#[derive(Clone, Debug, PartialEq, Error)]
pub enum MorphError {
    /// A mistura não tem alvo nenhum.
    #[error("a lista de alvos de morph está vazia")]
    EmptyTargets,

    /// A quantidade de pesos não corresponde à quantidade de alvos.
    #[error("a lista de pesos ({weights}) não bate com a lista de alvos ({targets})")]
    WeightCountMismatch { weights: usize, targets: usize },

    /// Peso negativo: blends são frações de forma e não admitem sinal invertido.
    #[error("o peso {weight} do alvo {target} é negativo")]
    NegativeWeight { target: usize, weight: f32 },

    /// Peso `NaN`/`Inf`: poluiria a malha com posições inválidas.
    #[error("o peso {weight} do alvo {target} não é finito")]
    NonFiniteWeight { target: usize, weight: f32 },

    /// O alvo não tem a mesma contagem de vértices que a malha base.
    #[error("o alvo de morph {name:?} tem {count} vértices, mas a malha tem {mesh_verts}")]
    TargetVertexCountMismatch {
        name: String,
        count: usize,
        mesh_verts: usize,
    },

    /// O alvo tem uma posição não finita (`NaN`/`Inf`).
    #[error("o alvo de morph {target} tem a posição do vértice {index} não finita")]
    NonFiniteTargetPosition { target: usize, index: usize },
}

impl Mesh {
    /// Aplica a mistura de morph targets e devolve uma **cópia** da malha.
    ///
    /// A malha original não é modificada: faces, UV, seleções e arestas
    /// marcadas são copiados, e só as **posições** dos vértices são
    /// interpoladas. Todos os pesos zero devolvem a malha base intacta.
    ///
    /// # Erros
    ///
    /// Recusa (sem panic) quando a lista de alvos está vazia, quando o número
    /// de pesos não bate com o número de alvos, quando algum peso é negativo
    /// ou não finito, quando um alvo tem contagem de vértices diferente da
    /// malha ou quando um alvo tem posição não finita.
    pub fn morph_blend(&self, blend: &MorphBlend) -> Result<Mesh, MorphError> {
        if blend.targets.is_empty() {
            return Err(MorphError::EmptyTargets);
        }
        if blend.weights.len() != blend.targets.len() {
            return Err(MorphError::WeightCountMismatch {
                weights: blend.weights.len(),
                targets: blend.targets.len(),
            });
        }
        for (target_index, &weight) in blend.weights.iter().enumerate() {
            if !weight.is_finite() {
                return Err(MorphError::NonFiniteWeight {
                    target: target_index,
                    weight,
                });
            }
            if weight < 0.0 {
                return Err(MorphError::NegativeWeight {
                    target: target_index,
                    weight,
                });
            }
        }
        for (target_index, target) in blend.targets.iter().enumerate() {
            if target.positions.len() != self.verts.len() {
                return Err(MorphError::TargetVertexCountMismatch {
                    name: target.name.clone(),
                    count: target.positions.len(),
                    mesh_verts: self.verts.len(),
                });
            }
            for (index, position) in target.positions.iter().enumerate() {
                if !position.iter().all(|axis| axis.is_finite()) {
                    return Err(MorphError::NonFiniteTargetPosition {
                        target: target_index,
                        index,
                    });
                }
            }
        }

        let mut blended = self.clone();
        for (vert_index, vertex) in blended.verts.iter_mut().enumerate() {
            let base = vertex.pos;
            let mut position = base;
            for (target, &weight) in blend.targets.iter().zip(blend.weights.iter()) {
                // Sem panic: a contagem já foi validada acima; a checagem
                // cobre qualquer inconsistência futura de tipo ou tamanho.
                let Some(target_position) = target.positions.get(vert_index) else {
                    return Err(MorphError::TargetVertexCountMismatch {
                        name: target.name.clone(),
                        count: target.positions.len(),
                        mesh_verts: self.verts.len(),
                    });
                };
                for ((out_axis, target_axis), base_axis) in position
                    .iter_mut()
                    .zip(target_position.iter())
                    .zip(base.iter())
                {
                    *out_axis += weight * (target_axis - base_axis);
                }
            }
            vertex.pos = position;
        }
        Ok(blended)
    }
}

#[cfg(test)]
mod tests {
    use super::{MorphBlend, MorphError, MorphTarget};
    use crate::Mesh;

    /// Cubo base de 2.0: 8 vértices em ±1.0 (coordenadas exatas em f32).
    fn base_cube() -> Mesh {
        Mesh::cube(2.0)
    }

    /// Alvo deslocado 2.0 em X: as coordenadas continuam exatas em f32,
    /// o que permite comparar posições com igualdade exata.
    fn target_shifted_x(name: &str) -> MorphTarget {
        let mut cube = base_cube();
        let indices: Vec<u32> = (0..cube.verts.len() as u32).collect();
        cube.translate_vertices(&indices, [2.0, 0.0, 0.0]);
        MorphTarget::from_mesh(name, &cube)
    }

    /// Alvo deslocado 2.0 para baixo em Y (exato em f32).
    fn target_shifted_y(name: &str) -> MorphTarget {
        let mut cube = base_cube();
        let indices: Vec<u32> = (0..cube.verts.len() as u32).collect();
        cube.translate_vertices(&indices, [0.0, -2.0, 0.0]);
        MorphTarget::from_mesh(name, &cube)
    }

    /// Posições dos vértices, para comparação exata antes/depois.
    fn positions(mesh: &Mesh) -> Vec<[f32; 3]> {
        mesh.verts.iter().map(|vertex| vertex.pos).collect()
    }

    #[test]
    fn morph_blend_com_peso_um_atinge_exatamente_o_alvo() {
        let base = base_cube();
        let target = target_shifted_x("deslocado-x");
        let blend = MorphBlend {
            targets: vec![target.clone()],
            weights: vec![1.0],
        };

        let blended = base.morph_blend(&blend).expect("mistura válida");

        assert_eq!(positions(&blended), target.positions);
        // A topologia é preservada: só as posições mudam.
        assert_eq!(blended.faces.len(), base.faces.len());
    }

    #[test]
    fn morph_blend_com_peso_zero_mantem_a_forma_base() {
        let base = base_cube();
        let target = target_shifted_x("deslocado-x");
        let blend = MorphBlend {
            targets: vec![target],
            weights: vec![0.0],
        };

        let blended = base.morph_blend(&blend).expect("mistura válida");

        assert_eq!(positions(&blended), positions(&base));
    }

    #[test]
    fn morph_blend_com_peso_meio_fica_no_meio() {
        let base = base_cube();
        let target = target_shifted_x("deslocado-x");
        let blend = MorphBlend {
            targets: vec![target.clone()],
            weights: vec![0.5],
        };

        let blended = base.morph_blend(&blend).expect("mistura válida");

        let expected: Vec<[f32; 3]> = positions(&base)
            .iter()
            .zip(target.positions.iter())
            .map(|(base_pos, target_pos)| {
                [
                    (base_pos[0] + target_pos[0]) / 2.0,
                    (base_pos[1] + target_pos[1]) / 2.0,
                    (base_pos[2] + target_pos[2]) / 2.0,
                ]
            })
            .collect();
        assert_eq!(positions(&blended), expected);
    }

    #[test]
    fn morph_blend_some_dois_alvos() {
        let base = base_cube();
        let alvo_x = target_shifted_x("deslocado-x");
        let alvo_y = target_shifted_y("deslocado-y");
        let blend = MorphBlend {
            targets: vec![alvo_x, alvo_y],
            weights: vec![1.0, 1.0],
        };

        let blended = base.morph_blend(&blend).expect("mistura válida");

        let expected: Vec<[f32; 3]> = positions(&base)
            .iter()
            .map(|base_pos| [base_pos[0] + 2.0, base_pos[1] - 2.0, base_pos[2]])
            .collect();
        assert_eq!(positions(&blended), expected);
    }

    #[test]
    fn morph_blend_recusa_lista_de_alvos_vazia() {
        let base = base_cube();
        let blend = MorphBlend {
            targets: Vec::new(),
            weights: Vec::new(),
        };

        let result = base.morph_blend(&blend);

        assert!(matches!(result, Err(MorphError::EmptyTargets)));
    }

    #[test]
    fn morph_blend_recusa_numero_de_pesos_diferente_do_numero_de_alvos() {
        let base = base_cube();
        let blend = MorphBlend {
            targets: vec![target_shifted_x("deslocado-x")],
            weights: Vec::new(),
        };

        let result = base.morph_blend(&blend);

        assert!(matches!(
            result,
            Err(MorphError::WeightCountMismatch {
                weights: 0,
                targets: 1
            })
        ));
    }

    #[test]
    fn morph_blend_recusa_peso_negativo() {
        let base = base_cube();
        let blend = MorphBlend {
            targets: vec![target_shifted_x("deslocado-x")],
            weights: vec![-0.5],
        };

        let result = base.morph_blend(&blend);

        assert!(matches!(result, Err(MorphError::NegativeWeight { weight, .. }) if weight < 0.0));
    }

    #[test]
    fn morph_blend_recusa_peso_nao_finito() {
        let base = base_cube();
        let blend = MorphBlend {
            targets: vec![target_shifted_x("deslocado-x")],
            weights: vec![f32::NAN],
        };

        let result = base.morph_blend(&blend);

        assert!(matches!(result, Err(MorphError::NonFiniteWeight { .. })));
    }

    #[test]
    fn morph_blend_recusa_alvo_com_contagem_de_vertices_diferente() {
        let base = base_cube();
        // Só 3 posições para uma malha de 8 vértices.
        let alvo = MorphTarget::new("incompleto", vec![[0.0, 0.0, 0.0]; 3]);
        let blend = MorphBlend {
            targets: vec![alvo],
            weights: vec![1.0],
        };

        let result = base.morph_blend(&blend);

        assert!(matches!(
            result,
            Err(MorphError::TargetVertexCountMismatch {
                count: 3,
                mesh_verts: 8,
                ..
            })
        ));
    }

    #[test]
    fn morph_blend_recusa_alvo_com_posicao_nao_finita() {
        let base = base_cube();
        let mut positions = positions(&base);
        positions[0][1] = f32::NAN;
        let blend = MorphBlend {
            targets: vec![MorphTarget::new("corrompido", positions)],
            weights: vec![1.0],
        };

        let result = base.morph_blend(&blend);

        assert!(matches!(
            result,
            Err(MorphError::NonFiniteTargetPosition { index: 0, .. })
        ));
    }

    #[test]
    fn morph_blend_nao_modifica_a_malha_original() {
        let base = base_cube();
        let antes = positions(&base);
        let faces_antes = base.faces.len();
        let blend = MorphBlend {
            targets: vec![target_shifted_x("deslocado-x")],
            weights: vec![1.0],
        };

        let mut blended = base.morph_blend(&blend).expect("mistura válida");

        assert_eq!(positions(&base), antes, "a malha base não pode mudar");
        assert_eq!(base.faces.len(), faces_antes);
        assert_ne!(positions(&blended), antes, "a cópia blendada é diferente");

        // Mexer na cópia tampouco afeta a base.
        if let Some(vertex) = blended.verts.first_mut() {
            vertex.pos = [99.0, 99.0, 99.0];
        }
        assert_eq!(positions(&base), antes);
    }
}
