// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Nidvue Drive desktop application.
//!
//! Qt 6 + QML, no business logic in QML: every value arrives from the engine
//! through the C ABI and the Bridge, and every command goes back the same way.

#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>

#include "bridge.h"

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("Nidvue Drive"));
    app.setOrganizationName(QStringLiteral("Nidvue"));
    app.setApplicationVersion(QStringLiteral(NIDVUE_UI_VERSION));

    // Basic is always available and keeps the dependency set LGPL-compatible.
    QQuickStyle::setStyle(QStringLiteral("Basic"));

    Bridge bridge;
    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty(QStringLiteral("Bridge"), &bridge);
    QObject::connect(
        &engine, &QQmlApplicationEngine::objectCreationFailed, &app,
        []() { QCoreApplication::exit(1); }, Qt::QueuedConnection);
    engine.load(QUrl(QStringLiteral("qrc:/qml/Main.qml")));
    if (engine.rootObjects().isEmpty())
        return 1;
    return app.exec();
}
