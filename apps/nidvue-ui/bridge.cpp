// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

#include "bridge.h"

#include <QDesktopServices>
#include <QDir>
#include <QUrl>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QThread>
#include <QTimer>
#include <QDateTime>
#include <QClipboard>
#include <QCoreApplication>
#include <QGuiApplication>
#include <QTime>

#include "nidvue.h"

namespace {
/// Copy a JSON array of conflicts into a QVariantList for QML.
QVariantList parseConflicts(const QString &json)
{
    QVariantList out;
    const auto doc = QJsonDocument::fromJson(json.toUtf8());
    if (!doc.isArray())
        return out;
    for (const auto &value : doc.array()) {
        const auto object = value.toObject();
        QVariantMap row;
        row.insert("id", object.value("id").toString());
        row.insert("path", object.value("path").toString());
        row.insert("conflictPath", object.value("conflict_path").toString());
        row.insert("kept", object.value("kept").toString());
        row.insert("localSize", object.value("local_size").toVariant());
        row.insert("remoteSize", object.value("remote_size").toVariant());
        out.append(row);
    }
    return out;
}

/// Copy the transfer array out of a status document.
QVariantList parseTransfers(const QJsonArray &array)
{
    QVariantList out;
    out.reserve(array.size());
    for (const auto &value : array) {
        const auto object = value.toObject();
        QVariantMap row;
        row.insert("id", object.value("id").toString());
        row.insert("path", object.value("path").toString());
        row.insert("direction", object.value("direction").toString());
        row.insert("state", object.value("state").toString());
        const double done = object.value("done").toDouble();
        const double total = object.value("total").toDouble();
        row.insert("done", done);
        row.insert("total", total);
        row.insert("fraction", total > 0.0 ? qBound(0.0, done / total, 1.0) : 0.0);
        const auto error = object.value("error");
        row.insert("error", error.isString() ? error.toString() : QString());
        row.insert("retryable", object.value("retryable").toBool());
        row.insert("updatedMs", object.value("updated_ms").toVariant());
        out.append(row);
    }
    return out;
}

QVariantList parseSyncPairs(const QString &json)
{
    QVariantList out;
    const auto doc = QJsonDocument::fromJson(json.toUtf8());
    if (!doc.isArray())
        return out;
    for (const auto &value : doc.array()) {
        const auto object = value.toObject();
        QVariantMap row;
        row.insert("name", object.value("name").toString());
        row.insert("local", object.value("local").toString());
        row.insert("remote", object.value("remote").toString());
        row.insert("paused", object.value("paused").toBool());
        out.append(row);
    }
    return out;
}

QVariantList parseSyncReports(const QString &json)
{
    QVariantList out;
    const auto doc = QJsonDocument::fromJson(json.toUtf8());
    if (!doc.isArray())
        return out;
    for (const auto &value : doc.array()) {
        const auto object = value.toObject();
        QVariantMap row;
        row.insert("pair", object.value("pair").toString());
        row.insert("scanned", object.value("scanned").toVariant());
        row.insert("planned", object.value("planned").toVariant());
        row.insert("applied", object.value("applied").toVariant());
        row.insert("conflicts", object.value("conflicts").toVariant());
        row.insert("failed", object.value("failed").toVariant());
        out.append(row);
    }
    return out;
}

QVariantList parsePlugins(const QString &json)
{
    QVariantList out;
    const auto doc = QJsonDocument::fromJson(json.toUtf8());
    if (!doc.isArray())
        return out;
    for (const auto &value : doc.array()) {
        const auto object = value.toObject();
        QVariantMap row;
        row.insert("id", object.value("id").toString());
        row.insert("name", object.value("name").toString());
        row.insert("version", object.value("version").toString());
        row.insert("license", object.value("license").toString());
        row.insert("hooks", object.value("hooks").toVariant());
        row.insert("dir", object.value("dir").toString());
        out.append(row);
    }
    return out;
}

QString takeError(char *err)
{
    if (!err)
        return QStringLiteral("unknown error");
    const QString message = QString::fromUtf8(err);
    nidvue_string_free(err);
    return message;
}

void eventCallback(const char *json, void *userdata)
{
    auto *worker = static_cast<Worker *>(userdata);
    if (worker && json)
        QMetaObject::invokeMethod(worker, "poll", Qt::QueuedConnection);
}
} // namespace

Worker::Worker(QObject *parent)
    : QObject(parent)
{
}

Worker::~Worker()
{
    if (m_client)
        nidvue_client_free(m_client);
}

void Worker::start(const QString &stateDir)
{
    char *err = nullptr;
    m_client = nidvue_client_new(stateDir.toUtf8().constData(), &err);
    if (!m_client) {
        emit error(takeError(err));
        return;
    }
    // The engine's event stream nudges a refresh; the timer is the heartbeat.
    nidvue_client_subscribe(m_client, eventCallback, this);
    m_timer = new QTimer(this);
    m_timer->setInterval(1000);
    connect(m_timer, &QTimer::timeout, this, &Worker::poll);
    m_timer->start();
    poll();
}

void Worker::poll()
{
    if (!m_client)
        return;
    char *out = nullptr;
    if (nidvue_client_status_json(m_client, &out) == NIDVUE_OK) {
        emit statusJson(QString::fromUtf8(out));
        nidvue_string_free(out);
    }
    out = nullptr;
    if (nidvue_client_conflicts_json(m_client, &out) == NIDVUE_OK) {
        emit conflictsJson(QString::fromUtf8(out));
        nidvue_string_free(out);
    }
    out = nullptr;
    if (nidvue_client_sync_pairs_json(m_client, &out) == NIDVUE_OK) {
        emit syncPairsJson(QString::fromUtf8(out));
        nidvue_string_free(out);
    }
}

void Worker::login(const QString &token)
{
    if (!m_client)
        return;
    emit busy(QStringLiteral("logging in"), true);
    char *err = nullptr;
    const int code = nidvue_client_login(m_client, token.toUtf8().constData(), &err);
    if (code != NIDVUE_OK)
        emit error(takeError(err));
    emit busy(QStringLiteral("logging in"), false);
    poll();
}

void Worker::logout()
{
    if (!m_client)
        return;
    char *err = nullptr;
    if (nidvue_client_logout(m_client, &err) != NIDVUE_OK)
        emit error(takeError(err));
    poll();
}

void Worker::mount(const QString &mountpoint, bool readOnly)
{
    if (!m_client)
        return;
    emit busy(QStringLiteral("mounting"), true);
    char *err = nullptr;
    const int code = nidvue_client_mount(m_client, mountpoint.toUtf8().constData(),
                                         readOnly ? 1 : 0, &err);
    if (code != NIDVUE_OK)
        emit error(takeError(err));
    else
        emit mountState(mountpoint, true);
    emit busy(QStringLiteral("mounting"), false);
    poll();
}

void Worker::unmount()
{
    if (!m_client)
        return;
    if (nidvue_client_unmount(m_client) == NIDVUE_OK)
        emit mountState(QString(), false);
    poll();
}

void Worker::setPaused(bool paused)
{
    if (!m_client)
        return;
    char *err = nullptr;
    if (nidvue_client_set_paused(m_client, paused ? 1 : 0, &err) != NIDVUE_OK)
        emit error(takeError(err));
    poll();
}

void Worker::setBandwidthLimit(qulonglong bytesPerSecond)
{
    if (!m_client)
        return;
    char *err = nullptr;
    if (nidvue_client_set_bandwidth_limit(m_client, bytesPerSecond, &err) != NIDVUE_OK)
        emit error(takeError(err));
    poll();
}

void Worker::setCacheLimit(qulonglong bytes)
{
    if (!m_client)
        return;
    char *err = nullptr;
    if (nidvue_client_set_cache_limit(m_client, bytes, &err) != NIDVUE_OK)
        emit error(takeError(err));
    poll();
}

void Worker::refreshPlugins()
{
    if (!m_client)
        return;
    char *out = nullptr;
    if (nidvue_client_plugins_json(m_client, &out) == NIDVUE_OK) {
        emit pluginsJson(QString::fromUtf8(out));
        nidvue_string_free(out);
    }
}

void Worker::syncPairNow(const QString &name)
{
    if (!m_client)
        return;
    emit busy(QStringLiteral("syncing %1").arg(name), true);
    char *out = nullptr;
    char *err = nullptr;
    if (nidvue_client_sync_pair_now(m_client, name.toUtf8().constData(), &out, &err)
        == NIDVUE_OK) {
        emit syncReportsJson(QStringLiteral("[%1]").arg(QString::fromUtf8(out)));
        nidvue_string_free(out);
    } else {
        emit error(takeError(err));
    }
    emit busy(QStringLiteral("syncing"), false);
    poll();
}

void Worker::syncNow()
{
    if (!m_client)
        return;
    emit busy(QStringLiteral("syncing"), true);
    char *out = nullptr;
    char *err = nullptr;
    const int code = nidvue_client_sync_now(m_client, &out, &err);
    if (code != NIDVUE_OK) {
        emit error(takeError(err));
    } else if (out) {
        emit syncReportsJson(QString::fromUtf8(out));
        nidvue_string_free(out);
    }
    emit busy(QStringLiteral("syncing"), false);
    poll();
}

void Worker::clearTransfers()
{
    if (!m_client)
        return;
    nidvue_client_clear_transfers(m_client);
    poll();
}

void Worker::addSyncPair(const QString &name, const QString &local, const QString &remote)
{
    if (!m_client)
        return;
    char *out = nullptr;
    char *err = nullptr;
    if (nidvue_client_sync_pair_add(m_client, name.toUtf8().constData(),
                                    local.toUtf8().constData(), remote.toUtf8().constData(),
                                    &out, &err)
        == NIDVUE_OK) {
        emit syncPairsJson(QString::fromUtf8(out));
        nidvue_string_free(out);
    } else {
        emit error(takeError(err));
    }
    poll();
}

void Worker::removeSyncPair(const QString &name)
{
    if (!m_client)
        return;
    char *out = nullptr;
    char *err = nullptr;
    if (nidvue_client_sync_pair_remove(m_client, name.toUtf8().constData(), &out, &err)
        == NIDVUE_OK) {
        emit syncPairsJson(QString::fromUtf8(out));
        nidvue_string_free(out);
    } else {
        emit error(takeError(err));
    }
    poll();
}

void Worker::setSyncPairPaused(const QString &name, bool paused)
{
    if (!m_client)
        return;
    char *out = nullptr;
    char *err = nullptr;
    if (nidvue_client_sync_pair_set_paused(m_client, name.toUtf8().constData(),
                                           paused ? 1 : 0, &out, &err)
        == NIDVUE_OK) {
        emit syncPairsJson(QString::fromUtf8(out));
        nidvue_string_free(out);
    } else {
        emit error(takeError(err));
    }
    poll();
}

void Worker::refreshConfig()
{
    if (!m_client)
        return;
    char *out = nullptr;
    char *err = nullptr;
    if (nidvue_client_config_list_json(m_client, &out, &err) == NIDVUE_OK) {
        emit configJson(QString::fromUtf8(out));
        nidvue_string_free(out);
    } else {
        emit error(takeError(err));
    }
}

void Worker::setConfigValue(const QString &key, const QString &value)
{
    if (!m_client)
        return;
    char *err = nullptr;
    if (nidvue_client_config_set(m_client, key.toUtf8().constData(),
                                 value.toUtf8().constData(), &err)
        != NIDVUE_OK) {
        emit error(takeError(err));
    }
    refreshConfig();
    poll();
}

void Worker::setSyncPairFilters(const QString &name, const QString &include,
                                const QString &exclude)
{
    if (!m_client)
        return;
    char *out = nullptr;
    char *err = nullptr;
    if (nidvue_client_sync_pair_set_filters(m_client, name.toUtf8().constData(),
                                            include.toUtf8().constData(),
                                            exclude.toUtf8().constData(), &out, &err)
        == NIDVUE_OK) {
        emit syncPairsJson(QString::fromUtf8(out));
        nidvue_string_free(out);
    } else {
        emit error(takeError(err));
    }
    poll();
}

void Worker::resolve(const QString &id, const QString &resolution)
{
    if (!m_client)
        return;
    char *err = nullptr;
    if (nidvue_client_resolve_conflict(m_client, id.toUtf8().constData(),
                                       resolution.toUtf8().constData(), &err)
        != NIDVUE_OK) {
        emit error(takeError(err));
    }
    poll();
}

Bridge::Bridge(QObject *parent)
    : QObject(parent)
{
    m_startAtLogin = nidvue_autostart_enabled() == 1;
    m_thread = new QThread(this);
    m_worker = new Worker();
    m_worker->moveToThread(m_thread);
    connect(m_thread, &QThread::finished, m_worker, &QObject::deleteLater);
    connect(m_worker, &Worker::statusJson, this, &Bridge::onStatus);
    connect(m_worker, &Worker::conflictsJson, this, &Bridge::onConflicts);
    connect(m_worker, &Worker::syncPairsJson, this, &Bridge::onSyncPairs);
    connect(m_worker, &Worker::syncReportsJson, this, &Bridge::onSyncReports);
    connect(m_worker, &Worker::pluginsJson, this, &Bridge::onPlugins);
    connect(m_worker, &Worker::configJson, this, &Bridge::onConfig);
    connect(m_worker, &Worker::eventJson, this, &Bridge::onEvent);
    connect(m_worker, &Worker::error, this, &Bridge::onError);
    connect(m_worker, &Worker::busy, this, &Bridge::onBusy);
    connect(m_worker, &Worker::mountState, this, [this](const QString &path, bool mountedNow) {
        m_mountpoint = path;
        m_mounted = mountedNow;
        emit mountedChanged();
    });
    connect(this, &Bridge::requestLogin, m_worker, &Worker::login);
    connect(this, &Bridge::requestLogout, m_worker, &Worker::logout);
    connect(this, &Bridge::requestMount, m_worker, &Worker::mount);
    connect(this, &Bridge::requestUnmount, m_worker, &Worker::unmount);
    connect(this, &Bridge::requestSync, m_worker, &Worker::syncNow);
    connect(this, &Bridge::requestSyncPair, m_worker, &Worker::syncPairNow);
    connect(this, &Bridge::requestSetCacheLimit, m_worker, &Worker::setCacheLimit);
    connect(this, &Bridge::requestRefreshPlugins, m_worker, &Worker::refreshPlugins);
    connect(this, &Bridge::requestRefreshConfig, m_worker, &Worker::refreshConfig);
    connect(this, &Bridge::requestSetConfigValue, m_worker, &Worker::setConfigValue);
    connect(this, &Bridge::requestSetPaused, m_worker, &Worker::setPaused);
    connect(this, &Bridge::requestSetBandwidth, m_worker, &Worker::setBandwidthLimit);
    connect(this, &Bridge::requestResolve, m_worker, &Worker::resolve);
    connect(this, &Bridge::requestAddSyncPair, m_worker, &Worker::addSyncPair);
    connect(this, &Bridge::requestRemoveSyncPair, m_worker, &Worker::removeSyncPair);
    connect(this, &Bridge::requestSetSyncPairPaused, m_worker, &Worker::setSyncPairPaused);
    connect(this, &Bridge::requestSetSyncPairFilters, m_worker, &Worker::setSyncPairFilters);

    const QString state = qEnvironmentVariable("NIDVUE_STATE",
        QDir::homePath() + QStringLiteral("/.local/share/nidvue-drive"));
    m_thread->start();
    QMetaObject::invokeMethod(m_worker, "start", Qt::QueuedConnection, Q_ARG(QString, state));
    appendLog(QStringLiteral("state directory: %1").arg(state));
}

Bridge::~Bridge()
{
    if (m_thread) {
        m_thread->quit();
        m_thread->wait(5000);
    }
}

void Bridge::login(const QString &token) { emit requestLogin(token); }
void Bridge::logout() { emit requestLogout(); }
void Bridge::mount(const QString &mountpoint, bool readOnly) { emit requestMount(mountpoint, readOnly); }
void Bridge::unmount() { emit requestUnmount(); }
void Bridge::syncNow() { emit requestSync(); }

void Bridge::syncPairNow(const QString &name) { emit requestSyncPair(name); }

QString Bridge::syncSummary(const QString &name) const
{
    for (const auto &value : m_lastSync) {
        const auto row = value.toMap();
        if (row.value("pair").toString() != name)
            continue;
        const qulonglong applied = row.value("applied").toULongLong();
        const qulonglong conflicts = row.value("conflicts").toULongLong();
        const qulonglong failed = row.value("failed").toULongLong();
        QStringList parts;
        parts << QStringLiteral("%1 applied").arg(applied);
        if (conflicts > 0)
            parts << QStringLiteral("%1 conflicts").arg(conflicts);
        if (failed > 0)
            parts << QStringLiteral("%1 failed").arg(failed);
        return parts.join(QStringLiteral(" · "));
    }
    return QString();
}

QString Bridge::syncTotals() const
{
    if (m_lastSync.isEmpty())
        return QString();
    qulonglong applied = 0;
    qulonglong conflicts = 0;
    qulonglong failed = 0;
    for (const auto &value : m_lastSync) {
        const auto row = value.toMap();
        applied += row.value("applied").toULongLong();
        conflicts += row.value("conflicts").toULongLong();
        failed += row.value("failed").toULongLong();
    }
    QStringList parts;
    parts << QStringLiteral("%1 applied").arg(applied);
    if (conflicts > 0)
        parts << QStringLiteral("%1 conflicts").arg(conflicts);
    if (failed > 0)
        parts << QStringLiteral("%1 failed").arg(failed);
    return parts.join(QStringLiteral(" · "));
}

void Bridge::clearLogs()
{
    m_logs.clear();
    emit logsChanged();
    emit logsCleared();
}

void Bridge::copyLogs()
{
    QGuiApplication::clipboard()->setText(m_logs.join(QLatin1Char('\n')));
}

void Bridge::setPaused(bool paused)
{
    m_paused = paused;
    emit pausedChanged();
    emit requestSetPaused(paused);
}

void Bridge::setBandwidthLimitMbps(int mbps)
{
    m_bandwidthMbps = qMax(0, mbps);
    emit bandwidthChanged();
    emit requestSetBandwidth(static_cast<qulonglong>(m_bandwidthMbps) * 1000ull * 1000ull / 8ull);
}

void Bridge::openMount()
{
    // Mounting needs the engine, so it goes through the worker; opening the
    // folder happens once the mount reports success.
    if (!m_mounted) {
        mount(m_mountpoint, false);
        return;
    }
    QDesktopServices::openUrl(QUrl::fromLocalFile(m_mountpoint));
}

void Bridge::showMainWindow() { emit showMainRequested(); }
void Bridge::resolve(const QString &id, const QString &resolution) { emit requestResolve(id, resolution); }

void Bridge::clearTransfers()
{
    QMetaObject::invokeMethod(m_worker, "clearTransfers", Qt::QueuedConnection);
}

void Bridge::addSyncPair(const QString &name, const QString &local, const QString &remote)
{
    emit requestAddSyncPair(name, local, remote);
}

void Bridge::removeSyncPair(const QString &name) { emit requestRemoveSyncPair(name); }

void Bridge::setCacheLimitGb(int gb)
{
    if (gb <= 0)
        return;
    emit requestSetCacheLimit(qulonglong(gb) * 1024ull * 1024ull * 1024ull);
}

void Bridge::refreshPlugins() { emit requestRefreshPlugins(); }

void Bridge::refreshConfig() { emit requestRefreshConfig(); }

void Bridge::setConfigValue(const QString &key, const QString &value)
{
    emit requestSetConfigValue(key, value);
}

void Bridge::setLanguage(const QString &language)
{
    setConfigValue(QStringLiteral("ui.language"), language);
}

void Bridge::setStartAtLogin(bool enabled)
{
    const QByteArray exec = QCoreApplication::applicationFilePath().toUtf8();
    char *err = nullptr;
    if (nidvue_autostart_set(exec.constData(), enabled ? 1 : 0, &err) != NIDVUE_OK) {
        if (err != nullptr) {
            m_lastError = QString::fromUtf8(err);
            nidvue_string_free(err);
        }
        emit lastErrorChanged();
        return;
    }
    m_startAtLogin = nidvue_autostart_enabled() == 1;
    emit startAtLoginChanged();
}

void Bridge::setSyncPairPaused(const QString &name, bool paused)
{
    emit requestSetSyncPairPaused(name, paused);
}

void Bridge::setSyncPairFilters(const QString &name, const QString &include,
                                const QString &exclude)
{
    emit requestSetSyncPairFilters(name, include, exclude);
}

void Bridge::clearError()
{
    m_lastError.clear();
    emit lastErrorChanged();
}

void Bridge::onStatus(const QString &json)
{
    const auto object = QJsonDocument::fromJson(json.toUtf8()).object();
    m_version = object.value("version").toString();
    m_loggedIn = object.value("logged_in").toBool();
    m_account = object.value("username").toString();
    m_health = object.value("health").toString();
    m_indexEntries = object.value("index_entries").toVariant().toULongLong();
    m_conflictsOpen = object.value("conflicts_open").toVariant().toULongLong();
    const bool paused = object.value("paused").toBool();
    if (paused != m_paused) {
        m_paused = paused;
        emit pausedChanged();
    }
    const auto bps = object.value("bandwidth_limit_bps").toVariant().toULongLong();
    const int mbps = static_cast<int>((bps * 8ull) / (1000ull * 1000ull));
    if (mbps != m_bandwidthMbps) {
        m_bandwidthMbps = mbps;
        emit bandwidthChanged();
    }
    const auto cache = object.value("cache").toObject();
    m_cacheBytes = cache.value("bytes").toVariant().toULongLong();
    m_cacheCapacity = cache.value("capacity").toVariant().toULongLong();
    const auto transfers = parseTransfers(object.value("transfers").toArray());
    const int limit = object.value("transfer_limit").toInt(512);
    int live = 0;
    for (const auto &row : transfers) {
        const auto state = row.toMap().value("state").toString();
        if (state == QStringLiteral("queued") || state == QStringLiteral("running"))
            ++live;
    }
    if (transfers != m_transfers || limit != m_transferLimit || live != m_liveTransfers) {
        m_transfers = transfers;
        m_transferLimit = limit;
        m_liveTransfers = live;
        emit transfersChanged();
    }
    emit statusChanged();
}

void Bridge::onConflicts(const QString &json)
{
    m_conflicts = parseConflicts(json);
    emit conflictsChanged();
}

void Bridge::onSyncPairs(const QString &json)
{
    m_syncPairs = parseSyncPairs(json);
    emit syncPairsChanged();
}

void Bridge::onSyncReports(const QString &json)
{
    m_lastSync = parseSyncReports(json);
    m_lastSyncAt = QTime::currentTime().toString(QStringLiteral("HH:mm:ss"));
    for (const auto &value : m_lastSync) {
        const auto row = value.toMap();
        appendLog(QStringLiteral("sync %1: %2 applied, %3 conflicts, %4 failed")
                      .arg(row.value("pair").toString())
                      .arg(row.value("applied").toULongLong())
                      .arg(row.value("conflicts").toULongLong())
                      .arg(row.value("failed").toULongLong()));
    }
    emit lastSyncChanged();
}

void Bridge::onPlugins(const QString &json)
{
    m_plugins = parsePlugins(json);
    emit pluginsChanged();
}

void Bridge::onConfig(const QString &json)
{
    QVariantList rows;
    const auto object = QJsonDocument::fromJson(json.toUtf8()).object();
    rows.reserve(object.size());
    QStringList keys = object.keys();
    keys.sort();
    for (const auto &key : keys) {
        const auto value = object.value(key);
        QString text;
        if (value.isArray()) {
            text = QString::fromUtf8(
                QJsonDocument(value.toArray()).toJson(QJsonDocument::Compact));
        } else if (value.isObject()) {
            text = QString::fromUtf8(
                QJsonDocument(value.toObject()).toJson(QJsonDocument::Compact));
        } else if (value.isString()) {
            text = value.toString();
        } else if (value.isBool()) {
            text = value.toBool() ? QStringLiteral("true") : QStringLiteral("false");
        } else if (value.isNull()) {
            text = QString();
        } else {
            text = QString::number(value.toDouble());
        }
        QVariantMap row;
        row.insert("key", key);
        row.insert("value", text);
        rows.append(row);
    }
    m_configRows = rows;
    m_language = object.value(QStringLiteral("ui.language")).toString();
    if (m_language.isEmpty())
        m_language = QStringLiteral("system");
    emit configChanged();
}

void Bridge::onEvent(const QString &json)
{
    const auto object = QJsonDocument::fromJson(json.toUtf8()).object();
    const QString kind = object.value("type").toString();

    if (kind.startsWith(QLatin1String("transfer_"))) {
        const QString path = object.value("path").toString();
        const QString state = kind == QLatin1String("transfer_queued")    ? QStringLiteral("active")
                              : kind == QLatin1String("transfer_done")    ? QStringLiteral("done")
                              : kind == QLatin1String("transfer_progress") ? QStringLiteral("active")
                                                                          : QStringLiteral("failed");
        const QString direction = object.value("direction").toString();
        const qulonglong size = object.value("size").toVariant().toULongLong();

        int at = -1;
        for (int i = 0; i < m_transfers.size(); ++i) {
            const auto row = m_transfers.at(i).toMap();
            if (row.value("path").toString() == path
                && row.value("direction").toString() == direction) {
                at = i;
                break;
            }
        }
        QVariantMap row;
        if (at >= 0)
            row = m_transfers.at(at).toMap();
        else
            row.insert("started", QTime::currentTime().toString(QStringLiteral("HH:mm:ss")));
        row.insert("path", path);
        row.insert("direction", direction);
        row.insert("state", state);
        if (size > 0)
            row.insert("size", size);
        const qulonglong done = object.value("done").toVariant().toULongLong();
        const qulonglong total = object.value("total").toVariant().toULongLong();
        if (total > 0) {
            row.insert("done", done);
            row.insert("total", total);
        }
        if (kind == QLatin1String("transfer_failed"))
            row.insert("error", object.value("error").toString());

        if (at >= 0)
            m_transfers[at] = row;
        else
            m_transfers.prepend(row);
        while (m_transfers.size() > 200)
            m_transfers.removeLast();
        emit transfersChanged();
    }
    if (kind.isEmpty()) {
        const auto inner = object.value("event").toObject();
        appendLog(QStringLiteral("%1 %2").arg(inner.value("type").toString(),
                                              QString::fromUtf8(QJsonDocument(inner).toJson(QJsonDocument::Compact))));
    } else {
        appendLog(QStringLiteral("%1 %2").arg(kind,
                                              QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact))));
    }
}

void Bridge::onError(const QString &message)
{
    m_lastError = message;
    appendLog(QStringLiteral("error: %1").arg(message));
    emit lastErrorChanged();
}

void Bridge::onBusy(const QString &what, bool active)
{
    m_busy = active;
    m_busyWhat = active ? what : QString();
    emit busyChanged();
}

void Bridge::appendLog(const QString &line)
{
    const QString stamp = QDateTime::currentDateTime().toString(QStringLiteral("HH:mm:ss"));
    m_logs.prepend(QStringLiteral("%1  %2").arg(stamp, line));
    while (m_logs.size() > 500)
        m_logs.removeLast();
    emit logsChanged();
}
