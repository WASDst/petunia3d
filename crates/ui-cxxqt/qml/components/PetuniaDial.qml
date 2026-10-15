import QtQuick
import QtQuick.Controls
import "../tokens"

Dial {
    id: root

    from: 0
    to: 360
    stepSize: 1
    wrap: true

    implicitWidth: 48
    implicitHeight: 48

    Accessible.role: Accessible.Dial
    Accessible.name: qsTr("Ângulo")
    Accessible.description: qsTr("Ajuste angular de 0 a 360 graus")

    background: Rectangle {
        id: bg
        x: root.width / 2 - width / 2
        y: root.height / 2 - height / 2
        width: Math.max(16, Math.min(root.width, root.height) - 4)
        height: width
        radius: width / 2
        color: DesignTokens.surface
        border.color: root.visualFocus ? DesignTokens.focusRing : (root.hovered ? DesignTokens.borderStrong : DesignTokens.border)
        border.width: root.visualFocus ? 2 : 1

        // Pivot central
        Rectangle {
            anchors.centerIn: parent
            width: 6
            height: 6
            radius: 3
            color: DesignTokens.surfacePressed
            border.color: DesignTokens.border
            border.width: 1
        }
    }

    handle: Item {
        id: handleItem
        x: root.width / 2
        y: root.height / 2
        transformOrigin: Item.TopLeft
        rotation: root.angle

        Rectangle {
            x: -1
            y: -Math.min(root.width, root.height) / 2 + 6
            width: 2
            height: Math.max(4, Math.min(root.width, root.height) / 2 - 9)
            radius: 1
            color: root.pressed ? Qt.lighter(DesignTokens.accent, 1.2) : DesignTokens.accent
        }

        Rectangle {
            x: -3
            y: -Math.min(root.width, root.height) / 2 + 4
            width: 6
            height: 6
            radius: 3
            color: root.pressed ? Qt.lighter(DesignTokens.accent, 1.2) : DesignTokens.accent
        }
    }
}
