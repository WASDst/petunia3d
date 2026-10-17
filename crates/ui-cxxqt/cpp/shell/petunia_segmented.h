#pragma once

#include <QButtonGroup>
#include <QHash>
#include <QList>
#include <QPushButton>
#include <QWidget>

/// Controle segmentado exclusivo (P0 do design system): uma escolha entre poucas
/// opções, com foco em seta (roving), nome acessível e estado desabilitado com
/// motivo. Só emite `activated` por ação do usuário — `setCurrent` é silencioso.
class PetuniaSegmented : public QWidget {
    Q_OBJECT

  public:
    struct Item {
        QString id;
        QString text;     ///< vazio = só ícone (o tooltip e o nome acessível continuam)
        QString tooltip;
        QString icon;     ///< id do ícone petunia-outline
    };

    explicit PetuniaSegmented(QWidget *parent = nullptr);

    void setItems(const QList<Item> &items);
    QString current() const { return m_current; }
    /// Marca o item sem emitir sinal (sincronização com o estado do editor).
    void setCurrent(const QString &id);
    void setItemVisible(const QString &id, bool visible);
    void setItemEnabled(const QString &id, bool enabled, const QString &reason = QString());
    void setGroupLabel(const QString &label);
    QPushButton *button(const QString &id) const { return m_buttons.value(id); }
    QList<QPushButton *> buttons() const { return m_order; }

  signals:
    void activated(const QString &id);

  protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

  private:
    void rebuildCorners();
    void applyRoving();
    void refreshIcons();

    QHash<QString, QPushButton *> m_buttons;
    QList<QPushButton *> m_order;
    QHash<QString, QString> m_icons;
    QButtonGroup *m_group{nullptr};
    QString m_current;
};
