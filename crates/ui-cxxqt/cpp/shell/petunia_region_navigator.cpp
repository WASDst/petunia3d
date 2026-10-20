#include "petunia_region_navigator.h"

#include <QApplication>

PetuniaRegionNavigator::PetuniaRegionNavigator(QObject *parent) : QObject(parent) {
    connect(qApp, &QApplication::focusChanged, this, &PetuniaRegionNavigator::onFocusChanged);
}

void PetuniaRegionNavigator::setRegions(const QList<QWidget *> &regions) {
    m_regions.clear();
    for (auto *region : regions)
        m_regions.append(region);
}

int PetuniaRegionNavigator::regionOf(const QWidget *focus) const {
    if (!focus)
        return -1;
    for (int i = 0; i < m_regions.size(); ++i) {
        const auto &region = m_regions[i];
        if (region && (region == focus || region->isAncestorOf(focus)))
            return i;
    }
    return -1;
}

void PetuniaRegionNavigator::onFocusChanged(QWidget *, QWidget *now) {
    const int index = regionOf(now);
    if (index >= 0)
        m_lastFocus.insert(m_regions[index], now);
}

QWidget *PetuniaRegionNavigator::firstFocusable(QWidget *region) {
    if (!region || !region->isVisible() || !region->isEnabled())
        return nullptr;
    if (region->focusPolicy() & Qt::TabFocus)
        return region;
    for (auto *child : region->findChildren<QWidget *>()) {
        if (child->isVisible() && child->isEnabled() && (child->focusPolicy() & Qt::TabFocus))
            return child;
    }
    return nullptr;
}

QWidget *PetuniaRegionNavigator::cycle(int direction, QWidget *current) {
    if (m_regions.isEmpty())
        return nullptr;
    if (!current)
        current = QApplication::focusWidget();
    const int count = m_regions.size();
    const int start = regionOf(current);
    const int step = direction >= 0 ? 1 : -1;
    for (int n = 1; n <= count; ++n) {
        const int index = ((start < 0 ? (step > 0 ? -1 : 0) : start) + step * n + count * 4) % count;
        QWidget *region = m_regions[index];
        if (!region || !region->isVisible() || !region->isEnabled())
            continue;
        QWidget *target = m_lastFocus.value(region);
        if (!target || !target->isVisible() || !target->isEnabled() || !region->isAncestorOf(target))
            target = firstFocusable(region);
        if (!target)
            continue;
        target->setFocus(Qt::TabFocusReason);
        return target;
    }
    return nullptr;
}
