// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

#include "tray.h"

#include "bridge.h"

#include <QAction>
#include <QApplication>
#include <QMenu>
#include <QPainter>
#include <QPixmap>
#include <QQmlComponent>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQuickWindow>
#include <QSystemTrayIcon>

namespace {
constexpr int kIconSize = 22;

QColor healthColour(const QString &health)
{
    if (health == QLatin1String("ok"))
        return QColor(0x3d, 0xc0, 0x7a);
    if (health == QLatin1String("offline"))
        return QColor(0x9a, 0x9a, 0x9a);
    if (health == QLatin1String("error"))
        return QColor(0xe0, 0x52, 0x52);
    return QColor(0xe0, 0xa8, 0x3c); // degraded
}

QString humanBytes(qulonglong bytes)
{
    if (bytes >= 1000000000ull)
        return QString::number(bytes / 1000000000.0, 'f', 1) + QStringLiteral(" GB");
    if (bytes >= 1000000ull)
        return QString::number(bytes / 1000000.0, 'f', 1) + QStringLiteral(" MB");
    if (bytes >= 1000ull)
        return QString::number(bytes / 1000.0, 'f', 1) + QStringLiteral(" kB");
    return QString::number(bytes) + QStringLiteral(" B");
}
} // namespace

TrayController::TrayController(Bridge *bridge, QQmlEngine *engine, QObject *parent)
    : QObject(parent)
    , m_bridge(bridge)
    , m_engine(engine)
{
    if (!QSystemTrayIcon::isSystemTrayAvailable()) {
        m_statusText = QStringLiteral("no system tray on this desktop");
        return;
    }
    m_tray = new QSystemTrayIcon(this);
    m_tray->setToolTip(QStringLiteral("Nidvue Drive"));
    m_tray->setIcon(paintIcon(QStringLiteral("degraded")));
    buildMenu();
    m_tray->setContextMenu(m_menu);
    connect(m_tray, &QSystemTrayIcon::activated, this, &TrayController::onActivated);
    connect(m_bridge, &Bridge::statusChanged, this, &TrayController::refresh);
    m_tray->show();
    refresh();
}

TrayController::~TrayController()
{
    if (m_popup)
        m_popup->close();
    if (m_settings)
        m_settings->close();
}

bool TrayController::available() const
{
    return m_tray != nullptr;
}

QIcon TrayController::paintIcon(const QString &health) const
{
    // Drawn rather than shipped: it scales with the tray and follows the
    // health colour without four bitmap variants.
    QPixmap pixmap(kIconSize, kIconSize);
    pixmap.fill(Qt::transparent);
    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(0x2b, 0x2f, 0x36));
    painter.drawRoundedRect(QRectF(1, 1, kIconSize - 2, kIconSize - 2), 4, 4);

    painter.setPen(QPen(QColor(0xea, 0xef, 0xf7), 2.0, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    // Two arrows: one up, one down, the product's whole idea.
    const qreal mid = kIconSize / 2.0;
    painter.drawLine(QPointF(7, mid - 4), QPointF(7, mid + 3));
    painter.drawLine(QPointF(4.5, mid - 1.5), QPointF(7, mid - 4));
    painter.drawLine(QPointF(9.5, mid - 1.5), QPointF(7, mid - 4));
    painter.drawLine(QPointF(15, mid + 4), QPointF(15, mid - 3));
    painter.drawLine(QPointF(12.5, mid + 1.5), QPointF(15, mid + 4));
    painter.drawLine(QPointF(17.5, mid + 1.5), QPointF(15, mid + 4));

    painter.setPen(Qt::NoPen);
    painter.setBrush(healthColour(health));
    painter.drawEllipse(QPointF(kIconSize - 4.5, kIconSize - 4.5), 3.0, 3.0);
    painter.end();
    return QIcon(pixmap);
}

void TrayController::buildMenu()
{
    m_menu = new QMenu();

    m_statusAction = m_menu->addAction(QStringLiteral("Nidvue Drive"));
    m_statusAction->setEnabled(false);

    m_menu->addSeparator();
    m_syncAction = m_menu->addAction(QStringLiteral("Sync now"));
    connect(m_syncAction, &QAction::triggered, m_bridge, &Bridge::syncNow);

    m_pauseAction = m_menu->addAction(QStringLiteral("Pause sync"));
    m_pauseAction->setCheckable(true);
    connect(m_pauseAction, &QAction::triggered, this, [this](bool checked) {
        m_bridge->setPaused(checked);
    });

    m_mountAction = m_menu->addAction(QStringLiteral("Mount…"));
    connect(m_mountAction, &QAction::triggered, this, [this]() {
        if (m_bridge->mounted()) {
            m_bridge->unmount();
        } else {
            m_bridge->openMount();
        }
    });

    m_menu->addSeparator();
    QAction *popupAction = m_menu->addAction(QStringLiteral("Status and quick settings…"));
    connect(popupAction, &QAction::triggered, this, &TrayController::showPopup);

    QAction *settingsAction = m_menu->addAction(QStringLiteral("Detailed settings…"));
    connect(settingsAction, &QAction::triggered, this, &TrayController::openSettingsWindow);

    QAction *mainAction = m_menu->addAction(QStringLiteral("Show main window"));
    connect(mainAction, &QAction::triggered, this, &TrayController::openMainWindow);

    m_menu->addSeparator();
    QAction *quitAction = m_menu->addAction(QStringLiteral("Quit Nidvue Drive"));
    connect(quitAction, &QAction::triggered, qApp, &QApplication::quit);
}

void TrayController::refresh()
{
    if (!m_tray)
        return;
    const QString health = m_bridge->health();
    m_tray->setIcon(paintIcon(health));

    QString state = health;
    if (m_bridge->paused())
        state = QStringLiteral("paused");
    const QString account = m_bridge->account().isEmpty()
                                ? QStringLiteral("not logged in")
                                : m_bridge->account();
    m_statusText = QStringLiteral("%1 · %2 · %3 cached · %4 conflicts")
                       .arg(account, state, humanBytes(m_bridge->cacheBytes()))
                       .arg(m_bridge->conflictsOpen());
    m_tray->setToolTip(QStringLiteral("Nidvue Drive\n%1").arg(m_statusText));
    m_statusAction->setText(m_statusText);
    m_pauseAction->setChecked(m_bridge->paused());
    m_pauseAction->setText(m_bridge->paused() ? QStringLiteral("Resume sync")
                                              : QStringLiteral("Pause sync"));
    m_mountAction->setText(m_bridge->mounted() ? QStringLiteral("Unmount")
                                               : QStringLiteral("Mount…"));
    m_syncAction->setEnabled(m_bridge->loggedIn() && !m_bridge->busy());

    // A new conflict or a fresh error is the one thing a tray exists to say.
    const qulonglong conflicts = m_bridge->conflictsOpen();
    if (!m_firstRefresh) {
        if (conflicts > m_lastConflicts) {
            m_tray->showMessage(
                QStringLiteral("Nidvue Drive"),
                conflicts == 1
                    ? QStringLiteral("1 file needs a decision — open the Conflict Center")
                    : QStringLiteral("%1 files need a decision — open the Conflict Center")
                          .arg(conflicts),
                QSystemTrayIcon::Information, 8000);
        }
        if (health == QLatin1String("error") && m_lastHealth != health) {
            const QString detail = m_bridge->lastError();
            m_tray->showMessage(
                QStringLiteral("Nidvue Drive"),
                detail.isEmpty() ? QStringLiteral("Sync stopped with an error") : detail,
                QSystemTrayIcon::Critical, 10000);
        }
    }
    m_firstRefresh = false;
    m_lastConflicts = conflicts;
    m_lastHealth = health;

    emit statusTextChanged();
}

void TrayController::onActivated(QSystemTrayIcon::ActivationReason reason)
{
    switch (reason) {
    case QSystemTrayIcon::Trigger:
    case QSystemTrayIcon::DoubleClick:
        togglePopup();
        break;
    default:
        break;
    }
}

void TrayController::togglePopup()
{
    if (m_popup && m_popup->isVisible()) {
        m_popup->hide();
        return;
    }
    showPopup();
}

void TrayController::showPopup()
{
    if (!m_popup) {
        QQmlComponent component(m_engine, QUrl(QStringLiteral("qrc:/qml/Popup.qml")));
        if (component.isError()) {
            qWarning("tray: popup component failed: %s",
                     qPrintable(component.errorString()));
            return;
        }
        QObject *object = component.create();
        m_popup = qobject_cast<QQuickWindow *>(object);
        if (!m_popup) {
            delete object;
            return;
        }
        m_popup->setProperty("trayAvailable", available());
    }
    // Anchor beside the tray icon when the platform reports its geometry.
    const QRect tray = m_tray ? m_tray->geometry() : QRect();
    if (tray.isValid()) {
        m_popup->setPosition(tray.left() - m_popup->width() / 2,
                             tray.top() - m_popup->height() - 8);
    }
    m_popup->show();
    m_popup->raise();
    m_popup->requestActivate();
    qInfo("tray: popup shown");
}

void TrayController::showSettingsWindow()
{
    if (!m_settings) {
        QQmlComponent component(m_engine, QUrl(QStringLiteral("qrc:/qml/SettingsWindow.qml")));
        if (component.isError()) {
            qWarning("tray: settings window failed: %s",
                     qPrintable(component.errorString()));
            return;
        }
        m_settings = qobject_cast<QQuickWindow *>(component.create());
        if (!m_settings)
            return;
    }
    m_settings->show();
    m_settings->raise();
    m_settings->requestActivate();
    qInfo("tray: settings window shown");
}

void TrayController::openSettingsWindow()
{
    emit openSettingsRequested();
    showSettingsWindow();
}

void TrayController::openMainWindow()
{
    emit openMainRequested();
}
