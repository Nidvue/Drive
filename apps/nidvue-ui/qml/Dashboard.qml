// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    function human(bytes) {
        if (bytes >= 1e9) return (bytes / 1e9).toFixed(1) + " GB"
        if (bytes >= 1e6) return (bytes / 1e6).toFixed(1) + " MB"
        if (bytes >= 1e3) return (bytes / 1e3).toFixed(1) + " kB"
        return bytes + " B"
    }

    ScrollView {
        anchors.fill: parent
        contentWidth: availableWidth
        ColumnLayout {
            width: page.width
            spacing: 16
            anchors.margins: 20

            GridLayout {
                columns: 4
                columnSpacing: 16
                rowSpacing: 16
                Layout.fillWidth: true
                Layout.margins: 20

                Repeater {
                    model: [
                        { title: qsTr("Health"), value: Bridge.health, detail: Bridge.loggedIn ? qsTr("account connected") : qsTr("not logged in") },
                        { title: qsTr("Indexed"), value: Bridge.indexEntries.toLocaleString(), detail: qsTr("metadata entries") },
                        { title: qsTr("Cache"), value: human(Bridge.cacheBytes), detail: qsTr("served without the network") },
                        { title: qsTr("Conflicts"), value: Bridge.conflictsOpen, detail: qsTr("awaiting a decision") }
                    ]
                    delegate: Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 92
                        radius: 8
                        color: palette.base
                        border.color: "#33ffffff"
                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 12
                            Label { text: modelData.title; opacity: 0.6; font.pixelSize: 12 }
                            Label { text: modelData.value; font.pixelSize: 22; font.bold: true }
                            Label { text: modelData.detail; opacity: 0.6; font.pixelSize: 11; elide: Text.ElideRight; Layout.fillWidth: true }
                        }
                    }
                }
            }

            GroupBox {
                title: qsTr("Mount")
                Layout.fillWidth: true
                Layout.leftMargin: 20
                Layout.rightMargin: 20
                RowLayout {
                    anchors.fill: parent
                    TextField {
                        id: mountPath
                        Layout.fillWidth: true
                        placeholderText: qsTr("/home/you/Jottacloud")
                        text: Bridge.mountpoint
                        Accessible.name: qsTr("Mountpoint path")
                    }
                    CheckBox {
                        id: readOnly
                        text: qsTr("read-only")
                        Accessible.name: qsTr("Mount read-only")
                    }
                    Button {
                        text: Bridge.mounted ? qsTr("Unmount") : qsTr("Mount")
                        enabled: Bridge.loggedIn
                        onClicked: Bridge.mounted ? Bridge.unmount() : Bridge.mount(mountPath.text, readOnly.checked)
                        Accessible.name: text
                    }
                }
            }

            GroupBox {
                title: qsTr("Conflicts needing a decision")
                Layout.fillWidth: true
                Layout.leftMargin: 20
                Layout.rightMargin: 20
                Layout.bottomMargin: 20
                ColumnLayout {
                    anchors.fill: parent
                    Label {
                        visible: Bridge.conflicts.length === 0
                        text: qsTr("No conflicts. Both sides agree.")
                        opacity: 0.7
                    }
                    Repeater {
                        model: Bridge.conflicts.slice(0, 5)
                        delegate: RowLayout {
                            Layout.fillWidth: true
                            Label {
                                Layout.fillWidth: true
                                text: modelData.path
                                elide: Text.ElideMiddle
                            }
                            Button {
                                text: qsTr("Resolve")
                                onClicked: {
                                    stack.currentIndex = 1
                                }
                                Accessible.name: qsTr("Open the Conflict Center")
                            }
                        }
                    }
                    Button {
                        visible: Bridge.conflicts.length > 0
                        text: qsTr("Open Conflict Center")
                        onClicked: stack.currentIndex = 1
                    }
                }
            }
        }
    }
}
