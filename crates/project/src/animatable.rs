//! Registro das propriedades animáveis (ADR-C16, B4).
//!
//! Fonte única do que pode receber chave de animação: timeline, editor de curvas
//! (Onda C5), o diamante de chave dos campos e scripts leem daqui o id estável,
//! o dono, o tipo, a unidade e a faixa válida. Nada aqui avalia animação: é só o
//! contrato. Um valor fora da faixa é recusado com o motivo, nunca grampeado em
//! silêncio, e valores não finitos nunca entram.

use crate::animation::CurveChannel;

/// A quem a propriedade pertence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropertyOwner {
    Object,
    Bone,
    Material,
    Light,
}

impl PropertyOwner {
    pub fn id(self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Bone => "bone",
            Self::Material => "material",
            Self::Light => "light",
        }
    }
}

/// Forma do valor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropertyType {
    Scalar,
    Vector3,
    /// Euler XYZ em graus (a rotação autoral; o quaternion é derivado).
    EulerRotation,
    ColorRgb,
    ColorRgba,
    /// Liga/desliga: só chaves constantes (sem interpolar entre 0 e 1).
    Toggle,
}

impl PropertyType {
    pub fn id(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Vector3 => "vector3",
            Self::EulerRotation => "euler_rotation",
            Self::ColorRgb => "color_rgb",
            Self::ColorRgba => "color_rgba",
            Self::Toggle => "toggle",
        }
    }

    /// Quantos componentes escalares o valor tem (canais de curva).
    pub fn components(self) -> usize {
        match self {
            Self::Scalar | Self::Toggle => 1,
            Self::Vector3 | Self::EulerRotation | Self::ColorRgb => 3,
            Self::ColorRgba => 4,
        }
    }

    /// Interpolar entre chaves faz sentido? (Toggle só aceita chave constante.)
    pub fn interpolates(self) -> bool {
        !matches!(self, Self::Toggle)
    }
}

/// Unidade de exibição.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropertyUnit {
    Meter,
    Degree,
    /// Multiplicador (escala).
    Factor,
    /// 0..1 exibido como %.
    Fraction,
    /// Intensidade de luz/emissão (sem unidade física fixa).
    Intensity,
    None,
}

impl PropertyUnit {
    pub fn id(self) -> &'static str {
        match self {
            Self::Meter => "m",
            Self::Degree => "deg",
            Self::Factor => "factor",
            Self::Fraction => "fraction",
            Self::Intensity => "intensity",
            Self::None => "none",
        }
    }

    /// Sufixo curto para campos.
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Meter => "m",
            Self::Degree => "°",
            Self::Factor => "×",
            Self::Fraction => "%",
            Self::Intensity | Self::None => "",
        }
    }
}

/// Uma propriedade animável.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimatableProperty {
    /// Id estável (`dono.campo`), usado em arquivos, scripts e na timeline.
    pub id: &'static str,
    pub owner: PropertyOwner,
    /// Nome para a interface (pt-BR; a GUI traduz pelo catálogo).
    pub label: &'static str,
    pub value_type: PropertyType,
    pub unit: PropertyUnit,
    /// Faixa válida por componente (inclusive).
    pub min: f32,
    pub max: f32,
    /// Passo sugerido para arrastar/setas.
    pub step: f32,
    /// Zero é inválido (escala zero colapsa a malha).
    pub nonzero: bool,
}

/// Recusa de um valor para uma propriedade.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum AnimatableError {
    #[error("propriedade animável desconhecida: {0}")]
    Unknown(String),
    #[error("{property} espera {expected} componente(s), recebeu {got}")]
    Arity {
        property: &'static str,
        expected: usize,
        got: usize,
    },
    #[error("{property}[{component}] não é um número finito")]
    NonFinite {
        property: &'static str,
        component: usize,
    },
    #[error("{property}[{component}] = {value} fora da faixa {min}..={max}")]
    OutOfRange {
        property: &'static str,
        component: usize,
        value: f32,
        min: f32,
        max: f32,
    },
    #[error("{property}[{component}] não pode ser zero")]
    Zero {
        property: &'static str,
        component: usize,
    },
}

const fn prop(
    id: &'static str,
    owner: PropertyOwner,
    label: &'static str,
    value_type: PropertyType,
    unit: PropertyUnit,
    range: (f32, f32),
    step: f32,
) -> AnimatableProperty {
    AnimatableProperty {
        id,
        owner,
        label,
        value_type,
        unit,
        min: range.0,
        max: range.1,
        step,
        nonzero: false,
    }
}

const POSITION_RANGE: (f32, f32) = (-100_000.0, 100_000.0);
const ROTATION_RANGE: (f32, f32) = (-36_000.0, 36_000.0);
const SCALE_RANGE: (f32, f32) = (-10_000.0, 10_000.0);
const UNIT_RANGE: (f32, f32) = (0.0, 1.0);

/// O registro. A ordem é a de exibição (dono, depois transform antes de aparência).
pub const ANIMATABLE_PROPERTIES: &[AnimatableProperty] = &[
    prop(
        "object.position",
        PropertyOwner::Object,
        "Posição",
        PropertyType::Vector3,
        PropertyUnit::Meter,
        POSITION_RANGE,
        0.1,
    ),
    prop(
        "object.rotation",
        PropertyOwner::Object,
        "Rotação",
        PropertyType::EulerRotation,
        PropertyUnit::Degree,
        ROTATION_RANGE,
        1.0,
    ),
    AnimatableProperty {
        nonzero: true,
        ..prop(
            "object.scale",
            PropertyOwner::Object,
            "Escala",
            PropertyType::Vector3,
            PropertyUnit::Factor,
            SCALE_RANGE,
            0.1,
        )
    },
    prop(
        "object.visible",
        PropertyOwner::Object,
        "Visível",
        PropertyType::Toggle,
        PropertyUnit::None,
        UNIT_RANGE,
        1.0,
    ),
    prop(
        "bone.translation",
        PropertyOwner::Bone,
        "Posição do osso",
        PropertyType::Vector3,
        PropertyUnit::Meter,
        POSITION_RANGE,
        0.1,
    ),
    prop(
        "bone.rotation",
        PropertyOwner::Bone,
        "Rotação do osso",
        PropertyType::EulerRotation,
        PropertyUnit::Degree,
        ROTATION_RANGE,
        1.0,
    ),
    AnimatableProperty {
        nonzero: true,
        ..prop(
            "bone.scale",
            PropertyOwner::Bone,
            "Escala do osso",
            PropertyType::Vector3,
            PropertyUnit::Factor,
            SCALE_RANGE,
            0.1,
        )
    },
    prop(
        "material.base_color",
        PropertyOwner::Material,
        "Cor base",
        PropertyType::ColorRgba,
        PropertyUnit::Fraction,
        UNIT_RANGE,
        0.01,
    ),
    prop(
        "material.roughness",
        PropertyOwner::Material,
        "Rugosidade",
        PropertyType::Scalar,
        PropertyUnit::Fraction,
        UNIT_RANGE,
        0.01,
    ),
    prop(
        "material.metallic",
        PropertyOwner::Material,
        "Metálico",
        PropertyType::Scalar,
        PropertyUnit::Fraction,
        UNIT_RANGE,
        0.01,
    ),
    prop(
        "material.emission_color",
        PropertyOwner::Material,
        "Cor de emissão",
        PropertyType::ColorRgb,
        PropertyUnit::Fraction,
        UNIT_RANGE,
        0.01,
    ),
    prop(
        "material.emission_strength",
        PropertyOwner::Material,
        "Força da emissão",
        PropertyType::Scalar,
        PropertyUnit::Intensity,
        (0.0, 1000.0),
        0.1,
    ),
    prop(
        "light.color",
        PropertyOwner::Light,
        "Cor da luz",
        PropertyType::ColorRgb,
        PropertyUnit::Fraction,
        UNIT_RANGE,
        0.01,
    ),
    prop(
        "light.intensity",
        PropertyOwner::Light,
        "Intensidade",
        PropertyType::Scalar,
        PropertyUnit::Intensity,
        (0.0, 100_000.0),
        0.1,
    ),
    prop(
        "light.enabled",
        PropertyOwner::Light,
        "Acesa",
        PropertyType::Toggle,
        PropertyUnit::None,
        UNIT_RANGE,
        1.0,
    ),
];

/// Propriedade pelo id estável.
pub fn find(id: &str) -> Option<&'static AnimatableProperty> {
    ANIMATABLE_PROPERTIES
        .iter()
        .find(|property| property.id == id)
}

/// Propriedades de um dono, na ordem de exibição.
pub fn for_owner(owner: PropertyOwner) -> impl Iterator<Item = &'static AnimatableProperty> {
    ANIMATABLE_PROPERTIES
        .iter()
        .filter(move |property| property.owner == owner)
}

/// Canal de curva de osso → (propriedade, componente). Liga o editor de curvas
/// existente ao registro: o canal de curva é um componente de uma propriedade.
pub fn curve_channel_property(channel: CurveChannel) -> (&'static str, usize) {
    match channel {
        CurveChannel::TranslationX => ("bone.translation", 0),
        CurveChannel::TranslationY => ("bone.translation", 1),
        CurveChannel::TranslationZ => ("bone.translation", 2),
        CurveChannel::RotationX => ("bone.rotation", 0),
        CurveChannel::RotationY => ("bone.rotation", 1),
        CurveChannel::RotationZ => ("bone.rotation", 2),
        CurveChannel::ScaleX => ("bone.scale", 0),
        CurveChannel::ScaleY => ("bone.scale", 1),
        CurveChannel::ScaleZ => ("bone.scale", 2),
    }
}

impl AnimatableProperty {
    pub fn components(&self) -> usize {
        self.value_type.components()
    }

    /// Nome do componente para canais de curva (`X`, `R`, …; vazio se escalar).
    pub fn component_label(&self, component: usize) -> &'static str {
        let names: &[&str] = match self.value_type {
            PropertyType::Scalar | PropertyType::Toggle => &[""],
            PropertyType::Vector3 | PropertyType::EulerRotation => &["X", "Y", "Z"],
            PropertyType::ColorRgb => &["R", "G", "B"],
            PropertyType::ColorRgba => &["R", "G", "B", "A"],
        };
        names.get(component).copied().unwrap_or("")
    }

    /// Confere aridade, finitude, faixa e zero. Não altera o valor.
    pub fn validate(&self, value: &[f32]) -> Result<(), AnimatableError> {
        let expected = self.components();
        if value.len() != expected {
            return Err(AnimatableError::Arity {
                property: self.id,
                expected,
                got: value.len(),
            });
        }
        for (component, &v) in value.iter().enumerate() {
            if !v.is_finite() {
                return Err(AnimatableError::NonFinite {
                    property: self.id,
                    component,
                });
            }
            if v < self.min || v > self.max {
                return Err(AnimatableError::OutOfRange {
                    property: self.id,
                    component,
                    value: v,
                    min: self.min,
                    max: self.max,
                });
            }
            if self.nonzero && v.abs() < 1.0e-6 {
                return Err(AnimatableError::Zero {
                    property: self.id,
                    component,
                });
            }
            if self.value_type == PropertyType::Toggle && v != 0.0 && v != 1.0 {
                return Err(AnimatableError::OutOfRange {
                    property: self.id,
                    component,
                    value: v,
                    min: 0.0,
                    max: 1.0,
                });
            }
        }
        Ok(())
    }
}

/// `validate` pelo id.
pub fn validate(id: &str, value: &[f32]) -> Result<(), AnimatableError> {
    find(id)
        .ok_or_else(|| AnimatableError::Unknown(id.to_string()))?
        .validate(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique_ranges_sane_and_owners_prefix_the_id() {
        let mut seen = HashSet::new();
        for property in ANIMATABLE_PROPERTIES {
            assert!(seen.insert(property.id), "id repetido: {}", property.id);
            assert!(property.min < property.max, "{}", property.id);
            assert!(property.step > 0.0, "{}", property.id);
            assert!(
                property
                    .id
                    .starts_with(&format!("{}.", property.owner.id())),
                "{} não começa com o dono",
                property.id
            );
        }
    }

    #[test]
    fn every_curve_channel_maps_to_a_registered_component() {
        for channel in [
            CurveChannel::TranslationX,
            CurveChannel::TranslationY,
            CurveChannel::TranslationZ,
            CurveChannel::RotationX,
            CurveChannel::RotationY,
            CurveChannel::RotationZ,
            CurveChannel::ScaleX,
            CurveChannel::ScaleY,
            CurveChannel::ScaleZ,
        ] {
            let (id, component) = curve_channel_property(channel);
            let property = find(id).expect("canal mapeado no registro");
            assert!(component < property.components());
        }
    }

    #[test]
    fn hostile_values_are_refused_with_the_reason() {
        assert!(validate("object.position", &[1.0, 2.0, 3.0]).is_ok());
        assert!(matches!(
            validate("object.position", &[1.0, 2.0]),
            Err(AnimatableError::Arity {
                expected: 3,
                got: 2,
                ..
            })
        ));
        assert!(matches!(
            validate("object.rotation", &[f32::NAN, 0.0, 0.0]),
            Err(AnimatableError::NonFinite { component: 0, .. })
        ));
        assert!(matches!(
            validate("object.scale", &[1.0, 0.0, 1.0]),
            Err(AnimatableError::Zero { component: 1, .. })
        ));
        assert!(matches!(
            validate("material.roughness", &[1.5]),
            Err(AnimatableError::OutOfRange { .. })
        ));
        assert!(matches!(
            validate("object.visible", &[0.5]),
            Err(AnimatableError::OutOfRange { .. })
        ));
        assert!(matches!(
            validate("nada.disso", &[1.0]),
            Err(AnimatableError::Unknown(_))
        ));
        assert!(
            !find("object.visible")
                .expect("registrada")
                .value_type
                .interpolates()
        );
    }
}
