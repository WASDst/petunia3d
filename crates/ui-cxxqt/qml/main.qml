import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "tokens"
import "shell"
import "viewport"

ApplicationWindow {
    id: window
    width: 1280
    height: 800
    visible: true
    title: "Petunia3D — CXX-Qt Studio"
    color: DesignTokens.canvas

    Shortcut {
        sequence: "Ctrl+P"
        onActivated: commandPalette.visible = true
    }
    Shortcut {
        sequence: "Ctrl+K"
        onActivated: commandPalette.visible = true
    }
    Shortcut {
        sequence: "F3"
        onActivated: commandPalette.visible = true
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // 1. Header com Pílula Animada de Workspaces
        Header {
            id: header
            Layout.fillWidth: true
            onWorkspaceChanged: (ws) => {
                console.log("Workspace selecionado:", ws);
                if (typeof shellBridge !== "undefined") {
                    shellBridge.switch_workspace(ws);
                }
            }
            onActionTriggered: (action) => {
                console.log("Ação disparada:", action);
                if (action === "palette.open") {
                    commandPalette.visible = true;
                } else if (typeof shellBridge !== "undefined") {
                    shellBridge.dispatch_action(action);
                }
            }
        }

        // 2. Área Central (ToolRail + Viewport 3D + Inspetor)
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // Trilho de Ferramentas Esquerdo
            ToolRail {
                id: toolRail
                Layout.fillHeight: true
                onToolSelected: (tool) => {
                    statusBar.activeToolName = tool;
                    if (tool === "Cubo" || tool === "Esfera") {
                        primitiveCard.primitiveType = tool;
                        primitiveCard.visible = true;
                    }
                }
            }

            // Viewport Central com Overlays
            Item {
                id: viewportContainer
                Layout.fillWidth: true
                Layout.fillHeight: true

                // Fundo neutro caso o widget GL esteja desacoplado
                Rectangle {
                    anchors.fill: parent
                    color: DesignTokens.canvas
                }

                // Gizmo de Navegação (Canto Superior Direito)
                NavigationGizmo {
                    anchors.top: parent.top
                    anchors.right: parent.right
                    anchors.margins: 16
                    onViewAligned: (axis) => {
                        console.log("Alinhar câmera ao eixo:", axis);
                    }
                }

                // Card Flutuante de Ferramenta (Loop Cut)
                LoopCutCard {
                    anchors.left: parent.left
                    anchors.bottom: parent.bottom
                    anchors.margins: 16
                }

                // Card Flutuante de Criação Paramétrica de Primitivas
                PrimitiveCard {
                    id: primitiveCard
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.margins: 16
                    visible: false
                    onConfirmed: (type, size, subs, segs) => {
                        if (typeof shellBridge !== "undefined") {
                            shellBridge.dispatch_action("model.add_cube");
                        }
                        primitiveCard.visible = false;
                    }
                    onCancelled: {
                        primitiveCard.visible = false;
                    }
                }
            }

            // Inspetor Direito (Painel de Propriedades)
            Rectangle {
                width: 280
                Layout.fillHeight: true
                color: DesignTokens.surface
                border.color: DesignTokens.border
                border.width: 1

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 8

                    // Seção Outliner (Partes da Cena)
                    Text {
                        text: qsTr("OUTLINER")
                        color: DesignTokens.textSecondary
                        font.family: DesignTokens.fontFamily
                        font.pixelSize: DesignTokens.fontSmall
                        font.bold: true
                    }

                    OutlinerTreeView {
                        id: outliner
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        Layout.minimumHeight: 180
                        onAddPrimitiveRequested: {
                            primitiveCard.primitiveType = "Cubo";
                            primitiveCard.visible = true;
                        }
                        onVisibilityToggled: (id, visible) => {
                            if (typeof shellBridge !== "undefined") {
                                shellBridge.dispatch_action("model.toggle_visibility");
                            }
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        height: 1
                        color: DesignTokens.border
                    }

                    // Seção Inspetor de Propriedades
                    Text {
                        text: qsTr("INSPECTOR")
                        color: DesignTokens.textSecondary
                        font.family: DesignTokens.fontFamily
                        font.pixelSize: DesignTokens.fontSmall
                        font.bold: true
                    }

                    Text {
                        text: qsTr("Transform")
                        color: DesignTokens.textPrimary
                        font.bold: true
                        font.pixelSize: DesignTokens.fontSmall
                    }

                    RowLayout {
                        Text { text: "X:"; color: DesignTokens.axisX; font.bold: true }
                        Text { text: "0.00 m"; color: DesignTokens.textPrimary }
                    }
                    RowLayout {
                        Text { text: "Y:"; color: DesignTokens.axisY; font.bold: true }
                        Text { text: "0.00 m"; color: DesignTokens.textPrimary }
                    }
                    RowLayout {
                        Text { text: "Z:"; color: DesignTokens.axisZ; font.bold: true }
                        Text { text: "0.00 m"; color: DesignTokens.textPrimary }
                    }

                    Item { Layout.preferredHeight: 10 }
                }
            }
        }

        // 3. Barra de Status Inferior
        StatusBar {
            id: statusBar
            Layout.fillWidth: true
        }
    }

    // 4. Command Palette Overlay (Spotlight search flutuante no topo)
    CommandPalette {
        id: commandPalette
        anchors.fill: parent
        visible: false
        z: 999
        onCommandSelected: (cmdId) => {
            if (typeof shellBridge !== "undefined") {
                shellBridge.dispatch_action(cmdId);
            }
        }
        onClosed: {
            commandPalette.visible = false;
        }
    }
}
