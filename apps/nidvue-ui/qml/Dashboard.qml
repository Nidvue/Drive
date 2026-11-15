// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

    signal openPage(int index)

    function human(bytes) {
        return Theme.humanBytes(bytes)
    }

    ScrollView {
        id: scroll
        anchors.fill: parent
        contentWidth: availableWidth
        clip: true
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

        ColumnLayout {
            x: Theme.pageGutter
            width: scroll.availableWidth - Theme.pageGutter * 2
            spacing: Theme.gapLg

            Rectangle {
                visible: !Bridge.loggedIn
                Layout.fillWidth: true
                implicitHeight: 52
                radius: Theme.radiusLg
                color: Theme.surface
                border.width: 1
                border.color: Theme.border

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.gapMd
                    spacing: Theme.gapMd

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2

                        Label {
                            text: qsTr("Connect your Jottacloud account")
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontBody
                            font.bold: true
                        }

                        Label {
                            text: qsTr("Generate a Personal Login Token in Jottacloud → Settings → Security, then paste it here.")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontCaption
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                    }

                    PillButton {
                        variant: "primary"
                        text: qsTr("Sign in")
                        onClicked: page.openPage(2)
                        Accessible.name: qsTr("Open settings to sign in")
                    }
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 4
                columnSpacing: Theme.gapLg
                rowSpacing: Theme.gapLg

                StatCard {
                    Layout.fillWidth: true
                    overline: qsTr("Status")
                    value: Theme.healthLabel(Bridge.health, Bridge.paused, Bridge.loggedIn)
                    detail: Bridge.mounted ? qsTr("mounted") : qsTr("not mounted")
                    tone: Theme.healthColor(Bridge.health, Bridge.paused)
                }

                StatCard {
                    Layout.fillWidth: true
                    overline: qsTr("Indexed")
                    value: Bridge.indexEntries.toLocaleString()
                    detail: qsTr("metadata entries")
                    tone: Theme.accentAlt
                }

                StatCard {
                    Layout.fillWidth: true
                    overline: qsTr("Cache")
                    value: Theme.humanBytes(Bridge.cacheBytes)
                    detail: qsTr("%1% of limit").arg(Bridge.cacheCapacity > 0 ? Math.round(Bridge.cacheBytes * 100 / Bridge.cacheCapacity) : 0)
                    tone: Theme.accent
                }

                StatCard {
                    Layout.fillWidth: true
                    overline: qsTr("Conflicts")
                    value: Bridge.conflictsOpen
                    detail: Bridge.conflictsOpen > 0 ? qsTr("awaiting a decision") : qsTr("nothing to resolve")
                    tone: Bridge.conflictsOpen > 0 ? Theme.warn : Theme.ok
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.gapLg

                Card {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 620
                    title: qsTr("Mount")
                    subtitle: qsTr("The archive as a local folder, served on demand from the cache.")

                    content: RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gapSm

                        Field {
                            id: mountPath
                            Layout.fillWidth: true
                            placeholderText: qsTr("/home/you/Jottacloud")
                            text: Bridge.mountpoint
                            Accessible.name: qsTr("Mountpoint path")
                        }

                        Label {
                            text: qsTr("Read-only")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontSmall
                        }

                        Toggle {
                            id: readOnly
                            checked: false
                            accessibleName: qsTr("Mount read-only")
                        }

                        PillButton {
                            variant: Bridge.mounted ? "ghost" : "primary"
                            text: Bridge.mounted ? qsTr("Unmount") : qsTr("Mount")
                            enabled: Bridge.loggedIn
                            onClicked: Bridge.mounted ? Bridge.unmount() : Bridge.mount(mountPath.text, readOnly.checked)
                            Accessible.name: text
                        }
                    }
                }

                Card {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 340
                    title: qsTr("Cache")
                    subtitle: qsTr("%1 of %2").arg(Theme.humanBytes(Bridge.cacheBytes)).arg(Theme.humanBytes(Bridge.cacheCapacity))

                    content: ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gapSm

                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: 6
                            radius: 3
                            color: Theme.surfaceHover

                            Rectangle {
                                width: parent.width * Math.min(1, Bridge.cacheCapacity > 0 ? Bridge.cacheBytes / Bridge.cacheCapacity : 0)
                                height: parent.height
                                radius: 3
                                color: Theme.accent
                            }
                        }

                        Label {
                            text: qsTr("Warm reads never touch the network.")
                            color: Theme.textTertiary
                            font.pixelSize: Theme.fontCaption
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }
                }
            }

            Card {
                Layout.fillWidth: true
                visible: Bridge.transfers.length > 0
                title: qsTr("Transfers")
                subtitle: Bridge.liveTransfers > 0
                          ? qsTr("%1 running · %2 recent").arg(Bridge.liveTransfers).arg(Bridge.transfers.length)
                          : qsTr("%1 recent").arg(Bridge.transfers.length)

                content: ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.gapXs

                    Repeater {
                        model: Bridge.transfers.slice(0, 8)

                        delegate: RowLayout {
                            Layout.fillWidth: true
                            spacing: Theme.gapSm

                            Label {
                                text: modelData.direction === "download" ? "\u2193" : "\u2191"
                                color: modelData.direction === "download" ? Theme.accentAlt : Theme.accent
                                font.pixelSize: Theme.fontBody
                                font.bold: true
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                Label {
                                    Layout.fillWidth: true
                                    text: modelData.path
                                    color: modelData.state === "failed" ? Theme.danger : Theme.textPrimary
                                    font.pixelSize: Theme.fontSmall
                                    elide: Text.ElideMiddle
                                }

                                Rectangle {
                                    Layout.fillWidth: true
                                    implicitHeight: 3
                                    radius: 2
                                    color: Theme.surfaceHover
                                    visible: modelData.state === "active" && modelData.total > 0

                                    Rectangle {
                                        width: parent.width * Math.min(1, modelData.total > 0 ? modelData.done / modelData.total : 0)
                                        height: parent.height
                                        radius: 2
                                        color: Theme.accent
                                    }
                                }
                            }

                            Label {
                                text: modelData.total > 0
                                      ? Theme.humanBytes(modelData.done) + " / " + Theme.humanBytes(modelData.total)
                                      : modelData.size > 0 ? Theme.humanBytes(modelData.size) : ""
                                color: Theme.textTertiary
                                font.pixelSize: Theme.fontCaption
                            }

                            Label {
                                text: modelData.state === "active" ? qsTr("running")
                                    : modelData.state === "failed" ? qsTr("failed")
                                                                   : qsTr("done")
                                color: modelData.state === "active" ? Theme.warn
                                     : modelData.state === "failed" ? Theme.danger
                                                                    : Theme.ok
                                font.pixelSize: Theme.fontCaption
                                Layout.preferredWidth: 52
                                horizontalAlignment: Text.AlignRight
                            }
                        }
                    }

                    PillButton {
                        variant: "quiet"
                        text: qsTr("Clear")
                        onClicked: Bridge.clearTransfers()
                        Accessible.name: qsTr("Clear the transfer list")
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.bottomMargin: Theme.gapLg
                spacing: Theme.gapLg

                Card {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 520
                    title: qsTr("Conflicts needing a decision")
                    subtitle: Bridge.conflicts.length > 0 ? qsTr("%1 open").arg(Bridge.conflicts.length)
                                                         : qsTr("Nothing is ever overwritten silently.")

                    content: ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gapXs

                        Label {
                            visible: Bridge.conflicts.length === 0
                            text: qsTr("No conflicts. Both sides agree.")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontSmall
                        }

                        Repeater {
                            model: Bridge.conflicts.slice(0, 4)

                            delegate: RowLayout {
                                Layout.fillWidth: true
                                spacing: Theme.gapSm

                                Label {
                                    Layout.fillWidth: true
                                    text: modelData.path
                                    color: Theme.textPrimary
                                    font.pixelSize: Theme.fontSmall
                                    elide: Text.ElideMiddle
                                }

                                PillButton {
                                    variant: "ghost"
                                    text: qsTr("Resolve")
                                    onClicked: page.openPage(1)
                                    Accessible.name: qsTr("Open the Conflict Center")
                                }
                            }
                        }

                        PillButton {
                            visible: Bridge.conflicts.length > 0
                            variant: "ghost"
                            text: qsTr("Open Conflict Center")
                            onClicked: page.openPage(1)
                        }
                    }
                }

                Card {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 440
                    title: qsTr("Recent activity")
                    subtitle: Bridge.logs.length === 1 ? qsTr("1 line") : qsTr("%1 lines").arg(Bridge.logs.length)

                    content: ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1

                        Label {
                            visible: Bridge.logs.length === 0
                            text: qsTr("Nothing logged yet.")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontSmall
                        }

                        Repeater {
                            model: Bridge.logs.slice(-8)

                            delegate: Label {
                                Layout.fillWidth: true
                                text: modelData
                                color: Theme.textTertiary
                                font.family: Theme.fontMono
                                font.pixelSize: Theme.fontCaption
                                elide: Text.ElideRight
                            }
                        }

                        PillButton {
                            variant: "quiet"
                            text: qsTr("Open activity log")
                            onClicked: page.openPage(3)
                        }
                    }
                }
            }
        }
    }
}
