import QtQuick
import "../tokens"

Rectangle {
    id: root

    property color swatchColor: "#ffffff"
    property bool selected: false
    signal clicked()

    width: 24
    height: 24
    radius: 4
    color: swatchColor
    border.width: root.selected ? 2 : 1
    border.color: root.selected ? DesignTokens.focusRing : DesignTokens.border

    // Indicador sutil de seleção interna
    Rectangle {
        anchors.fill: parent
        anchors.margins: 2
        radius: 2
        color: "transparent"
        border.color: "#ffffff"
        border.width: 1
        visible: root.selected
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
