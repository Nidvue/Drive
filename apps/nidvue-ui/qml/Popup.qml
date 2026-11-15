// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Window {
    id: popup

    width: 330
    height: 276
    flags: Qt.Tool | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    color: "transparent"
    visible: false
    title: qsTr("Nidvue Drive")

    property bool trayAvailable: true

    palette.window: Theme.bg
    palette.windowText: Theme.textPrimary
    palette.base: Theme.surfaceAlt
    palette.text: Theme.textPrimary
    palette.button: Theme.surfaceAlt
    palette.buttonText: Theme.textPrimary
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.ink
    palette.placeholderText: Theme.textTertiary

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusXl
        color: Theme.bg
        border.width: 1
        border.color: Theme.borderStrong

        Rectangle {
            id: band
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 70
            radius: Theme.radiusXl

            gradient: Gradient {
                GradientStop {
                    position: 0.0
                    color: Theme.headerFrom
                }
                GradientStop {
                    position: 1.0
                    color: Theme.headerTo
                }
            }
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Theme.gapLg
            spacing: Theme.gapSm

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.gapMd

                Rectangle {
                    width: 26
                    height: 26
                    radius: Theme.radiusSm
                    color: Theme.accent

                    Text {
                        anchors.centerIn: parent
                        text: "⇅"
                        color: Theme.ink
                        font.pixelSize: 14
                        font.bold: true
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 1

                    Label {
                        text: qsTr("Nidvue Drive")
                        color: Theme.textPrimary
                        font.pixelSize: Theme.fontBody
                        font.bold: true
                    }

                    Label {
                        text: Bridge.account.length > 0 ? Bridge.account : qsTr("not signed in")
                        color: Theme.textTertiary
                        font.pixelSize: Theme.fontCaption
                        elide: Text.ElideMiddle
                        Layout.fillWidth: true
                    }
                }

                Rectangle {
                    width: 8
                    height: 8
                    radius: 4
                    color: Theme.healthColor(Bridge.health, Bridge.paused)
                }
            }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 1
                color: Theme.border
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.gapSm

                Label {
                    text: Theme.healthLabel(Bridge.health, Bridge.paused, Bridge.loggedIn)
                    color: Theme.healthColor(Bridge.health, Bridge.paused)
                    font.pixelSize: Theme.fontSmall
                    font.bold: true
                    Layout.fillWidth: true
                }

                BusyIndicator {
                    running: Bridge.busy
                    visible: Bridge.busy
                    implicitWidth: 16
                    implicitHeight: 16
                }
            }

            GridLayout {
                columns: 2
                Layout.fillWidth: true
                rowSpacing: Theme.gapXs
                columnSpacing: Theme.gapMd

                Label {
                    text: qsTr("Indexed")
                    color: Theme.textTertiary
                    font.pixelSize: Theme.fontCaption
                }

                Label {
                    text: Bridge.indexEntries.toLocaleString()
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontSmall
                    horizontalAlignment: Text.AlignRight
                    Layout.fillWidth: true
                }

                Label {
                    text: qsTr("Cache")
                    color: Theme.textTertiary
                    font.pixelSize: Theme.fontCaption
                }

                Label {
                    text: Theme.humanBytes(Bridge.cacheBytes)
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontSmall
                    horizontalAlignment: Text.AlignRight
                    Layout.fillWidth: true
                }

                Label {
                    text: qsTr("Conflicts")
                    color: Theme.textTertiary
                    font.pixelSize: Theme.fontCaption
                }

                Label {
                    text: Bridge.conflictsOpen
                    color: Bridge.conflictsOpen > 0 ? Theme.warn : Theme.textPrimary
                    font.pixelSize: Theme.fontSmall
                    horizontalAlignment: Text.AlignRight
                    Layout.fillWidth: true
                }

                Label {
                    text: qsTr("Mount")
                    color: Theme.textTertiary
                    font.pixelSize: Theme.fontCaption
                }

                Label {
                    text: Bridge.mounted ? Bridge.mountpoint : qsTr("not mounted")
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontSmall
                    elide: Text.ElideMiddle
                    horizontalAlignment: Text.AlignRight
                    Layout.fillWidth: true
                }
            }

            ToggleRow {
                Layout.fillWidth: true
                label: qsTr("Pause sync")
                checked: Bridge.paused
                onToggled: function (checked) {
                    Bridge.setPaused(checked)
                }
            }

            SliderRow {
                Layout.fillWidth: true
                label: qsTr("Limit")
                maximum: 200
                value: Bridge.bandwidthMbps
                valueText: value === 0 ? qsTr("∞") : qsTr("%1 Mbit").arg(Math.round(value))
                onMoved: function (value) {
                    Bridge.setBandwidthLimitMbps(Math.round(value))
                }
            }

            Item { Layout.fillHeight: true }

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.gapSm

                PillButton {
                    variant: "primary"
                    text: qsTr("Sync now")
                    Layout.fillWidth: true
                    enabled: Bridge.loggedIn && !Bridge.busy
                    onClicked: Bridge.syncNow()
                }

                PillButton {
                    variant: "ghost"
                    text: qsTr("Settings…")
                    onClicked: {
                        Tray.openSettingsWindow()
                        popup.hide()
                    }
                }

                PillButton {
                    variant: "quiet"
                    text: qsTr("Open")
                    onClicked: {
                        Bridge.showMainWindow()
                        popup.hide()
                    }
                }
            }

            Label {
                visible: !popup.trayAvailable
                text: qsTr("No system tray on this desktop; use the main window.")
                color: Theme.textTertiary
                font.pixelSize: Theme.fontCaption
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }
}
