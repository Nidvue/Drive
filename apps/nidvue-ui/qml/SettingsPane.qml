// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The settings content, shared by the main window's Settings page and the
// standalone detailed-settings window the tray opens.
ColumnLayout {
    id: pane
    spacing: 16

    GroupBox {
        title: qsTr("Account")
        Layout.fillWidth: true
        ColumnLayout {
            anchors.fill: parent
            Label {
                text: Bridge.loggedIn
                      ? qsTr("Signed in as %1").arg(Bridge.account)
                      : qsTr("Paste a Personal Login Token from Jottacloud → Settings → Security.")
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
            RowLayout {
                Layout.fillWidth: true
                TextField {
                    id: token
                    Layout.fillWidth: true
                    echoMode: TextInput.Password
                    placeholderText: qsTr("Personal Login Token")
                    visible: !Bridge.loggedIn
                    Accessible.name: qsTr("Personal Login Token")
                }
                Button {
                    text: qsTr("Sign in")
                    visible: !Bridge.loggedIn
                    enabled: token.text.length > 0
                    onClicked: {
                        Bridge.login(token.text)
                        token.text = ""
                    }
                }
                Button {
                    text: qsTr("Sign out")
                    visible: Bridge.loggedIn
                    onClicked: Bridge.logout()
                }
            }
            Label {
                text: qsTr("The token is stored encrypted at rest, never in the configuration and never in a log line.")
                opacity: 0.6
                font.pixelSize: 11
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }

    GroupBox {
        title: qsTr("Sync")
        Layout.fillWidth: true
        ColumnLayout {
            anchors.fill: parent
            Switch {
                text: qsTr("Pause sync")
                checked: Bridge.paused
                onToggled: Bridge.setPaused(checked)
                Accessible.name: qsTr("Pause sync")
            }
            RowLayout {
                Layout.fillWidth: true
                Label { text: qsTr("Bandwidth limit") }
                Slider {
                    id: bandwidth
                    Layout.fillWidth: true
                    from: 0
                    to: 200
                    stepSize: 1
                    value: Bridge.bandwidthMbps
                    onMoved: Bridge.setBandwidthLimitMbps(Math.round(value))
                    Accessible.name: qsTr("Bandwidth limit in megabits per second")
                }
                Label {
                    text: bandwidth.value === 0 ? qsTr("unlimited")
                                                : qsTr("%1 Mbit/s").arg(Math.round(bandwidth.value))
                    Layout.preferredWidth: 92
                    horizontalAlignment: Text.AlignRight
                }
            }
            Button {
                text: qsTr("Run a sync pass now")
                enabled: Bridge.loggedIn && !Bridge.busy
                onClicked: Bridge.syncNow()
            }
            Label {
                text: qsTr("Pairs are edited in config.toml or with `nidvue-cli sync add`.")
                opacity: 0.7
                wrapMode: Text.WordWrap
                font.pixelSize: 11
                Layout.fillWidth: true
            }
        }
    }

    GroupBox {
        title: qsTr("Mount")
        Layout.fillWidth: true
        RowLayout {
            anchors.fill: parent
            TextField {
                id: mountPath
                Layout.fillWidth: true
                placeholderText: qsTr("/home/you/Jottacloud")
                text: Bridge.mountpoint
                Accessible.name: qsTr("Mountpoint path")
            }
            Button {
                text: Bridge.mounted ? qsTr("Unmount") : qsTr("Mount")
                enabled: Bridge.loggedIn
                onClicked: Bridge.mounted ? Bridge.unmount() : Bridge.mount(mountPath.text, false)
            }
        }
    }

    GroupBox {
        title: qsTr("About")
        Layout.fillWidth: true
        ColumnLayout {
            anchors.fill: parent
            Label { text: qsTr("Nidvue Drive %1").arg(Bridge.version) }
            Label {
                text: qsTr("An unofficial Jottacloud client. Jottacloud is a trademark of its owner. Licensed AGPL-3.0-only.")
                wrapMode: Text.WordWrap
                opacity: 0.7
                Layout.fillWidth: true
            }
        }
    }

    Item { Layout.fillHeight: true }
}
