#pragma once

#include <QJsonObject>
#include <QSettings>
#include <QString>

/// Persistência versionada do layout do usuário por workspace (UI-005/014).
///
/// O layout é preferência de interface: nunca suja o documento 3D. Valores
/// inválidos, de versão desconhecida ou corrompidos nunca derrubam a GUI — são
/// saneados para limites seguros e o conteúdo original é guardado em backup.
class PetuniaLayoutStore {
  public:
    static constexpr int kSchemaVersion = 1;

    /// `settings` nulo usa o armazenamento padrão da aplicação.
    explicit PetuniaLayoutStore(QSettings *settings = nullptr);

    /// Layout salvo do workspace ou o padrão (sempre saneado).
    QJsonObject load(const QString &workspaceId) const;
    /// Marca `userModified` e grava.
    void save(const QString &workspaceId, QJsonObject layout);
    /// Remove o layout do workspace (volta ao padrão).
    void reset(const QString &workspaceId);
    void resetAll();

    static QJsonObject defaults(const QString &workspaceId);
    /// Aplica limites e tipos; entradas inválidas voltam ao padrão.
    static QJsonObject sanitized(const QJsonObject &raw, const QString &workspaceId);

    // Chaves do layout
    static constexpr const char *kRightWidth = "right_width";
    static constexpr const char *kStructureRatio = "structure_ratio";
    static constexpr const char *kDrawerHeight = "drawer_height";
    static constexpr const char *kDrawerOpen = "drawer_open";
    static constexpr const char *kRailExpanded = "rail_expanded";
    static constexpr const char *kRightCollapsed = "right_collapsed";
    static constexpr const char *kSections = "sections";
    /// PAINT/UV: "3d" | "split" | "2d" e a fração da largura dada à vista 3D.
    static constexpr const char *kSurfaceMode = "surface_mode";
    static constexpr const char *kSurfaceRatio = "surface_ratio";

  private:
    QString key(const QString &workspaceId) const;
    QSettings *m_settings;
    QSettings m_owned;
};
