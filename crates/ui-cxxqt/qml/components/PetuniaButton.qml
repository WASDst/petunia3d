import QtQuick
import QtQuick.Controls
import "../tokens"

Button {
    id: root

    property string variant: "secondary" // "primary" | "secondary" | "danger" | "ghost"
    property int buttonHeight: DesignTokens.controlHeight

    height: buttonHeight
    implicitWidth: Math.max(contentLabel.implicitWidth + 20, 64)

    background: Rectangle {
        radius: DesignTokens.radiusControl
        border.width: 1
        border.color: {
            if (root.activeFocus) return DesignTokens.focusRing;
            if (root.variant === "primary") return DesignTokens.accent;
            if (root.variant === "danger") return DesignTokens.danger;
            if (root.variant === "ghost") return "transparent";
            return DesignTokens.border;
        }
        color: {
            if (root.variant === "primary") {
                return root.down ? Qt.darker(DesignTokens.accent, 1.15)
                                 : (root.hovered ? Qt.lighter(DesignTokens.accent, 1.1) : DesignTokens.accent);
            }
            if (root.variant === "danger") {
                return root.down ? Qt.darker(DesignTokens.danger, 1.2)
                                 : (root.hovered ? Qt.lighter(DesignTokens.danger, 1.1) : DesignTokens.danger);
            }
            if (root.variant === "ghost") {
                return root.down ? DesignTokens.surfacePressed
                                 : (root.hovered ? DesignTokens.surfaceHover : "transparent");
            }
            // Secondary (default)
            return root.down ? DesignTokens.surfacePressed
                             : (root.hovered ? DesignTokens.surfaceHover : DesignTokens.surfaceRaised);
        }

        Behavior on color {
            ColorAnimation { duration: 100 }
        }
    }

    contentItem: Text {
        id: contentLabel
        text: root.text
        font.family: DesignTokens.fontFamily
        font.pixelSize: DesignTokens.fontSmall
        font.weight: root.variant === "primary" ? Font.Bold : Font.Normal
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        color: {
            if (root.variant === "primary") return DesignTokens.onAccent;
            if (root.variant === "danger") return "#ffffff";
            return DesignTokens.textPrimary;
        }
    }
}
