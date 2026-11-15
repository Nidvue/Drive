// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: page

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

            SettingsPane {
                Layout.fillWidth: true
                Layout.bottomMargin: Theme.gapXl
            }
        }
    }
}
