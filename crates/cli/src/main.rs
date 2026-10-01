//! Petunia3D CLI — Utilitário de terminal headless (§Gauntlet G8 / F-011).
//!
//! Permite manipular, transformar, inspecionar e converter projetos e malhas
//! tridimensionais sem requerer servidor gráfico, GPU ou janelas.

use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use petunia_core::{
    AddPrimitiveCmd, AppState, ClearSelectionCmd, CommandIntent, ExtrudeSelectedCmd,
    InvertSelectionCmd, PrimitiveKind, ProjectService, ScaleSelectionCmd, SelectAllCmd,
    SubdivideSelectionCmd,
};
use petunia_mesh::Mesh;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = match args.next() {
        Some(cmd) => cmd,
        None => {
            print_help();
            return Ok(());
        }
    };

    match command.as_str() {
        "new" => {
            let output_path = args
                .next()
                .context("Uso: petunia-cli new <arquivo.petunia> [primitiva]")?;
            let prim = args.next().unwrap_or_else(|| "Cube".to_string());
            cmd_new(&output_path, &prim)?;
        }
        "info" => {
            let file_path = args.next().context("Uso: petunia-cli info <arquivo>")?;
            cmd_info(&file_path)?;
        }
        "convert" => {
            let input = args
                .next()
                .context("Uso: petunia-cli convert <entrada> <saida>")?;
            let output = args
                .next()
                .context("Uso: petunia-cli convert <entrada> <saida>")?;
            cmd_convert(&input, &output)?;
        }
        "transform" => {
            let input = args
                .next()
                .context("Uso: petunia-cli transform <entrada.petunia> <saida> [opções]")?;
            let output = args
                .next()
                .context("Uso: petunia-cli transform <entrada.petunia> <saida> [opções]")?;
            let rest_args: Vec<String> = args.collect();
            cmd_transform(&input, &output, &rest_args)?;
        }
        "bench" => {
            cmd_bench()?;
        }
        "mcp" | "--mcp-stdio" => {
            petunia_mcp::serve_stdio_blocking().context("Falha ao servir MCP sobre stdio")?;
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        other => {
            eprintln!("Comando desconhecido: '{other}'\n");
            print_help();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        r#"Petunia3D CLI — Terminal Headless & Automação 3D

USO:
    petunia-cli <COMANDO> [ARGUMENTOS...]

COMANDOS:
    new <saida.petunia> [primitiva]
        Cria um novo projeto com uma primitiva (Cube, Plane, Sphere, Cylinder8, Capsule, Cone).

    info <arquivo>
        Inspeciona metadados, malhas, vértices e faces de arquivos .petunia, .obj, .glb ou .gltf.

    convert <entrada> <saida>
        Converte entre formatos suportados (.petunia, .obj, .glb, .gltf).

    transform <entrada.petunia> <saida> [--select-all] [--extrude <dist>] [--subdivide]
        Carrega projeto, executa mutações geométricas e exporta o resultado.

    bench
        Executa bateria de desempenho headless e mede tempo de resposta.

    mcp, --mcp-stdio
        Inicia o servidor Model Context Protocol (MCP) sobre stdio.

    help
        Exibe esta mensagem de ajuda.
"#
    );
}

fn cmd_new(output_path: &str, primitive: &str) -> Result<()> {
    let start = Instant::now();
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);

    let kind = PrimitiveKind::parse(primitive).map_err(|e| anyhow::anyhow!("{e}"))?;
    if let Some(first) = state.project.active_mut() {
        first.name = kind.default_name().to_string();
        first.mesh = kind.generate_mesh();
    }

    let p = Path::new(output_path);
    ProjectService::save_project(&mut state, p).context("Falha ao salvar novo projeto")?;

    println!(
        "✅ Projeto criado com sucesso em '{}' com primitiva '{}' ({:.2?})",
        output_path,
        primitive,
        start.elapsed()
    );
    Ok(())
}

fn cmd_info(file_path: &str) -> Result<()> {
    let path = Path::new(file_path);
    if !path.exists() {
        bail!("Arquivo não encontrado: {}", path.display());
    }

    println!("📄 Inspecionando: {}", path.display());
    let mut state = AppState::new("en");

    if file_path.ends_with(".petunia") {
        ProjectService::load_project(&mut state, path)
            .with_context(|| format!("Falha ao carregar {}", path.display()))?;

        let prj = &state.project.project;
        println!("  • Quantidade de Assets: {}", prj.assets.len());
        println!("  • Asset Ativo: {}", prj.active);
        println!("  • Paleta de Cores: {} cores", prj.palette.len());
        println!("  • Coleções: {:?}", prj.collections);
        println!("  • Anotações: {}", prj.annotations.len());
        println!("  • Medições: {}", prj.measurements.len());

        let (total_verts, total_faces) = prj.totals();
        println!("  • Vértices Totais: {}", total_verts);
        println!("  • Faces Totais: {}", total_faces);

        for (i, asset) in prj.assets.iter().enumerate() {
            println!(
                "    - [{}] \"{}\": {} vértices, {} faces (visível: {}, bloqueado: {})",
                i,
                asset.name,
                asset.mesh.vert_count(),
                asset.mesh.faces.len(),
                asset.visible,
                asset.locked
            );
        }
    } else if file_path.ends_with(".obj") {
        let content = std::fs::read_to_string(path).context("Falha ao ler arquivo OBJ")?;
        let mesh = Mesh::from_obj(&content);
        println!("  • Formato: Wavefront OBJ");
        println!("  • Vértices: {}", mesh.vert_count());
        println!("  • Faces: {}", mesh.faces.len());
    } else if file_path.ends_with(".glb") || file_path.ends_with(".gltf") {
        let pipeline = petunia_project::pipeline::DeliveryPipeline::new();
        let payload = pipeline
            .import_file(path, &petunia_project::pipeline::ImportOptions::default())
            .with_context(|| format!("Falha ao ler {}", path.display()))?;
        println!(
            "  • Formato: glTF 2.0 ({})",
            path.extension().and_then(|e| e.to_str()).unwrap_or("?")
        );
        println!("  • Malhas: {}", payload.meshes.len());
        println!("  • Materiais: {}", payload.materials.len());
        for (name, mesh) in &payload.meshes {
            println!(
                "    - \"{name}\": {} vértices, {} faces",
                mesh.vert_count(),
                mesh.faces.len()
            );
        }
        if !payload.warnings.is_empty() {
            println!("  • Avisos: {}", payload.warnings.join("; "));
        }
    } else {
        bail!("Extensão não suportada para info: {}", path.display());
    }

    Ok(())
}

fn cmd_convert(input_path: &str, output_path: &str) -> Result<()> {
    let start = Instant::now();
    let mut state = AppState::new("en");
    let in_p = Path::new(input_path);
    let out_p = Path::new(output_path);

    if !in_p.exists() {
        bail!("Arquivo de entrada não encontrado: {}", in_p.display());
    }

    // Carrega entrada
    if input_path.ends_with(".petunia") {
        ProjectService::load_project(&mut state, in_p)?;
    } else if input_path.ends_with(".obj") {
        ProjectService::import_obj(&mut state, in_p)?;
    } else if input_path.ends_with(".glb") || input_path.ends_with(".gltf") {
        let names = ProjectService::import_file_pipeline(
            &mut state,
            in_p,
            &petunia_project::pipeline::ImportOptions::default(),
        )?;
        if names.is_empty() {
            bail!("Nenhuma malha importada de: {input_path}");
        }
    } else {
        bail!("Formato de entrada não suportado: {input_path}");
    }

    // Exporta saída
    if output_path.ends_with(".petunia") {
        ProjectService::save_project(&mut state, out_p)?;
    } else if output_path.ends_with(".obj") {
        let active_idx = state.project.active;
        ProjectService::export_obj(&state, active_idx, out_p)?;
    } else if output_path.ends_with(".glb") {
        let all_indices: Vec<usize> = (0..state.project.assets.len()).collect();
        ProjectService::export_glb(&state, &all_indices, out_p)?;
    } else {
        bail!("Formato de saída não suportado: {output_path}");
    }

    println!(
        "✅ Conversão concluída: '{}' -> '{}' ({:.2?})",
        input_path,
        output_path,
        start.elapsed()
    );
    Ok(())
}

fn cmd_transform(input_path: &str, output_path: &str, options: &[String]) -> Result<()> {
    let start = Instant::now();
    let mut state = AppState::new("en");
    let in_p = Path::new(input_path);
    let out_p = Path::new(output_path);

    ProjectService::load_project(&mut state, in_p)?;

    let mut i = 0;
    while i < options.len() {
        match options[i].as_str() {
            "--select-all" => {
                state.dispatch(&SelectAllCmd)?;
                println!("  ↳ Executado: SelectAllCmd");
                i += 1;
            }
            "--clear-selection" => {
                state.dispatch(&ClearSelectionCmd)?;
                println!("  ↳ Executado: ClearSelectionCmd");
                i += 1;
            }
            "--invert-selection" => {
                state.dispatch(&InvertSelectionCmd)?;
                println!("  ↳ Executado: InvertSelectionCmd");
                i += 1;
            }
            "--extrude" => {
                if i + 1 >= options.len() {
                    bail!("Faltando argumento para --extrude <dist>");
                }
                let dist: f32 = options[i + 1]
                    .parse()
                    .context("Valor numérico inválido para --extrude")?;
                state.dispatch_intent(&CommandIntent {
                    command: "model.extrude".into(),
                    asset_id: None,
                    args: vec![dist as f64],
                })?;
                println!("  ↳ Executado: ExtrudeSelectedCmd({dist})");
                i += 2;
            }
            "--subdivide" => {
                state.dispatch(&SubdivideSelectionCmd)?;
                println!("  ↳ Executado: SubdivideSelectionCmd");
                i += 1;
            }
            "--scale" => {
                if i + 1 >= options.len() {
                    bail!("Faltando argumento para --scale <fator>");
                }
                let s: f32 = options[i + 1]
                    .parse()
                    .context("Valor numérico inválido para --scale")?;
                state.dispatch(&ScaleSelectionCmd { factor: s })?;
                println!("  ↳ Executado: ScaleSelectionCmd({s})");
                i += 2;
            }
            "--add-primitive" => {
                if i + 1 >= options.len() {
                    bail!("Faltando argumento para --add-primitive <nome>");
                }
                let prim = &options[i + 1];
                let kind = PrimitiveKind::parse(prim).map_err(|e| anyhow::anyhow!("{e}"))?;
                state.dispatch(&AddPrimitiveCmd::new(kind))?;
                println!("  ↳ Executado: AddPrimitiveCmd({prim})");
                i += 2;
            }
            other => {
                bail!("Opção de transformação desconhecida: {other}");
            }
        }
    }

    if output_path.ends_with(".petunia") {
        ProjectService::save_project(&mut state, out_p)?;
    } else if output_path.ends_with(".obj") {
        let active_idx = state.project.active;
        ProjectService::export_obj(&state, active_idx, out_p)?;
    } else if output_path.ends_with(".glb") {
        let all_indices: Vec<usize> = (0..state.project.assets.len()).collect();
        ProjectService::export_glb(&state, &all_indices, out_p)?;
    } else {
        bail!("Formato de saída desconhecido: {output_path}");
    }

    println!(
        "✅ Transformação headless concluída e salva em '{}' ({:.2?})",
        output_path,
        start.elapsed()
    );
    Ok(())
}

fn cmd_bench() -> Result<()> {
    println!("⏱️ Executando benchmark headless do Petunia3D...");
    let start = Instant::now();

    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);

    // 1. Cria 5 primitivas via AddPrimitiveCmd
    let prims = [
        PrimitiveKind::Cube,
        PrimitiveKind::Plane,
        PrimitiveKind::Sphere,
        PrimitiveKind::Cylinder,
        PrimitiveKind::Capsule,
    ];
    for p in prims {
        let cmd = AddPrimitiveCmd::new(p);
        state.dispatch(&cmd)?;
    }
    let after_prims = start.elapsed();

    // 2. Seleciona e extrude
    state.dispatch(&SelectAllCmd)?;
    state.dispatch(&ExtrudeSelectedCmd { dist: 1.5 })?;
    let after_extrude = start.elapsed();

    // 3. Undo e Redo
    let _ = state.undo();
    let _ = state.redo();
    let after_undo_redo = start.elapsed();

    // 4. Salva temporariamente
    let tmp_petunia = std::env::temp_dir().join("bench_test.petunia");
    ProjectService::save_project(&mut state, &tmp_petunia)?;
    let after_save = start.elapsed();

    // 5. Exporta GLB e OBJ
    let tmp_glb = std::env::temp_dir().join("bench_test.glb");
    let tmp_obj = std::env::temp_dir().join("bench_test.obj");
    let all_indices: Vec<usize> = (0..state.project.assets.len()).collect();
    ProjectService::export_glb(&state, &all_indices, &tmp_glb)?;
    let active_idx = state.project.active;
    ProjectService::export_obj(&state, active_idx, &tmp_obj)?;
    let after_export = start.elapsed();

    // Limpeza
    let _ = std::fs::remove_file(tmp_petunia);
    let _ = std::fs::remove_file(tmp_glb);
    let _ = std::fs::remove_file(tmp_obj);

    println!("  • Criação de 5 primitivas: {:.2?}", after_prims);
    println!(
        "  • Seleção e Extrusão: {:.2?}",
        after_extrude - after_prims
    );
    println!(
        "  • Ciclo de Undo e Redo: {:.2?}",
        after_undo_redo - after_extrude
    );
    println!(
        "  • Serialização .petunia: {:.2?}",
        after_save - after_undo_redo
    );
    println!(
        "  • Exportação glTF/GLB + OBJ: {:.2?}",
        after_export - after_save
    );
    println!("🚀 Tempo Total Headless: {:.2?}", start.elapsed());

    assert!(
        start.elapsed().as_millis() < 500,
        "Benchmark headless deve executar em menos de 500ms"
    );
    println!("✅ Soberania Headless comprovada com sucesso!");
    Ok(())
}
