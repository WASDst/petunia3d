#pragma once

#include <QCoreApplication>
#include <QString>

/// Traduz um texto-fonte (português do código ou inglês dos catálogos do editor)
/// pelo tradutor nativo instalado (`petuniaSetTranslations`). Sem entrada no
/// catálogo, devolve o próprio texto. O teste `ui_i18n_catalog` garante que os
/// textos do Shell v2 existam em `native-translations.json`.
inline QString petuniaTr(const QString &source) {
    if (source.isEmpty())
        return source;
    return QCoreApplication::translate("PetuniaShell", source.toUtf8().constData());
}
