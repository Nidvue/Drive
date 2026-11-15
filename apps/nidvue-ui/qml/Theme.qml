// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue
pragma Singleton

import QtQuick
import Nidvue.Preferences 1.0

QtObject {
    id: theme

    readonly property string mode: Preferences.themeMode
    readonly property bool systemDark: Preferences.systemDark
    readonly property bool dark: Preferences.dark

    readonly property color bg: dark ? "#0a0c11" : "#f5f7fb"
    readonly property color surface: dark ? "#11141b" : "#ffffff"
    readonly property color surfaceAlt: dark ? "#161a23" : "#eef2f9"
    readonly property color surfaceHover: dark ? "#1c2230" : "#e3e9f4"
    readonly property color border: dark ? "#1affffff" : "#14000000"
    readonly property color borderStrong: dark ? "#2effffff" : "#24000000"

    readonly property color textPrimary: dark ? "#edf0f7" : "#101725"
    readonly property color textSecondary: dark ? "#a2aab9" : "#4b5567"
    readonly property color textTertiary: dark ? "#6d7585" : "#79839a"

    readonly property color accent: dark ? "#7b61ff" : "#5a3cf0"
    readonly property color accentBright: dark ? "#8f79ff" : "#6d51f7"
    readonly property color accentDeep: dark ? "#6547e8" : "#4526cf"
    readonly property color accentAlt: dark ? "#3ce0c0" : "#0a9e83"
    readonly property color ink: dark ? "#0a0c11" : "#ffffff"

    readonly property color ok: dark ? "#3ddc84" : "#0f9d58"
    readonly property color warn: dark ? "#f5b544" : "#b8770a"
    readonly property color danger: dark ? "#ff5c72" : "#cf2440"
    readonly property color dangerSoft: dark ? "#1fff5c72" : "#14cf2440"
    readonly property color dangerBorder: dark ? "#3fff5c72" : "#3ccf2440"

    readonly property color headerFrom: dark ? "#1b1a3a" : "#e8e2ff"
    readonly property color headerTo: surface

    readonly property int radiusSm: 6
    readonly property int radiusMd: 9
    readonly property int radiusLg: 12
    readonly property int radiusXl: 15

    readonly property int gapXs: 2
    readonly property int gapSm: 5
    readonly property int gapMd: 8
    readonly property int gapLg: 10
    readonly property int gapXl: 14
    readonly property int gap2xl: 20

    readonly property int fontDisplay: 24
    readonly property int fontTitle: 17
    readonly property int fontBody: 14
    readonly property int fontSmall: 13
    readonly property int fontCaption: 12

    readonly property string fontMono: "JetBrains Mono, Hack, DejaVu Sans Mono, monospace"

    readonly property int sidebarWidth: 188
    readonly property int headerHeight: 44
    readonly property int pageGutter: 14
    readonly property int controlHeight: 28

    function healthColor(health, paused) {
        if (paused)
            return warn
        if (health === "ok")
            return ok
        if (health === "error")
            return danger
        if (health === "offline")
            return textTertiary
        return warn
    }

    function healthLabel(health, paused, loggedIn) {
        if (!loggedIn)
            return qsTr("Not signed in")
        if (paused)
            return qsTr("Paused")
        if (health === "ok")
            return qsTr("Up to date")
        if (health === "error")
            return qsTr("Attention")
        if (health === "offline")
            return qsTr("Offline")
        return qsTr("Syncing")
    }

    function humanBytes(bytes) {
        const n = Number(bytes)
        if (n >= 1024 * 1024 * 1024 * 1024)
            return (n / (1024 * 1024 * 1024 * 1024)).toFixed(1) + " TB"
        if (n >= 1024 * 1024 * 1024)
            return (n / (1024 * 1024 * 1024)).toFixed(1) + " GB"
        if (n >= 1024 * 1024)
            return (n / (1024 * 1024)).toFixed(1) + " MB"
        if (n >= 1024)
            return (n / 1024).toFixed(1) + " kB"
        return n + " B"
    }

    function overline(text) {
        return String(text).toUpperCase()
    }
}
