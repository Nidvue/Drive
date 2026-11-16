// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

#pragma once

#include <QObject>
#include <QString>

class QFileSystemWatcher;
class QTimer;

/// User preferences that belong to the desktop shell rather than the engine.
///
/// The theme mode lives here instead of `config.toml` because it describes this
/// window, not the sync engine, and because it must be readable before the first
/// frame: a value loaded after `engine.load()` would flash the wrong theme.
///
/// `dark` is the single resolved answer the QML layer consumes. It follows the
/// platform colour scheme when the mode is `system`, so a desktop switch
/// repaints the window without a restart.
class Preferences : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString themeMode READ themeMode WRITE setThemeMode NOTIFY themeModeChanged)
    Q_PROPERTY(bool systemDark READ systemDark NOTIFY systemDarkChanged)
    Q_PROPERTY(bool dark READ dark NOTIFY darkChanged)

public:
    explicit Preferences(QObject *parent = nullptr);

    /// The default theme from `config.toml` (`ui.theme`), read before the first
    /// frame. A choice the user made in this window always wins over it.
    static void setConfigDefaultTheme(const QString &mode);

    QString themeMode() const { return m_themeMode; }
    void setThemeMode(const QString &mode);

    bool systemDark() const;
    bool dark() const;

signals:
    void themeModeChanged();
    void systemDarkChanged();
    void darkChanged();

private:
    void watchConfig();

    QString m_themeMode;
    QFileSystemWatcher *m_watcher = nullptr;
    QTimer *m_recheck = nullptr;
};
