#pragma once

#include <QFile>
#include <QIcon>
#include <QPainter>
#include <QPixmap>
#include <QString>
#include <QStringList>
#include <QSvgRenderer>

namespace PetuniaIcons {

inline QIcon get(const QString &name, const QString &colorHex = QStringLiteral("#eeeeee")) {
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

    QFile file(foundPath);
    if (file.open(QIODevice::ReadOnly)) {
        QByteArray svgData = file.readAll();
        svgData.replace("currentColor", colorHex.toUtf8());
        QSvgRenderer renderer(svgData);
        if (renderer.isValid()) {
            QPixmap pix(64, 64);
            pix.fill(Qt::transparent);
            QPainter painter(&pix);
            painter.setRenderHint(QPainter::Antialiasing);
            painter.setRenderHint(QPainter::SmoothPixmapTransform);
            renderer.render(&painter);
            return QIcon(pix);
        }
    }

    return QIcon(foundPath);
}

} // namespace PetuniaIcons
