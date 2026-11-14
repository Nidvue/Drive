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
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            Label {
                text: qsTr("Conflict Center")
                font.pixelSize: 20
                font.bold: true
            }
            Item { Layout.fillWidth: true }
            Label {
                text: qsTr("%1 open").arg(Bridge.conflicts.length)
                opacity: 0.7
            }
        }

        Label {
            text: qsTr("Both versions are always kept. The losing side is written as a conflict copy; nothing is overwritten silently.")
            wrapMode: Text.WordWrap
            opacity: 0.75
            Layout.fillWidth: true
        }

        Label {
            visible: Bridge.conflicts.length === 0
            text: qsTr("No conflicts.")
            opacity: 0.6
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            reuseItems: true
            model: Bridge.conflicts
            spacing: 8
            ScrollBar.vertical: ScrollBar {}

            delegate: Rectangle {
                width: list.width
                implicitHeight: 92
                radius: 8
                color: palette.base

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 4
                    Label {
                        text: modelData.path
                        font.bold: true
                        elide: Text.ElideMiddle
                        Layout.fillWidth: true
                    }
                    Label {
                        text: qsTr("conflict copy: %1").arg(modelData.conflictPath)
                        opacity: 0.7
                        font.pixelSize: 11
                        elide: Text.ElideMiddle
                        Layout.fillWidth: true
                    }
                    RowLayout {
                        Label {
                            text: qsTr("kept: %1 · local %2 B · remote %3 B")
                                .arg(modelData.kept).arg(modelData.localSize).arg(modelData.remoteSize)
                            opacity: 0.6
                            font.pixelSize: 11
                            Layout.fillWidth: true
                        }
                        Button {
                            text: qsTr("Keep local")
                            onClicked: Bridge.resolve(modelData.id, "keep_local")
                            Accessible.name: qsTr("Keep the local version")
                        }
                        Button {
                            text: qsTr("Keep remote")
                            onClicked: Bridge.resolve(modelData.id, "keep_remote")
                            Accessible.name: qsTr("Keep the remote version")
                        }
                        Button {
                            text: qsTr("Keep both")
                            onClicked: Bridge.resolve(modelData.id, "keep_both")
                            Accessible.name: qsTr("Keep both versions")
                        }
                    }
                }
            }
        }
    }
}
