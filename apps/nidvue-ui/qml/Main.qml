// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: window
    width: 1080
    height: 720
    visible: true
    title: qsTr("Nidvue Drive")
    color: palette.window

    // Colours follow the platform palette, so the system light/dark theme is
    // honoured without depending on a Qt version-specific API.
    palette.highlight: "#3d7eff"
    palette.highlightedText: "#ffffff"

    header: ToolBar {
        RowLayout {
            anchors.fill: parent
            Label {
                text: qsTr("Nidvue Drive")
                font.bold: true
                Layout.leftMargin: 12
            }
            Label {
                text: Bridge.account.length > 0 ? Bridge.account : qsTr("not logged in")
                opacity: 0.7
                Layout.leftMargin: 8
            }
            Item { Layout.fillWidth: true }
            BusyIndicator {
                running: Bridge.busy
                visible: Bridge.busy
                implicitWidth: 24
                implicitHeight: 24
            }
            Label {
                text: Bridge.busy ? Bridge.busyWhat : ""
                opacity: 0.7
                visible: Bridge.busy
            }
            Button {
                text: qsTr("Sync now")
                enabled: Bridge.loggedIn && !Bridge.busy
                onClicked: Bridge.syncNow()
                Accessible.name: qsTr("Run a sync pass now")
            }
            ToolButton {
                text: qsTr("Logs")
                onClicked: stack.currentIndex = 3
                Accessible.name: qsTr("Open the log view")
            }
        }
    }

    footer: ToolBar {
        RowLayout {
            anchors.fill: parent
            Label {
                text: Bridge.lastError.length > 0 ? Bridge.lastError : qsTr("healthy")
                color: Bridge.lastError.length > 0 ? "#e05252" : palette.windowText
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
            ToolButton {
                text: qsTr("Dismiss")
                visible: Bridge.lastError.length > 0
                onClicked: Bridge.clearError()
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Sidebar navigation.
        Rectangle {
            Layout.preferredWidth: 190
            Layout.fillHeight: true
            color: palette.base
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 4
                Repeater {
                    model: [
                        { label: qsTr("Dashboard"), icon: "●" },
                        { label: qsTr("Conflicts"), icon: "⚠" },
                        { label: qsTr("Settings"), icon: "⚙" },
                        { label: qsTr("Logs"), icon: "≡" }
                    ]
                    delegate: Button {
                        Layout.fillWidth: true
                        text: modelData.icon + "  " + modelData.label
                        flat: stack.currentIndex !== index
                        highlighted: stack.currentIndex === index
                        onClicked: stack.currentIndex = index
                        Accessible.name: modelData.label
                    }
                }
                Item { Layout.fillHeight: true }
                Label {
                    text: qsTr("AGPL-3.0-only\nUnofficial Jottacloud client")
                    font.pixelSize: 10
                    opacity: 0.6
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }

        StackLayout {
            id: stack
            currentIndex: 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            Dashboard {}
            Conflicts {}
            Settings {}
            Logs {}
        }
    }

    Shortcut { sequence: "Ctrl+1"; onActivated: stack.currentIndex = 0 }
    Shortcut { sequence: "Ctrl+2"; onActivated: stack.currentIndex = 1 }
    Shortcut { sequence: "Ctrl+3"; onActivated: stack.currentIndex = 2 }
    Shortcut { sequence: "Ctrl+4"; onActivated: stack.currentIndex = 3 }
    Shortcut { sequence: "Ctrl+R"; onActivated: Bridge.syncNow() }
}
