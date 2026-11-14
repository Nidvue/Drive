// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 8

        RowLayout {
            Layout.fillWidth: true
            Label {
                text: qsTr("Activity")
                font.pixelSize: 20
                font.bold: true
            }
            Item { Layout.fillWidth: true }
            Label {
                text: qsTr("%1 lines").arg(Bridge.logs.length)
                opacity: 0.6
            }
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            reuseItems: true
            model: Bridge.logs
            ScrollBar.vertical: ScrollBar {}
            delegate: Label {
                width: list.width
                text: modelData
                font.family: "monospace"
                font.pixelSize: 11
                elide: Text.ElideRight
            }
        }
    }
}
