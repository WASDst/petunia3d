import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../tokens"

Item {
    id: root

    property string iconSource: ""
    property string title: ""
    property string explanation: ""
    property string actionText: ""

    signal actionTriggered()

    implicitWidth: 300
    implicitHeight: 200

    Accessible.role: Accessible.StaticText
    Accessible.name: root.title
    Accessible.description: root.explanation

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 10
        width: Math.min(parent.width - 32, 280)

        // Ícone Central Opcional
        Image {
            id: iconItem
            visible: root.iconSource !== ""
            source: root.iconSource
            sourceSize.width: 40
            sourceSize.height: 40
            Layout.alignment: Qt.AlignHCenter
            opacity: 0.7
        }

        // Título em Destaque
        Text {
            visible: root.title !== ""
            text: root.title
            font.family: DesignTokens.fontFamily
            font.pixelSize: DesignTokens.fontStrong
            font.bold: true
            color: DesignTokens.textPrimary
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        // Explicação / Instrução de Próximo Passo
        Text {
            visible: root.explanation !== ""
            text: root.explanation
            font.family: DesignTokens.fontFamily
            font.pixelSize: DesignTokens.fontSmall
            color: DesignTokens.textMuted
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        // Botão de Ação Primária
        PetuniaButton {
            visible: root.actionText !== ""
            text: root.actionText
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 4
            onClicked: root.actionTriggered()
        }
    }
}
