#pragma once

#include <QColor>
#include <QJsonObject>
#include <QObject>
#include <QString>

/// Canonical C++ theme tokens provider synchronized with petunia_config snapshots.
class PetuniaTheme {
public:
    enum class Density {
        Compact,
        Standard,
        Comfortable
    };

    static PetuniaTheme &instance() {
        static PetuniaTheme theme;
        return theme;
    }

    void updateFromSnapshot(const QJsonObject &themeColors, const QString &themeId = QStringLiteral("petunia-dark")) {
        m_themeId = themeId;
        auto parse = [&](const QString &key, const QString &def) -> QColor {
            QString val = themeColors.value(key).toString();
            return val.isEmpty() ? QColor(def) : QColor(val);
        };

        m_bgCanvas = parse(QStringLiteral("bg_canvas"), QStringLiteral("#121212"));
        m_bgHeader = parse(QStringLiteral("bg_header"), QStringLiteral("#191919"));
        m_bgPanel = parse(QStringLiteral("bg_panel"), QStringLiteral("#202020"));
        m_bgPanelHeader = parse(QStringLiteral("bg_panel_header"), QStringLiteral("#252525"));
        m_bgSurface = parse(QStringLiteral("bg_surface"), QStringLiteral("#292929"));
        m_bgSurfaceHover = parse(QStringLiteral("bg_surface_hover"), QStringLiteral("#353535"));
        m_bgSurfaceActive = parse(QStringLiteral("bg_surface_active"), QStringLiteral("#404040"));

        m_textPrimary = parse(QStringLiteral("text_primary"), QStringLiteral("#eeeeee"));
        m_textSecondary = parse(QStringLiteral("text_secondary"), QStringLiteral("#b5b5b5"));
        m_textMuted = parse(QStringLiteral("text_muted"), QStringLiteral("#777777"));
        m_textActive = parse(QStringLiteral("text_active"), QStringLiteral("#121212"));

        m_accentPrimary = parse(QStringLiteral("accent_blue"), QStringLiteral("#b58cff")); // floral purple
        m_accentSelection = parse(QStringLiteral("accent_orange"), QStringLiteral("#e96a00")); // selection orange
        m_accentSelectionActive = parse(QStringLiteral("accent_hover"), QStringLiteral("#ffaf29"));

        m_borderSubtle = parse(QStringLiteral("border_subtle"), QStringLiteral("#404040"));
        m_borderStrong = parse(QStringLiteral("border_strong"), QStringLiteral("#5a5a5a"));
        m_borderFocus = parse(QStringLiteral("border_focus"), QStringLiteral("#b58cff"));

        m_statusInfo = parse(QStringLiteral("status_info"), QStringLiteral("#6cb6ff"));
        m_statusWarning = parse(QStringLiteral("status_warning"), QStringLiteral("#e5bd67"));
        m_statusError = parse(QStringLiteral("status_error"), QStringLiteral("#ef6b73"));
        m_statusSuccess = parse(QStringLiteral("status_success"), QStringLiteral("#73d59b"));

        // High contrast overrides for axes if needed
        if (themeId == QStringLiteral("petunia-high-contrast")) {
            m_axisX = QColor(QStringLiteral("#ff3333"));
            m_axisY = QColor(QStringLiteral("#33ff33"));
            m_axisZ = QColor(QStringLiteral("#3399ff"));
        } else {
            m_axisX = QColor(QStringLiteral("#ff6e6e"));
            m_axisY = QColor(QStringLiteral("#69db7c"));
            m_axisZ = QColor(QStringLiteral("#4dabf7"));
        }
    }

    // Getters
    const QString &themeId() const { return m_themeId; }
    Density density() const { return m_density; }
    void setDensity(Density d) { m_density = d; }

    int controlHeight() const {
        switch (m_density) {
        case Density::Compact: return 24;
        case Density::Standard: return 28;
        case Density::Comfortable: return 36;
        }
        return 28;
    }

    const QColor &bgCanvas() const { return m_bgCanvas; }
    const QColor &bgHeader() const { return m_bgHeader; }
    const QColor &bgPanel() const { return m_bgPanel; }
    const QColor &bgPanelHeader() const { return m_bgPanelHeader; }
    const QColor &bgSurface() const { return m_bgSurface; }
    const QColor &bgSurfaceHover() const { return m_bgSurfaceHover; }
    const QColor &bgSurfaceActive() const { return m_bgSurfaceActive; }

    const QColor &textPrimary() const { return m_textPrimary; }
    const QColor &textSecondary() const { return m_textSecondary; }
    const QColor &textMuted() const { return m_textMuted; }
    const QColor &textActive() const { return m_textActive; }

    const QColor &accentPrimary() const { return m_accentPrimary; }
    const QColor &accentSelection() const { return m_accentSelection; }
    const QColor &accentSelectionActive() const { return m_accentSelectionActive; }

    const QColor &borderSubtle() const { return m_borderSubtle; }
    const QColor &borderStrong() const { return m_borderStrong; }
    const QColor &borderFocus() const { return m_borderFocus; }

    const QColor &statusInfo() const { return m_statusInfo; }
    const QColor &statusWarning() const { return m_statusWarning; }
    const QColor &statusError() const { return m_statusError; }
    const QColor &statusSuccess() const { return m_statusSuccess; }

    QColor axisColor(int axis) const {
        switch (axis) {
        case 0: return m_axisX;
        case 1: return m_axisY;
        case 2: return m_axisZ;
        default: return m_textPrimary;
        }
    }

private:
    PetuniaTheme() {
        QJsonObject defaultColors;
        updateFromSnapshot(defaultColors);
    }

    QString m_themeId{QStringLiteral("petunia-dark")};
    Density m_density{Density::Standard};

    QColor m_bgCanvas{QStringLiteral("#121212")};
    QColor m_bgHeader{QStringLiteral("#191919")};
    QColor m_bgPanel{QStringLiteral("#202020")};
    QColor m_bgPanelHeader{QStringLiteral("#252525")};
    QColor m_bgSurface{QStringLiteral("#292929")};
    QColor m_bgSurfaceHover{QStringLiteral("#353535")};
    QColor m_bgSurfaceActive{QStringLiteral("#404040")};

    QColor m_textPrimary{QStringLiteral("#eeeeee")};
    QColor m_textSecondary{QStringLiteral("#b5b5b5")};
    QColor m_textMuted{QStringLiteral("#777777")};
    QColor m_textActive{QStringLiteral("#121212")};

    QColor m_accentPrimary{QStringLiteral("#b58cff")};
    QColor m_accentSelection{QStringLiteral("#e96a00")};
    QColor m_accentSelectionActive{QStringLiteral("#ffaf29")};

    QColor m_borderSubtle{QStringLiteral("#404040")};
    QColor m_borderStrong{QStringLiteral("#5a5a5a")};
    QColor m_borderFocus{QStringLiteral("#b58cff")};

    QColor m_statusInfo{QStringLiteral("#6cb6ff")};
    QColor m_statusWarning{QStringLiteral("#e5bd67")};
    QColor m_statusError{QStringLiteral("#ef6b73")};
    QColor m_statusSuccess{QStringLiteral("#73d59b")};

    QColor m_axisX{QStringLiteral("#ff6e6e")};
    QColor m_axisY{QStringLiteral("#69db7c")};
    QColor m_axisZ{QStringLiteral("#4dabf7")};
};
