import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../tokens"

DoubleSpinBox {
    id: root

    property string label: ""
    property real minimumValue: -10000.0
    property real maximumValue: 10000.0
    property string unit: ""

    // Aliases para compatibilidade com código existente e convenções diversas
    property alias min: root.minimumValue
    property alias max: root.maximumValue
    property alias step: root.stepSize
    property alias suffix: root.unit

    from: minimumValue
    to: maximumValue
    stepSize: 0.01
    decimals: 2
    editable: true

    signal valueModified(real newValue)

    // Acessibilidade semântica nativa do Qt Quick Controls
    Accessible.role: Accessible.SpinBox
    Accessible.name: root.label !== "" ? root.label : "Numeric Input"
    Accessible.description: root.unit

    implicitWidth: 120
    implicitHeight: DesignTokens.controlHeight

    // Remove botões convencionais de seta do estilo clássico de formulário para comportamento DCC
    up.indicator: Item { width: 0; height: 0; visible: false }
    down.indicator: Item { width: 0; height: 0; visible: false }

    background: Rectangle {
        implicitWidth: 120
        implicitHeight: DesignTokens.controlHeight
        radius: DesignTokens.radiusControl
        color: scrubArea.pressed ? DesignTokens.surfacePressed : (scrubArea.containsMouse ? DesignTokens.surfaceHover : DesignTokens.surfaceRaised)
        border.color: root.activeFocus ? DesignTokens.accent : DesignTokens.border
        border.width: 1
    }

    contentItem: Item {
        implicitHeight: DesignTokens.controlHeight

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 6
            anchors.rightMargin: 6
            spacing: 4

            // Rótulo opcional (X, Y, Z, Tamanho, etc.)
            Text {
                visible: root.label !== ""
                text: root.label
                font.family: DesignTokens.fontFamily
                font.pixelSize: DesignTokens.fontSmall
                font.bold: true
                color: DesignTokens.textSecondary
                Layout.alignment: Qt.AlignVCenter
            }

            // Entrada de texto nativa do SpinBox
            TextInput {
                id: textInput
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                text: root.textFromValue(root.value, root.locale)
                font.family: DesignTokens.fontFamily
                font.pixelSize: DesignTokens.fontSmall
                color: DesignTokens.textPrimary
                selectionColor: DesignTokens.accent
                selectedTextColor: DesignTokens.onAccent
                horizontalAlignment: Text.AlignRight
                verticalAlignment: Text.AlignVCenter
                readOnly: !root.editable
                validator: root.validator
                inputMethodHints: Qt.ImhFormattedNumbersOnly
                selectByMouse: true

                onAccepted: {
                    let parsed = root.valueFromText(text, root.locale);
                    if (!isNaN(parsed)) {
                        root.value = Math.max(root.from, Math.min(root.to, parsed));
                        root.valueModified(root.value);
                    }
                    root.focus = false;
                }
            }

            // Unidade ou sufixo (m, px, °, etc.)
            Text {
                visible: root.unit !== ""
                text: root.unit
                font.family: DesignTokens.fontFamily
                font.pixelSize: DesignTokens.fontCaption
                color: DesignTokens.textMuted
                Layout.alignment: Qt.AlignVCenter
            }
        }

        // Camada de Scrub do padrão 3D DCC (arraste horizontal do mouse)
        MouseArea {
            id: scrubArea
            anchors.fill: parent
            cursorShape: Qt.SizeHorCursor
            hoverEnabled: true
            enabled: !textInput.activeFocus

            property real startX: 0
            property real startValue: 0

            onPressed: (mouse) => {
                startX = mouse.x;
                startValue = root.value;
                root.forceActiveFocus();
            }

            onPositionChanged: (mouse) => {
                if (pressed) {
                    let delta = mouse.x - startX;
                    let scale = (mouse.modifiers & Qt.ShiftModifier) ? 0.001 : (root.stepSize);
                    let newVal = startValue + (delta * scale);
                    root.value = Math.max(root.from, Math.min(root.to, newVal));
                    root.valueModified(root.value);
                }
            }

            onDoubleClicked: {
                textInput.forceActiveFocus();
                textInput.selectAll();
            }
        }
    }
}
