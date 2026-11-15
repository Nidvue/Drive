// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Nidvue Drive desktop application.
//!
//! Qt 6 + QML, no business logic in QML: every value arrives from the engine
//! through the C ABI and the Bridge, and every command goes back the same way.

#include <QApplication>
#include <QDebug>
#include <QFont>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QTimer>

#include "bridge.h"
#include "preferences.h"
#include "tray.h"

int main(int argc, char *argv[])
{
    // Qt keys its compiled-QML cache on a resource timestamp, which a qrc entry
    // does not carry: a rebuilt UI would keep running the first compilation.
    qputenv("QML_DISABLE_DISK_CACHE", QByteArrayLiteral("1"));

    QApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("Nidvue Drive"));
    app.setOrganizationName(QStringLiteral("Nidvue"));
    app.setApplicationVersion(QStringLiteral(NIDVUE_UI_VERSION));

    QFont base = app.font();
    base.setFamilies({
        QStringLiteral("Inter"),
        QStringLiteral("SF Pro Text"),
        QStringLiteral("Segoe UI Variable Text"),
        QStringLiteral("Segoe UI"),
        QStringLiteral("Cantarell"),
        QStringLiteral("Noto Sans"),
        QStringLiteral("DejaVu Sans"),
    });
    base.setHintingPreference(QFont::PreferFullHinting);
    app.setFont(base);

    // Basic is always available and keeps the dependency set LGPL-compatible.
    QQuickStyle::setStyle(QStringLiteral("Basic"));

    // Closing the main window hides it when a tray is available: the app keeps
    // syncing, and the tray is the way back. Without a tray it quits.
    QGuiApplication::setQuitOnLastWindowClosed(false);

    Bridge bridge;
    Preferences preferences;
    qInfo("theme: mode=%s dark=%d systemDark=%d", qPrintable(preferences.themeMode()),
          preferences.dark(), preferences.systemDark());
    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty(QStringLiteral("Bridge"), &bridge);
    qmlRegisterSingletonInstance("Nidvue.Preferences", 1, 0, "Preferences", &preferences);
    TrayController tray(&bridge, &engine);
    engine.rootContext()->setContextProperty(QStringLiteral("Tray"), &tray);
    QObject::connect(
        &engine, &QQmlApplicationEngine::objectCreationFailed, &app,
        []() { QCoreApplication::exit(1); }, Qt::QueuedConnection);
    engine.load(QUrl(QStringLiteral("qrc:/qml/Main.qml")));
    if (engine.rootObjects().isEmpty())
        return 1;
    auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
    if (window) {
        QObject::connect(&tray, &TrayController::openMainRequested, window, [window]() {
            window->show();
            window->raise();
            window->requestActivate();
        });
        QObject::connect(&bridge, &Bridge::showMainRequested, window, [window]() {
            window->show();
            window->raise();
            window->requestActivate();
        });
    }
    // `--smoke-windows` opens the tray popup and the detailed settings window
    // once, so a headless CI run proves they instantiate without QML errors.
    if (app.arguments().contains(QStringLiteral("--smoke-windows"))) {
        QTimer::singleShot(200, &tray, [&tray]() {
            tray.showPopup();
            tray.openSettingsWindow();
        });
        QTimer::singleShot(1200, &app, &QCoreApplication::quit);
    }
    return app.exec();
}
