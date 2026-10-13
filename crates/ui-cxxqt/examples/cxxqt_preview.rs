use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

fn main() {
    println!("Inicializando Petunia3D Qt Quick Preview...");
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    let url = QUrl::from("qrc:/main.qml");
    if let Some(engine) = engine.as_mut() {
        engine.load(&url);
    }

    if let Some(app) = app.as_mut() {
        println!("Executando event loop Qt 6...");
        app.exec();
    }
}
