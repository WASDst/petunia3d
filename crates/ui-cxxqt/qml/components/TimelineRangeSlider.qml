import QtQuick
import QtQuick.Controls
import "../tokens"

RangeSlider {
    id: root

    property int totalFrames: 250
    property int startFrame: 1
    property int endFrame: 250

    signal playbackRangeModified(int start, int end)

    from: 1
    to: Math.max(1, totalFrames)
    stepSize: 1
    snapMode: RangeSlider.SnapAlways

    first.value: startFrame
    second.value: endFrame

    first.onMoved: {
        root.startFrame = Math.round(first.value);
        root.playbackRangeModified(root.startFrame, root.endFrame);
    }

    second.onMoved: {
        root.endFrame = Math.round(second.value);
        root.playbackRangeModified(root.startFrame, root.endFrame);
    }

    implicitWidth: 260
    implicitHeight: 22

    Accessible.role: Accessible.Slider
    Accessible.name: qsTr("Timeline Playback Range")
    Accessible.description: qsTr("Intervalo de reprodução de frames ativo")

    // Trilho (Track)
    background: Rectangle {
        x: root.leftPadding
        y: root.topPadding + root.availableHeight / 2 - height / 2
        implicitWidth: 200
        implicitHeight: 4
        width: root.availableWidth
        height: implicitHeight
        radius: 2
        color: DesignTokens.surfaceRaised
        border.color: DesignTokens.border
        border.width: 1

        // Intervalo Ativo Selecionado (In / Out Highlight)
        Rectangle {
            x: root.first.visualPosition * parent.width
            width: (root.second.visualPosition - root.first.visualPosition) * parent.width
            height: parent.height
            color: DesignTokens.selectionActive
            radius: 2
        }
    }

    // Alça Inicial (In-Point Handle)
    first.handle: Rectangle {
        x: root.leftPadding + root.first.visualPosition * (root.availableWidth - width)
        y: root.topPadding + root.availableHeight / 2 - height / 2
        implicitWidth: 10
        implicitHeight: 18
        radius: 2
        color: root.first.pressed ? DesignTokens.surfacePressed : (root.first.hovered ? DesignTokens.surfaceHover : DesignTokens.accent)
        border.color: DesignTokens.textPrimary
        border.width: 1

        ToolTip.visible: root.first.hovered || root.first.pressed
        ToolTip.text: qsTr("In: %1").arg(Math.round(root.first.value))
        ToolTip.delay: 300
    }

    // Alça Final (Out-Point Handle)
    second.handle: Rectangle {
        x: root.leftPadding + root.second.visualPosition * (root.availableWidth - width)
        y: root.topPadding + root.availableHeight / 2 - height / 2
        implicitWidth: 10
        implicitHeight: 18
        radius: 2
        color: root.second.pressed ? DesignTokens.surfacePressed : (root.second.hovered ? DesignTokens.surfaceHover : DesignTokens.accent)
        border.color: DesignTokens.textPrimary
        border.width: 1

        ToolTip.visible: root.second.hovered || root.second.pressed
        ToolTip.text: qsTr("Out: %1").arg(Math.round(root.second.value))
        ToolTip.delay: 300
    }
}
