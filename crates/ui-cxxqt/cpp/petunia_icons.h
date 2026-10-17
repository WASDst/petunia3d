#pragma once

#include <QFile>
#include <QHash>
#include <QIcon>
#include <QPainter>
#include <QPixmap>
#include <QString>
#include <QStringList>
#include <QSvgRenderer>

#include "petunia_theme.h"

namespace PetuniaIcons {

inline QHash<QString, QIcon> &iconCache() {
    static QHash<QString, QIcon> cache;
    return cache;
}

inline void clearCache() {
    iconCache().clear();
}

/// Ícone do conjunto `petunia-outline` (embutido em `:/icons/<id>.svg`).
/// Sem `colorHex`, usa os tokens do tema (texto normal e texto atenuado para o
/// modo Disabled); o cache é invalidado quando o tema muda.
inline QIcon get(const QString &name, const QString &colorHexArg = QString()) {
    static const bool connected = [] {
        QObject::connect(&PetuniaTheme::instance(), &PetuniaTheme::changed, [] { iconCache().clear(); });
        return true;
    }();
    (void)connected;
    const auto &theme = PetuniaTheme::instance();
    const QString colorHex = colorHexArg.isEmpty() ? theme.textPrimary().name() : colorHexArg;
    const QString cacheKey = name + QLatin1Char(':') + colorHex;
    auto &cache = iconCache();
    auto it = cache.constFind(cacheKey);
    if (it != cache.constEnd()) {
        return *it;
    }

    QString foundPath;
    const QString qrcPath = QStringLiteral(":/icons/") + name + QStringLiteral(".svg");
    if (QFile::exists(qrcPath)) {
        foundPath = qrcPath;
    } else {
        const QString toolbarPath = QStringLiteral(":/toolbar/outline/") + name + QStringLiteral(".svg");
        if (QFile::exists(toolbarPath)) {
            foundPath = toolbarPath;
        } else {
            const QStringList searchDirs = {
                QStringLiteral("assets/icons/petunia-outline/svg/"),
                QStringLiteral("../../assets/icons/petunia-outline/svg/"),
                QStringLiteral("../../../assets/icons/petunia-outline/svg/")
            };
            for (const auto &dir : searchDirs) {
                for (const auto &cat : {
                         "interface", "selection-transform", "mesh-modeling",
                         "viewport-scene", "painting", "primitives", "future-roadmap",
                         "properties", "uv-materials", "shape-first"
                     }) {
                    const QString candidate = dir + QLatin1String(cat) + QLatin1Char('/') + name + QStringLiteral(".svg");
                    if (QFile::exists(candidate)) {
                        foundPath = candidate;
                        break;
                    }
                }
                if (!foundPath.isEmpty()) break;
            }
        }
    }

    if (foundPath.isEmpty()) {
        return QIcon();
    }

    QIcon result;
    QFile file(foundPath);
    if (file.open(QIODevice::ReadOnly)) {
        const QByteArray source = file.readAll();
        auto render = [&](const QString &color) {
            QByteArray svgData = source;
            svgData.replace("currentColor", color.toUtf8());
            QSvgRenderer renderer(svgData);
            QPixmap pix(64, 64);
            pix.fill(Qt::transparent);
            if (renderer.isValid()) {
                QPainter painter(&pix);
                painter.setRenderHint(QPainter::Antialiasing);
                painter.setRenderHint(QPainter::SmoothPixmapTransform);
                renderer.render(&painter);
            }
            return pix;
        };
        const QPixmap normal = render(colorHex);
        if (!normal.isNull()) {
            result.addPixmap(normal, QIcon::Normal);
            result.addPixmap(normal, QIcon::Active);
            result.addPixmap(render(theme.textMuted().name()), QIcon::Disabled);
        }
    }

    if (result.isNull()) {
        result = QIcon(foundPath);
    }
    cache.insert(cacheKey, result);
    return result;
}

} // namespace PetuniaIcons
