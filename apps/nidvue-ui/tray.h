// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

#pragma once

#include <QIcon>
#include <QObject>

class QAction;
#include <QPoint>
#include <QPointer>
#include <QSystemTrayIcon>

class Bridge;
class QQmlEngine;
class QQuickWindow;
class QMenu;

/// The cross-platform tray icon.
///
/// It shows sync health at a glance, offers the shortcuts a user actually
/// wants (pause, sync now, mount, settings) and opens a compact popup with the
/// detail; the full settings window is one click away.
///
/// The icon is painted at runtime, so it scales with the tray and needs no
/// bitmap asset. Where the desktop has no tray (some Wayland sessions without
/// a status-notifier host, headless CI), `isSystemTrayAvailable()` is false and
/// the controller simply does nothing: the main window is still complete.
class TrayController : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool available READ available CONSTANT)
    Q_PROPERTY(QString statusText READ statusText NOTIFY statusTextChanged)

public:
    TrayController(Bridge *bridge, QQmlEngine *engine, QObject *parent = nullptr);
    ~TrayController() override;

    bool available() const;
    QString statusText() const { return m_statusText; }

    /// Show the compact popup next to the tray, or hide it when visible.
    Q_INVOKABLE void togglePopup();
    Q_INVOKABLE void showPopup();
    Q_INVOKABLE void openSettingsWindow();
    Q_INVOKABLE void openMainWindow();

signals:
    void statusTextChanged();
    void openSettingsRequested();
    void openMainRequested();

private slots:
    void onActivated(QSystemTrayIcon::ActivationReason reason);
    void refresh();

private:
    QIcon paintIcon(const QString &health) const;
    void buildMenu();
    void showSettingsWindow();

    Bridge *m_bridge;
    QQmlEngine *m_engine;
    QSystemTrayIcon *m_tray = nullptr;
    QMenu *m_menu = nullptr;
    QAction *m_statusAction = nullptr;
    QAction *m_pauseAction = nullptr;
    QAction *m_mountAction = nullptr;
    QAction *m_syncAction = nullptr;
    QPointer<QQuickWindow> m_popup;
    QPointer<QQuickWindow> m_settings;
    QString m_statusText;
};
