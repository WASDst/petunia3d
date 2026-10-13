import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../tokens"

Rectangle {
    id: root

    signal commandSelected(string cmdId)
    signal closed()

    color: Qt.rgba(0, 0, 0, 0.6)
    anchors.fill: parent

    // Captura cliques fora para fechar
    MouseArea {
        anchors.fill: parent
        onClicked: root.closed()
    }

    Rectangle {
        id: container
        width: Math.min(540, parent.width - 40)
        height: 360
        anchors.centerIn: parent
        radius: DesignTokens.radiusMd
        color: DesignTokens.surfaceElevated
        border.color: DesignTokens.borderMuted
        border.width: 1

        MouseArea {
            anchors.fill: parent
            // Impede fechar ao clicar dentro do card
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 8

            // Campo de busca
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 38
                radius: DesignTokens.radiusSm
                color: DesignTokens.surfaceBase
                border.color: searchField.activeFocus ? DesignTokens.accentPrimary : DesignTokens.borderMuted
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 8

                    Text {
                        text: "🔍"
                        font.pixelSize: 12
                        color: DesignTokens.textMuted
                    }

                    TextInput {
                        id: searchField
                        Layout.fillWidth: true
                        color: DesignTokens.textPrimary
                        font.pixelSize: DesignTokens.fontSizeSm
                        selectByMouse: true
                        clip: true

                        Text {
                            text: "Pesquisar comandos (ex: cubo, desfazer, duplicar)..."
                            color: DesignTokens.textMuted
                            font.pixelSize: DesignTokens.fontSizeSm
                            visible: !searchField.text && !searchField.activeFocus
                        }

                        Keys.onDownPressed: {
                            if (cmdList.count > 0) {
                                cmdList.currentIndex = (cmdList.currentIndex + 1) % cmdList.count;
                            }
                        }
                        Keys.onUpPressed: {
                            if (cmdList.count > 0) {
                                cmdList.currentIndex = (cmdList.currentIndex - 1 + cmdList.count) % cmdList.count;
                            }
                        }
                        Keys.onReturnPressed: {
                            if (cmdList.currentItem) {
                                root.commandSelected(cmdList.model.get(cmdList.currentIndex).cmdId);
                                root.closed();
                            }
                        }
                        Keys.onEscapePressed: {
                            root.closed();
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: DesignTokens.borderMuted
            }

            // Lista de resultados
            ListView {
                id: cmdList
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true

                model: ListModel {
                    id: cmdModel
                    ListElement { name: "Adicionar Cubo"; category: "MODEL"; shortcut: "Shift+A"; cmdId: "model.add_cube" }
                    ListElement { name: "Adicionar Esfera"; category: "MODEL"; shortcut: ""; cmdId: "model.add_sphere" }
                    ListElement { name: "Adicionar Cilindro"; category: "MODEL"; shortcut: ""; cmdId: "model.add_cylinder" }
                    ListElement { name: "Adicionar Plano"; category: "MODEL"; shortcut: ""; cmdId: "model.add_plane" }
                    ListElement { name: "Duplicar Objeto"; category: "EDIT"; shortcut: "Shift+D"; cmdId: "edit.duplicate" }
                    ListElement { name: "Deletar Seleção"; category: "EDIT"; shortcut: "Del"; cmdId: "edit.delete" }
                    ListElement { name: "Desfazer"; category: "EDIT"; shortcut: "Ctrl+Z"; cmdId: "app.undo" }
                    ListElement { name: "Refazer"; category: "EDIT"; shortcut: "Ctrl+Y"; cmdId: "app.redo" }
                    ListElement { name: "Selecionar Tudo"; category: "SELECT"; shortcut: "A"; cmdId: "select.all" }
                    ListElement { name: "Limpar Seleção"; category: "SELECT"; shortcut: "Alt+A"; cmdId: "select.none" }
                }

                delegate: Rectangle {
                    width: cmdList.width
                    height: 36
                    radius: DesignTokens.radiusSm
                    color: ListView.isCurrentItem ? DesignTokens.accentPrimary : (hoverArea.containsMouse ? DesignTokens.surfaceHover : "transparent")

                    MouseArea {
                        id: hoverArea
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            cmdList.currentIndex = index;
                            root.commandSelected(model.cmdId);
                            root.closed();
                        }
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: 8

                        // Categoria badge
                        Rectangle {
                            Layout.preferredWidth: catText.implicitWidth + 8
                            Layout.preferredHeight: 18
                            radius: 3
                            color: ListView.isCurrentItem ? Qt.rgba(0,0,0,0.2) : DesignTokens.surfaceBase

                            Text {
                                id: catText
                                anchors.centerIn: parent
                                text: model.category
                                font.pixelSize: 10
                                font.weight: Font.Bold
                                color: ListView.isCurrentItem ? DesignTokens.bgCanvas : DesignTokens.textSecondary
                            }
                        }

                        // Nome do comando
                        Text {
                            text: model.name
                            font.pixelSize: DesignTokens.fontSizeSm
                            font.weight: ListView.isCurrentItem ? Font.Bold : Font.Normal
                            color: ListView.isCurrentItem ? DesignTokens.bgCanvas : DesignTokens.textPrimary
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                        }

                        // Atalho de teclado
                        Text {
                            text: model.shortcut
                            font.pixelSize: DesignTokens.fontSizeXs
                            color: ListView.isCurrentItem ? DesignTokens.bgCanvas : DesignTokens.textMuted
                            visible: model.shortcut !== ""
                        }
                    }
                }
            }
        }
    }

    Component.onCompleted: {
        searchField.forceActiveFocus();
    }
}
