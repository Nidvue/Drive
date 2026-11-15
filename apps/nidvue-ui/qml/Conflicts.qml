// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    signal openPage(int index)

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.pageGutter
        anchors.rightMargin: Theme.pageGutter
        anchors.topMargin: Theme.gapLg
        anchors.bottomMargin: Theme.gapLg
        spacing: Theme.gapMd

        Label {
            text: qsTr("Both versions are always kept. The losing side is written as a conflict copy; nothing is overwritten silently.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSmall
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        Card {
            Layout.fillWidth: true
            Layout.fillHeight: true
            title: qsTr("Open conflicts")
            subtitle: qsTr("%1 awaiting a decision").arg(Bridge.conflicts.length)

            content: Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                ColumnLayout {
                    visible: Bridge.conflicts.length === 0
                    anchors.centerIn: parent
                    spacing: Theme.gapSm

                    Label {
                        text: "\u2691"
                        color: Theme.textTertiary
                        font.pixelSize: 28
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Label {
                        text: qsTr("No conflicts — both sides agree.")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontBody
                        Layout.alignment: Qt.AlignHCenter
                    }

                    Label {
                        text: qsTr("When both sides change one file, both versions are kept and listed here.")
                        color: Theme.textTertiary
                        font.pixelSize: Theme.fontCaption
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                        Layout.maximumWidth: 360
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                ListView {
                    id: list
                    anchors.fill: parent
                    visible: Bridge.conflicts.length > 0
                    clip: true
                    reuseItems: true
                    model: Bridge.conflicts
                    spacing: Theme.gapSm
                    ScrollBar.vertical: ScrollBar {}

                    delegate: Rectangle {
                        width: list.width
                        implicitHeight: 82
                        radius: Theme.radiusMd
                        color: Theme.surfaceAlt
                        border.width: 1
                        border.color: Theme.border

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: Theme.gapMd
                            spacing: 2

                            Label {
                                text: modelData.path
                                color: Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                                font.bold: true
                                elide: Text.ElideMiddle
                                Layout.fillWidth: true
                            }

                            Label {
                                text: qsTr("conflict copy: %1").arg(modelData.conflictPath)
                                color: Theme.textTertiary
                                font.pixelSize: Theme.fontCaption
                                elide: Text.ElideMiddle
                                Layout.fillWidth: true
                            }

                            Item { Layout.fillHeight: true }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: Theme.gapSm

                                Label {
                                    text: qsTr("kept: %1 · local %2 · remote %3")
                                        .arg(modelData.kept)
                                        .arg(Theme.humanBytes(modelData.localSize))
                                        .arg(Theme.humanBytes(modelData.remoteSize))
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.fontCaption
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }

                                PillButton {
                                    variant: "ghost"
                                    text: qsTr("Keep local")
                                    onClicked: Bridge.resolve(modelData.id, "keep_local")
                                    Accessible.name: qsTr("Keep the local version")
                                }

                                PillButton {
                                    variant: "ghost"
                                    text: qsTr("Keep remote")
                                    onClicked: Bridge.resolve(modelData.id, "keep_remote")
                                    Accessible.name: qsTr("Keep the remote version")
                                }

                                PillButton {
                                    variant: "primary"
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
    }
}
