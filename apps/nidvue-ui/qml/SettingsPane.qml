// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Nidvue.Preferences 1.0

ColumnLayout {
    id: pane

    spacing: Theme.gapLg

    /// The settings rows the Advanced card renders, filtered by the search box.
    readonly property var configRows: {
        var needle = configFilter.text.toLowerCase()
        if (needle.length === 0)
            return Bridge.configRows
        var out = []
        for (var i = 0; i < Bridge.configRows.length; ++i) {
            if (Bridge.configRows[i].key.toLowerCase().indexOf(needle) >= 0)
                out.push(Bridge.configRows[i])
        }
        return out
    }

    Component.onCompleted: {
        Bridge.refreshPlugins()
        Bridge.refreshConfig()
    }

    Card {
        Layout.fillWidth: true
        title: qsTr("Appearance")
        subtitle: qsTr("Follows the desktop colour scheme unless you choose one.")

        content: ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.gapMd

            SegmentedChoice {
                options: [
                    {
                        value: "system",
                        label: qsTr("Match desktop")
                    },
                    {
                        value: "light",
                        label: qsTr("Light")
                    },
                    {
                        value: "dark",
                        label: qsTr("Dark")
                    }
                ]
                current: Theme.mode
                onSelected: function (value) {
                    Preferences.themeMode = value
                }
            }

            Label {
                text: Theme.mode === "system"
                      ? qsTr("Desktop reports a %1 scheme.").arg(Theme.systemDark ? qsTr("dark") : qsTr("light"))
                      : qsTr("Always %1.").arg(Theme.dark ? qsTr("dark") : qsTr("light"))
                color: Theme.textTertiary
                font.pixelSize: Theme.fontCaption
                Layout.fillWidth: true
            }

            SegmentedChoice {
                options: [
                    {
                        value: "system",
                        label: qsTr("System")
                    },
                    {
                        value: "en",
                        label: qsTr("English")
                    },
                    {
                        value: "de",
                        label: qsTr("Deutsch")
                    }
                ]
                current: Bridge.language
                onSelected: function (value) {
                    Bridge.setLanguage(value)
                }
            }

            Label {
                text: qsTr("A language change applies the next time the window opens.")
                color: Theme.textTertiary
                font.pixelSize: Theme.fontCaption
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            ToggleRow {
                objectName: "startAtLogin"
                Layout.fillWidth: true
                label: qsTr("Start at login")
                description: qsTr("Writes an XDG autostart entry for this desktop session.")
                checked: Bridge.startAtLogin
                onToggled: function (checked) {
                    Bridge.setStartAtLogin(checked)
                }
            }
        }
    }

    Card {
        Layout.fillWidth: true
        title: qsTr("Account")
        subtitle: Bridge.loggedIn
                  ? qsTr("Signed in as %1").arg(Bridge.account)
                  : qsTr("Paste a Personal Login Token from Jottacloud → Settings → Security.")

        content: ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.gapMd

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.gapSm

                Field {
                    id: token
                    Layout.fillWidth: true
                    echoMode: TextInput.Password
                    placeholderText: qsTr("Personal Login Token")
                    visible: !Bridge.loggedIn
                    Accessible.name: qsTr("Personal Login Token")
                }

                PillButton {
                    variant: "primary"
                    text: qsTr("Sign in")
                    visible: !Bridge.loggedIn
                    enabled: token.text.length > 0
                    onClicked: {
                        Bridge.login(token.text)
                        token.text = ""
                    }
                }

                PillButton {
                    variant: "ghost"
                    text: qsTr("Sign out")
                    visible: Bridge.loggedIn
                    onClicked: Bridge.logout()
                }
            }

            Label {
                text: qsTr("The token is stored encrypted at rest, never in the configuration and never in a log line.")
                color: Theme.textTertiary
                font.pixelSize: Theme.fontCaption
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }

    Card {
        Layout.fillWidth: true
        title: qsTr("Sync")
        subtitle: qsTr("Local changes are pushed immediately; remote changes are found by a budgeted poll.")

        content: ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.gapLg

            ToggleRow {
                Layout.fillWidth: true
                label: qsTr("Pause sync")
                description: qsTr("Stop all transfers without losing the queue.")
                checked: Bridge.paused
                onToggled: function (checked) {
                    Bridge.setPaused(checked)
                }
            }

            SliderRow {
                Layout.fillWidth: true
                label: qsTr("Bandwidth")
                maximum: 200
                value: Bridge.bandwidthMbps
                valueText: value === 0 ? qsTr("unlimited") : qsTr("%1 Mbit/s").arg(Math.round(value))
                onMoved: function (value) {
                    Bridge.setBandwidthLimitMbps(Math.round(value))
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.gapSm

                PillButton {
                    variant: "primary"
                    text: qsTr("Run a sync pass now")
                    enabled: Bridge.loggedIn && !Bridge.busy
                    onClicked: Bridge.syncNow()
                }

                Label {
                    text: qsTr("One pass covers every unpaused pair.")
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
        title: qsTr("Sync pairs")
        subtitle: Bridge.syncPairs.length === 1
                  ? qsTr("1 folder kept in step with Jottacloud.")
                  : qsTr("%1 folders kept in step with Jottacloud.").arg(Bridge.syncPairs.length)

        content: ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.gapSm

            Label {
                visible: Bridge.syncPairs.length === 0
                text: qsTr("No pairs yet — add a local folder and the Jottacloud folder it mirrors.")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSmall
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            Repeater {
                model: Bridge.syncPairs

                delegate: ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.gapXs

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.gapSm

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0

                            Label {
                                text: modelData.name
                                color: modelData.paused ? Theme.textSecondary : Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                                font.bold: true
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            Label {
                                text: modelData.local + "  \u21c5  " + modelData.remote
                                color: Theme.textTertiary
                                font.pixelSize: Theme.fontCaption
                                elide: Text.ElideMiddle
                                Layout.fillWidth: true
                            }
                        }

                        Label {
                            text: Bridge.syncSummary(modelData.name)
                            color: Theme.textTertiary
                            font.pixelSize: Theme.fontCaption
                            visible: text.length > 0
                            elide: Text.ElideRight
                        }

                        Label {
                            text: qsTr("Paused")
                            color: Theme.textTertiary
                            font.pixelSize: Theme.fontCaption
                            visible: modelData.paused
                        }

                        PillButton {
                            variant: "ghost"
                            text: qsTr("Sync")
                            enabled: Bridge.loggedIn && !Bridge.busy
                            onClicked: Bridge.syncPairNow(modelData.name)
                            Accessible.name: qsTr("Sync %1 now").arg(modelData.name)
                        }

                        Toggle {
                            checked: modelData.paused
                            accessibleName: qsTr("Pause %1").arg(modelData.name)
                            onToggled: function (value) {
                                Bridge.setSyncPairPaused(modelData.name, value)
                            }
                        }

                        PillButton {
                            variant: "danger"
                            text: qsTr("Remove")
                            onClicked: Bridge.removeSyncPair(modelData.name)
                            Accessible.name: qsTr("Remove %1").arg(modelData.name)
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        Layout.leftMargin: Theme.gapMd
                        spacing: Theme.gapSm

                        Field {
                            id: excludeField
                            Layout.fillWidth: true
                            placeholderText: qsTr("Exclude globs, comma separated (e.g. **/node_modules, *.tmp)")
                            text: (modelData.exclude || []).join(", ")
                            Accessible.name: qsTr("Exclude globs for %1").arg(modelData.name)
                            onEditingFinished: Bridge.setSyncPairFilters(
                                modelData.name, (modelData.include || []).join(", "), text)
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 1
                color: Theme.border
                visible: Bridge.syncPairs.length > 0
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.gapSm

                Field {
                    id: pairName
                    Layout.fillWidth: true
                    Layout.preferredWidth: 110
                    placeholderText: qsTr("name")
                    Accessible.name: qsTr("Pair name")
                }

                Field {
                    id: pairLocal
                    Layout.fillWidth: true
                    Layout.preferredWidth: 230
                    placeholderText: qsTr("/home/you/Documents")
                    Accessible.name: qsTr("Local folder")
                }

                Field {
                    id: pairRemote
                    Layout.fillWidth: true
                    Layout.preferredWidth: 180
                    placeholderText: qsTr("/Archive/Documents")
                    Accessible.name: qsTr("Jottacloud folder")
                }

                PillButton {
                    variant: "primary"
                    text: qsTr("Add")
                    enabled: pairName.text.length > 0 && pairLocal.text.length > 0
                             && pairRemote.text.length > 0
                    onClicked: {
                        Bridge.addSyncPair(pairName.text, pairLocal.text, pairRemote.text)
                        pairName.text = ""
                        pairLocal.text = ""
                        pairRemote.text = ""
                    }
                    Accessible.name: qsTr("Add the sync pair")
                }
            }
        }
    }

    Card {
        Layout.fillWidth: true
        title: qsTr("Mount")
        subtitle: qsTr("Serve the archive as a local path.")

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

            PillButton {
                variant: Bridge.mounted ? "ghost" : "primary"
                text: Bridge.mounted ? qsTr("Unmount") : qsTr("Mount")
                enabled: Bridge.loggedIn
                onClicked: Bridge.mounted ? Bridge.unmount() : Bridge.mount(mountPath.text, false)
            }
        }
    }

    Card {
        Layout.fillWidth: true
        title: qsTr("Cache")
        subtitle: qsTr("%1 of %2 in use — warm reads never touch the network.")
                      .arg(Theme.humanBytes(Bridge.cacheBytes))
                      .arg(Theme.humanBytes(Bridge.cacheCapacity))

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

            SliderRow {
                Layout.fillWidth: true
                label: qsTr("Limit")
                maximum: 200
                value: Math.max(1, Math.round(Bridge.cacheCapacity / (1024 * 1024 * 1024)))
                valueText: qsTr("%1 GB").arg(Math.max(1, Math.round(value)))
                onMoved: function (value) {
                    Bridge.setCacheLimitGb(Math.round(value))
                }
            }
        }
    }

    Card {
        Layout.fillWidth: true
        title: qsTr("Plugins")
        subtitle: Bridge.plugins.length === 0
                  ? qsTr("Sandboxed WebAssembly, capability-scoped.")
                  : qsTr("%1 installed").arg(Bridge.plugins.length)

        content: ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.gapSm

            Label {
                visible: Bridge.plugins.length === 0
                text: qsTr("No plugins installed. Install one with `nidvue-cli plugins install <dir>`.")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSmall
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            Repeater {
                model: Bridge.plugins

                delegate: RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.gapSm

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0

                        Label {
                            text: modelData.name + " " + modelData.version
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontBody
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }

                        Label {
                            text: modelData.id + " · " + modelData.hooks.join(", ")
                            color: Theme.textTertiary
                            font.pixelSize: Theme.fontCaption
                            elide: Text.ElideMiddle
                            Layout.fillWidth: true
                        }
                    }

                    Label {
                        text: modelData.license
                        color: Theme.textTertiary
                        font.pixelSize: Theme.fontCaption
                    }
                }
            }

            PillButton {
                variant: "ghost"
                text: qsTr("Refresh")
                onClicked: Bridge.refreshPlugins()
                Accessible.name: qsTr("Reload the installed plugin list")
            }
        }
    }

    Card {
        Layout.fillWidth: true
        title: qsTr("Advanced")
        subtitle: qsTr("Every setting in config.toml. Pause, bandwidth, cache size and the sync pairs apply at once; the rest applies on the next start.")

        content: ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.gapSm

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.gapSm

                Field {
                    id: configFilter
                    Layout.fillWidth: true
                    placeholderText: qsTr("Filter settings…")
                    Accessible.name: qsTr("Filter the settings list")
                }

                Label {
                    text: pane.configRows.length === 1
                          ? qsTr("1 setting")
                          : qsTr("%1 settings").arg(pane.configRows.length)
                    color: Theme.textTertiary
                    font.pixelSize: Theme.fontCaption
                }
            }

            ListView {
                id: configList
                Layout.fillWidth: true
                implicitHeight: Math.min(contentHeight, 320)
                clip: true
                reuseItems: true
                model: pane.configRows
                spacing: 1
                ScrollBar.vertical: ScrollBar {}

                delegate: RowLayout {
                    required property var modelData
                    width: configList.width
                    spacing: Theme.gapSm

                    Label {
                        Layout.preferredWidth: 250
                        text: modelData.key
                        color: Theme.textSecondary
                        font.family: Theme.fontMono
                        font.pixelSize: Theme.fontCaption
                        elide: Text.ElideMiddle
                    }

                    Field {
                        Layout.fillWidth: true
                        text: modelData.value
                        Accessible.name: qsTr("Value of %1").arg(modelData.key)
                        onEditingFinished: {
                            if (text !== modelData.value)
                                Bridge.setConfigValue(modelData.key, text)
                        }
                    }
                }
            }
        }
    }

    Card {
        Layout.fillWidth: true
        title: qsTr("About")

        content: ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.gapXs

            Label {
                text: qsTr("Nidvue Drive %1").arg(Bridge.version)
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
            }

            Label {
                text: qsTr("An unofficial Jottacloud client. Jottacloud is a trademark of its owner. Licensed AGPL-3.0-only.")
                color: Theme.textTertiary
                font.pixelSize: Theme.fontCaption
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }
}
