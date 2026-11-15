// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    property string title: ""
    property string subtitle: ""
    property alias content: body.data

    implicitHeight: layout.implicitHeight + Theme.gapMd * 2 + 8

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusLg
        color: Theme.surface
        border.width: 1
        border.color: Theme.border
    }

    ColumnLayout {
        id: layout
        x: Theme.gapMd
        y: Theme.gapMd
        width: parent.width - Theme.gapMd * 2
        spacing: Theme.gapSm

        ColumnLayout {
            visible: root.title.length > 0
            Layout.fillWidth: true
            spacing: 1

            Label {
                text: root.title
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.bold: true
            }

            Label {
                visible: root.subtitle.length > 0
                text: root.subtitle
                color: Theme.textTertiary
                font.pixelSize: Theme.fontCaption
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
                lineHeight: 1.25
            }
        }

        ColumnLayout {
            id: body
            Layout.fillWidth: true
            spacing: Theme.gapMd
        }
    }
}
