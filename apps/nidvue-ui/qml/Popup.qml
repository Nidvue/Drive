// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The compact panel the tray icon opens: status at a glance, the two switches
// people actually use, and a door to the detailed settings.
Window {
    id: popup
    width: 360
    height: 330
    flags: Qt.Tool | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    color: "transparent"
    visible: false
    title: qsTr("Nidvue Drive")

    property bool trayAvailable: true

    Rectangle {
        anchors.fill: parent
        radius: 10
        color: palette.window
        border.color: "#33000000"
        border.width: 1

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 10

            RowLayout {
                Layout.fillWidth: true
                Label {
                    text: qsTr("Nidvue Drive")
                    font.bold: true
                    font.pixelSize: 15
                }
                Item { Layout.fillWidth: true }
                Label {
                    text: Bridge.paused ? qsTr("paused") : Bridge.health
                    color: Bridge.paused ? "#e0a83c"
                         : Bridge.health === "ok" ? "#3dc07a"
                         : Bridge.health === "error" ? "#e05252"
                         : "#9a9a9a"
                }
            }

            Label {
                Layout.fillWidth: true
                text: Bridge.account.length > 0 ? Bridge.account : qsTr("not logged in")
                elide: Text.ElideMiddle
                opacity: 0.8
            }

            GridLayout {
                columns: 2
                Layout.fillWidth: true
                rowSpacing: 4
                columnSpacing: 10
                Label { text: qsTr("Indexed"); opacity: 0.6; font.pixelSize: 12 }
                Label { text: Bridge.indexEntries.toLocaleString(); font.pixelSize: 12 }
                Label { text: qsTr("Cache"); opacity: 0.6; font.pixelSize: 12 }
                Label {
                    text: {
                        const b = Bridge.cacheBytes
                        return b >= 1e9 ? (b / 1e9).toFixed(1) + " GB"
                             : b >= 1e6 ? (b / 1e6).toFixed(1) + " MB"
                             : (b / 1e3).toFixed(1) + " kB"
                    }
                    font.pixelSize: 12
                }
                Label { text: qsTr("Conflicts"); opacity: 0.6; font.pixelSize: 12 }
                Label { text: Bridge.conflictsOpen; font.pixelSize: 12 }
                Label { text: qsTr("Mount"); opacity: 0.6; font.pixelSize: 12 }
                Label { text: Bridge.mounted ? Bridge.mountpoint : qsTr("not mounted"); font.pixelSize: 12; elide: Text.ElideMiddle; Layout.fillWidth: true }
            }

            RowLayout {
                Layout.fillWidth: true
                Switch {
                    text: qsTr("Pause")
                    checked: Bridge.paused
                    onToggled: Bridge.setPaused(checked)
                    Accessible.name: qsTr("Pause sync")
                }
                Item { Layout.fillWidth: true }
                Button {
                    text: qsTr("Sync now")
                    enabled: Bridge.loggedIn && !Bridge.busy
                    onClicked: Bridge.syncNow()
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Label { text: qsTr("Limit"); font.pixelSize: 12; opacity: 0.6 }
                Slider {
                    id: quickBandwidth
                    Layout.fillWidth: true
                    from: 0
                    to: 200
                    value: Bridge.bandwidthMbps
                    onMoved: Bridge.setBandwidthLimitMbps(Math.round(value))
                    Accessible.name: qsTr("Bandwidth limit")
                }
                Label {
                    text: quickBandwidth.value === 0 ? qsTr("∞") : Math.round(quickBandwidth.value) + qsTr(" Mbit")
                    font.pixelSize: 11
                    Layout.preferredWidth: 56
                    horizontalAlignment: Text.AlignRight
                }
            }

            Item { Layout.fillHeight: true }

            RowLayout {
                Layout.fillWidth: true
                Button {
                    text: qsTr("Detailed settings…")
                    Layout.fillWidth: true
                    onClicked: {
                        Tray.openSettingsWindow()
                        popup.hide()
                    }
                }
                Button {
                    text: qsTr("Main window")
                    onClicked: {
                        Bridge.showMainWindow()
                        popup.hide()
                    }
                }
            }

            Label {
                visible: !popup.trayAvailable
                text: qsTr("No system tray on this desktop; use the main window.")
                font.pixelSize: 10
                opacity: 0.6
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }
}
