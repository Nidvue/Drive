// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    property string label: ""
    property string description: ""
    property bool checked: false

    signal toggled(bool checked)

    Layout.fillWidth: true
    implicitHeight: row.implicitHeight

    RowLayout {
        id: row
        anchors.fill: parent
        spacing: Theme.gapMd

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1

            Label {
                text: root.label
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
            }

            Label {
                visible: root.description.length > 0
                text: root.description
                color: Theme.textTertiary
                font.pixelSize: Theme.fontCaption
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }

        Toggle {
            checked: root.checked
            accessibleName: root.label
            onToggled: function (value) {
                root.checked = value
                root.toggled(value)
            }
        }
    }
}
