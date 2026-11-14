// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The detailed settings window the tray opens. It shares SettingsPane with the
// main window, so there is one definition of every control.
Window {
    id: settingsWindow
    width: 720
    height: 640
    visible: false
    title: qsTr("Nidvue Drive — settings")
    color: palette.window

    ScrollView {
        anchors.fill: parent
        contentWidth: availableWidth
        ColumnLayout {
            width: settingsWindow.width
            anchors.margins: 20
            SettingsPane {}
        }
    }
}
