import QtQuick
import QtQuick.Controls
import "../tokens"

Rectangle {
    id: root

    property string label: ""
    property real value: 0.0
    property real minimumValue: -10000.0
    property real maximumValue: 10000.0
    property real stepSize: 0.01
    property string unit: ""
    property int decimals: 2

    signal valueModified(real newValue)

    height: DesignTokens.controlHeight
    implicitWidth: 120
    radius: DesignTokens.radiusControl
    color: scrubArea.pressed ? DesignTokens.surfacePressed : (scrubArea.containsMouse ? DesignTokens.surfaceHover : DesignTokens.surfaceRaised)
    border.color: textEdit.activeFocus ? DesignTokens.accent : DesignTokens.border
    border.width: 1

    property bool isEditing: false

    Row {
        anchors.fill: parent
        anchors.margins: 4
        spacing: 4

        // Rótulo opcional (X, Y, Z, R, etc.)
        Text {
            id: labelItem
            visible: root.label !== ""
            text: root.label
            font.family: DesignTokens.fontFamily
            font.pixelSize: DesignTokens.fontSmall
            font.bold: true
            color: DesignTokens.textSecondary
            anchors.verticalCenter: parent.verticalCenter
        }

        // Valor Numérico / Campo de Edição
        TextInput {
            id: textEdit
            width: parent.width - (labelItem.visible ? labelItem.width + 8 : 0) - (unitItem.visible ? unitItem.width + 4 : 0)
            anchors.verticalCenter: parent.verticalCenter
            text: root.value.toFixed(root.decimals)
            font.family: DesignTokens.fontFamily
            font.pixelSize: DesignTokens.fontSmall
            color: DesignTokens.textPrimary
            selectByMouse: true
            horizontalAlignment: Text.AlignRight

            onAccepted: {
                let parsed = parseFloat(text);
                if (!isNaN(parsed)) {
                    root.value = Math.max(root.minimumValue, Math.min(root.maximumValue, parsed));
                    root.valueModified(root.value);
                }
                root.isEditing = false;
                focus = false;
            }

            onActiveFocusChanged: {
                if (!activeFocus) {
                    text = root.value.toFixed(root.decimals);
                    root.isEditing = false;
                }
            }
        }

        // Unidade (m, px, °)
        Text {
            id: unitItem
            visible: root.unit !== ""
            text: root.unit
            font.family: DesignTokens.fontFamily
            font.pixelSize: DesignTokens.fontCaption
            color: DesignTokens.textMuted
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    // Área de Scrub (Arrastar Horizontalmente)
    MouseArea {
        id: scrubArea
        anchors.fill: parent
        enabled: !root.isEditing
        cursorShape: Qt.SizeHorCursor
        hoverEnabled: true

        property real startX: 0
        property real startValue: 0

        onPressed: (mouse) => {
            startX = mouse.x;
            startValue = root.value;
        }

        onPositionChanged: (mouse) => {
            if (pressed) {
                let delta = mouse.x - startX;
                let scale = (mouse.modifiers & Qt.ShiftModifier) ? 0.001 : (root.stepSize);
                let newVal = startValue + (delta * scale);
                root.value = Math.max(root.minimumValue, Math.min(root.maximumValue, newVal));
                textEdit.text = root.value.toFixed(root.decimals);
                root.valueModified(root.value);
            }
        }

        onDoubleClicked: {
            root.isEditing = true;
            textEdit.forceActiveFocus();
            textEdit.selectAll();
        }
    }
}
