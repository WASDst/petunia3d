import QtQuick
import QtQuick.Controls
import "../tokens"

Slider {
    id: root

    from: 0.0
    to: 1.0
    value: 0.5

    background: Rectangle {
        x: root.leftPadding
        y: root.topPadding + root.availableHeight / 2 - height / 2
        implicitWidth: 120
        implicitHeight: 4
        width: root.availableWidth
        height: implicitHeight
        radius: 2
        color: DesignTokens.surfacePressed

        Rectangle {
            width: root.visualPosition * parent.width
            height: parent.height
            color: DesignTokens.accent
            radius: 2
        }
    }

    handle: Rectangle {
        x: root.leftPadding + root.visualPosition * (root.availableWidth - width)
        y: root.topPadding + root.availableHeight / 2 - height / 2
        implicitWidth: 14
        implicitHeight: 14
        radius: 7
        color: root.pressed ? Qt.darker(DesignTokens.accent, 1.2) : (root.hovered ? Qt.lighter(DesignTokens.accent, 1.1) : DesignTokens.accent)
        border.color: DesignTokens.focusRing
        border.width: 1
    }
}
