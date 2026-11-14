// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// The main window's Settings page. The same pane opens from the tray as a
// standalone detailed-settings window.
Item {
    ScrollView {
        anchors.fill: parent
        contentWidth: availableWidth
        ColumnLayout {
            width: parent.width
            anchors.margins: 20
            SettingsPane {}
        }
    }
}
