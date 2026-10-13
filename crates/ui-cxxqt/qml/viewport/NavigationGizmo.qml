import QtQuick
import "../tokens"

Rectangle {
    id: root
    width: 80
    height: 80
    color: "transparent"

    signal viewAligned(string axis)

    // Círculo translúcido de fundo
    Rectangle {
        anchors.fill: parent
        radius: width / 2
        color: "#20202066"
        border.color: DesignTokens.border
        border.width: 1
    }

    // Eixo X (Vermelho)
    Rectangle {
        width: 18
        height: 18
        radius: 9
        color: DesignTokens.axisX
        x: parent.width - 24
        y: parent.height / 2 - 9
        Text {
            anchors.centerIn: parent
            text: "X"
            color: "#ffffff"
            font.bold: true
            font.pixelSize: 10
        }
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: root.viewAligned("X")
        }
    }

    // Eixo Y (Verde)
    Rectangle {
        width: 18
        height: 18
        radius: 9
        color: DesignTokens.axisY
        x: parent.width / 2 - 9
        y: 6
        Text {
            anchors.centerIn: parent
            text: "Y"
            color: "#ffffff"
            font.bold: true
            font.pixelSize: 10
        }
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: root.viewAligned("Y")
        }
    }

    // Eixo Z (Azul)
    Rectangle {
        width: 18
        height: 18
        radius: 9
        color: DesignTokens.axisZ
        x: 6
        y: parent.height / 2 - 9
        Text {
            anchors.centerIn: parent
            text: "Z"
            color: "#ffffff"
            font.bold: true
            font.pixelSize: 10
        }
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: root.viewAligned("Z")
        }
    }
}
