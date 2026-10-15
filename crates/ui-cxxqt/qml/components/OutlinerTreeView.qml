import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "../tokens"

Item {
    id: root

    property alias model: treeView.model
    property alias filterText: searchField.text
    property bool filterSelectedOnly: false
    property int selectedCount: 0

    signal objectSelected(string id, int index)
    signal visibilityToggled(string id, bool visible)
    signal lockToggled(string id, bool locked)
    signal addPrimitiveRequested()
    signal deleteRequested()

    Accessible.role: Accessible.Tree
    Accessible.name: qsTr("Árvore de partes da cena (Outliner)")

    ColumnLayout {
        anchors.fill: parent
        spacing: 4

        // 1. Barra de Busca e Filtros Rápidos
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 4
            spacing: 4

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 26
                color: DesignTokens.surfaceRaised
                radius: DesignTokens.radiusControl
                border.color: searchField.activeFocus ? DesignTokens.focusRing : DesignTokens.border
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 6
                    anchors.rightMargin: 6
                    spacing: 4

                    Text {
                        text: "🔍"
                        font.pixelSize: 11
                        color: DesignTokens.textMuted
                    }

                    TextField {
                        id: searchField
                        Layout.fillWidth: true
                        placeholderText: qsTr("Buscar partes...")
                        placeholderTextColor: DesignTokens.textMuted
                        color: DesignTokens.textPrimary
                        font.family: DesignTokens.fontFamily
                        font.pixelSize: DesignTokens.fontSmall
                        background: null
                        leftPadding: 0
                        rightPadding: 0
                        topPadding: 0
                        bottomPadding: 0
                        Accessible.name: qsTr("Filtrar partes por nome")
                    }

                    ToolButton {
                        visible: searchField.text.length > 0
                        text: "✕"
                        Layout.preferredWidth: 16
                        Layout.preferredHeight: 16
                        onClicked: searchField.text = ""
                    }
                }
            }

            ToolButton {
                id: filterBtn
                checkable: true
                checked: root.filterSelectedOnly
                text: "⚡"
                Layout.preferredWidth: 26
                Layout.preferredHeight: 26
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Exibir apenas selecionados")
                Accessible.name: qsTr("Exibir apenas selecionados")
                onToggled: root.filterSelectedOnly = checked
            }

            ToolButton {
                id: addBtn
                text: "+"
                Layout.preferredWidth: 26
                Layout.preferredHeight: 26
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Adicionar Primitiva (Shift+A)")
                Accessible.name: qsTr("Adicionar primitiva")
                onClicked: root.addPrimitiveRequested()
            }
        }

        // 2. TreeView Hierárquico de Alta Performance
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            TreeView {
                id: treeView
                anchors.fill: parent
                clip: true
                boundsBehavior: Flickable.StopAtBounds

                Accessible.role: Accessible.Tree
                Accessible.name: qsTr("Hierarquia de Objetos")

                delegate: TreeViewDelegate {
                    id: control
                    implicitHeight: 24

                    Accessible.role: Accessible.TreeItem
                    Accessible.name: model ? (model.display || qsTr("Parte")) : qsTr("Parte")

                    background: Rectangle {
                        color: control.selected
                               ? DesignTokens.selection
                               : (control.hovered ? DesignTokens.surfaceHover : "transparent")
                        radius: 2
                    }

                    contentItem: RowLayout {
                        spacing: 4

                        Text {
                            text: (model && model.iconName) ? "📦" : "🧊"
                            font.pixelSize: 11
                            color: control.selected ? "#ffffff" : DesignTokens.accent
                        }

                        Text {
                            Layout.fillWidth: true
                            text: model ? (model.display || "") : ""
                            color: control.selected ? "#ffffff" : DesignTokens.textPrimary
                            font.family: DesignTokens.fontFamily
                            font.pixelSize: DesignTokens.fontSmall
                            elide: Text.ElideRight
                        }

                        // Botão de Visibilidade (Eye)
                        ToolButton {
                            id: eyeBtn
                            Layout.preferredWidth: 20
                            Layout.preferredHeight: 20
                            text: (model && model.visible === false) ? "🚫" : "👁"
                            font.pixelSize: 10
                            Accessible.name: qsTr("Alternar visibilidade")
                            onClicked: {
                                if (model && model.id) {
                                    root.visibilityToggled(model.id, model.visible === false)
                                }
                            }
                        }

                        // Botão de Bloqueio (Lock)
                        ToolButton {
                            id: lockBtn
                            Layout.preferredWidth: 20
                            Layout.preferredHeight: 20
                            text: (model && model.locked) ? "🔒" : "🔓"
                            font.pixelSize: 10
                            opacity: (model && model.locked) ? 1.0 : (control.hovered ? 0.6 : 0.0)
                            Accessible.name: qsTr("Alternar bloqueio")
                            onClicked: {
                                if (model && model.id) {
                                    root.lockToggled(model.id, !model.locked)
                                }
                            }
                        }
                    }
                }
            }

            // 3. Empty State quando não houver itens
            PlaceholderMessage {
                anchors.fill: parent
                visible: treeView.rows === 0
                title: qsTr("Cena Vazia")
                explanation: searchField.text.length > 0
                             ? qsTr("Nenhum objeto corresponde à busca.")
                             : qsTr("Nenhum objeto na cena. Adicione primitivas para começar a modelar.")
                actionText: qsTr("+ Adicionar Primitiva")
                onActionTriggered: root.addPrimitiveRequested()
            }
        }
    }
}
