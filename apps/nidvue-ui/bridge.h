// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>

class QThread;
class QTimer;
struct NidvueClient;

/// Everything that touches the engine runs here, on its own thread.
///
/// The UI thread never calls into Rust: a status poll is a SQLite read, a mount
/// is a syscall, and neither belongs on the frame loop.
class Worker : public QObject {
    Q_OBJECT
public:
    explicit Worker(QObject *parent = nullptr);
    ~Worker() override;

public slots:
    void start(const QString &stateDir);
    void poll();
    void login(const QString &token);
    void logout();
    void mount(const QString &mountpoint, bool readOnly);
    void unmount();
    void syncNow();
    void resolve(const QString &id, const QString &resolution);

signals:
    void statusJson(const QString &json);
    void conflictsJson(const QString &json);
    void eventJson(const QString &json);
    void error(const QString &message);
    void busy(const QString &what, bool active);
    void mountState(const QString &mountpoint, bool mounted);

private:
    NidvueClient *m_client = nullptr;
    QTimer *m_timer = nullptr;
};

/// The QML-facing facade: properties and invokables, no I/O.
class Bridge : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString account READ account NOTIFY statusChanged)
    Q_PROPERTY(QString health READ health NOTIFY statusChanged)
    Q_PROPERTY(QString version READ version NOTIFY statusChanged)
    Q_PROPERTY(qulonglong indexEntries READ indexEntries NOTIFY statusChanged)
    Q_PROPERTY(qulonglong conflictsOpen READ conflictsOpen NOTIFY statusChanged)
    Q_PROPERTY(qulonglong cacheBytes READ cacheBytes NOTIFY statusChanged)
    Q_PROPERTY(qulonglong cacheCapacity READ cacheCapacity NOTIFY statusChanged)
    Q_PROPERTY(bool loggedIn READ loggedIn NOTIFY statusChanged)
    Q_PROPERTY(bool mounted READ mounted NOTIFY mountedChanged)
    Q_PROPERTY(QString mountpoint READ mountpoint NOTIFY mountedChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)
    Q_PROPERTY(QString busyWhat READ busyWhat NOTIFY busyChanged)
    Q_PROPERTY(QVariantList conflicts READ conflicts NOTIFY conflictsChanged)
    Q_PROPERTY(QStringList logs READ logs NOTIFY logsChanged)
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)

public:
    explicit Bridge(QObject *parent = nullptr);
    ~Bridge() override;

    QString account() const { return m_account; }
    QString health() const { return m_health; }
    QString version() const { return m_version; }
    qulonglong indexEntries() const { return m_indexEntries; }
    qulonglong conflictsOpen() const { return m_conflictsOpen; }
    qulonglong cacheBytes() const { return m_cacheBytes; }
    qulonglong cacheCapacity() const { return m_cacheCapacity; }
    bool loggedIn() const { return m_loggedIn; }
    bool mounted() const { return m_mounted; }
    QString mountpoint() const { return m_mountpoint; }
    bool busy() const { return m_busy; }
    QString busyWhat() const { return m_busyWhat; }
    QVariantList conflicts() const { return m_conflicts; }
    QStringList logs() const { return m_logs; }
    QString lastError() const { return m_lastError; }

    Q_INVOKABLE void login(const QString &token);
    Q_INVOKABLE void logout();
    Q_INVOKABLE void mount(const QString &mountpoint, bool readOnly);
    Q_INVOKABLE void unmount();
    Q_INVOKABLE void syncNow();
    Q_INVOKABLE void resolve(const QString &id, const QString &resolution);
    Q_INVOKABLE void clearError();

signals:
    void statusChanged();
    void mountedChanged();
    void busyChanged();
    void conflictsChanged();
    void logsChanged();
    void lastErrorChanged();
    void requestLogin(const QString &token);
    void requestLogout();
    void requestMount(const QString &mountpoint, bool readOnly);
    void requestUnmount();
    void requestSync();
    void requestResolve(const QString &id, const QString &resolution);

private slots:
    void onStatus(const QString &json);
    void onConflicts(const QString &json);
    void onEvent(const QString &json);
    void onError(const QString &message);
    void onBusy(const QString &what, bool active);

private:
    void appendLog(const QString &line);

    QThread *m_thread = nullptr;
    Worker *m_worker = nullptr;
    QString m_account;
    QString m_health;
    QString m_version;
    qulonglong m_indexEntries = 0;
    qulonglong m_conflictsOpen = 0;
    qulonglong m_cacheBytes = 0;
    qulonglong m_cacheCapacity = 0;
    bool m_loggedIn = false;
    bool m_mounted = false;
    QString m_mountpoint;
    bool m_busy = false;
    QString m_busyWhat;
    QVariantList m_conflicts;
    QStringList m_logs;
    QString m_lastError;
};
