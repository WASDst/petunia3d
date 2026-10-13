use petunia_ui_cxxqt::widgets::{PetuniaStudioWindow, PetuniaWidgetsApp};

fn main() {
    println!("Inicializando Petunia3D Qt Widgets Studio...");
    println!("Recursos ativos:");
    println!("  - Viewport OpenGL 3.3 com Blit FBO zero-copy");
    println!("  - Suporte nativo a pressão e inclinação de caneta (QTabletEvent)");
    println!(
        "  - Docas Multijanela (Outliner, Inspetor, Timeline destacáveis para múltiplos monitores)"
    );
    println!("  - Tema escuro fiel aos Design Tokens do Petunia3D");

    let app = PetuniaWidgetsApp::new();
    let window = PetuniaStudioWindow::new();
    window.show();

    println!("Executando loop de eventos Qt 6 Widgets...");
    app.exec();
}
