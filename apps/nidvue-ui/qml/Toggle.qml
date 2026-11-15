// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick

Rectangle {
    id: root

    property bool checked: false
    property string accessibleName: ""

    signal toggled(bool checked)

    implicitWidth: 38
    implicitHeight: 22
    radius: 11
    color: root.checked ? Theme.accent : Theme.surfaceHover
    border.width: root.checked ? 0 : 1
    border.color: Theme.borderStrong

    Behavior on color {
        ColorAnimation { duration: 140 }
    }

    Rectangle {
        width: 16
        height: 16
        radius: 8
        y: 3
        x: root.checked ? root.width - width - 3 : 3
        color: root.checked ? "#ffffff" : Theme.textSecondary

        Behavior on x {
            NumberAnimation {
                duration: 150
                easing.type: Easing.OutCubic
            }
        }
    }

    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }

    TapHandler {
        onTapped: {
            root.checked = !root.checked
            root.toggled(root.checked)
        }
    }

    Accessible.role: Accessible.CheckBox
    Accessible.name: root.accessibleName
    Accessible.checked: root.checked
}
