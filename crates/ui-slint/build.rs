fn main() {
    println!("cargo:rerun-if-changed=ui");
    println!("cargo:rerun-if-changed=../../assets/icons");

    // A suíte de gestos consulta a geometria acessível de controles reais.
    // Preserve esses metadados em builds de desenvolvimento/teste, sem inflar
    // os artefatos de produção.
    let debug_ui = std::env::var("PROFILE").is_ok_and(|profile| profile != "release");
    let config = slint_build::CompilerConfiguration::new().with_debug_info(debug_ui);

    slint_build::compile_with_config("ui/app.slint", config)
        .expect("failed to compile the experimental Slint frontend");
}
