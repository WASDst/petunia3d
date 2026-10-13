import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "../tokens"

Rectangle {
    id: root
    height: 44
    color: DesignTokens.surface
    border.color: DesignTokens.border
    border.width: 1

    property string activeWorkspace: "DRAW"
    signal workspaceChanged(string workspace)
    signal actionTriggered(string action)

    readonly property var workspaces: [
        { id: "DRAW", label: "DRAW", shortcut: "F1" },
        { id: "POLY", label: "POLY", shortcut: "F2" },
        { id: "PAINT", label: "PAINT", shortcut: "F3" },
        { id: "UV", label: "UV", shortcut: "F4" },
        { id: "ANIMATE", label: "ANIMATE", shortcut: "F5" }
    ]

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 12
        spacing: 12

        // Logo e Identidade
        RowLayout {
            spacing: 8
            Rectangle {
                width: 20
                height: 20
                radius: 4
                color: DesignTokens.accent
                Text {
                    anchors.centerIn: parent
                    text: "P"
                    color: DesignTokens.onAccent
                    font.bold: true
                    font.pixelSize: 12
                }
            }
            Text {
                text: "Petunia3D"
                color: DesignTokens.textPrimary
                font.family: DesignTokens.fontFamily
                font.pixelSize: DesignTokens.fontStrong
                font.bold: true
            }
        }

        Item { Layout.fillWidth: true }

        // Workspace Sliding Pill (Fiel ao Slint)
        Rectangle {
            id: pillContainer
            width: workspaces.length * 76 + 4
            height: 30
            radius: DesignTokens.radiusPill
            color: DesignTokens.surfaceRaised
            border.color: DesignTokens.border
            border.width: 1

            // Pílula deslizante animada
            Rectangle {
                id: activeIndicator
                width: 76
                height: 26
                radius: DesignTokens.radiusPill - 2
                color: DesignTokens.accent
                y: 2

                property int targetIndex: {
                    for (let i = 0; i < root.workspaces.length; ++i) {
                        if (root.workspaces[i].id === root.activeWorkspace) return i;
                    }
                    return 0;
                }

                x: 2 + targetIndex * 76

                Behavior on x {
                    NumberAnimation {
                        duration: 180
                        easing.type: Easing.OutCubic
                    }
                }
            }

            // Abas de Workspace
            Row {
                anchors.fill: parent
                anchors.margins: 2

                Repeater {
                    model: root.workspaces
                    delegate: Rectangle {
                        width: 76
                        height: 26
                        color: "transparent"

                        Text {
                            anchors.centerIn: parent
                            text: modelData.label
                            color: root.activeWorkspace === modelData.id ? DesignTokens.onAccent : DesignTokens.textSecondary
                            font.family: DesignTokens.fontFamily
                            font.pixelSize: DesignTokens.fontSmall
                            font.bold: root.activeWorkspace === modelData.id
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                root.activeWorkspace = modelData.id;
                                root.workspaceChanged(modelData.id);
                            }
                        }
                    }
                }
            }
        }

        Item { Layout.fillWidth: true }

        // Botões de Ação Rápida (Desfazer, Refazer, Configurações)
        RowLayout {
            spacing: 6

            Button {
                text: "↶ Undo"
                height: DesignTokens.controlHeight
                background: Rectangle {
                    color: parent.hovered ? DesignTokens.surfaceHover : DesignTokens.surfaceRaised
                    radius: DesignTokens.radiusControl
                    border.color: DesignTokens.border
                }
                contentItem: Text {
                    text: parent.text
                    color: DesignTokens.textPrimary
                    font.pixelSize: DesignTokens.fontSmall
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                onClicked: root.actionTriggered("app.undo")
            }

            Button {
                text: "↷ Redo"
                height: DesignTokens.controlHeight
                background: Rectangle {
                    color: parent.hovered ? DesignTokens.surfaceHover : DesignTokens.surfaceRaised
                    radius: DesignTokens.radiusControl
                    border.color: DesignTokens.border
                }
                contentItem: Text {
                    text: parent.text
                    color: DesignTokens.textPrimary
                    font.pixelSize: DesignTokens.fontSmall
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                onClicked: root.actionTriggered("app.redo")
            }
        }
    }
}
