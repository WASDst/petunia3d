#pragma once

#include <QColor>
#include <QJsonObject>
#include <QObject>
#include <QPalette>
#include <QString>

/// Fonte única de tokens visuais da GUI Qt (newdocs UI-001 §5, UI-002/011).
///
/// Os valores de cor vêm do snapshot `petunia_config` (nenhuma paleta paralela
/// no C++). Aqui moram as métricas derivadas da densidade e da escala de UI, a
/// paleta Qt e a folha de estilo global — gerada **uma vez por troca de tema**,
/// nunca por evento. Widgets pintam e estilizam a partir destes tokens; literais
/// hex em widgets são proibidos (teste `ui_theme_tokens`).
class PetuniaTheme : public QObject {
    Q_OBJECT

public:
    enum class Density { Compact, Standard, Comfortable };
    /// Papéis tipográficos (px lógicos, já escalados por densidade e ui_scale).
    enum class TextRole { Caption, Body, Title, Mono };

    static PetuniaTheme &instance();

    /// Atualiza as cores. Emite `changed()` somente se algum token mudou.
    void updateFromSnapshot(const QJsonObject &themeColors,
                            const QString &themeId = QStringLiteral("petunia-dark"));
    void setDensity(Density density);
    void setUiScale(double scale);
    void setReducedMotion(bool reduced);

    const QString &themeId() const { return m_themeId; }
    Density density() const { return m_density; }
    double uiScale() const { return m_uiScale; }
    bool reducedMotion() const { return m_reducedMotion; }
    bool isHighContrast() const { return m_themeId == QLatin1String("petunia-high-contrast"); }

    // ── Métricas (px lógicos) ────────────────────────────────────────────
    int controlHeight() const;
    /// Altura de botões de ferramenta e de linhas densas (cabeçalho da vista, rail,
    /// barra contextual, listas). Nunca abaixo do alvo mínimo de 28 px (WCAG 2.5.8).
    int compactControlHeight() const;
    int fontPx(TextRole role) const;
    /// `step` em unidades de espaçamento (0, 1, 2, 3, 4, 6, 8…).
    int space(int step) const;
    int radius() const;
    int iconPx() const;
    int railWidth() const;
    /// Escala um valor em px lógicos pela escala de UI.
    int scaled(int px) const;
    int minHitTarget() const { return scaled(28) < 28 ? 28 : scaled(28); }
    int focusRingWidth() const { return isHighContrast() ? 3 : 2; }
    /// Duração de transição em ms; 0 com movimento reduzido.
    int motionMs(int normal = 140) const { return m_reducedMotion ? 0 : normal; }

    // ── Cores ───────────────────────────────────────────────────────────
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

    QColor axisColor(int axis) const;
    /// Letra do eixo (0 → "X"…): o eixo nunca depende só de cor (UI-015).
    static QString axisLetter(int axis);
    /// Painel translúcido para HUD/overlays sobre o viewport.
    QColor overlaySurface() const;

    // ── Derivados ───────────────────────────────────────────────────────
    QPalette palette() const;
    /// Folha de estilo global (regenerada só quando o tema muda).
    const QString &styleSheet() const;
    /// Aplica paleta + folha de estilo ao `QApplication`.
    void applyToApplication() const;
    /// Razão de contraste WCAG entre duas cores opacas.
    static double contrastRatio(const QColor &a, const QColor &b);

signals:
    void changed();

private:
    PetuniaTheme();
    QString buildStyleSheet() const;
    void markDirty();

    QString m_themeId{QStringLiteral("petunia-dark")};
    Density m_density{Density::Standard};
    double m_uiScale{1.0};
    bool m_reducedMotion{false};

    QColor m_bgCanvas{QStringLiteral("#121212")};
    QColor m_bgHeader{QStringLiteral("#191919")};
    QColor m_bgPanel{QStringLiteral("#202020")};
    QColor m_bgPanelHeader{QStringLiteral("#252525")};
    QColor m_bgSurface{QStringLiteral("#292929")};
    QColor m_bgSurfaceHover{QStringLiteral("#353535")};
    QColor m_bgSurfaceActive{QStringLiteral("#404040")};

    QColor m_textPrimary{QStringLiteral("#eeeeee")};
    QColor m_textSecondary{QStringLiteral("#b5b5b5")};
    QColor m_textMuted{QStringLiteral("#8a8a8a")};
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

    mutable QString m_styleSheet;
    mutable bool m_styleDirty{true};
};
