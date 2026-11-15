// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls

Button {
    id: control

    property string variant: "ghost"

    implicitHeight: Theme.controlHeight
    implicitWidth: Math.max(72, contentItem.implicitWidth + leftPadding + rightPadding)
    leftPadding: 14
    rightPadding: 14
    topPadding: 0
    bottomPadding: 0
    hoverEnabled: true

    contentItem: Text {
        text: control.text
        font.pixelSize: Theme.fontSmall
        font.bold: control.variant === "primary"
        color: {
            if (!control.enabled)
                return Theme.textTertiary
            if (control.variant === "primary")
                return Theme.ink
            if (control.variant === "danger")
                return Theme.danger
            return Theme.textPrimary
        }
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        radius: Theme.radiusSm
        border.width: control.variant === "primary" ? 0 : 1
        border.color: control.variant === "danger" ? Theme.dangerBorder : Theme.borderStrong
        opacity: control.enabled ? 1.0 : 0.45

        color: {
            if (control.variant === "primary") {
                if (control.down)
                    return Theme.accentDeep
                return control.hovered ? Theme.accentBright : Theme.accent
            }
            if (control.variant === "quiet")
                return "transparent"
            if (control.variant === "danger")
                return control.hovered ? Theme.dangerSoft : "transparent"
            if (control.down)
                return Theme.surfaceHover
            return control.hovered ? Theme.surfaceAlt : "transparent"
        }

        Behavior on color {
            ColorAnimation { duration: 110 }
        }
    }
}
