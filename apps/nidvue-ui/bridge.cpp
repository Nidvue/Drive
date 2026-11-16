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
        emit eventJson(QStringLiteral("{\"type\":\"sync_finished\",\"reports\":%1}").arg(QString::fromUtf8(out)));
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
    m_thread = new QThread(this);
    m_worker = new Worker();
    m_worker->moveToThread(m_thread);
    connect(m_thread, &QThread::finished, m_worker, &QObject::deleteLater);
    connect(m_worker, &Worker::statusJson, this, &Bridge::onStatus);
    connect(m_worker, &Worker::conflictsJson, this, &Bridge::onConflicts);
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
    connect(this, &Bridge::requestSetPaused, m_worker, &Worker::setPaused);
    connect(this, &Bridge::requestSetBandwidth, m_worker, &Worker::setBandwidthLimit);
    connect(this, &Bridge::requestResolve, m_worker, &Worker::resolve);

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
    if (transfers != m_transfers || limit != m_transferLimit) {
        m_transfers = transfers;
        m_transferLimit = limit;
        emit transfersChanged();
    }
    emit statusChanged();
}

void Bridge::onConflicts(const QString &json)
{
    m_conflicts = parseConflicts(json);
    emit conflictsChanged();
}

void Bridge::onEvent(const QString &json)
{
    const auto object = QJsonDocument::fromJson(json.toUtf8()).object();
    const QString kind = object.value("type").toString();
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
