fn main() {
    // CLI mínimo (o resto é env): --smoke-test | --mcp-stdio | --screenshot ARQ [--frames N] [--size WxH]
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--smoke-test") {
        match petunia_application::smoke_test() {
            Ok(()) => {}
            Err(e) => {
                eprintln!("SMOKE FAIL: {e:?}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "--mcp-stdio") {
        if let Err(e) = petunia_mcp::serve_stdio_blocking() {
            eprintln!("MCP FAIL: {e:?}");
            std::process::exit(1);
        }
        return;
    }
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" => {
                if let Some(p) = args.get(i + 1) {
                    // SAFETY: Called during single-threaded startup before initializing threads or UI.
                    unsafe { std::env::set_var("PETUNIA_SCREENSHOT", p) };
                    i += 1;
                }
            }
            "--frames" => {
                if let Some(n) = args.get(i + 1) {
                    // SAFETY: Called during single-threaded startup before initializing threads or UI.
                    unsafe { std::env::set_var("PETUNIA_SHOT_FRAMES", n) };
                    i += 1;
                }
            }
            "--size" => {
                if let Some(s) = args.get(i + 1) {
                    // SAFETY: Called during single-threaded startup before initializing threads or UI.
                    unsafe { std::env::set_var("PETUNIA_SHOT_SIZE", s) };
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Interface única: shell Qt Widgets com viewport OpenGL 3.3.
    if let Err(e) = petunia_ui_cxxqt::widgets::run() {
        eprintln!("petunia3d: {e}");
        std::process::exit(1);
    }
}
