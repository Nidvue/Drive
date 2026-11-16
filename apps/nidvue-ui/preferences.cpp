// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

#include "preferences.h"

#include <QColor>
#include <QFile>
#include <QFileSystemWatcher>
#include <QGuiApplication>
#include <QPalette>
#include <QSettings>
#include <QStandardPaths>
#include <QStyleHints>
#include <QStringList>
#include <QTimer>

namespace {
constexpr auto kThemeKey = "appearance/themeMode";
constexpr qreal kDarkThreshold = 0.5;

QString normalise(const QString &mode)
{
    if (mode == QLatin1String("light") || mode == QLatin1String("dark"))
        return mode;
    return QStringLiteral("system");
}

QString iniValue(const QString &text, const QString &section, const QString &key)
{
    bool inSection = false;
    const QStringList lines = text.split(QLatin1Char('\n'));
    for (const QString &raw : lines) {
        const QString line = raw.trimmed();
        if (line.startsWith(QLatin1Char('[')))
            inSection = line == QLatin1Char('[') + section + QLatin1Char(']');
        else if (inSection && line.startsWith(key + QLatin1Char('=')))
            return line.mid(key.size() + 1).trimmed();
    }
    return QString();
}

QString kdeGlobalsPath()
{
    return QStandardPaths::writableLocation(QStandardPaths::ConfigLocation)
           + QLatin1String("/kdeglobals");
}

bool isKdeDesktop()
{
    return qEnvironmentVariable("XDG_CURRENT_DESKTOP").contains(QLatin1String("KDE"), Qt::CaseInsensitive);
}

qreal kdeWindowLuminance()
{
    if (!isKdeDesktop())
        return -1.0;

    QFile file(kdeGlobalsPath());
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text))
        return -1.0;
    const QString text = QString::fromUtf8(file.readAll());

    const QStringList parts = iniValue(text, QStringLiteral("Colors:Window"),
                                       QStringLiteral("BackgroundNormal"))
                                  .split(QLatin1Char(','));
    if (parts.size() != 3)
        return -1.0;

    bool ok = false;
    const int r = parts.at(0).trimmed().toInt(&ok);
    if (!ok)
        return -1.0;
    const int g = parts.at(1).trimmed().toInt(&ok);
    if (!ok)
        return -1.0;
    const int b = parts.at(2).trimmed().toInt(&ok);
    if (!ok)
        return -1.0;
    return QColor(r, g, b).lightnessF();
}
} // namespace

Preferences::Preferences(QObject *parent)
    : QObject(parent)
    , m_themeMode(normalise(
          QSettings().value(QLatin1String(kThemeKey), QStringLiteral("system")).toString()))
{
    m_recheck = new QTimer(this);
    m_recheck->setSingleShot(true);
    m_recheck->setInterval(150);
    connect(m_recheck, &QTimer::timeout, this, [this]() {
        emit systemDarkChanged();
        emit darkChanged();
    });

    const auto changed = [this]() { m_recheck->start(); };

#if QT_VERSION >= QT_VERSION_CHECK(6, 5, 0)
    connect(QGuiApplication::styleHints(), &QStyleHints::colorSchemeChanged, this, changed);
#endif

    m_watcher = new QFileSystemWatcher(this);
    watchConfig();
    connect(m_watcher, &QFileSystemWatcher::fileChanged, this, [this, changed](const QString &) {
        watchConfig();
        changed();
    });
    connect(m_watcher, &QFileSystemWatcher::directoryChanged, this, [this, changed](const QString &) {
        watchConfig();
        changed();
    });
}

void Preferences::watchConfig()
{
    const QString dir = QStandardPaths::writableLocation(QStandardPaths::ConfigLocation);
    if (QFile::exists(dir) && !m_watcher->directories().contains(dir))
        m_watcher->addPath(dir);
    const QString globals = kdeGlobalsPath();
    if (isKdeDesktop() && QFile::exists(globals) && !m_watcher->files().contains(globals))
        m_watcher->addPath(globals);
}

bool Preferences::systemDark() const
{
#if QT_VERSION >= QT_VERSION_CHECK(6, 5, 0)
    const Qt::ColorScheme scheme = QGuiApplication::styleHints()->colorScheme();
    if (scheme == Qt::ColorScheme::Dark)
        return true;
    if (scheme == Qt::ColorScheme::Light)
        return false;
#endif
    const qreal desktop = kdeWindowLuminance();
    if (desktop >= 0.0)
        return desktop < kDarkThreshold;
    return QGuiApplication::palette().color(QPalette::Window).lightnessF() < kDarkThreshold;
}

bool Preferences::dark() const
{
    if (m_themeMode == QLatin1String("dark"))
        return true;
    if (m_themeMode == QLatin1String("light"))
        return false;
    return systemDark();
}

void Preferences::setThemeMode(const QString &mode)
{
    const QString wanted = normalise(mode);
    if (wanted == m_themeMode)
        return;

    const bool wasDark = dark();
    m_themeMode = wanted;
    QSettings().setValue(QLatin1String(kThemeKey), wanted);
    emit themeModeChanged();
    if (dark() != wasDark)
        emit darkChanged();
}
