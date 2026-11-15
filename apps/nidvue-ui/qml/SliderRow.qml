// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

RowLayout {
    id: root

    property string label: ""
    property real value: 0
    property real maximum: 200
    property string valueText: ""

    signal moved(real value)

    Layout.fillWidth: true
    spacing: Theme.gapLg

    Label {
        text: root.label
        color: Theme.textSecondary
        font.pixelSize: Theme.fontSmall
        Layout.preferredWidth: 84
    }

    Slider {
        id: slider
        Layout.fillWidth: true
        from: 0
        to: root.maximum
        stepSize: 1
        value: root.value
        onMoved: root.moved(slider.value)

        background: Rectangle {
            x: slider.leftPadding
            y: slider.topPadding + slider.availableHeight / 2 - height / 2
            width: slider.availableWidth
            height: 3
            radius: 2
            color: Theme.surfaceHover

            Rectangle {
                width: slider.visualPosition * parent.width
                height: parent.height
                radius: 2
                color: Theme.accent
            }
        }

        handle: Rectangle {
            x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
            y: slider.topPadding + slider.availableHeight / 2 - height / 2
            width: 13
            height: 13
            radius: 7
            color: Theme.textPrimary
            border.width: 3
            border.color: Theme.accent
        }
    }

    Label {
        text: root.valueText
        color: Theme.textPrimary
        font.pixelSize: Theme.fontSmall
        Layout.preferredWidth: 78
        horizontalAlignment: Text.AlignRight
    }
}
