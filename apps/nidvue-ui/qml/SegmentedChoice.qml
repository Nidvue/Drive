// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    property var options: []
    property string current: ""

    signal selected(string value)

    implicitWidth: row.implicitWidth + 6
    implicitHeight: 32
    radius: Theme.radiusMd
    color: Theme.surfaceAlt
    border.width: 1
    border.color: Theme.border

    RowLayout {
        id: row
        anchors.fill: parent
        anchors.margins: 3
        spacing: 3

        Repeater {
            model: root.options

            delegate: Rectangle {
                implicitHeight: 24
                implicitWidth: Math.max(64, text.implicitWidth + 20)
                radius: Theme.radiusSm
                color: root.current === modelData.value ? Theme.accent : "transparent"

                Behavior on color {
                    ColorAnimation { duration: 120 }
                }

                Text {
                    id: text
                    anchors.centerIn: parent
                    text: modelData.label
                    color: root.current === modelData.value ? Theme.ink : Theme.textSecondary
                    font.pixelSize: Theme.fontSmall
                    font.bold: root.current === modelData.value
                }

                HoverHandler {
                    cursorShape: Qt.PointingHandCursor
                }

                TapHandler {
                    onTapped: root.selected(modelData.value)
                }

                Accessible.role: Accessible.RadioButton
                Accessible.name: modelData.label
                Accessible.checked: root.current === modelData.value
            }
        }
    }
}
