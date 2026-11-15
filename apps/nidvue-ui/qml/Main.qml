// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: window

    width: 1040
    height: 560
    minimumWidth: 860
    minimumHeight: 440
    visible: true
    title: qsTr("Nidvue Drive")
    color: Theme.bg

    palette.window: Theme.bg
    palette.windowText: Theme.textPrimary
    palette.base: Theme.surfaceAlt
    palette.alternateBase: Theme.surface
    palette.text: Theme.textPrimary
    palette.button: Theme.surfaceAlt
    palette.buttonText: Theme.textPrimary
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.ink
    palette.placeholderText: Theme.textTertiary
    palette.toolTipBase: Theme.surfaceHover
    palette.toolTipText: Theme.textPrimary

    readonly property var pages: [
        { label: qsTr("Overview"), glyph: "◈" },
        { label: qsTr("Conflicts"), glyph: "⚑" },
        { label: qsTr("Transfers"), glyph: "⇅" },
        { label: qsTr("Settings"), glyph: "⚙" },
        { label: qsTr("Activity"), glyph: "≡" }
    ]

    onClosing: function (close) {
        if (Tray.available) {
            close.accepted = false
            window.hide()
        } else {
            Qt.quit()
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.preferredWidth: Theme.sidebarWidth
            Layout.fillHeight: true
            color: Theme.surface

            Rectangle {
                anchors.right: parent.right
                width: 1
                height: parent.height
                color: Theme.border
            }

            ColumnLayout {
                anchors.fill: parent
                anchors.topMargin: Theme.gapMd
                anchors.bottomMargin: Theme.gapSm
                anchors.leftMargin: Theme.gapSm
                anchors.rightMargin: Theme.gapXs
                spacing: 1

                RowLayout {
                    Layout.fillWidth: true
                    Layout.bottomMargin: Theme.gapMd
                    spacing: Theme.gapSm

                    Rectangle {
                        width: 24
                        height: 24
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
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }

                        Label {
                            text: qsTr("Jottacloud sync")
                            color: Theme.textTertiary
                            font.pixelSize: Theme.fontCaption
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }
                }

                Repeater {
                    model: window.pages

                    delegate: NavItem {
                        Layout.fillWidth: true
                        glyph: modelData.glyph
                        label: modelData.label
                        hint: "Ctrl+" + (index + 1)
                        active: stack.currentIndex === index
                        onClicked: stack.currentIndex = index
                    }
                }

                Item { Layout.fillHeight: true }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.topMargin: Theme.gapSm
                    Layout.bottomMargin: Theme.gapSm
                    implicitHeight: 1
                    color: Theme.border
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.gapSm

                    Rectangle {
                        width: 8
                        height: 8
                        radius: 4
                        color: Theme.healthColor(Bridge.health, Bridge.paused)
                    }

                    Label {
                        text: Theme.healthLabel(Bridge.health, Bridge.paused, Bridge.loggedIn)
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontSmall
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }

                Label {
                    Layout.fillWidth: true
                    text: qsTr("Unofficial client · AGPL-3.0-only")
                    color: Theme.textTertiary
                    font.pixelSize: Theme.fontCaption
                    wrapMode: Text.WordWrap
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: Theme.headerHeight
                color: Theme.bg

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 1
                    color: Theme.border
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.pageGutter
                    anchors.rightMargin: Theme.pageGutter
                    spacing: Theme.gapMd

                    ColumnLayout {
                        spacing: 2

                        Label {
                            text: window.pages[stack.currentIndex].label
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontBody + 2
                            font.bold: true
                        }

                        Label {
                            text: Bridge.account.length > 0 ? Bridge.account : qsTr("not signed in")
                            color: Theme.textTertiary
                            font.pixelSize: Theme.fontCaption
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }

                    Item { Layout.fillWidth: true }

                    RowLayout {
                        spacing: Theme.gapSm
                        visible: Bridge.busy

                        BusyIndicator {
                            running: Bridge.busy
                            visible: Bridge.busy
                            implicitWidth: 18
                            implicitHeight: 18
                        }

                        Label {
                            text: Bridge.busyWhat
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontSmall
                        }
                    }

                    PillButton {
                        variant: "primary"
                        text: qsTr("Sync now")
                        enabled: Bridge.loggedIn && !Bridge.busy
                        onClicked: Bridge.syncNow()
                        Accessible.name: qsTr("Run a sync pass now")
                    }

                    PillButton {
                        variant: "ghost"
                        text: qsTr("Activity")
                        onClicked: stack.currentIndex = 3
                        Accessible.name: qsTr("Open the activity log")
                    }
                }
            }

            StackLayout {
                id: stack
                currentIndex: 0
                Layout.fillWidth: true
                Layout.fillHeight: true

                Dashboard {
                    onOpenPage: function (index) {
                        stack.currentIndex = index
                    }
                }

                Conflicts {
                    onOpenPage: function (index) {
                        stack.currentIndex = index
                    }
                }

                Transfers {}

                Settings {}

                Logs {}
            }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 22
                color: Theme.surface

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    height: 1
                    color: Theme.border
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.pageGutter
                    anchors.rightMargin: Theme.pageGutter
                    spacing: Theme.gapMd

                    Label {
                        text: Bridge.lastError.length > 0
                              ? Bridge.lastError
                              : Bridge.lastSyncAt.length > 0
                                ? qsTr("Last pass %1 · %2").arg(Bridge.lastSyncAt).arg(Bridge.syncTotals())
                                : qsTr("No sync pass yet")
                        color: Bridge.lastError.length > 0 ? Theme.danger : Theme.textTertiary
                        font.pixelSize: Theme.fontCaption
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Label {
                        text: qsTr("Dismiss")
                        visible: Bridge.lastError.length > 0
                        color: Theme.accent
                        font.pixelSize: Theme.fontCaption

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: Bridge.clearError()
                        }
                    }

                    Label {
                        text: qsTr("v%1").arg(Bridge.version)
                        color: Theme.textTertiary
                        font.pixelSize: Theme.fontCaption
                    }
                }
            }
        }
    }

    Shortcut {
        sequence: "Ctrl+1"
        onActivated: stack.currentIndex = 0
    }
    Shortcut {
        sequence: "Ctrl+2"
        onActivated: stack.currentIndex = 1
    }
    Shortcut {
        sequence: "Ctrl+3"
        onActivated: stack.currentIndex = 2
    }
    Shortcut {
        sequence: "Ctrl+4"
        onActivated: stack.currentIndex = 3
    }
    Shortcut {
        sequence: "Ctrl+R"
        onActivated: Bridge.syncNow()
    }
}
