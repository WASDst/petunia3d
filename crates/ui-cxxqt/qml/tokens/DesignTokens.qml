pragma Singleton
import QtQuick

QtObject {
    // Fundo da viewport e superfícies (Tokens Petunia3D 1:1)
    readonly property color canvas: "#121212"
    readonly property color surface: "#202020"
    readonly property color surfaceRaised: "#292929"
    readonly property color surfaceHover: "#353535"
    readonly property color surfacePressed: "#404040"
    readonly property color border: "#404040"
    readonly property color borderStrong: "#5a5a5a"

    // Tipografia e Cores de Texto
    readonly property color textPrimary: "#eeeeee"
    readonly property color textSecondary: "#b5b5b5"
    readonly property color textMuted: "#9e9e9e"

    // Cores de Destaque e Seleção (Laranja/Amarelo Blender)
    readonly property color accent: "#b58cff"
    readonly property color onAccent: "#121212"
    readonly property color focusRing: "#d1b8ff"
    readonly property color selection: "#e96a00"
    readonly property color selectionActive: "#ffaf29"
    readonly property color hover: "#7ddcff"

    // Eixos 3D
    readonly property color axisX: "#ff6e6e"
    readonly property color axisY: "#69db7c"
    readonly property color axisZ: "#4dabf7"

    // Estados e Feedback
    readonly property color success: "#73d59b"
    readonly property color warning: "#e5bd67"
    readonly property color danger: "#ef6b73"
    readonly property color info: "#6cb6ff"

    // Geometria de Controles
    readonly property int controlHeight: 28
    readonly property int radiusControl: 4
    readonly property int radiusPanel: 8
    readonly property int radiusPill: 14
    readonly property int railWidth: 48

    // Tipografia (Inter)
    readonly property string fontFamily: "Inter"
    readonly property int fontCaption: 11
    readonly property int fontSmall: 12
    readonly property int fontDefault: 13
    readonly property int fontStrong: 14
}
