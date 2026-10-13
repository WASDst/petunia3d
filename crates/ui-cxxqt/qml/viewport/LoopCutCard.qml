import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import "../tokens"

Rectangle {
    id: root
    width: 200
    height: 110
    radius: DesignTokens.radiusPanel
    color: DesignTokens.surface
    border.color: DesignTokens.borderStrong
    border.width: 1

    property int cuts: 1
    property real slide: 0.0

    signal cutsChanged(int count)
    signal slideChanged(real value)
    signal confirmed()

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 6

        // Cabeçalho do Card
        RowLayout {
            Text {
                text: "Loop Cut & Slide"
                color: DesignTokens.textPrimary
                font.bold: true
                font.pixelSize: DesignTokens.fontSmall
            }
            Item { Layout.fillWidth: true }
            Text {
                text: "Ctrl+R"
                color: DesignTokens.accent
                font.pixelSize: DesignTokens.fontCaption
            }
        }

        // Número de Cortes
        RowLayout {
            Text {
                text: "Cuts:"
                color: DesignTokens.textSecondary
                font.pixelSize: DesignTokens.fontSmall
            }
            Item { Layout.fillWidth: true }
            Button {
                text: "-"
                width: 22
                height: 22
                onClicked: {
                    if (root.cuts > 1) {
                        root.cuts -= 1;
                        root.cutsChanged(root.cuts);
                    }
                }
            }
            Text {
                text: root.cuts.toString()
                color: DesignTokens.textPrimary
                font.bold: true
                font.pixelSize: DesignTokens.fontSmall
            }
            Button {
                text: "+"
                width: 22
                height: 22
                onClicked: {
                    root.cuts += 1;
                    root.cutsChanged(root.cuts);
                }
            }
        }

        // Deslizamento (Slide)
        RowLayout {
            Text {
                text: "Slide:"
                color: DesignTokens.textSecondary
                font.pixelSize: DesignTokens.fontSmall
            }
            Slider {
                Layout.fillWidth: true
                from: -1.0
                to: 1.0
                value: root.slide
                onMoved: {
                    root.slide = value;
                    root.slideChanged(value);
                }
            }
        }
    }
}
