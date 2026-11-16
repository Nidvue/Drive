// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    readonly property int liveCount: {
        let n = 0
        for (let i = 0; i < Bridge.transfers.length; ++i) {
            const state = Bridge.transfers[i].state
            if (state === "queued" || state === "running")
                ++n
        }
        return n
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            Label {
                text: qsTr("Transfers")
                font.pixelSize: 20
                font.bold: true
            }
            Item { Layout.fillWidth: true }
            Label {
                text: qsTr("%1 live of %2").arg(page.liveCount).arg(Bridge.transfers.length)
                opacity: 0.7
            }
            Button {
                text: qsTr("Clear")
                enabled: Bridge.transfers.length > 0
                onClicked: Bridge.clearTransfers()
                Accessible.name: qsTr("Clear the finished transfers")
            }
        }

        Label {
            text: qsTr("The most recent %1 transfers. Finished rows are dropped oldest-first, so a long sync cannot grow this list without bound.").arg(Bridge.transferLimit)
            wrapMode: Text.WordWrap
            opacity: 0.75
            Layout.fillWidth: true
        }

        Label {
            visible: Bridge.transfers.length === 0
            text: qsTr("Nothing has been transferred since the engine started.")
            opacity: 0.6
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            reuseItems: true
            model: Bridge.transfers
            spacing: 6
            ScrollBar.vertical: ScrollBar {}
            ScrollBar.horizontal: ScrollBar {}

            delegate: Frame {
                required property var modelData
                width: list.width
                padding: 10

                RowLayout {
                    anchors.fill: parent
                    spacing: 12

                    Label {
                        text: modelData.direction === "upload" ? "↑" : "↓"
                        font.pixelSize: 18
                        Layout.alignment: Qt.AlignVCenter
                        Accessible.name: modelData.direction
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Label {
                            text: modelData.path
                            elide: Text.ElideMiddle
                            Layout.fillWidth: true
                        }

                        ProgressBar {
                            Layout.fillWidth: true
                            from: 0
                            to: 1
                            value: modelData.fraction
                            visible: modelData.state === "running" || modelData.state === "queued"
                        }

                        Label {
                            visible: modelData.error !== undefined && modelData.error !== null
                            text: modelData.error === undefined || modelData.error === null ? "" : modelData.error
                            color: "#e05252"
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }

                    Label {
                        text: modelData.state
                        opacity: 0.7
                        Layout.alignment: Qt.AlignVCenter
                    }
                }
            }
        }
    }
}
