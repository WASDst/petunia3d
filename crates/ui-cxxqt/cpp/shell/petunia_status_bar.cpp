#include "petunia_status_bar.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"

#include <QAccessibleEvent>

PetuniaStatusBar::PetuniaStatusBar(QWidget *parent) : QStatusBar(parent) {
    setObjectName(QStringLiteral("PetuniaStatusBar"));
    setSizeGripEnabled(false);
    m_badge = new QLabel(this);
    m_badge->setProperty("role", QStringLiteral("caption"));
    m_badge->setAccessibleName(tr("Ferramenta ativa"));
    m_message = new QLabel(this);
    m_message->setAccessibleName(tr("Mensagem de estado"));
    m_stats = new QLabel(this);
    m_stats->setProperty("role", QStringLiteral("muted"));
    m_stats->setAccessibleName(tr("Estatísticas da cena"));
    m_selection = new QLabel(this);
    m_selection->setProperty("role", QStringLiteral("caption"));
    m_selection->setAccessibleName(tr("Seleção"));
    addWidget(m_badge);
    addWidget(m_message, 1);
    addPermanentWidget(m_stats);
    addPermanentWidget(m_selection);
}

void PetuniaStatusBar::sync(const QJsonObject &state) {
    const QString tool = state["active_tool"].toString();
    const QString badge = tool.isEmpty() ? QString() : petuniaTr(tool.left(1).toUpper() + tool.mid(1)).replace(QLatin1Char('_'), QLatin1Char(' '));
    if (m_badge->text() != badge)
        m_badge->setText(badge);

    QString text = state["error"].toString(state["status_message"].toString());
    const int severity = state["status_severity"].toInt();
    // Severidade com marca textual (não depende só de cor).
    if (severity >= 3)
        text = QStringLiteral("✕ ") + text;
    else if (severity == 2)
        text = QStringLiteral("⚠ ") + text;
    if (m_message->text() != text) {
        m_message->setText(text);
        if (severity >= 3 && !text.isEmpty()) {
            QAccessibleEvent alert(m_message, QAccessible::Alert);
            QAccessible::updateAccessibility(&alert);
        }
    }
    m_stats->setText(state["scene_stats"].toString());
    m_selection->setText(state["selection_summary"].toString());
}

void PetuniaStatusBar::showProblem(const QString &message) {
    m_message->setText(QStringLiteral("✕ ") + message);
    QAccessibleEvent alert(m_message, QAccessible::Alert);
    QAccessible::updateAccessibility(&alert);
}
