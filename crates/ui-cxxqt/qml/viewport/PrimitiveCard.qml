import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../tokens"
import "../components"

Rectangle {
    id: root

    property string primitiveType: "Cubo"
    property real sizeValue: 2.0
    property int subdivisions: 4
    property int segments: 16

    signal confirmed(string type, real size, int subs, int segs)
    signal cancelled()

    width: 260
    height: contentCol.height + 24
    radius: DesignTokens.radiusMd
    color: DesignTokens.surfaceElevated
    border.color: DesignTokens.borderMuted
    border.width: 1

    ColumnLayout {
        id: contentCol
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 12
        spacing: 10

        // Cabeçalho
        RowLayout {
            Layout.fillWidth: true
            Text {
                text: "Parâmetros: " + root.primitiveType
                color: DesignTokens.textPrimary
                font.pixelSize: DesignTokens.fontSizeSm
                font.weight: Font.DemiBold
                Layout.fillWidth: true
            }

            PetuniaIconButton {
                iconText: "✕"
                onClicked: root.cancelled()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: DesignTokens.borderMuted
        }

        // Tamanho / Dimensão
        RowLayout {
            Layout.fillWidth: true
            Text {
                text: "Tamanho"
                color: DesignTokens.textSecondary
                font.pixelSize: DesignTokens.fontSizeXs
                Layout.preferredWidth: 70
            }
            NumericInput {
                Layout.fillWidth: true
                value: root.sizeValue
                suffix: " m"
                onValueChanged: root.sizeValue = value
            }
        }

        // Subdivisões
        RowLayout {
            Layout.fillWidth: true
            Text {
                text: "Subdivisões"
                color: DesignTokens.textSecondary
                font.pixelSize: DesignTokens.fontSizeXs
                Layout.preferredWidth: 70
            }
            NumericInput {
                Layout.fillWidth: true
                value: root.subdivisions
                step: 1
                min: 1
                max: 64
                onValueChanged: root.subdivisions = Math.round(value)
            }
        }

        // Segmentos (para esferas/cilindros)
        RowLayout {
            Layout.fillWidth: true
            visible: root.primitiveType !== "Cubo" && root.primitiveType !== "Plano"
            Text {
                text: "Segmentos"
                color: DesignTokens.textSecondary
                font.pixelSize: DesignTokens.fontSizeXs
                Layout.preferredWidth: 70
            }
            NumericInput {
                Layout.fillWidth: true
                value: root.segments
                step: 2
                min: 3
                max: 128
                onValueChanged: root.segments = Math.round(value)
            }
        }

        // Ações
        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Layout.topMargin: 4

            PetuniaButton {
                text: "Cancelar"
                variant: "ghost"
                Layout.fillWidth: true
                onClicked: root.cancelled()
            }

            PetuniaButton {
                text: "Adicionar"
                variant: "primary"
                Layout.fillWidth: true
                onClicked: root.confirmed(root.primitiveType, root.sizeValue, root.subdivisions, root.segments)
            }
        }
    }
}
