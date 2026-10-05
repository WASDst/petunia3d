//! Métricas reproduzíveis do shell (plano de UI, F0 e §11).
//!
//! Conta os controles interativos **visíveis** ao abrir cada workspace em
//! 1800 × 1012 (tamanho das capturas do diagnóstico): papel acessível de
//! controle, tamanho maior que zero, opacidade efetiva visível e dentro da
//! janela. Rodar com `cargo run -p xtask -- ui-metrics` (ou `cargo test -p
//! petunia_ui_slint --test ui_metrics -- --nocapture`) imprime a tabela.

use i_slint_backend_testing::{AccessibleRole, ElementQuery};
use petunia_ui_slint::PetuniaSlintShell;
use slint::{ComponentHandle, LogicalSize};

const WIDTH: f32 = 1800.0;
const HEIGHT: f32 = 1012.0;
/// Meta do plano de UI (§11) para POLY ao abrir.
const POLY_BUDGET: usize = 45;

fn is_control(role: AccessibleRole) -> bool {
    matches!(
        role,
        AccessibleRole::Button
            | AccessibleRole::Checkbox
            | AccessibleRole::Combobox
            | AccessibleRole::RadioButton
            | AccessibleRole::Slider
            | AccessibleRole::Spinbox
            | AccessibleRole::Switch
            | AccessibleRole::Tab
            | AccessibleRole::TextInput
    )
}

/// Um controle visível: rótulo acessível e tamanho em px lógicos.
struct Control {
    label: String,
    width: f32,
    height: f32,
    /// Tipo e id do elemento, para achar no `.slint` quem falhou.
    origin: String,
}

/// Controles visíveis: papel de controle, tamanho > 0, opacidade efetiva
/// visível e dentro da janela.
fn visible_controls(shell: &PetuniaSlintShell) -> Vec<Control> {
    let mut controls = Vec::new();
    for element in ElementQuery::from_root(shell).find_all() {
        let Some(role) = element.accessible_role() else {
            continue;
        };
        if !is_control(role) {
            continue;
        }
        let size = element.size();
        let position = element.absolute_position();
        let on_screen = position.x < WIDTH
            && position.y < HEIGHT
            && position.x + size.width > 0.0
            && position.y + size.height > 0.0;
        if size.width > 1.0 && size.height > 1.0 && element.computed_opacity() > 0.05 && on_screen {
            controls.push(Control {
                label: element.accessible_label().unwrap_or_default().to_string(),
                width: size.width,
                height: size.height,
                origin: format!(
                    "{}#{} @({:.0},{:.0})",
                    element.type_name().unwrap_or_default(),
                    element.id().unwrap_or_default(),
                    position.x,
                    position.y
                ),
            });
        }
    }
    controls
}

const WORKSPACES: [(&str, &str, &str); 3] = [
    ("DRAW", "MODEL", "DRAW"),
    ("POLY", "MODEL", "POLY"),
    ("PAINT", "PAINT", "POLY"),
];

/// Um único shell, como em produção (a tradução é instalada uma vez).
fn open() -> PetuniaSlintShell {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(WIDTH, HEIGHT));
    shell.show().expect("headless window");
    // Rótulos reais: vários vêm do bridge (TextId), não do `Tr.t` do markup.
    let bridge = petunia_ui_slint::SlintUiBridge::new(
        petunia_core::AppState::default(),
        petunia_ui_slint::PlaceholderViewport::default(),
    );
    petunia_ui_slint::sync_shell_for_tests(&shell, &bridge.view_model());
    shell
}

fn switch_to(shell: &PetuniaSlintShell, workspace: &str, mode: &str) {
    shell.set_active_workspace(workspace.into());
    shell.set_modeling_mode(mode.into());
    shell.set_selection_domain("OBJECT".into());
    shell.set_object_has_selection(true);
}

/// Uma só função de teste: o backend de testes do Slint é inicializado uma
/// vez por processo, e testes paralelos no mesmo binário colidiriam.
#[test]
fn shell_audit() {
    let mut rows = Vec::new();
    let mut problems = Vec::new();
    let shell = open();
    for (name, workspace, mode) in WORKSPACES {
        switch_to(&shell, workspace, mode);
        let controls = visible_controls(&shell);
        if std::env::var_os("PETUNIA_UI_METRICS_VERBOSE").is_some() {
            for control in &controls {
                eprintln!(
                    "{name}: {:?} {}×{} {}",
                    control.label, control.width, control.height, control.origin
                );
            }
        }
        rows.push((name, controls.len()));
        // WCAG 2.2 — 4.1.2 (nome, função, valor): leitor de tela precisa do nome.
        for control in controls.iter().filter(|c| c.label.trim().is_empty()) {
            problems.push(format!("{name}: sem nome acessível — {}", control.origin));
        }
        // WCAG 2.2 — 2.5.8 (tamanho mínimo do alvo, AA): 24 × 24 px lógicos.
        for control in controls
            .iter()
            .filter(|c| c.width < 23.5 || c.height < 23.5)
        {
            problems.push(format!(
                "{name}: alvo {}×{} < 24 px — {:?} {}",
                control.width, control.height, control.label, control.origin
            ));
        }
    }
    eprintln!("| Workspace | Controles visíveis (1800 × 1012) |");
    eprintln!("|---|---|");
    for (name, count) in &rows {
        eprintln!("| {name} | {count} |");
    }
    let poly = rows
        .iter()
        .find(|(name, _)| *name == "POLY")
        .map(|row| row.1);
    if !poly.is_some_and(|count| count <= POLY_BUDGET) {
        problems.push(format!(
            "POLY passou da meta de {POLY_BUDGET} controles: {poly:?}"
        ));
    }
    assert!(
        problems.is_empty(),
        "{} problema(s):\n{}",
        problems.len(),
        problems.join("\n")
    );
}
