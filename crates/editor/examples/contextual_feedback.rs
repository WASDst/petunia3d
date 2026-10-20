//! Reproducible headless projection evidence, not native Qt acceptance.
use petunia_core::AppState;
use petunia_editor::{SlintUiBridge, Software3dViewport, UiIntent};
use std::{error::Error, fmt::Write, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: contextual_feedback <output-directory>")?;
    std::fs::create_dir_all(&output)?;
    let mut bridge = SlintUiBridge::new(AppState::default(), Software3dViewport::new(800, 600));
    bridge.resize_viewport(800, 600);
    bridge.state.dispatch(&petunia_core::SelectAllCmd)?;
    bridge.apply(UiIntent::SetActiveTool("select".into()));
    bridge.render_viewport();
    image::save_buffer(
        output.join("select-source.png"),
        &bridge.viewport.color_buffer,
        800,
        600,
        image::ColorType::Rgba8,
    )?;
    let g = bridge.view_model().gizmo;
    if !g.visible {
        return Err("select gizmo is not visible".into());
    }
    let mut svg = String::from(
        "<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='800' height='600' viewBox='0 0 800 600'><title>Headless Select gizmo projection</title><image xlink:href='select-source.png' width='800' height='600'/>",
    );
    for (color, rod, arrow, scale, rotate) in [
        (
            "#e86161",
            &g.x_commands,
            &g.x_arrow_commands,
            &g.x_scale_commands,
            &g.x_rotate_commands,
        ),
        (
            "#77c98b",
            &g.y_commands,
            &g.y_arrow_commands,
            &g.y_scale_commands,
            &g.y_rotate_commands,
        ),
        (
            "#719ff3",
            &g.z_commands,
            &g.z_arrow_commands,
            &g.z_scale_commands,
            &g.z_rotate_commands,
        ),
    ] {
        write!(
            svg,
            "<g stroke='{color}' stroke-width='2'><path d='{rotate}' fill='none' opacity='.6'/><path d='{rod}' fill='none'/><path d='{arrow}' fill='{color}'/><path d='{scale}' fill='{color}'/></g>"
        )?;
    }
    svg.push_str("<text x='24' y='32' fill='#efefef' font-family='sans-serif' font-size='16'>Select: explicit handles; geometry drag is protected</text></svg>");
    std::fs::write(output.join("select-gizmo.svg"), svg)?;
    Ok(())
}
