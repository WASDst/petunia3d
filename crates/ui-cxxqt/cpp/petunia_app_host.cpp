// Ciclo de vida do QApplication e da janela principal (Shell v2, ADR-C14).
// O shell legado (PetuniaMainWindow) foi removido em U19: há uma única janela.
#include "shell/petunia_shell_window.h"

#include <QApplication>
#include <QSurfaceFormat>
#include <QThread>

extern "C" {
bool petunia_qt_claim_ui_thread();

static int s_argc = 1;
static char s_app_name[] = "petunia3d";
static char *s_argv[] = {s_app_name, nullptr};

void *petunia_qt_app_create() {
    // O documento pertence a esta thread; outras threads são recusadas pelo FFI.
    if (!petunia_qt_claim_ui_thread())
        return nullptr;
    QSurfaceFormat format;
    format.setVersion(3, 3);
    format.setProfile(QSurfaceFormat::CoreProfile);
    QSurfaceFormat::setDefaultFormat(format);
    return new QApplication(s_argc, s_argv);
}

int petunia_qt_app_exec(void *app) {
    return app ? static_cast<QApplication *>(app)->exec() : 0;
}

void petunia_qt_app_destroy(void *app) {
    delete static_cast<QApplication *>(app);
}

void *petunia_main_window_create() {
    if (!qApp || QThread::currentThread() != qApp->thread())
        return nullptr;
    return new PetuniaShellWindow();
}

void petunia_main_window_show(void *window) {
    if (window)
        static_cast<QWidget *>(window)->show();
}

void petunia_main_window_destroy(void *window) {
    delete static_cast<QWidget *>(window);
}
}
