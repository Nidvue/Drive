// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    property string overline: ""
    property string value: ""
    property string detail: ""
    property color tone: Theme.accent

    implicitHeight: 86

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusMd
        color: Theme.surface
        border.width: 1
        border.color: Theme.border
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.gapSm + 2
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.gapSm

            Rectangle {
                width: 6
                height: 6
                radius: 3
                color: root.tone
            }

            Label {
                text: Theme.overline(root.overline)
                color: Theme.textTertiary
                font.pixelSize: Theme.fontCaption
                font.letterSpacing: 0.9
                font.bold: true
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
        }

        Label {
            text: root.value
            color: Theme.textPrimary
            font.pixelSize: Theme.fontDisplay
            font.bold: true
            elide: Text.ElideRight
            Layout.fillWidth: true
        }

        Item { Layout.fillHeight: true }

        Label {
            text: root.detail
            color: Theme.textSecondary
            font.pixelSize: Theme.fontCaption
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
    }
}
