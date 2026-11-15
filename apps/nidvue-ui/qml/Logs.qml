// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.pageGutter
        anchors.rightMargin: Theme.pageGutter
        anchors.topMargin: Theme.gapLg
        anchors.bottomMargin: Theme.gapLg
        spacing: Theme.gapMd

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.gapSm

            Item { Layout.fillWidth: true }

            PillButton {
                variant: "ghost"
                text: qsTr("Copy")
                enabled: Bridge.logs.length > 0
                onClicked: Bridge.copyLogs()
                Accessible.name: qsTr("Copy the log to the clipboard")
            }

            PillButton {
                variant: "ghost"
                text: qsTr("Clear")
                enabled: Bridge.logs.length > 0
                onClicked: Bridge.clearLogs()
                Accessible.name: qsTr("Clear the log")
            }
        }

        Card {
            Layout.fillWidth: true
            Layout.fillHeight: true
            title: qsTr("Log output")
            subtitle: Bridge.logs.length === 1 ? qsTr("1 line · newest last") : qsTr("%1 lines · newest last").arg(Bridge.logs.length)

            content: Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                Label {
                    visible: Bridge.logs.length === 0
                    anchors.centerIn: parent
                    text: qsTr("Nothing logged yet — sync passes, mounts and errors appear here.")
                    color: Theme.textTertiary
                    font.pixelSize: Theme.fontSmall
                }

                ListView {
                    id: list
                    anchors.fill: parent
                    visible: Bridge.logs.length > 0
                    clip: true
                    reuseItems: true
                    model: Bridge.logs
                    spacing: 1
                    ScrollBar.vertical: ScrollBar {}

                    delegate: Rectangle {
                        width: list.width
                        implicitHeight: 18
                        radius: 3
                        color: index % 2 === 0 ? "transparent" : Theme.surfaceAlt

                        Label {
                            anchors.fill: parent
                            anchors.leftMargin: 8
                            anchors.rightMargin: 8
                            text: modelData
                            color: Theme.textSecondary
                            font.family: Theme.fontMono
                            font.pixelSize: Theme.fontCaption
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                    }
                }
            }
        }
    }
}
