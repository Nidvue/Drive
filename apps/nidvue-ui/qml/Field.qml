// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls

TextField {
    id: control

    implicitHeight: Theme.controlHeight
    leftPadding: 10
    rightPadding: 10
    color: Theme.textPrimary
    placeholderTextColor: Theme.textTertiary
    selectionColor: Theme.accent
    selectedTextColor: Theme.ink
    font.pixelSize: Theme.fontSmall

    background: Rectangle {
        radius: Theme.radiusSm
        color: Theme.surfaceAlt
        border.width: 1
        border.color: control.activeFocus ? Theme.accent : Theme.borderStrong

        Behavior on border.color {
            ColorAnimation { duration: 110 }
        }
    }
}
