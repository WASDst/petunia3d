//! Percepção espacial, assistência visual e acessibilidade posterior (H33 / VSP-001/002/004, SEL-007).
//!
//! Reúne:
//! 1. VSP-001: Perfis de assistência visual (`VisualAssistanceProfile`): Essential, Assisted, Analytical.
//!    Assisted é o default do fluxo iniciante. Preferência manual prevalece e nenhum perfil
//!    modifica atalhos de teclado (keymap), ferramentas ativas ou o documento.
//! 2. VSP-002: Modos Clay (`ClayMode`): Disabled, Soft, Form, Technical, MatCap.
//!    Ancoragem de iluminação de modelagem (`LightingAnchor`): CameraRelative vs WorldFixed.
//!    Modos de trabalho na viewport desacoplados dos materiais/luzes exportados do projeto.
//! 3. VSP-004: Companion View (`CompanionViewState`, `CompanionViewTrigger`):
//!    Visualização auxiliar desacoplada com câmera independente (ortográfica por padrão),
//!    sem roubo de foco (`steals_focus == false`) e abertura manual/sugerida (auto-abertura apenas opt-in).
//!    Desacoplamento estrito entre X-Ray visual e Select Through.
//! 4. SEL-007: Universal Gizmo acessível e feedback semântico (`GizmoAccessibilityFeedback`):
//!    Eixos e planos identificados textualmente e por formas (não apenas por cor),
//!    leitura numérica com unidades, anúncios para leitores de tela e alças com área
//!    mínima de toque acessível (hit targets >= 24dp).

use serde::{Deserialize, Serialize};

use crate::modal::{ModalConstraint, ModalKind, ModalOp};
use crate::state::TransformOrientation;

/// Perfis de assistência visual na viewport (VSP-001).
///
/// Controlam a densidade de informações e sobreposições visuais projetadas na viewport.
/// Invariante estrita: preferências manuais prevalecem e nenhum perfil altera o keymap,
/// ferramentas ativas ou o documento (`Project`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VisualAssistanceProfile {
    /// Auxílios visuais mínimos; viewport desobstruída para modeladores experientes.
    Essential,
    /// Padrão do fluxo iniciante e modelagem padrão: guias de alinhamento, badges de contexto e dicas.
    #[default]
    Assisted,
    /// Auxílios analíticos detalhados para engenharia e inspeção de precisão: cotas completas,
    /// frames espaciais, indicadores de tolerância e linhas de referência.
    Analytical,
}

impl VisualAssistanceProfile {
    /// Indica se guias visuais de alinhamento e snapping devem ser projetados.
    pub fn guidance_overlays_enabled(&self) -> bool {
        matches!(self, Self::Assisted | Self::Analytical)
    }

    /// Indica se cotas dimensionais completas devem ser projetadas por padrão.
    pub fn dimension_annotations_enabled(&self) -> bool {
        matches!(self, Self::Analytical)
    }

    /// Indica se caixas delimitadoras de frames espaciais (OBB / PCA) devem ser projetadas.
    pub fn spatial_frames_enabled(&self) -> bool {
        matches!(self, Self::Analytical)
    }

    /// Indica se badges com atalhos e dicas contextuais devem ser exibidos na viewport.
    pub fn hint_badges_enabled(&self) -> bool {
        matches!(self, Self::Assisted)
    }

    /// Rótulo legível para interface de usuário e menus.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Essential => "Essential",
            Self::Assisted => "Assisted",
            Self::Analytical => "Analytical",
        }
    }
}

/// Modos Clay de sombreamento de trabalho na viewport (VSP-002).
///
/// Modos puramente visuais e interativos na viewport, totalmente desacoplados
/// dos materiais, texturas e luzes de produção ou exportação do documento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClayMode {
    /// Modo padrão desativado: usa materiais normais do documento ou sombreamento PBR/Flat/Smooth.
    #[default]
    Disabled,
    /// Sombreamento fosco de baixo contraste realçando curvatura suave e microformas orgânicas.
    Soft,
    /// Argila neutra de alto contraste enfatizando macroforma, distribuição de volume, planos e silhueta.
    Form,
    /// Renderização técnica destacando silhueta, arestas características e fluxo de malha com albedo neutro.
    Technical,
    /// MatCap de estúdio (esfera de reflexão ambiente) para escultura rápida e avaliação volumétrica.
    MatCap,
}

impl ClayMode {
    /// Indica se o modo Clay está ativo.
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Rótulo legível para interface e menus.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Disabled => "Disabled",
            Self::Soft => "Clay Soft",
            Self::Form => "Clay Form",
            Self::Technical => "Clay Technical",
            Self::MatCap => "Clay MatCap",
        }
    }
}

/// Ancoragem da fonte de iluminação de modelagem na viewport (VSP-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LightingAnchor {
    /// Luz se move com a câmera (efeito lanterna frontal; ideal para iluminar cavidades e reentrâncias).
    #[default]
    CameraRelative,
    /// Direção de luz fixa no espaço mundial (sombras consistentes e percepção de volume estável durante órbita).
    WorldFixed,
}

impl LightingAnchor {
    pub fn label(&self) -> &'static str {
        match self {
            Self::CameraRelative => "Camera Relative",
            Self::WorldFixed => "World Fixed",
        }
    }
}

/// Configurações do modo Clay de visualização da viewport (VSP-002).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaySettings {
    /// Modo clay ativo.
    pub mode: ClayMode,
    /// Ancoragem da iluminação.
    pub lighting: LightingAnchor,
    /// Exibir arestas características (feature edges) com largura constante.
    pub show_feature_edges: bool,
    /// Realce de cavidades e oclusão de contato (0.0..=1.0).
    pub cavity_enhancement: f32,
    /// Tonalidade base do albedo neutro [R, G, B].
    pub albedo_tint: [f32; 3],
}

impl Default for ClaySettings {
    fn default() -> Self {
        Self {
            mode: ClayMode::Disabled,
            lighting: LightingAnchor::CameraRelative,
            show_feature_edges: true,
            cavity_enhancement: 0.35,
            albedo_tint: [0.82, 0.80, 0.78],
        }
    }
}

/// Modos de disparo / abertura da Companion View (VSP-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CompanionViewTrigger {
    /// Abertura explicitamente manual pelo usuário via menu, atalho ou botão.
    #[default]
    Manual,
    /// Sugerida via notificação ou badge não-intrusivo em operações que se beneficiam de visualização ortogonal.
    Suggested,
    /// Auto-abertura opt-in acionada apenas quando o usuário habilitou expressamente essa preferência.
    OptInAuto,
}

/// Feedback semântico acessível do Universal Gizmo (SEL-007).
///
/// Garante conformidade com diretrizes de acessibilidade e neurodivergência:
/// 1. Não depende exclusivamente de cores para identificação de eixos (sempre expõe rótulos textuais e glifos).
/// 2. Fornece strings semânticas formatadas para leitores de tela e logs de acessibilidade.
/// 3. Garante alças com área de toque mínima acessível (hit target >= 24dp).
/// 4. Compatível com temas de alto contraste (contrast ratio >= 4.5:1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GizmoAccessibilityFeedback {
    /// Modo de transformação modal ativo ("Move", "Rotate", "Scale", "Idle").
    pub mode: String,
    /// Eixo ou plano ativo ("X", "Y", "Z", "XY", "XZ", "YZ", "Screen", "None").
    pub active_axis: String,
    /// Orientação ativa do frame de transformação ("Global", "Local", "View", "Custom", "Normal", "Auto").
    pub frame_orientation: String,
    /// Leitura numérica formatada do valor primário (ex.: "+1.500 m", "45.0°", "1.250x").
    pub numeric_readout: String,
    /// Leitura dos componentes delta em 3 eixos.
    pub delta_text: String,
    /// Texto descritivo para tecnologia assistiva / leitores de tela.
    pub screen_reader_announcement: String,
    /// Dimensão mínima da área de seleção da alça em densidade independente de pixel (DP).
    pub minimum_hit_target_dp: f32,
    /// Sinaliza conformidade com alto contraste e diferenciação não-cromática de eixos.
    pub high_contrast_compliant: bool,
}

/// Avalia o feedback semântico acessível do Gizmo com base no estado modal atual (SEL-007).
pub fn evaluate_gizmo_accessibility(
    modal: Option<&ModalOp>,
    orientation: TransformOrientation,
    snap_enabled: bool,
) -> GizmoAccessibilityFeedback {
    let frame_str = format!("{orientation:?}");

    let Some(op) = modal else {
        return GizmoAccessibilityFeedback {
            mode: "Idle".to_string(),
            active_axis: "None".to_string(),
            frame_orientation: frame_str.clone(),
            numeric_readout: "0.000".to_string(),
            delta_text: "Nenhum deslocamento ativo".to_string(),
            screen_reader_announcement: format!(
                "Universal Gizmo pronto em referencial {frame_str}. Snap: {}.",
                if snap_enabled { "ativo" } else { "desativado" }
            ),
            minimum_hit_target_dp: 24.0,
            high_contrast_compliant: true,
        };
    };

    let mode_str = op.kind.label().to_string();

    let (axis_str, axis_speech) = match op.constraint {
        ModalConstraint::Free => ("Screen / Free".to_string(), "livre na tela".to_string()),
        ModalConstraint::Axis(0) => ("X".to_string(), "eixo X".to_string()),
        ModalConstraint::Axis(1) => ("Y".to_string(), "eixo Y".to_string()),
        ModalConstraint::Axis(2) => ("Z".to_string(), "eixo Z".to_string()),
        ModalConstraint::Axis(other) => (format!("Axis {other}"), format!("eixo {other}")),
        ModalConstraint::Plane(0) => ("YZ Plane".to_string(), "plano YZ".to_string()),
        ModalConstraint::Plane(1) => ("XZ Plane".to_string(), "plano XZ".to_string()),
        ModalConstraint::Plane(2) => ("XY Plane".to_string(), "plano XY".to_string()),
        ModalConstraint::Plane(other) => (format!("Plane {other}"), format!("plano {other}")),
    };

    let (numeric_readout, delta_text, speech_val) = match op.kind {
        ModalKind::Move => {
            let val = op.value;
            let comp = op.components;
            let readout = format!("{val:+.3} m");
            let delta = format!(
                "dx: {:+.3}, dy: {:+.3}, dz: {:+.3} m",
                comp.x, comp.y, comp.z
            );
            let speech = format!("deslocamento de {val:+.3} metros");
            (readout, delta, speech)
        }
        ModalKind::Rotate => {
            let val = op.value;
            let readout = format!("{val:.1}°");
            let delta = format!("Ângulo: {val:.1}° em torno de {axis_str}");
            let speech = format!("rotação de {val:.1} graus");
            (readout, delta, speech)
        }
        ModalKind::Scale => {
            let val = op.value;
            let comp = op.components;
            let readout = format!("{val:.3}x");
            let delta = format!("sx: {:.3}, sy: {:.3}, sz: {:.3}", comp.x, comp.y, comp.z);
            let speech = format!("fator de escala {val:.3}");
            (readout, delta, speech)
        }
        ModalKind::Extrude | ModalKind::ExtrudeIndividual | ModalKind::PushPull => {
            let val = op.value;
            let readout = format!("{val:+.3} m");
            let delta = format!("distância: {val:+.3} m");
            let speech = format!("extrusão de {val:+.3} metros");
            (readout, delta, speech)
        }
        ModalKind::Inset => {
            let val = op.value;
            let readout = format!("{val:.3} factor");
            let delta = format!("fator: {val:.3}");
            let speech = format!("fator de inserção {val:.3}");
            (readout, delta, speech)
        }
        ModalKind::Bevel => {
            let val = op.value;
            let readout = format!("{val:.3} m");
            let delta = format!("largura: {val:.3} m");
            let speech = format!("chanfro de {val:.3} metros");
            (readout, delta, speech)
        }
    };

    let announcement = format!(
        "Gesto ativo: {mode_str} restrito a {axis_speech} no referencial {frame_str}. {speech_val}."
    );

    GizmoAccessibilityFeedback {
        mode: mode_str,
        active_axis: axis_str,
        frame_orientation: frame_str,
        numeric_readout,
        delta_text,
        screen_reader_announcement: announcement,
        minimum_hit_target_dp: 24.0,
        high_contrast_compliant: true,
    }
}
