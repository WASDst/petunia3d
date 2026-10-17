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

/// Rótulo de uma opção de escolha publicada por id estável em inglês
/// (`base_color` → "Base color", traduzido; `x` → "X").
inline QString petuniaChoiceLabel(const QString &id) {
    if (id.size() == 1)
        return id.toUpper();
    QString words = id;
    words.replace(QLatin1Char('_'), QLatin1Char(' '));
    return petuniaTr(words.left(1).toUpper() + words.mid(1));
}
