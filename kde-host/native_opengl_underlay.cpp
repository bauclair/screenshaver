#include "native_opengl_underlay.h"
#include "native_opengl_renderer.h"

#include <QCoreApplication>
#include <QDBusConnection>
#include <QDBusMessage>
#include <QDBusPendingCall>
#include <QEvent>
#include <QFile>
#include <QFileInfo>
#include <QKeyEvent>
#include <QQuickWindow>
#include <QTimer>

#include <signal.h>
#include <cerrno>

NativeOpenGLUnderlay::NativeOpenGLUnderlay(QQuickItem *parent)
    : QQuickItem(parent)
{
    setFlag(ItemHasContents, true);

    // KScreenLocker's UnlockApp installs an application event filter during
    // greeter initialization. Qt invokes application event filters in reverse
    // installation order, so installing Screenshaver's filter when this QML
    // item is constructed lets us consume Escape KeyRelease before UnlockApp
    // can translate it into a DPMS-off request.
    if (QCoreApplication::instance())
        QCoreApplication::instance()->installEventFilter(this);

    // The overlay is persistent filesystem state, but its behavior is not.
    // A live Screenshaver process writes a PID marker into XDG_RUNTIME_DIR.
    // Poll that marker so a crash, SIGKILL, or other abnormal termination
    // disables the native renderer and auth presentation without waiting for
    // Screenshaver to run again.
    refreshRuntimeActive();

    m_runtimeStateTimer = new QTimer(this);
    m_runtimeStateTimer->setInterval(1000);
    m_runtimeStateTimer->setTimerType(Qt::CoarseTimer);
    connect(m_runtimeStateTimer, &QTimer::timeout,
            this, &NativeOpenGLUnderlay::refreshRuntimeActive);
    m_runtimeStateTimer->start();

    // Plasma 6 PowerDevil deliberately ignores ordinary screen-management
    // inhibitions while the screen locker is active and, by default, registers
    // a 60-second locked-screen DPMS timeout. Keep KDE's idle clock alive only
    // while a live Screenshaver process owns this lock-screen integration.
    m_idleHeartbeat = new QTimer(this);
    m_idleHeartbeat->setInterval(30000);
    m_idleHeartbeat->setTimerType(Qt::CoarseTimer);

    connect(m_idleHeartbeat, &QTimer::timeout, this, [this] {
        if (!m_runtimeActive)
            return;

        QDBusMessage message = QDBusMessage::createMethodCall(
            QStringLiteral("org.freedesktop.ScreenSaver"),
            QStringLiteral("/ScreenSaver"),
            QStringLiteral("org.freedesktop.ScreenSaver"),
            QStringLiteral("SimulateUserActivity"));

        QDBusConnection::sessionBus().asyncCall(message);
    });

    if (m_runtimeActive)
        m_idleHeartbeat->start();
}

bool NativeOpenGLUnderlay::eventFilter(QObject *watched, QEvent *event)
{
    Q_UNUSED(watched);

    if (event && event->type() == QEvent::KeyRelease) {
        auto *keyEvent = static_cast<QKeyEvent *>(event);

        if (keyEvent->key() == Qt::Key_Escape) {
            event->accept();
            return true;
        }
    }

    return false;
}

void NativeOpenGLUnderlay::refreshRuntimeActive()
{
    const bool active = runtimeMarkerIsLive();

    if (active == m_runtimeActive)
        return;

    m_runtimeActive = active;

    if (m_idleHeartbeat) {
        if (m_runtimeActive)
            m_idleHeartbeat->start();
        else
            m_idleHeartbeat->stop();
    }

    emit runtimeActiveChanged();
    update();
}

bool NativeOpenGLUnderlay::runtimeMarkerIsLive() const
{
    const QString runtimeDirectory = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (runtimeDirectory.isEmpty())
        return false;

    QFile marker(runtimeDirectory + QStringLiteral("/screenshaver-kde-lock-active"));
    if (!marker.open(QIODevice::ReadOnly | QIODevice::Text))
        return false;

    bool ok = false;
    const qint64 pid = marker.readLine().trimmed().toLongLong(&ok);
    if (!ok || pid <= 1)
        return false;

    errno = 0;
    if (::kill(static_cast<pid_t>(pid), 0) != 0 && errno != EPERM)
        return false;

    const QFileInfo executable(QStringLiteral("/proc/%1/exe").arg(pid));
    const QString target = executable.symLinkTarget();
    if (target.isEmpty())
        return false;

    return QFileInfo(target).fileName() == QStringLiteral("screenshaver");
}

void NativeOpenGLUnderlay::setTime(qreal value)
{
    if (qFuzzyCompare(m_time, value))
        return;

    m_time = value;
    emit timeChanged();
    update();
}

QSGNode *NativeOpenGLUnderlay::updatePaintNode(QSGNode *oldNode,
                                                UpdatePaintNodeData *)
{
    auto *node = static_cast<NativeOpenGLRenderNode *>(oldNode);

    if (!node)
        node = new NativeOpenGLRenderNode();

    node->setRect(boundingRect());

    if (window()) {
        const qreal dpr = window()->effectiveDevicePixelRatio();
        node->setPixelSize(
            QSize(qMax(1, qRound(window()->width() * dpr)),
                  qMax(1, qRound(window()->height() * dpr))));
    }

    return node;
}
