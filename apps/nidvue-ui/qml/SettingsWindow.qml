// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Window {
    id: settingsWindow

    width: 720
    height: 700
    minimumWidth: 600
    minimumHeight: 460
    visible: false
    title: qsTr("Nidvue Drive — settings")
    color: Theme.bg

    palette.window: Theme.bg
    palette.windowText: Theme.textPrimary
    palette.base: Theme.surfaceAlt
    palette.text: Theme.textPrimary
    palette.button: Theme.surfaceAlt
    palette.buttonText: Theme.textPrimary
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.ink
    palette.placeholderText: Theme.textTertiary

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

            Item { implicitHeight: Theme.gapXs }

            Label {
                text: qsTr("Settings")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontTitle
                font.bold: true
            }

            SettingsPane {
                Layout.fillWidth: true
                Layout.bottomMargin: Theme.gapXl
            }
        }
    }
}
