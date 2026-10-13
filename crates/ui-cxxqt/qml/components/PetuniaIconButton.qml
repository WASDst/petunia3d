import QtQuick
import QtQuick.Controls
import "../tokens"

Button {
    id: root

    property string tooltipText: ""
    property int size: DesignTokens.controlHeight
    property bool selected: false

    width: size
    height: size

    background: Rectangle {
        radius: DesignTokens.radiusControl
        border.width: 1
        border.color: root.selected ? DesignTokens.focusRing : (root.activeFocus ? DesignTokens.focusRing : "transparent")
        color: {
            if (root.selected) return DesignTokens.accent;
            if (root.down) return DesignTokens.surfacePressed;
            if (root.hovered) return DesignTokens.surfaceHover;
            return "transparent";
        }
    }

    contentItem: Text {
        text: root.text
        font.family: DesignTokens.fontFamily
        font.pixelSize: 14
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        color: root.selected ? DesignTokens.onAccent : (root.hovered ? DesignTokens.textPrimary : DesignTokens.textSecondary)
    }

    ToolTip.visible: root.hovered && root.tooltipText !== ""
    ToolTip.text: root.tooltipText
    ToolTip.delay: 400
}
