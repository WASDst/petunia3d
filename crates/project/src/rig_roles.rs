//! Rig Roles — semântica dos ossos (P3D-169, cap. 45 F0).
//!
//! Geradores de movimento, IK e retarget falam em **papéis** (`Hips`, coxa,
//! canela, cauda…), nunca em nomes de osso nem em espécies. Um [`RigRoleMap`]
//! liga `bone_id → RigRole` para um `Skeleton`; presets e importadores o
//! preenchem por [`RigRoleMap::infer`] e o usuário pode corrigir à mão.
//!
//! Persistência: o mapa vive em `Project::rig_roles` (campo append-only), não
//! dentro de `Skeleton`, para não deslocar o layout postcard legado.

use crate::rig::Skeleton;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Parte de uma perna.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LimbPart {
    /// Segmento superior (coxa, `Coxa`, perna dianteira/traseira do quadrúpede).
    Thigh,
    /// Segmento inferior (canela, `Tibia`, pata do quadrúpede).
    Shin,
    /// Pé (opcional: o efetuador do IK é a ponta da canela ou a cabeça do pé).
    Foot,
    /// Dedos/ponta do pé.
    Toe,
}

/// Parte de um braço.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArmPart {
    Shoulder,
    Upper,
    Lower,
    Hand,
}

/// Papel semântico de um osso. Índices (`limb`, `n`) começam em 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RigRole {
    Root,
    Hips,
    /// Segmento `n` da coluna, da base para a cabeça.
    Spine(u8),
    Chest,
    Neck,
    Head,
    Jaw,
    Leg {
        limb: u8,
        part: LimbPart,
    },
    Arm {
        limb: u8,
        part: ArmPart,
    },
    Wing {
        limb: u8,
        segment: u8,
    },
    /// Segmento `n` da cauda, da base para a ponta.
    Tail(u8),
    Tentacle {
        limb: u8,
        segment: u8,
    },
    /// Segmento `n` de um acessório com movimento secundário (P3D-173).
    Wiggle(u8),
}

/// Perna completa (coxa e canela obrigatórias).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegChain {
    pub limb: u8,
    pub thigh: u32,
    pub shin: u32,
    pub foot: Option<u32>,
}

/// Erros de atribuição de papéis.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoleError {
    #[error("osso {0} não existe no esqueleto")]
    BoneNotFound(u32),
    #[error("o papel {role:?} já pertence ao osso '{held_by}'")]
    RoleTaken { role: RigRole, held_by: String },
    #[error("o mapa de papéis pertence a outro esqueleto")]
    WrongSkeleton,
}

/// Contrato de rig que um gerador exige (P3D-169).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RigRequirement {
    /// Ao menos `min` pernas completas (coxa + canela).
    Legs {
        min: usize,
    },
    /// Cadeia de `min_len` ossos ou mais (`Spine`, `Tail`).
    Spine {
        min_len: usize,
    },
    Tail {
        min_len: usize,
    },
    /// Papéis avulsos obrigatórios.
    Roles(Vec<RigRole>),
}

/// Rig que não satisfaz o contrato de um gerador (mensagem legível).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoleContractError {
    #[error(
        "este movimento precisa de {needed} ou mais pernas com coxa e canela; o rig tem {found}"
    )]
    NotEnoughLegs { needed: usize, found: usize },
    #[error("este movimento precisa de uma {chain} com {needed} ou mais ossos; o rig tem {found}")]
    ChainTooShort {
        chain: &'static str,
        needed: usize,
        found: usize,
    },
    #[error("falta o papel {0:?} no rig")]
    MissingRole(RigRole),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAssignment {
    pub bone_id: u32,
    pub role: RigRole,
}

/// Papéis dos ossos de um esqueleto. Cada papel pertence a no máximo um osso.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RigRoleMap {
    pub skeleton_id: Uuid,
    entries: Vec<RoleAssignment>,
    #[serde(default)]
    pub revision: u64,
}

impl RigRoleMap {
    pub fn new(skeleton_id: Uuid) -> Self {
        Self {
            skeleton_id,
            entries: Vec::new(),
            revision: 0,
        }
    }

    pub fn entries(&self) -> &[RoleAssignment] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn role_of(&self, bone_id: u32) -> Option<RigRole> {
        self.entries
            .iter()
            .find(|e| e.bone_id == bone_id)
            .map(|e| e.role)
    }

    pub fn bone_of(&self, role: RigRole) -> Option<u32> {
        self.entries
            .iter()
            .find(|e| e.role == role)
            .map(|e| e.bone_id)
    }

    /// Atribui `role` ao osso, substituindo o papel anterior do osso. Falha se o
    /// papel já pertencer a **outro** osso (nomeando-o) ou se o osso não existir.
    pub fn assign(
        &mut self,
        skeleton: &Skeleton,
        bone_id: u32,
        role: RigRole,
    ) -> Result<(), RoleError> {
        if skeleton.id != self.skeleton_id {
            return Err(RoleError::WrongSkeleton);
        }
        if skeleton.get_bone(bone_id).is_none() {
            return Err(RoleError::BoneNotFound(bone_id));
        }
        if let Some(holder) = self.bone_of(role)
            && holder != bone_id
        {
            return Err(RoleError::RoleTaken {
                role,
                held_by: skeleton
                    .get_bone(holder)
                    .map_or_else(|| holder.to_string(), |b| b.name.clone()),
            });
        }
        if self.role_of(bone_id) == Some(role) {
            return Ok(());
        }
        self.entries.retain(|e| e.bone_id != bone_id);
        self.entries.push(RoleAssignment { bone_id, role });
        self.entries.sort_by_key(|e| (e.role, e.bone_id));
        self.revision += 1;
        Ok(())
    }

    /// Remove o papel do osso. `false` se ele não tinha papel.
    pub fn clear(&mut self, bone_id: u32) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.bone_id != bone_id);
        let changed = self.entries.len() != before;
        if changed {
            self.revision += 1;
        }
        changed
    }

    /// Remove entradas de ossos que não existem mais no esqueleto.
    pub fn prune(&mut self, skeleton: &Skeleton) {
        let before = self.entries.len();
        self.entries
            .retain(|e| skeleton.get_bone(e.bone_id).is_some());
        if self.entries.len() != before {
            self.revision += 1;
        }
    }

    /// Pernas completas (coxa + canela), ordenadas por `limb`.
    pub fn legs(&self) -> Vec<LegChain> {
        let mut limbs: Vec<u8> = self
            .entries
            .iter()
            .filter_map(|e| match e.role {
                RigRole::Leg { limb, .. } => Some(limb),
                _ => None,
            })
            .collect();
        limbs.sort_unstable();
        limbs.dedup();
        limbs
            .into_iter()
            .filter_map(|limb| {
                let part = |p| self.bone_of(RigRole::Leg { limb, part: p });
                Some(LegChain {
                    limb,
                    thigh: part(LimbPart::Thigh)?,
                    shin: part(LimbPart::Shin)?,
                    foot: part(LimbPart::Foot),
                })
            })
            .collect()
    }

    fn chain(&self, pick: impl Fn(RigRole) -> Option<u8>) -> Vec<u32> {
        let mut v: Vec<(u8, u32)> = self
            .entries
            .iter()
            .filter_map(|e| pick(e.role).map(|n| (n, e.bone_id)))
            .collect();
        v.sort_unstable();
        v.into_iter().map(|(_, b)| b).collect()
    }

    /// Ossos da coluna, da base para a cabeça.
    pub fn spine_chain(&self) -> Vec<u32> {
        self.chain(|r| match r {
            RigRole::Spine(n) => Some(n),
            _ => None,
        })
    }

    /// Ossos da cauda, da base para a ponta.
    pub fn tail_chain(&self) -> Vec<u32> {
        self.chain(|r| match r {
            RigRole::Tail(n) => Some(n),
            _ => None,
        })
    }

    /// Verifica um contrato de rig (P3D-169); o primeiro requisito violado vence.
    pub fn require(&self, req: &RigRequirement) -> Result<(), RoleContractError> {
        match req {
            RigRequirement::Legs { min } => {
                let found = self.legs().len();
                if found < *min {
                    return Err(RoleContractError::NotEnoughLegs {
                        needed: *min,
                        found,
                    });
                }
            }
            RigRequirement::Spine { min_len } => {
                let found = self.spine_chain().len();
                if found < *min_len {
                    return Err(RoleContractError::ChainTooShort {
                        chain: "coluna",
                        needed: *min_len,
                        found,
                    });
                }
            }
            RigRequirement::Tail { min_len } => {
                let found = self.tail_chain().len();
                if found < *min_len {
                    return Err(RoleContractError::ChainTooShort {
                        chain: "cauda",
                        needed: *min_len,
                        found,
                    });
                }
            }
            RigRequirement::Roles(roles) => {
                if let Some(missing) = roles.iter().find(|r| self.bone_of(**r).is_none()) {
                    return Err(RoleContractError::MissingRole(*missing));
                }
            }
        }
        Ok(())
    }

    pub fn require_all(&self, reqs: &[RigRequirement]) -> Result<(), RoleContractError> {
        reqs.iter().try_for_each(|r| self.require(r))
    }

    /// Infere papéis a partir dos nomes: presets canônicos do Petunia (humanoide,
    /// quadrúpede, multi-leg) e nomes no estilo Mixamo/genérico. Nomes que não
    /// reconhece ficam sem papel (o usuário corrige à mão).
    pub fn infer(skeleton: &Skeleton) -> Self {
        let mut map = Self::new(skeleton.id);
        for bone in &skeleton.bones {
            if let Some(role) = role_from_name(&bone.name) {
                // Nomes ambíguos (dois ossos com o mesmo papel): o primeiro vence.
                let _ = map.assign(skeleton, bone.id, role);
            }
        }
        map.revision = 0;
        map
    }

    /// Papéis por nome de osso (estável entre renumerações de id): usado no glTF.
    pub fn by_bone_name(&self, skeleton: &Skeleton) -> Vec<(String, RigRole)> {
        self.entries
            .iter()
            .filter_map(|e| {
                skeleton
                    .get_bone(e.bone_id)
                    .map(|b| (b.name.clone(), e.role))
            })
            .collect()
    }

    /// Reconstrói o mapa a partir de pares (nome do osso, papel); ignora nomes
    /// desconhecidos e papéis repetidos.
    pub fn from_names(skeleton: &Skeleton, pairs: &[(String, RigRole)]) -> Self {
        let mut map = Self::new(skeleton.id);
        for (name, role) in pairs {
            if let Some(id) = skeleton.find_bone(name) {
                let _ = map.assign(skeleton, id, *role);
            }
        }
        map.revision = 0;
        map
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

/// Separa lado (`.L`, `_R`, `Left…`) e devolve o resto em minúsculas.
fn split_side(name: &str) -> (Option<Side>, String) {
    let mut n = name.trim().to_ascii_lowercase();
    if let Some((_, rest)) = n.split_once(':') {
        n = rest.to_string(); // `mixamorig:LeftArm`
    }
    for (suffix, side) in [
        (".l", Side::Left),
        ("_l", Side::Left),
        (".left", Side::Left),
        (".r", Side::Right),
        ("_r", Side::Right),
        (".right", Side::Right),
    ] {
        if let Some(base) = n.strip_suffix(suffix) {
            return (Some(side), base.to_string());
        }
    }
    if let Some(base) = n.strip_prefix("left") {
        return (
            Some(Side::Left),
            base.trim_start_matches(['_', '.']).to_string(),
        );
    }
    if let Some(base) = n.strip_prefix("right") {
        return (
            Some(Side::Right),
            base.trim_start_matches(['_', '.']).to_string(),
        );
    }
    (None, n)
}

fn side_index(side: Option<Side>) -> u8 {
    u8::from(side == Some(Side::Right))
}

/// `tail`, `tail_2`, `tail.2` → índice do segmento.
fn numbered(base: &str, prefix: &str) -> Option<u8> {
    let rest = base.strip_prefix(prefix)?;
    if rest.is_empty() {
        return Some(0);
    }
    rest.trim_start_matches(['_', '.']).parse().ok()
}

fn role_from_name(name: &str) -> Option<RigRole> {
    let (side, base) = split_side(name);
    let b = base.as_str();

    // Corpo
    match b {
        "root" | "thorax" | "pelvis_root" => return Some(RigRole::Root),
        "hips" | "pelvis" => return Some(RigRole::Hips),
        "chest" | "spine2" | "upperchest" => return Some(RigRole::Chest),
        "neck" => return Some(RigRole::Neck),
        "head" => return Some(RigRole::Head),
        "jaw" => return Some(RigRole::Jaw),
        _ => {}
    }
    if let Some(n) = numbered(b, "spine") {
        return Some(RigRole::Spine(n)); // `spine`, `spine1` (Mixamo), `spine_2`
    }
    if let Some(n) = numbered(b, "tail") {
        return Some(RigRole::Tail(n));
    }
    if let Some(n) = numbered(b, "wing") {
        return Some(RigRole::Wing {
            limb: side_index(side),
            segment: n,
        });
    }

    // Multi-leg: `leg_{i}_coxa` / `leg_{i}_tibia`
    if let Some(rest) = b.strip_prefix("leg_")
        && let Some((idx, part)) = rest.split_once('_')
        && let Ok(i) = idx.parse::<u8>()
    {
        let limb = i.saturating_mul(2).saturating_add(side_index(side));
        let part = match part {
            "coxa" | "femur" => LimbPart::Thigh,
            "tibia" => LimbPart::Shin,
            "tarsus" | "foot" => LimbPart::Foot,
            _ => return None,
        };
        return Some(RigRole::Leg { limb, part });
    }

    // Quadrúpede: `LegFront/LegBack` + `PawFront/PawBack`
    for (key, base_limb, part) in [
        ("legfront", 0u8, LimbPart::Thigh),
        ("pawfront", 0, LimbPart::Shin),
        ("legback", 2, LimbPart::Thigh),
        ("pawback", 2, LimbPart::Shin),
    ] {
        if b == key {
            return Some(RigRole::Leg {
                limb: base_limb + side_index(side),
                part,
            });
        }
    }

    // Humanoide / Mixamo / genérico
    let limb = side_index(side);
    let leg = |part| Some(RigRole::Leg { limb, part });
    let arm = |part| Some(RigRole::Arm { limb, part });
    match b {
        "upperleg" | "upleg" | "thigh" => leg(LimbPart::Thigh),
        "lowerleg" | "leg" | "calf" | "shin" => leg(LimbPart::Shin),
        "foot" => leg(LimbPart::Foot),
        "toe" | "toebase" | "toes" => leg(LimbPart::Toe),
        "shoulder" | "clavicle" => arm(ArmPart::Shoulder),
        "upperarm" | "arm" => arm(ArmPart::Upper),
        "lowerarm" | "forearm" => arm(ArmPart::Lower),
        "hand" => arm(ArmPart::Hand),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Project, RigPreset};

    fn role(sk: &Skeleton, map: &RigRoleMap, name: &str) -> Option<RigRole> {
        map.role_of(sk.find_bone(name).unwrap_or_else(|| panic!("osso {name}")))
    }

    #[test]
    fn humanoid_preset_roles_are_inferred() {
        let sk = RigPreset::humanoid(1.0);
        let m = RigRoleMap::infer(&sk);
        assert_eq!(role(&sk, &m, "Hips"), Some(RigRole::Hips));
        assert_eq!(role(&sk, &m, "Spine"), Some(RigRole::Spine(0)));
        assert_eq!(role(&sk, &m, "Chest"), Some(RigRole::Chest));
        assert_eq!(role(&sk, &m, "Head"), Some(RigRole::Head));
        assert_eq!(
            role(&sk, &m, "UpperLeg.R"),
            Some(RigRole::Leg {
                limb: 1,
                part: LimbPart::Thigh
            })
        );
        assert_eq!(
            role(&sk, &m, "Hand.L"),
            Some(RigRole::Arm {
                limb: 0,
                part: ArmPart::Hand
            })
        );
        let legs = m.legs();
        assert_eq!(legs.len(), 2);
        assert!(legs.iter().all(|l| l.foot.is_some()));
        assert_eq!(legs[0].thigh, sk.find_bone("UpperLeg.L").unwrap());
        assert_eq!(legs[0].shin, sk.find_bone("LowerLeg.L").unwrap());
    }

    #[test]
    fn quadruped_and_multi_leg_presets_have_all_legs() {
        let q = RigPreset::quadruped(1.0);
        let m = RigRoleMap::infer(&q);
        assert_eq!(m.legs().len(), 4);
        assert_eq!(m.tail_chain(), vec![q.find_bone("Tail").unwrap()]);
        assert_eq!(m.spine_chain(), vec![q.find_bone("Spine").unwrap()]);
        assert_eq!(role(&q, &m, "Root"), Some(RigRole::Root));

        let spider = RigPreset::multi_leg(8, 1.0);
        let m = RigRoleMap::infer(&spider);
        let legs = m.legs();
        assert_eq!(legs.len(), 8);
        let limbs: Vec<u8> = legs.iter().map(|l| l.limb).collect();
        assert_eq!(limbs, (0..8).collect::<Vec<u8>>());
        assert!(legs.iter().all(|l| l.foot.is_none()));
    }

    #[test]
    fn serpent_fish_and_bird_presets_have_their_roles() {
        let snake = RigPreset::serpent(12, 1.0);
        let m = RigRoleMap::infer(&snake);
        assert_eq!(m.spine_chain().len(), 6);
        assert_eq!(m.tail_chain().len(), 6);
        assert_eq!(role(&snake, &m, "Root"), Some(RigRole::Root));
        assert_eq!(role(&snake, &m, "Head"), Some(RigRole::Head));
        assert_eq!(role(&snake, &m, "Spine_3"), Some(RigRole::Spine(3)));
        assert!(m.legs().is_empty());
        snake.validate().unwrap();

        let fish = RigPreset::fish(1.0);
        let m = RigRoleMap::infer(&fish);
        assert_eq!(m.spine_chain().len() + m.tail_chain().len(), 9);
        assert_eq!(role(&fish, &m, "Fin.L"), None, "nadadeiras sem papel");

        let bird = RigPreset::bird(1.0);
        bird.validate().unwrap();
        let m = RigRoleMap::infer(&bird);
        assert_eq!(m.legs().len(), 2);
        assert!(m.legs().iter().all(|l| l.foot.is_some()));
        assert_eq!(m.tail_chain().len(), 2);
        assert_eq!(role(&bird, &m, "Hips"), Some(RigRole::Hips));
        assert_eq!(role(&bird, &m, "Chest"), Some(RigRole::Chest));
        assert_eq!(
            role(&bird, &m, "Wing_2.R"),
            Some(RigRole::Wing {
                limb: 1,
                segment: 2
            })
        );
        assert_eq!(
            role(&bird, &m, "Wing_0.L"),
            Some(RigRole::Wing {
                limb: 0,
                segment: 0
            })
        );
    }

    #[test]
    fn mixamo_and_generic_names_are_recognised() {
        let mut sk = Skeleton::new("Mixamo");
        let mut add = |n: &str, p: Option<u32>, y: f32| {
            sk.add_bone(n, p, [0.0, y, 0.0], [0.0, y + 0.1, 0.0])
                .unwrap()
        };
        let hips = add("mixamorig:Hips", None, 1.0);
        let spine = add("mixamorig:Spine", Some(hips), 1.1);
        let s1 = add("mixamorig:Spine1", Some(spine), 1.2);
        let s2 = add("mixamorig:Spine2", Some(s1), 1.3);
        add("mixamorig:Neck", Some(s2), 1.4);
        add("mixamorig:LeftUpLeg", Some(hips), 0.9);
        add("mixamorig:LeftLeg", Some(hips), 0.5);
        add("mixamorig:LeftFoot", Some(hips), 0.1);
        add("mixamorig:RightArm", Some(s2), 1.35);
        add("mixamorig:RightForeArm", Some(s2), 1.3);
        add("Whatever", Some(s2), 1.3);
        let m = RigRoleMap::infer(&sk);
        assert_eq!(role(&sk, &m, "mixamorig:Spine2"), Some(RigRole::Chest));
        assert_eq!(role(&sk, &m, "mixamorig:Spine1"), Some(RigRole::Spine(1)));
        assert_eq!(
            role(&sk, &m, "mixamorig:LeftLeg"),
            Some(RigRole::Leg {
                limb: 0,
                part: LimbPart::Shin
            })
        );
        assert_eq!(
            role(&sk, &m, "mixamorig:RightArm"),
            Some(RigRole::Arm {
                limb: 1,
                part: ArmPart::Upper
            })
        );
        assert_eq!(role(&sk, &m, "Whatever"), None);
        // Só a perna esquerda tem coxa, canela e pé; o braço direito é parcial.
        let legs = m.legs();
        assert_eq!(legs.len(), 1);
        assert_eq!(legs[0].limb, 0);
        assert!(legs[0].foot.is_some());
    }

    #[test]
    fn assignment_rules_and_revisions() {
        let sk = RigPreset::humanoid(1.0);
        let mut m = RigRoleMap::infer(&sk);
        let head = sk.find_bone("Head").unwrap();
        let neck = sk.find_bone("Neck").unwrap();
        assert_eq!(m.revision, 0);

        // Papel de outro osso: erro que nomeia o dono.
        let err = m.assign(&sk, neck, RigRole::Head).unwrap_err();
        assert_eq!(
            err,
            RoleError::RoleTaken {
                role: RigRole::Head,
                held_by: "Head".into()
            }
        );
        assert_eq!(
            m.assign(&sk, 999, RigRole::Jaw),
            Err(RoleError::BoneNotFound(999))
        );
        let other = RigPreset::quadruped(1.0);
        assert_eq!(
            m.assign(&other, 0, RigRole::Jaw),
            Err(RoleError::WrongSkeleton)
        );

        // Reatribuir o mesmo papel ao mesmo osso não muda nada.
        assert!(m.assign(&sk, head, RigRole::Head).is_ok());
        assert_eq!(m.revision, 0);

        // Trocar o papel do osso libera o antigo.
        m.assign(&sk, head, RigRole::Jaw).unwrap();
        assert_eq!(m.role_of(head), Some(RigRole::Jaw));
        assert_eq!(m.bone_of(RigRole::Head), None);
        assert_eq!(m.revision, 1);
        assert!(m.clear(head));
        assert!(!m.clear(head));
        assert_eq!(m.revision, 2);
    }

    #[test]
    fn requirements_report_readable_errors() {
        let human = RigRoleMap::infer(&RigPreset::humanoid(1.0));
        assert!(human.require(&RigRequirement::Legs { min: 2 }).is_ok());
        let err = human.require(&RigRequirement::Legs { min: 4 }).unwrap_err();
        assert_eq!(
            err,
            RoleContractError::NotEnoughLegs {
                needed: 4,
                found: 2
            }
        );
        assert!(err.to_string().contains("4 ou mais pernas"));

        assert_eq!(
            human.require(&RigRequirement::Tail { min_len: 1 }),
            Err(RoleContractError::ChainTooShort {
                chain: "cauda",
                needed: 1,
                found: 0
            })
        );
        assert_eq!(
            human.require(&RigRequirement::Roles(vec![RigRole::Hips, RigRole::Jaw])),
            Err(RoleContractError::MissingRole(RigRole::Jaw))
        );
        assert!(
            human
                .require_all(&[
                    RigRequirement::Legs { min: 2 },
                    RigRequirement::Spine { min_len: 1 }
                ])
                .is_ok()
        );
    }

    #[test]
    fn project_infers_persists_and_prunes_roles() {
        let mut p = Project::new();
        let sk = RigPreset::humanoid(1.0);
        let id = sk.id;
        p.add_skeleton(sk);
        assert_eq!(p.rig_roles_of(id).unwrap().legs().len(), 2);

        // Serde round-trip e retrocompatibilidade (JSON antigo sem `rig_roles`).
        let json = serde_json::to_value(&p).unwrap();
        let back: Project = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(back.rig_roles, p.rig_roles);
        let mut old = json;
        old.as_object_mut().unwrap().remove("rig_roles");
        let legacy: Project = serde_json::from_value(old).unwrap();
        assert!(legacy.rig_roles.is_empty());

        // Papel manual sobrevive; Undo/redo é clone do Project.
        let hand = p.skeletons[0].find_bone("Hand.L").unwrap();
        assert!(p.assign_rig_role(id, hand, RigRole::Wiggle(0)).unwrap());
        assert!(!p.assign_rig_role(id, hand, RigRole::Wiggle(0)).unwrap());
        assert_eq!(
            p.rig_roles_of(id).unwrap().role_of(hand),
            Some(RigRole::Wiggle(0))
        );
        assert!(p.clear_rig_role(id, hand));

        // Redetectar restaura o papel inferido.
        assert!(p.infer_rig_roles(id));
        assert!(!p.infer_rig_roles(id));
        assert_eq!(
            p.rig_roles_of(id).unwrap().role_of(hand),
            Some(RigRole::Arm {
                limb: 0,
                part: ArmPart::Hand
            })
        );

        // Osso removido: `validate` poda a entrada; esqueleto removido: some o mapa.
        p.skeletons[0].remove_bone(hand).unwrap();
        p.validate();
        assert_eq!(p.rig_roles_of(id).unwrap().role_of(hand), None);
        p.remove_skeleton(id);
        assert!(p.rig_roles_of(id).is_none());
    }
}
