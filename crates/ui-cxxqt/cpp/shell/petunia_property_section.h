#pragma once

#include <QFrame>
#include <QToolButton>
#include <QVBoxLayout>

/// Seção recolhível do painel de Propriedades (padrão `PropertyGroup` do
/// design system): cabeçalho inteiro clicável, seta, resumo opcional e ação de
/// cabeçalho. Estado aberto/fechado é preferência de layout, não do documento.
class PetuniaPropertySection : public QFrame {
    Q_OBJECT

  public:
    PetuniaPropertySection(const QString &id, const QString &title, QWidget *content,
                           QWidget *parent = nullptr);

    const QString &sectionId() const { return m_id; }
    bool isOpen() const { return m_open; }
    /// Abre/fecha sem emitir `toggled` (restauração de layout).
    void setOpen(bool open);
    void setTitle(const QString &title);
    /// Texto curto ao lado do título (ex.: "2 operações").
    void setSummary(const QString &summary);
    /// Widget no canto direito do cabeçalho (ex.: botão "+").
    void setHeaderAction(QWidget *action);
    QWidget *content() const { return m_content; }
    QToolButton *header() const { return m_header; }

  signals:
    void toggled(const QString &id, bool open);

  private:
    void refreshHeader();
    void applyOpen(bool open);

    QString m_id;
    QString m_title;
    QString m_summary;
    bool m_open{true};
    QToolButton *m_header{nullptr};
    QWidget *m_content{nullptr};
    QWidget *m_actionHost{nullptr};
};
