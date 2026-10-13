import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "../tokens"

Rectangle {
    id: root
    width: DesignTokens.railWidth
    color: DesignTokens.surface
    border.color: DesignTokens.border
    border.width: 1

    property string activeTool: "select"
    signal toolSelected(string toolId)

    readonly property var tools: [
        { id: "select", icon: "⇱", label: "Select (Q)" },
        { id: "move", icon: "✥", label: "Translate (G/W)" },
        { id: "rotate", icon: "↻", label: "Rotate (R/E)" },
        { id: "scale", icon: "⤢", label: "Scale (S/R)" },
        { id: "pen", icon: "✎", label: "Draw Pen" },
        { id: "extrude", icon: "⤊", label: "Extrude (E)" },
        { id: "loop_cut", icon: "⌗", label: "Loop Cut (Ctrl+R)" },
        { id: "slice", icon: "✂", label: "Knife / Slice (K)" }
    ]

    ColumnLayout {
        anchors.fill: parent
        anchors.topMargin: 8
        anchors.bottomMargin: 8
        spacing: 4

        Repeater {
            model: root.tools
            delegate: Rectangle {
                Layout.alignment: Qt.AlignHCenter
                width: 36
                height: 36
                radius: DesignTokens.radiusControl
                color: root.activeTool === modelData.id
                       ? DesignTokens.accent
                       : (mouseArea.containsMouse ? DesignTokens.surfaceHover : "transparent")
                border.color: root.activeTool === modelData.id ? DesignTokens.focusRing : "transparent"
                border.width: 1

                Text {
                    anchors.centerIn: parent
                    text: modelData.icon
                    font.pixelSize: 18
                    color: root.activeTool === modelData.id ? DesignTokens.onAccent : DesignTokens.textPrimary
                }

                MouseArea {
                    id: mouseArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.activeTool = modelData.id;
                        root.toolSelected(modelData.id);
                    }
                }

                ToolTip.visible: mouseArea.containsMouse
                ToolTip.text: modelData.label
                ToolTip.delay: 400
            }
        }

        Item { Layout.fillHeight: true }
    }
}
