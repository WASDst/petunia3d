#include "petunia_layout_store.h"

#include <QJsonDocument>
#include <QJsonValue>
#include <algorithm>
#include <cmath>

namespace {
constexpr int kMinRightWidth = 240;
constexpr int kMaxRightWidth = 640;
constexpr int kMinDrawer = 120;
constexpr int kMaxDrawer = 900;

int clampInt(const QJsonValue &value, int fallback, int lo, int hi) {
    if (!value.isDouble())
        return fallback;
    const double v = value.toDouble();
    if (!std::isfinite(v))
        return fallback;
    return std::clamp(int(std::lround(v)), lo, hi);
}

double clampReal(const QJsonValue &value, double fallback, double lo, double hi) {
    if (!value.isDouble())
        return fallback;
    const double v = value.toDouble();
    if (!std::isfinite(v))
        return fallback;
    return std::clamp(v, lo, hi);
}
} // namespace

PetuniaLayoutStore::PetuniaLayoutStore(QSettings *settings)
    : m_settings(settings), m_owned(QStringLiteral("Petunia3D"), QStringLiteral("QtShell")) {
    if (!m_settings)
        m_settings = &m_owned;
}

QString PetuniaLayoutStore::key(const QString &workspaceId) const {
    return QStringLiteral("layout/v%1/%2").arg(kSchemaVersion).arg(workspaceId);
}

QJsonObject PetuniaLayoutStore::defaults(const QString &workspaceId) {
    QJsonObject layout;
    layout[kRightWidth] = 320;
    layout[kStructureRatio] = workspaceId == QLatin1String("UV") ? 0.22 : 0.28;
    // UV: a tela 2D é o foco (>= 55% da altura útil); Paint/Animate usam gaveta menor.
    layout[kDrawerHeight] = workspaceId == QLatin1String("ANIMATE") ? 260 : 220;
    layout[kDrawerOpen] = workspaceId == QLatin1String("ANIMATE");
    layout[kRailExpanded] = false;
    layout[kRightCollapsed] = false;
    layout[kSections] = QJsonObject();
    layout[QStringLiteral("version")] = kSchemaVersion;
    layout[QStringLiteral("user_modified")] = false;
    return layout;
}

QJsonObject PetuniaLayoutStore::sanitized(const QJsonObject &raw, const QString &workspaceId) {
    QJsonObject base = defaults(workspaceId);
    if (raw.value(QStringLiteral("version")).toInt(-1) != kSchemaVersion)
        return base; // versão desconhecida: padrão seguro
    base[kRightWidth] = clampInt(raw.value(kRightWidth), base[kRightWidth].toInt(), kMinRightWidth, kMaxRightWidth);
    base[kStructureRatio] = clampReal(raw.value(kStructureRatio), base[kStructureRatio].toDouble(), 0.15, 0.7);
    base[kDrawerHeight] = clampInt(raw.value(kDrawerHeight), base[kDrawerHeight].toInt(), kMinDrawer, kMaxDrawer);
    base[kDrawerOpen] = raw.value(kDrawerOpen).toBool(base[kDrawerOpen].toBool());
    base[kRailExpanded] = raw.value(kRailExpanded).toBool(false);
    base[kRightCollapsed] = raw.value(kRightCollapsed).toBool(false);
    QJsonObject sections;
    const auto rawSections = raw.value(kSections).toObject();
    for (auto it = rawSections.constBegin(); it != rawSections.constEnd(); ++it) {
        if (it.value().isBool() && !it.key().isEmpty() && it.key().size() <= 64)
            sections.insert(it.key(), it.value().toBool());
        if (sections.size() >= 64)
            break;
    }
    base[kSections] = sections;
    base[QStringLiteral("user_modified")] = raw.value(QStringLiteral("user_modified")).toBool(false);
    return base;
}

QJsonObject PetuniaLayoutStore::load(const QString &workspaceId) const {
    const QString stored = m_settings->value(key(workspaceId)).toString();
    if (stored.isEmpty())
        return defaults(workspaceId);
    QJsonParseError error;
    const auto doc = QJsonDocument::fromJson(stored.toUtf8(), &error);
    if (error.error != QJsonParseError::NoError || !doc.isObject()) {
        // Corrompido: guarda o original para suporte e volta ao padrão.
        m_settings->setValue(QStringLiteral("layout/backup/%1").arg(workspaceId), stored);
        return defaults(workspaceId);
    }
    const auto raw = doc.object();
    if (raw.value(QStringLiteral("version")).toInt(-1) != kSchemaVersion)
        m_settings->setValue(QStringLiteral("layout/backup/%1").arg(workspaceId), stored);
    return sanitized(raw, workspaceId);
}

void PetuniaLayoutStore::save(const QString &workspaceId, QJsonObject layout) {
    layout[QStringLiteral("version")] = kSchemaVersion;
    layout[QStringLiteral("user_modified")] = true;
    const auto clean = sanitized(layout, workspaceId);
    m_settings->setValue(key(workspaceId),
                         QString::fromUtf8(QJsonDocument(clean).toJson(QJsonDocument::Compact)));
}

void PetuniaLayoutStore::reset(const QString &workspaceId) {
    m_settings->remove(key(workspaceId));
}

void PetuniaLayoutStore::resetAll() {
    m_settings->remove(QStringLiteral("layout"));
}
