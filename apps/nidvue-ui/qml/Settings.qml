// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    ScrollView {
        anchors.fill: parent
        ColumnLayout {
            width: page.width
            spacing: 16

            GroupBox {
                title: qsTr("Account")
                Layout.fillWidth: true
                Layout.margins: 20
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
                Layout.leftMargin: 20
                Layout.rightMargin: 20
                ColumnLayout {
                    anchors.fill: parent
                    Label {
                        text: qsTr("Pairs are edited in config.toml or with `nidvue-cli sync add`; the UI shows their state and runs a pass.")
                        wrapMode: Text.WordWrap
                        opacity: 0.7
                        Layout.fillWidth: true
                    }
                    Button {
                        text: qsTr("Run a sync pass now")
                        enabled: Bridge.loggedIn && !Bridge.busy
                        onClicked: Bridge.syncNow()
                    }
                }
            }

            GroupBox {
                title: qsTr("About")
                Layout.fillWidth: true
                Layout.leftMargin: 20
                Layout.rightMargin: 20
                Layout.bottomMargin: 20
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
        }
    }
}
