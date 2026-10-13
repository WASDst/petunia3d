import QtQuick
import QtQuick.Layouts
import "../tokens"

Rectangle {
    id: root
    height: 24
    color: DesignTokens.surface
    border.color: DesignTokens.border
    border.width: 1

    property string modeText: "OBJECT"
    property string activeToolName: "Select"
    property string shortcutHint: "Middle Mouse: Orbit | Shift+MMB: Pan | Scroll: Zoom"
    property int selectedCount: 0
    property int fps: 60

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.rightMargin: 10
        spacing: 12

        // Modo Badge
        Rectangle {
            height: 16
            width: modeLabel.implicitWidth + 12
            radius: 3
            color: root.modeText === "EDIT" ? DesignTokens.warning : DesignTokens.accent
            Text {
                id: modeLabel
                anchors.centerIn: parent
                text: root.modeText
                color: DesignTokens.onAccent
                font.family: DesignTokens.fontFamily
                font.pixelSize: 10
                font.bold: true
            }
        }

        Text {
            text: root.activeToolName
            color: DesignTokens.textPrimary
            font.family: DesignTokens.fontFamily
            font.pixelSize: DesignTokens.fontSmall
            font.bold: true
        }

        Text {
            text: "·  " + root.shortcutHint
            color: DesignTokens.textMuted
            font.family: DesignTokens.fontFamily
            font.pixelSize: DesignTokens.fontCaption
        }

        Item { Layout.fillWidth: true }

        Text {
            text: root.selectedCount > 0 ? "Selected: " + root.selectedCount : "No selection"
            color: root.selectedCount > 0 ? DesignTokens.selection : DesignTokens.textSecondary
            font.family: DesignTokens.fontFamily
            font.pixelSize: DesignTokens.fontCaption
        }

        Text {
            text: root.fps + " FPS"
            color: DesignTokens.success
            font.family: DesignTokens.fontFamily
            font.pixelSize: DesignTokens.fontCaption
        }
    }
}
