// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    /// The log filtered by the search box; the whole list when the box is empty.
    readonly property var lines: {
        var needle = filter.text.toLowerCase()
        if (needle.length === 0)
            return Bridge.logs
        var out = []
        for (var i = 0; i < Bridge.logs.length; ++i) {
            var line = Bridge.logs[i]
            if (line.toLowerCase().indexOf(needle) >= 0)
                out.push(line)
        }
        return out
    }

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

            Field {
                id: filter
                Layout.preferredWidth: 280
                placeholderText: qsTr("Filter lines…")
                Accessible.name: qsTr("Filter the log")
            }

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
            subtitle: page.lines.length === 1
                      ? qsTr("1 line · newest last")
                      : qsTr("%1 lines · newest last").arg(page.lines.length)

            content: Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                Label {
                    visible: page.lines.length === 0
                    anchors.centerIn: parent
                    text: Bridge.logs.length === 0
                          ? qsTr("Nothing logged yet — sync passes, mounts and errors appear here.")
                          : qsTr("No line matches the filter.")
                    color: Theme.textTertiary
                    font.pixelSize: Theme.fontSmall
                }

                ListView {
                    id: list
                    anchors.fill: parent
                    visible: page.lines.length > 0
                    clip: true
                    reuseItems: true
                    model: page.lines
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
