// Auto-testes dos componentes da GUI Qt, executáveis pelo `cargo test` sem
// display (QT_QPA_PLATFORM=offscreen). Não dependem do módulo QtTest: os
// eventos são enviados diretamente (`QCoreApplication::sendEvent`).
//
// Cada verificação vira uma entrada `{name, pass, detail}` do JSON devolvido
// por `petunia_selftest_components`; o teste Rust falha listando as que não
// passaram (gate de componente: UI-001 §15, AGENTS.md Camada 3/6).
#include "petunia_expression.h"
#include "petunia_number_field.h"
#include "petunia_theme.h"
#include "petunia_vector3_field.h"

#include <QApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLineEdit>
#include <QMouseEvent>
#include <QWheelEvent>
#include <cmath>
#include <functional>

namespace {

struct Report {
    QJsonArray checks;
    void add(const QString &name, bool pass, const QString &detail = QString()) {
        QJsonObject o;
        o["name"] = name;
        o["pass"] = pass;
        if (!detail.isEmpty())
            o["detail"] = detail;
        checks.append(o);
    }
};

bool near(double a, double b, double eps = 1e-9) {
    return std::abs(a - b) <= eps;
}

void key(QWidget *target, int k, Qt::KeyboardModifiers mods = Qt::NoModifier) {
    QKeyEvent press(QEvent::KeyPress, k, mods);
    QKeyEvent release(QEvent::KeyRelease, k, mods);
    QCoreApplication::sendEvent(target, &press);
    QCoreApplication::sendEvent(target, &release);
}

void focusIn(QWidget *target) {
    QFocusEvent e(QEvent::FocusIn, Qt::OtherFocusReason);
    QCoreApplication::sendEvent(target, &e);
}

void focusOut(QWidget *target) {
    QFocusEvent e(QEvent::FocusOut, Qt::OtherFocusReason);
    QCoreApplication::sendEvent(target, &e);
}

struct Counters {
    int started{0}, preview{0}, commit{0}, cancel{0}, invalid{0};
    double lastCommit{0.0};
    QString lastText;
};

void wire(PetuniaNumberField *f, Counters *c) {
    QObject::connect(f, &PetuniaNumberField::editStarted, [c](const QString &) { ++c->started; });
    QObject::connect(f, &PetuniaNumberField::previewRequested,
                     [c](const QString &, double) { ++c->preview; });
    QObject::connect(f, &PetuniaNumberField::commitRequested,
                     [c](const QString &, double v, const QString &t) {
                         ++c->commit;
                         c->lastCommit = v;
                         c->lastText = t;
                     });
    QObject::connect(f, &PetuniaNumberField::editCanceled, [c](const QString &) { ++c->cancel; });
    QObject::connect(f, &PetuniaNumberField::invalidInput,
                     [c](const QString &, const QString &) { ++c->invalid; });
}

void expressionChecks(Report &r) {
    using PetuniaExpression::evaluate;
    struct Case { const char *text; bool ok; double value; };
    const Case cases[] = {
        {"42", true, 42}, {"2+3*4", true, 14}, {"(1+2)*3", true, 9}, {"10/4", true, 2.5},
        {"-3+5", true, 2}, {"--3", true, 3}, {"1,5*2", true, 3}, {".5+.25", true, 0.75},
        {"2^3^2", true, 512}, {"1e3", true, 1000}, {"2e-1", true, 0.2}, {"pi", true, M_PI},
        {" 7 ", true, 7}, {"1/0", false, 0}, {"", false, 0}, {"abc", false, 0}, {"2+", false, 0},
        {"(1+2", false, 0}, {"1+2)", false, 0}, {"1.2.3", false, 0}, {"1,2.3", false, 0},
        {"1e999", false, 0}, {"10^400", false, 0}, {"0/0", false, 0},
    };
    for (const auto &c : cases) {
        const auto res = evaluate(QString::fromUtf8(c.text));
        const bool pass = res.ok() == c.ok && (!c.ok || near(res.value, c.value, 1e-9));
        r.add(QStringLiteral("expression:%1").arg(c.text), pass,
              pass ? QString() : QStringLiteral("ok=%1 value=%2").arg(res.ok()).arg(res.value));
    }
    // Entrada hostil: tamanho e profundidade limitados sem estouro de pilha.
    r.add("expression:too-long", evaluate(QString(200, QLatin1Char('1'))).status ==
                                      PetuniaExpression::Status::TooComplex);
    r.add("expression:deep-parentheses",
          !evaluate(QString(5000, QLatin1Char('(')) + "1" + QString(5000, QLatin1Char(')'))).ok());
    QString nested;
    for (int i = 0; i < 20; ++i) nested += "(";
    nested += "1";
    for (int i = 0; i < 20; ++i) nested += ")";
    r.add("expression:moderate-nesting", evaluate(nested).ok());
}

void numberFieldChecks(Report &r) {
    PetuniaNumberField field;
    field.setPropertyId(QStringLiteral("test"));
    field.setRange(-100, 100);
    field.setSingleStep(1.0);
    field.setLabel(QStringLiteral("Teste"));
    field.show();
    Counters c;
    wire(&field, &c);
    auto *edit = field.findChild<QLineEdit *>();
    r.add("number:has-editor", edit != nullptr);
    if (!edit)
        return;
    r.add("number:accessible-name",
          edit->accessibleName() == QLatin1String("Teste") &&
              field.accessibleName() == QLatin1String("Teste"));

    field.setValue(5.0);
    r.add("number:display-trims-zeros", edit->text() == QLatin1String("5"), edit->text());

    // Foco sem editar não emite nada.
    focusIn(edit);
    r.add("number:focus-starts-edit", field.isEditing() && c.started == 1);
    focusOut(edit);
    r.add("number:focus-out-without-edit-is-silent",
          !field.isEditing() && c.commit == 0 && c.cancel == 0);

    // Expressão válida confirma uma única vez.
    focusIn(edit);
    edit->setText(QStringLiteral("2+3*4"));
    key(edit, Qt::Key_Return);
    r.add("number:enter-commits-expression-once",
          c.commit == 1 && near(c.lastCommit, 14.0) && near(field.value(), 14.0) && !field.isEditing(),
          QStringLiteral("commit=%1 value=%2").arg(c.commit).arg(field.value()));
    r.add("number:commit-reports-typed-text", c.lastText == QLatin1String("2+3*4"), c.lastText);

    // Inválido mantém rascunho, marca erro e não altera nada.
    const int commitsBefore = c.commit;
    focusIn(edit);
    edit->setText(QStringLiteral("2+"));
    key(edit, Qt::Key_Return);
    r.add("number:invalid-keeps-draft", field.isEditing() && field.isInvalid() &&
                                           edit->text() == QLatin1String("2+") &&
                                           c.commit == commitsBefore && c.invalid == 1);
    r.add("number:invalid-has-accessible-description",
          !edit->accessibleDescription().isEmpty());
    r.add("number:invalid-value-untouched", near(field.value(), 14.0));
    key(edit, Qt::Key_Escape);
    r.add("number:escape-restores-after-invalid",
          !field.isEditing() && !field.isInvalid() && near(field.value(), 14.0) &&
              edit->text() == QLatin1String("14"),
          edit->text());

    // Esc durante edição válida também restaura.
    focusIn(edit);
    edit->setText(QStringLiteral("77"));
    key(edit, Qt::Key_Escape);
    r.add("number:escape-rolls-back", near(field.value(), 14.0) && c.commit == commitsBefore);

    // Setas: prévia ao vivo, confirmação ao sair.
    focusIn(edit);
    const int previewsBefore = c.preview;
    key(edit, Qt::Key_Up);
    key(edit, Qt::Key_Up, Qt::ShiftModifier);
    r.add("number:arrows-preview", c.preview == previewsBefore + 2 && near(field.value(), 25.0),
          QString::number(field.value()));
    focusOut(edit);
    r.add("number:arrows-commit-on-blur", c.commit == commitsBefore + 1 && near(c.lastCommit, 25.0));

    // Limites: clampa.
    r.add("number:commit-text-clamps", field.commitText(QStringLiteral("1000")) &&
                                           near(field.value(), 100.0));

    // setValue do host não pisa na edição.
    focusIn(edit);
    edit->setText(QStringLiteral("3"));
    field.setValue(50.0);
    r.add("number:host-sync-ignored-while-editing", edit->text() == QLatin1String("3"));
    key(edit, Qt::Key_Escape);

    // Roda do mouse nunca altera.
    const double before = field.value();
    QWheelEvent wheel(QPointF(10, 10), QPointF(10, 10), QPoint(), QPoint(0, 120), Qt::NoButton,
                      Qt::NoModifier, Qt::NoScrollPhase, false);
    QCoreApplication::sendEvent(&field, &wheel);
    r.add("number:wheel-never-changes-value", near(field.value(), before) && !wheel.isAccepted());

    // Estado misto.
    field.setMixed(true);
    r.add("number:mixed-display", edit->text() == QStringLiteral("—"));
    field.setValue(1.0);
    r.add("number:set-value-clears-mixed", !field.isMixed());

    // Unidade: aceita digitar com sufixo.
    field.setSuffix(QStringLiteral("m"));
    field.setValue(2.5);
    r.add("number:suffix-display", edit->text() == QLatin1String("2.5 m"), edit->text());
    r.add("number:suffix-typed", field.commitText(QStringLiteral("3 m")) && near(field.value(), 3.0));

    // Somente leitura.
    field.setReadOnly(true);
    const double ro = field.value();
    r.add("number:read-only-rejects-commit", !field.commitText(QStringLiteral("9")) &&
                                                  near(field.value(), ro));
}

void scrubChecks(Report &r) {
    PetuniaNumberField field(QStringLiteral("X"));
    field.setPropertyId(QStringLiteral("x"));
    field.setRange(-1000, 1000);
    field.setSingleStep(0.5);
    field.resize(160, field.sizeHint().height());
    field.show();
    Counters c;
    wire(&field, &c);
    field.setValue(10.0);

    const QPointF inChip(6, field.height() / 2.0);
    auto mouse = [&](QEvent::Type type, const QPointF &local, const QPointF &global,
                     Qt::MouseButton button, Qt::MouseButtons buttons) {
        QMouseEvent e(type, local, global, button, buttons, Qt::NoModifier);
        QCoreApplication::sendEvent(&field, &e);
    };

    // Clique sem mover: não emite commit (só foca o editor).
    mouse(QEvent::MouseButtonPress, inChip, QPointF(100, 100), Qt::LeftButton, Qt::LeftButton);
    mouse(QEvent::MouseButtonRelease, inChip, QPointF(100, 100), Qt::LeftButton, Qt::NoButton);
    r.add("scrub:click-without-move-emits-nothing", c.commit == 0 && c.started <= 1 && c.preview == 0,
          QStringLiteral("commit=%1 preview=%2").arg(c.commit).arg(c.preview));
    field.findChild<QLineEdit *>()->clearFocus();
    focusOut(field.findChild<QLineEdit *>());

    // Arrastar no rótulo: prévias + um commit ao soltar.
    Counters d;
    PetuniaNumberField f2(QStringLiteral("Y"));
    f2.setRange(-1000, 1000);
    f2.setSingleStep(0.5);
    f2.resize(160, f2.sizeHint().height());
    f2.show();
    wire(&f2, &d);
    f2.setValue(10.0);
    auto mouse2 = [&](QEvent::Type type, const QPointF &local, const QPointF &global,
                      Qt::MouseButton button, Qt::MouseButtons buttons) {
        QMouseEvent e(type, local, global, button, buttons, Qt::NoModifier);
        QCoreApplication::sendEvent(&f2, &e);
    };
    mouse2(QEvent::MouseButtonPress, inChip, QPointF(100, 100), Qt::LeftButton, Qt::LeftButton);
    mouse2(QEvent::MouseMove, inChip, QPointF(110, 100), Qt::NoButton, Qt::LeftButton);
    mouse2(QEvent::MouseMove, inChip, QPointF(120, 100), Qt::NoButton, Qt::LeftButton);
    mouse2(QEvent::MouseButtonRelease, inChip, QPointF(120, 100), Qt::LeftButton, Qt::NoButton);
    r.add("scrub:drag-previews-and-commits-once",
          d.started == 1 && d.preview >= 1 && d.commit == 1 && near(f2.value(), 20.0, 1e-6),
          QStringLiteral("started=%1 preview=%2 commit=%3 value=%4")
              .arg(d.started).arg(d.preview).arg(d.commit).arg(f2.value()));

    // Esc durante o arrasto restaura sem commit.
    Counters e2;
    PetuniaNumberField f3(QStringLiteral("Z"));
    f3.setRange(-1000, 1000);
    f3.setSingleStep(0.5);
    f3.resize(160, f3.sizeHint().height());
    f3.show();
    wire(&f3, &e2);
    f3.setValue(10.0);
    auto mouse3 = [&](QEvent::Type type, const QPointF &local, const QPointF &global,
                      Qt::MouseButton button, Qt::MouseButtons buttons) {
        QMouseEvent ev(type, local, global, button, buttons, Qt::NoModifier);
        QCoreApplication::sendEvent(&f3, &ev);
    };
    mouse3(QEvent::MouseButtonPress, inChip, QPointF(100, 100), Qt::LeftButton, Qt::LeftButton);
    mouse3(QEvent::MouseMove, inChip, QPointF(130, 100), Qt::NoButton, Qt::LeftButton);
    QKeyEvent esc(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
    QCoreApplication::sendEvent(&f3, &esc);
    mouse3(QEvent::MouseButtonRelease, inChip, QPointF(130, 100), Qt::LeftButton, Qt::NoButton);
    r.add("scrub:escape-cancels", e2.commit == 0 && e2.cancel == 1 && near(f3.value(), 10.0),
          QStringLiteral("commit=%1 cancel=%2 value=%3").arg(e2.commit).arg(e2.cancel).arg(f3.value()));
}

void vectorChecks(Report &r) {
    PetuniaVector3Field v;
    v.setLabel(QStringLiteral("Posição"));
    v.setValues(1, 2, 3);
    r.add("vector3:axis-names", v.field(0)->label() == QStringLiteral("Posição X") &&
                                    v.field(2)->label() == QStringLiteral("Posição Z"));
    r.add("vector3:axis-letters-not-color-only",
          v.field(0)->prefix() == QLatin1String("X") && v.field(1)->prefix() == QLatin1String("Y") &&
              v.field(2)->prefix() == QLatin1String("Z"));
    v.setMixed(1, true);
    r.add("vector3:mixed-per-axis", v.isMixed(1) && !v.isMixed(0));
    v.resize(120, 120); // estreito: empilha sem cortar
    v.show();
    QCoreApplication::processEvents();
    r.add("vector3:stacks-when-narrow", v.field(0)->y() < v.field(1)->y());
}

void themeChecks(Report &r) {
    auto &t = PetuniaTheme::instance();
    const struct { const char *name; QColor a; QColor b; double min; } pairs[] = {
        {"text-primary/panel", t.textPrimary(), t.bgPanel(), 7.0},
        {"text-secondary/panel", t.textSecondary(), t.bgPanel(), 4.5},
        {"text-muted/panel", t.textMuted(), t.bgPanel(), 4.5},
        {"text-active/accent", t.textActive(), t.accentPrimary(), 4.5},
        {"focus/panel", t.borderFocus(), t.bgPanel(), 3.0},
        {"error/panel", t.statusError(), t.bgPanel(), 3.0},
        {"axis-x/surface", t.axisColor(0), t.bgSurface(), 3.0},
        {"axis-y/surface", t.axisColor(1), t.bgSurface(), 3.0},
        {"axis-z/surface", t.axisColor(2), t.bgSurface(), 3.0},
    };
    for (const auto &p : pairs) {
        const double ratio = PetuniaTheme::contrastRatio(p.a, p.b);
        r.add(QStringLiteral("theme:contrast:%1").arg(p.name), ratio >= p.min,
              QStringLiteral("%1 < %2").arg(ratio, 0, 'f', 2).arg(p.min));
    }

    // A folha de estilo só carrega tokens: nenhum hex fora dos valores do tema.
    const QString qss = t.styleSheet();
    r.add("theme:stylesheet-generated", qss.contains(QStringLiteral("QPushButton")) && qss.size() > 1000);
    r.add("theme:stylesheet-no-unresolved-tokens", !qss.contains(QLatin1Char('@')));

    // Densidade altera métricas; escala UI também.
    const auto before = t.controlHeight();
    t.setDensity(PetuniaTheme::Density::Comfortable);
    const bool grew = t.controlHeight() > before;
    t.setDensity(PetuniaTheme::Density::Compact);
    const bool compactOk = t.controlHeight() >= t.minHitTarget();
    t.setDensity(PetuniaTheme::Density::Standard);
    r.add("theme:density-changes-control-height", grew);
    r.add("theme:compact-meets-min-hit-target", compactOk);
    t.setUiScale(1.5);
    const bool scaled = t.controlHeight() > before;
    t.setUiScale(1.0);
    r.add("theme:ui-scale-scales-metrics", scaled);
    t.setReducedMotion(true);
    r.add("theme:reduced-motion-zero-duration", t.motionMs() == 0);
    t.setReducedMotion(false);
}

} // namespace

extern "C" const char *petunia_selftest_components() {
    static QByteArray out;
    Report r;
    expressionChecks(r);
    themeChecks(r);
    numberFieldChecks(r);
    scrubChecks(r);
    vectorChecks(r);
    out = QJsonDocument(QJsonObject{{"checks", r.checks}}).toJson(QJsonDocument::Compact);
    return out.constData();
}
