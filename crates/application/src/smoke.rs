//! Smoke test headless: exercita domínio + módulos + projeto sem janela, GPU,
//! egui nem efeitos de plataforma (lock de sessão, watcher). Vive na
//! `Application` neutra para que o binário principal o execute sem compilar a
//! UI egui legada.

use crate::{Application, ApplicationConfig};
use petunia_module_paint::PaintModule;
use petunia_module_uv::UvModule;

/// Smoke test scriptado (headless): exercita domínio + módulos + projeto
/// sem janela. Falha com `Err` legível. Uso: `petunia3d --smoke-test`.
pub fn smoke_test() -> anyhow::Result<()> {
    use anyhow::{Context as _, ensure};
    use petunia_mesh::Mesh;

    let mut app = Application::new(ApplicationConfig::default());
    let s = &mut app.state;

    // primitivas
    s.checkpoint("add");
    s.project.add("Cube2", Mesh::cube(2.0));
    s.project.add("Capsule", Mesh::capsule(8, 0.5, 2.0));
    s.project.add("Sphere", Mesh::sphere_low(8, 5, 1.0));
    ensure!(s.project.assets.len() == 4, "assets após add");
    eprintln!("SMOKE primitives ok ({} assets)", s.project.assets.len());

    // seleção + ops com undo/redo
    s.project
        .active_mesh_mut()
        .context("mesh ativa")?
        .select_all();
    s.checkpoint("extrude");
    s.project
        .active_mesh_mut()
        .context("mesh ativa")?
        .extrude_selected(0.4);
    s.checkpoint("inset");
    let mesh = s.project.active_mesh_mut().context("mesh ativa")?;
    let selected: Vec<usize> = (0..mesh.faces.len())
        .filter(|&f| mesh.faces[f].selected)
        .collect();
    mesh.inset_faces(&selected, 0.1, petunia_mesh::InsetMode::Region);
    s.checkpoint("subdivide");
    s.project
        .active_mesh_mut()
        .context("mesh ativa")?
        .subdivide_selected();
    // Bevel has a stricter supported topology than arbitrary subdivided spheres.
    let mut bevel_fixture = petunia_mesh::Mesh::cube(2.0);
    bevel_fixture.selected_edges.insert((0, 1));
    let (ok, skipped) = bevel_fixture.bevel_selected(0.05);
    ensure!(ok == 1 && skipped == 0, "bevel fixture");
    let (topology, defects) = petunia_mesh::HalfEdgeMesh::from_mesh(&bevel_fixture);
    ensure!(
        defects.is_empty() && topology.is_closed_manifold(),
        "bevel closed manifold"
    );
    s.checkpoint("mirror");
    s.project
        .active_mesh_mut()
        .context("mesh ativa")?
        .mirror(0, 0.001);
    s.checkpoint("pushpull");
    s.project
        .active_mesh_mut()
        .context("mesh ativa")?
        .push_pull(0.2);
    // novas operações de geometria da Gauntlet
    let m = s.project.active_mesh_mut().context("mesh ativa")?;
    m.slice_plane(glam::Vec3::ZERO, glam::Vec3::Y, true);
    m.flip_normals();
    m.recalculate_normals();
    let edge_for_dissolve = {
        m.faces.iter().find_map(|f| {
            let k = f.verts.len();
            (0..k).find_map(|i| {
                let a = f.verts[i];
                let b = f.verts[(i + 1) % k];
                if m.edge_faces(a, b).len() == 2 {
                    Some((a, b))
                } else {
                    None
                }
            })
        })
    };
    if let Some(edge) = edge_for_dissolve {
        m.selected_edges.insert(edge);
        m.dissolve_selected();
    }
    let rep = m.validate_topology();
    ensure!(
        rep.isolated_vertex_count == 0,
        "topology isolated vertices cleaned"
    );

    // sweep
    let mut sweep_mesh = petunia_mesh::Mesh::default();
    let sweep_prof = [[-0.2, -0.2], [0.2, -0.2], [0.2, 0.2], [-0.2, 0.2]];
    let path = [
        glam::vec3(0.0, 0.0, 0.0),
        glam::vec3(0.0, 1.0, 0.5),
        glam::vec3(1.0, 2.0, 1.0),
    ];
    sweep_mesh
        .sweep(&sweep_prof, &path, true)
        .map_err(|e| anyhow::anyhow!(e))?;
    ensure!(sweep_mesh.tri_count() > 0, "sweep gerou geometria");

    let (v, f) = s.project.totals();
    ensure!(v > 8 && f > 6, "malha cresceu v={v} f={f}");
    eprintln!("SMOKE ops ok (v={v} f={f})");

    ensure!(s.undo(), "undo 1");
    ensure!(s.undo(), "undo 2");
    ensure!(s.redo(), "redo 1");
    eprintln!("SMOKE undo/redo ok");

    // draw profile -> extrude + revolve (nível domínio)
    let square = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
    let pm = Mesh::from_polygon(&square, 0.5).map_err(|e| anyhow::anyhow!(e))?;
    ensure!(pm.tri_count() >= 2, "polígono triangulou");
    s.project.add("Profile", pm);
    let prof = [[0.0, 0.0], [1.0, 0.0], [1.0, 2.0], [0.0, 2.0]];
    let rm = Mesh::revolve(&prof, 10).map_err(|e| anyhow::anyhow!(e))?;
    ensure!(!rm.faces.is_empty(), "revolve gerou faces");
    s.project.add("Revolved", rm);
    eprintln!("SMOKE profile ok");

    // uv + paint
    s.project
        .active_mesh_mut()
        .context("mesh ativa")?
        .project_planar();
    UvModule::move_selected(s, 0.1, 0.0);
    ensure!(
        s.project
            .active_mesh_mut()
            .context("mesh ativa")?
            .faces
            .iter()
            .all(|f| f.uv.len() == f.verts.len()),
        "uv invariante"
    );
    PaintModule::fill_selection(s);
    PaintModule::ensure_canvas(s);
    PaintModule::canvas_brush(s, 10, 10, false);
    PaintModule::canvas_fill(s);
    eprintln!("SMOKE uv/paint ok");

    // projeto save/load + export
    let dir = std::env::temp_dir();
    let pp = dir.join("petunia_smoke.petunia");
    petunia_project::format::save(&s.project, &pp).context("save")?;
    let q = petunia_project::format::load(&pp).context("load")?;
    ensure!(q.assets.len() == s.project.assets.len(), "roundtrip assets");
    let glb = petunia_project::export::export_gltf(&s.project, &[0]).context("glb")?;
    ensure!(glb.starts_with(b"glTF"), "magic glb");
    let gp = dir.join("petunia_smoke.glb");
    std::fs::write(&gp, &glb).context("write glb")?;
    eprintln!(
        "SMOKE project ok ({} assets, glb {} bytes)",
        q.assets.len(),
        glb.len()
    );
    let _ = std::fs::remove_file(&pp);
    let _ = std::fs::remove_file(&gp);

    // eventos fluem?
    s.emit_mesh_changed();
    let pending = app.state.events.drain();
    ensure!(!pending.is_empty(), "emit_mesh_changed gerou eventos");
    for ev in &pending {
        app.dispatch_event(ev);
    }
    eprintln!("SMOKE OK");
    Ok(())
}
