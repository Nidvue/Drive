// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    property string glyph: ""
    property string label: ""
    property string hint: ""
    property bool active: false

    signal clicked()

    implicitHeight: 28
    radius: Theme.radiusMd
    color: root.active ? Theme.surfaceHover : (hover.hovered ? Theme.surfaceAlt : "transparent")
    border.width: root.active ? 1 : 0
    border.color: Theme.borderStrong

    Behavior on color {
        ColorAnimation { duration: 120 }
    }

    Rectangle {
        visible: root.active
        width: 3
        height: 14
        radius: 2
        color: Theme.accent
        anchors.left: parent.left
        anchors.leftMargin: 3
        anchors.verticalCenter: parent.verticalCenter
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.gapSm + 4
        anchors.rightMargin: Theme.gapSm
        spacing: Theme.gapSm

        Label {
            text: root.glyph
            color: root.active ? Theme.accent : Theme.textTertiary
            font.pixelSize: 13
        }

        Label {
            text: root.label
            color: root.active ? Theme.textPrimary : Theme.textSecondary
            font.pixelSize: Theme.fontBody
            font.bold: root.active
            elide: Text.ElideRight
            Layout.fillWidth: true
        }

        Label {
            text: root.hint
            visible: root.hint.length > 0 && hover.hovered
            color: Theme.textTertiary
            font.pixelSize: Theme.fontCaption
        }
    }

    HoverHandler {
        id: hover
        cursorShape: Qt.PointingHandCursor
    }

    TapHandler {
        onTapped: root.clicked()
    }

    Accessible.role: Accessible.Button
    Accessible.name: root.label
    Accessible.checkable: true
    Accessible.checked: root.active
}
