import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../tokens"

Item {
    id: root

    property real panX: 40.0
    property real panY: 40.0
    property real zoom: 1.0

    property var nodes: [
        {
            id: "node_texture_1",
            title: "Textura Base",
            x: 60,
            y: 80,
            width: 160,
            inputs: [],
            outputs: [{ name: "Texture", type: "Texture", color: "#b58cff" }]
        },
        {
            id: "node_color_1",
            title: "Cor de Realce",
            x: 60,
            y: 220,
            width: 160,
            inputs: [],
            outputs: [{ name: "Color", type: "Color", color: "#e5bd67" }]
        },
        {
            id: "node_mix_1",
            title: "Mix / Misturar",
            x: 300,
            y: 120,
            width: 170,
            inputs: [
                { name: "Base", type: "Texture", color: "#b58cff" },
                { name: "Camada", type: "Color", color: "#e5bd67" },
                { name: "Fator", type: "Value", color: "#6cb6ff" }
            ],
            outputs: [{ name: "Resultado", type: "Texture", color: "#b58cff" }]
        },
        {
            id: "node_output_1",
            title: "Saída de Material PBR",
            x: 550,
            y: 100,
            width: 180,
            inputs: [
                { name: "BaseColor", type: "Texture", color: "#b58cff" },
                { name: "Roughness", type: "Value", color: "#6cb6ff" },
                { name: "Metallic", type: "Value", color: "#6cb6ff" },
                { name: "Normal", type: "Texture", color: "#b58cff" }
            ],
            outputs: []
        }
    ]

    property var connections: [
        { fromNode: "node_texture_1", fromPort: 0, toNode: "node_mix_1", toPort: 0, color: "#b58cff" },
        { fromNode: "node_color_1", fromPort: 0, toNode: "node_mix_1", toPort: 1, color: "#e5bd67" },
        { fromNode: "node_mix_1", fromPort: 0, toNode: "node_output_1", toPort: 0, color: "#b58cff" }
    ]

    property string connectingFromNode: ""
    property int connectingFromPort: -1
    property real dragMouseX: 0
    property real dragMouseY: 0
    property color dragWireColor: "#b58cff"

    signal nodeAdded(string type, real x, real y)
    signal connectionCreated(string fromNode, int fromPort, string toNode, int toPort)
    signal connectionRemoved(int index)
    signal nodeMoved(string id, real x, real y)

    Accessible.role: Accessible.Graphic
    Accessible.name: qsTr("Editor de Grafo de Nós (Shader & Modifiers)")
    clip: true

    // 1. Fundo com Grid Cartesiano Escuro
    Rectangle {
        anchors.fill: parent
        color: "#181818"

        Canvas {
            id: gridCanvas
            anchors.fill: parent
            onPaint: {
                var ctx = getContext("2d");
                ctx.clearRect(0, 0, width, height);

                var step = 24 * root.zoom;
                var offsetX = root.panX % step;
                var offsetY = root.panY % step;

                // Grade fina
                ctx.strokeStyle = "#242424";
                ctx.lineWidth = 1;
                ctx.beginPath();
                for (var x = offsetX; x < width; x += step) {
                    ctx.moveTo(x, 0);
                    ctx.lineTo(x, height);
                }
                for (var y = offsetY; y < height; y += step) {
                    ctx.moveTo(0, y);
                    ctx.lineTo(width, y);
                }
                ctx.stroke();

                // Grade principal (a cada 4 passos)
                var majorStep = step * 4;
                var majorOffsetX = root.panX % majorStep;
                var majorOffsetY = root.panY % majorStep;

                ctx.strokeStyle = "#303030";
                ctx.lineWidth = 1.2;
                ctx.beginPath();
                for (var mx = majorOffsetX; mx < width; mx += majorStep) {
                    ctx.moveTo(mx, 0);
                    ctx.lineTo(mx, height);
                }
                for (var my = majorOffsetY; my < height; my += majorStep) {
                    ctx.moveTo(0, my);
                    ctx.lineTo(width, my);
                }
                ctx.stroke();
            }
        }
    }

    // 2. Área Interativa de Navegação (Pan e Zoom)
    MouseArea {
        id: panArea
        anchors.fill: parent
        acceptedButtons: Qt.MiddleButton | Qt.RightButton
        hoverEnabled: true

        property real lastX: 0
        property real lastY: 0

        onPressed: (mouse) => {
            lastX = mouse.x;
            lastY = mouse.y;
            if (mouse.button === Qt.RightButton) {
                addNodeMenu.x = mouse.x;
                addNodeMenu.y = mouse.y;
                addNodeMenu.open();
            }
        }

        onPositionChanged: (mouse) => {
            if (pressedButtons & Qt.MiddleButton) {
                root.panX += (mouse.x - lastX);
                root.panY += (mouse.y - lastY);
                lastX = mouse.x;
                lastY = mouse.y;
                gridCanvas.requestPaint();
                cablesCanvas.requestPaint();
            }
        }

        onWheel: (wheel) => {
            var factor = wheel.angleDelta.y > 0 ? 1.1 : 0.9;
            var newZoom = Math.max(0.4, Math.min(2.5, root.zoom * factor));
            root.zoom = newZoom;
            gridCanvas.requestPaint();
            cablesCanvas.requestPaint();
        }
    }

    // 3. Camada de Cabos / Arestas Bézier
    Canvas {
        id: cablesCanvas
        anchors.fill: parent
        renderTarget: Canvas.FramebufferObject

        onPaint: {
            var ctx = getContext("2d");
            ctx.clearRect(0, 0, width, height);

            function getNodeSocketPos(nodeId, isOutput, portIdx) {
                for (var i = 0; i < root.nodes.length; i++) {
                    var n = root.nodes[i];
                    if (n.id === nodeId) {
                        var nx = root.panX + n.x * root.zoom;
                        var ny = root.panY + n.y * root.zoom;
                        var nw = n.width * root.zoom;
                        var py = ny + (36 + portIdx * 24 + 12) * root.zoom;
                        var px = isOutput ? (nx + nw) : nx;
                        return { x: px, y: py };
                    }
                }
                return { x: 0, y: 0 };
            }

            // Conexões estabelecidas
            for (var c = 0; c < root.connections.length; c++) {
                var conn = root.connections[c];
                var p1 = getNodeSocketPos(conn.fromNode, true, conn.fromPort);
                var p2 = getNodeSocketPos(conn.toNode, false, conn.toPort);

                var dx = Math.abs(p2.x - p1.x) * 0.5;
                if (dx < 30) dx = 30;

                ctx.strokeStyle = conn.color || "#b58cff";
                ctx.lineWidth = 2.5 * root.zoom;
                ctx.beginPath();
                ctx.moveTo(p1.x, p1.y);
                ctx.bezierCurveTo(p1.x + dx, p1.y, p2.x - dx, p2.y, p2.x, p2.y);
                ctx.stroke();
            }

            // Cabo dinâmico durante arraste
            if (root.connectingFromNode !== "") {
                var dragStart = getNodeSocketPos(root.connectingFromNode, true, root.connectingFromPort);
                var dragEnd = { x: root.dragMouseX, y: root.dragMouseY };
                var ddx = Math.abs(dragEnd.x - dragStart.x) * 0.5;
                if (ddx < 30) ddx = 30;

                ctx.strokeStyle = root.dragWireColor;
                ctx.lineWidth = 2.0 * root.zoom;
                ctx.setLineDash([4, 4]);
                ctx.beginPath();
                ctx.moveTo(dragStart.x, dragStart.y);
                ctx.bezierCurveTo(dragStart.x + ddx, dragStart.y, dragEnd.x - ddx, dragEnd.y, dragEnd.x, dragEnd.y);
                ctx.stroke();
                ctx.setLineDash([]);
            }
        }
    }

    // 4. Instâncias Visuais dos Nós
    Repeater {
        model: root.nodes

        Item {
            id: nodeItem
            x: root.panX + modelData.x * root.zoom
            y: root.panY + modelData.y * root.zoom
            width: modelData.width * root.zoom
            height: (40 + Math.max(modelData.inputs.length, modelData.outputs.length) * 24 + 10) * root.zoom

            Rectangle {
                anchors.fill: parent
                color: "#242424"
                radius: 6 * root.zoom
                border.color: dragNodeArea.pressed ? DesignTokens.accent : "#3c3c3c"
                border.width: dragNodeArea.pressed ? 2 : 1

                // Barra de Título do Nó
                Rectangle {
                    width: parent.width
                    height: 28 * root.zoom
                    color: "#2c2c2c"
                    radius: 6 * root.zoom

                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 6 * root.zoom
                        color: "#2c2c2c"
                    }

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 8 * root.zoom
                        anchors.verticalCenter: parent.verticalCenter
                        text: modelData.title
                        color: "#ffffff"
                        font.family: DesignTokens.fontFamily
                        font.pixelSize: Math.max(9, 12 * root.zoom)
                        font.bold: true
                    }
                }

                // Sockets de Entrada (Esquerda)
                Column {
                    anchors.left: parent.left
                    anchors.leftMargin: -5 * root.zoom
                    anchors.top: parent.top
                    anchors.topMargin: 36 * root.zoom
                    spacing: 12 * root.zoom

                    Repeater {
                        model: modelData.inputs
                        Row {
                            spacing: 4 * root.zoom
                            Rectangle {
                                width: 10 * root.zoom
                                height: 10 * root.zoom
                                radius: 5 * root.zoom
                                color: modelData.color
                                border.color: "#ffffff"
                                border.width: 1

                                MouseArea {
                                    anchors.fill: parent
                                    onReleased: {
                                        if (root.connectingFromNode !== "") {
                                            root.connections.push({
                                                fromNode: root.connectingFromNode,
                                                fromPort: root.connectingFromPort,
                                                toNode: nodeItem.modelData.id,
                                                toPort: index,
                                                color: root.dragWireColor
                                            });
                                            root.connectionCreated(root.connectingFromNode, root.connectingFromPort, nodeItem.modelData.id, index);
                                            root.connectingFromNode = "";
                                            cablesCanvas.requestPaint();
                                        }
                                    }
                                }
                            }
                            Text {
                                text: modelData.name
                                color: "#aaaaaa"
                                font.pixelSize: Math.max(8, 10 * root.zoom)
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                    }
                }

                // Sockets de Saída (Direita)
                Column {
                    anchors.right: parent.right
                    anchors.rightMargin: -5 * root.zoom
                    anchors.top: parent.top
                    anchors.topMargin: 36 * root.zoom
                    spacing: 12 * root.zoom

                    Repeater {
                        model: modelData.outputs
                        Row {
                            spacing: 4 * root.zoom
                            Text {
                                text: modelData.name
                                color: "#aaaaaa"
                                font.pixelSize: Math.max(8, 10 * root.zoom)
                                anchors.verticalCenter: parent.verticalCenter
                            }
                            Rectangle {
                                width: 10 * root.zoom
                                height: 10 * root.zoom
                                radius: 5 * root.zoom
                                color: modelData.color
                                border.color: "#ffffff"
                                border.width: 1

                                MouseArea {
                                    anchors.fill: parent
                                    onPressed: {
                                        root.connectingFromNode = nodeItem.modelData.id;
                                        root.connectingFromPort = index;
                                        root.dragWireColor = modelData.color;
                                        root.dragMouseX = nodeItem.x + nodeItem.width;
                                        root.dragMouseY = nodeItem.y + (36 + index * 24 + 5) * root.zoom;
                                    }
                                    onPositionChanged: (mouse) => {
                                        var map = mapToItem(root, mouse.x, mouse.y);
                                        root.dragMouseX = map.x;
                                        root.dragMouseY = map.y;
                                        cablesCanvas.requestPaint();
                                    }
                                    onReleased: {
                                        root.connectingFromNode = "";
                                        cablesCanvas.requestPaint();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Área de Arraste do Nó
            MouseArea {
                id: dragNodeArea
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.right: parent.right
                height: 28 * root.zoom
                hoverEnabled: true

                property real startX: 0
                property real startY: 0

                onPressed: (mouse) => {
                    startX = mouse.x;
                    startY = mouse.y;
                }

                onPositionChanged: (mouse) => {
                    if (pressed) {
                        var dx = (mouse.x - startX) / root.zoom;
                        var dy = (mouse.y - startY) / root.zoom;
                        modelData.x += dx;
                        modelData.y += dy;
                        cablesCanvas.requestPaint();
                    }
                }

                onReleased: {
                    root.nodeMoved(modelData.id, modelData.x, modelData.y);
                }
            }
        }
    }

    // 5. Menu de Adição Rápida de Nós (Shift+A / Botão Direito)
    Menu {
        id: addNodeMenu

        MenuItem {
            text: qsTr("Entrada de Textura")
            onTriggered: {
                var rx = (addNodeMenu.x - root.panX) / root.zoom;
                var ry = (addNodeMenu.y - root.panY) / root.zoom;
                root.nodeAdded("TextureInput", rx, ry);
            }
        }
        MenuItem {
            text: qsTr("Cor Constante (RGB)")
            onTriggered: {
                var rx = (addNodeMenu.x - root.panX) / root.zoom;
                var ry = (addNodeMenu.y - root.panY) / root.zoom;
                root.nodeAdded("ColorInput", rx, ry);
            }
        }
        MenuItem {
            text: qsTr("Valor Numérico (Float)")
            onTriggered: {
                var rx = (addNodeMenu.x - root.panX) / root.zoom;
                var ry = (addNodeMenu.y - root.panY) / root.zoom;
                root.nodeAdded("ValueInput", rx, ry);
            }
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("Mix / Misturador")
            onTriggered: {
                var rx = (addNodeMenu.x - root.panX) / root.zoom;
                var ry = (addNodeMenu.y - root.panY) / root.zoom;
                root.nodeAdded("Mix", rx, ry);
            }
        }
        MenuItem {
            text: qsTr("Ruído Procedural (Perlin)")
            onTriggered: {
                var rx = (addNodeMenu.x - root.panX) / root.zoom;
                var ry = (addNodeMenu.y - root.panY) / root.zoom;
                root.nodeAdded("Noise", rx, ry);
            }
        }
    }

    // 6. HUD / Legenda de Sockets no Canto Inferior
    Rectangle {
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.margins: 12
        width: 210
        height: 28
        color: "#202020"
        radius: 4
        border.color: "#3a3a3a"
        border.width: 1

        RowLayout {
            anchors.fill: parent
            anchors.margins: 6
            spacing: 8

            Rectangle { width: 8; height: 8; radius: 4; color: "#b58cff" }
            Text { text: "Texture"; color: "#bbb"; font.pixelSize: 10 }

            Rectangle { width: 8; height: 8; radius: 4; color: "#e5bd67" }
            Text { text: "Color"; color: "#bbb"; font.pixelSize: 10 }

            Rectangle { width: 8; height: 8; radius: 4; color: "#6cb6ff" }
            Text { text: "Value"; color: "#bbb"; font.pixelSize: 10 }
        }
    }
}
