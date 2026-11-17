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
    void syncPairNow(const QString &name);
    void setPaused(bool paused);
    void setBandwidthLimit(qulonglong bytesPerSecond);
    void setCacheLimit(qulonglong bytes);
    void refreshPlugins();
    void refreshConfig();
    void setConfigValue(const QString &key, const QString &value);
    void resolve(const QString &id, const QString &resolution);
    void clearTransfers();
    void addSyncPair(const QString &name, const QString &local, const QString &remote);
    void removeSyncPair(const QString &name);
    void setSyncPairPaused(const QString &name, bool paused);
    void setSyncPairFilters(const QString &name, const QString &include,
                            const QString &exclude);

signals:
    void statusJson(const QString &json);
    void conflictsJson(const QString &json);
    void syncPairsJson(const QString &json);
    void syncReportsJson(const QString &json);
    void pluginsJson(const QString &json);
    void configJson(const QString &json);
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
    Q_PROPERTY(bool paused READ paused NOTIFY pausedChanged)
    Q_PROPERTY(int bandwidthMbps READ bandwidthMbps NOTIFY bandwidthChanged)
    Q_PROPERTY(QString mountpoint READ mountpoint NOTIFY mountedChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY busyChanged)
    Q_PROPERTY(QString busyWhat READ busyWhat NOTIFY busyChanged)
    Q_PROPERTY(QVariantList conflicts READ conflicts NOTIFY conflictsChanged)
    Q_PROPERTY(QVariantList syncPairs READ syncPairs NOTIFY syncPairsChanged)
    Q_PROPERTY(QString lastSyncAt READ lastSyncAt NOTIFY lastSyncChanged)
    Q_PROPERTY(QVariantList plugins READ plugins NOTIFY pluginsChanged)
    Q_PROPERTY(QVariantList configRows READ configRows NOTIFY configChanged)
    Q_PROPERTY(QVariantList transfers READ transfers NOTIFY transfersChanged)
    Q_PROPERTY(int transferLimit READ transferLimit CONSTANT)
    Q_PROPERTY(int liveTransfers READ liveTransfers NOTIFY transfersChanged)
    Q_PROPERTY(bool startAtLogin READ startAtLogin NOTIFY startAtLoginChanged)
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
    bool paused() const { return m_paused; }
    int bandwidthMbps() const { return m_bandwidthMbps; }
    QString mountpoint() const { return m_mountpoint; }
    bool busy() const { return m_busy; }
    QString busyWhat() const { return m_busyWhat; }
    QVariantList conflicts() const { return m_conflicts; }
    QVariantList syncPairs() const { return m_syncPairs; }
    QString lastSyncAt() const { return m_lastSyncAt; }
    QVariantList plugins() const { return m_plugins; }
    QVariantList configRows() const { return m_configRows; }
    QVariantList transfers() const { return m_transfers; }
    int transferLimit() const { return m_transferLimit; }
    int liveTransfers() const { return m_liveTransfers; }
    bool startAtLogin() const { return m_startAtLogin; }
    QStringList logs() const { return m_logs; }
    QString lastError() const { return m_lastError; }

    Q_INVOKABLE void login(const QString &token);
    Q_INVOKABLE void logout();
    Q_INVOKABLE void mount(const QString &mountpoint, bool readOnly);
    Q_INVOKABLE void unmount();
    Q_INVOKABLE void syncNow();
    Q_INVOKABLE void syncPairNow(const QString &name);
    Q_INVOKABLE QString syncSummary(const QString &name) const;
    Q_INVOKABLE QString syncTotals() const;
    Q_INVOKABLE void clearLogs();
    Q_INVOKABLE void clearTransfers();
    Q_INVOKABLE void copyLogs();
    Q_INVOKABLE void setPaused(bool paused);
    Q_INVOKABLE void setBandwidthLimitMbps(int mbps);
    Q_INVOKABLE void setCacheLimitGb(int gb);
    Q_INVOKABLE void refreshPlugins();
    Q_INVOKABLE void refreshConfig();
    Q_INVOKABLE void setConfigValue(const QString &key, const QString &value);
    Q_INVOKABLE void setStartAtLogin(bool enabled);
    Q_INVOKABLE void openMount();
    Q_INVOKABLE void showMainWindow();
    Q_INVOKABLE void resolve(const QString &id, const QString &resolution);
    Q_INVOKABLE void addSyncPair(const QString &name, const QString &local,
                                 const QString &remote);
    Q_INVOKABLE void removeSyncPair(const QString &name);
    Q_INVOKABLE void setSyncPairPaused(const QString &name, bool paused);
    Q_INVOKABLE void setSyncPairFilters(const QString &name, const QString &include,
                                        const QString &exclude);
    Q_INVOKABLE void clearError();

signals:
    void statusChanged();
    void mountedChanged();
    void pausedChanged();
    void bandwidthChanged();
    void showMainRequested();
    void busyChanged();
    void conflictsChanged();
    void syncPairsChanged();
    void lastSyncChanged();
    void pluginsChanged();
    void configChanged();
    void transfersChanged();
    void logsCleared();
    void logsChanged();
    void startAtLoginChanged();
    void lastErrorChanged();
    void requestLogin(const QString &token);
    void requestLogout();
    void requestMount(const QString &mountpoint, bool readOnly);
    void requestUnmount();
    void requestSync();
    void requestSyncPair(const QString &name);
    void requestSetPaused(bool paused);
    void requestSetBandwidth(qulonglong bytesPerSecond);
    void requestSetCacheLimit(qulonglong bytes);
    void requestRefreshPlugins();
    void requestRefreshConfig();
    void requestSetConfigValue(const QString &key, const QString &value);
    void requestResolve(const QString &id, const QString &resolution);
    void requestAddSyncPair(const QString &name, const QString &local,
                            const QString &remote);
    void requestRemoveSyncPair(const QString &name);
    void requestSetSyncPairPaused(const QString &name, bool paused);
    void requestSetSyncPairFilters(const QString &name, const QString &include,
                                   const QString &exclude);

private slots:
    void onStatus(const QString &json);
    void onConflicts(const QString &json);
    void onSyncPairs(const QString &json);
    void onSyncReports(const QString &json);
    void onPlugins(const QString &json);
    void onConfig(const QString &json);
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
    bool m_paused = false;
    int m_bandwidthMbps = 0;
    QString m_mountpoint;
    bool m_busy = false;
    bool m_startAtLogin = false;
    QString m_busyWhat;
    QVariantList m_conflicts;
    QVariantList m_syncPairs;
    QVariantList m_lastSync;
    QString m_lastSyncAt;
    QVariantList m_plugins;
    QVariantList m_configRows;
    QVariantList m_transfers;
    int m_transferLimit = 512;
    int m_liveTransfers = 0;
    QStringList m_logs;
    QString m_lastError;
};
