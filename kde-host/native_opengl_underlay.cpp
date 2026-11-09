#include "native_opengl_underlay.h"
#include "native_opengl_renderer.h"

#include <QDBusConnection>
#include <QDBusMessage>
#include <QDBusPendingCall>
#include <QFile>
#include <QFileInfo>
#include <QDir>
#include <QDebug>
#include <QQuickWindow>
#include <QTimer>
#include <QSaveFile>
#include <QFileDevice>
#include <QCoreApplication>
#include <QPointer>
#include <QThread>
#include <QRegularExpression>

#include <signal.h>
#include <cerrno>
#include <unistd.h>

// Implemented in native_opengl_renderer.cpp; shares its render-call fence.
void screenshaverCheckNativeWallpaperRevocation();
bool screenshaverNativeWallpaperGrantAuthorized();

// One revocation observer per loaded Plasma plugin process, not per QQuickItem.
// The timer belongs to the application, so deleting the last wallpaper item
// does not stop checking an outstanding native ownership transaction.
// This must be installed on the GUI thread; it does not authorize rendering.
static void ensureProcessWideWallpaperRevocationWatcher()
{
    static QPointer<QTimer> watcher;
    QCoreApplication *app = QCoreApplication::instance();
    if (!app || QThread::currentThread() != app->thread()) {
        qWarning() << "SCREENSHAVER PLASMA REVOCATION: cannot install GUI-thread watcher";
        return;
    }
    if (watcher) return;

    auto *timer = new QTimer(app);
    timer->setInterval(50);
    timer->setTimerType(Qt::PreciseTimer);
    QObject::connect(timer, &QTimer::timeout, app, []() {
        screenshaverCheckNativeWallpaperRevocation();
    });
    watcher = timer;
    // Observe an existing grant immediately, even if this item is not yet
    // configured as a wallpaper. Future withdrawals then have a known token.
    screenshaverCheckNativeWallpaperRevocation();
    timer->start();
    qInfo() << "SCREENSHAVER PLASMA REVOCATION: process-wide watcher started";
}

// An explicitly configured diagnostic directory isolates native pause testing
// from the production legacy wallpaper transaction. Never fall back to the
// production directory if a diagnostic override is invalid.
QString wallpaperPauseRequestPath()
{
    const QString diagnostic = qEnvironmentVariable("SCREENSHAVER_PLASMA_PAUSE_DIAG_DIR");
    if (diagnostic.isEmpty() || !QDir(diagnostic).exists())
        return {};
    return QDir(diagnostic).filePath(QStringLiteral("screenshaver-wallpaper.pause"));
}

// Read-only diagnostic records are per-item, not a singleton ownership marker.
// The PID/start-ticks pair permits readers to reject stale records after a crash.
// No diagnostic record is an ownership grant, readiness ACK, or resume signal.
static QByteArray processStartTicks()
{
    QFile stat(QStringLiteral("/proc/self/stat"));
    if (!stat.open(QIODevice::ReadOnly)) return {};
    const QByteArray line = stat.readAll();
    const qsizetype close = line.lastIndexOf(')');
    if (close < 0 || close + 2 >= line.size()) return {};
    const QList<QByteArray> fields = line.mid(close + 2).simplified().split(' ');
    // First token after comm is field 3 (state); starttime is field 22.
    if (fields.size() <= 19) return {};
    bool ok = false;
    const qulonglong ticks = fields.at(19).toULongLong(&ok);
    return ok && ticks != 0 ? QByteArray::number(ticks) : QByteArray{};
}

NativeOpenGLUnderlay::NativeOpenGLUnderlay(QQuickItem *parent)
    : QQuickItem(parent)
{
    setFlag(ItemHasContents, true);
    ensureProcessWideWallpaperRevocationWatcher();

    const QString runtimeDir = qEnvironmentVariable("XDG_RUNTIME_DIR");
    const QByteArray ticks = processStartTicks();
    if (!runtimeDir.isEmpty() && !ticks.isEmpty() && QDir(runtimeDir).exists()) {
        const QString instance = QString::number(reinterpret_cast<quintptr>(this), 16);
        m_presentationObservationPath = QDir(runtimeDir).filePath(
            QStringLiteral("screenshaver-plasma-presentation.%1.%2.%3.status")
                .arg(QCoreApplication::applicationPid())
                .arg(QString::fromLatin1(ticks))
                .arg(instance));
    } else {
        qWarning() << "SCREENSHAVER PLASMA STATUS: no safe runtime path or process identity";
    }

    // Lifecycle observations only: none of these events grants ownership,
    // acknowledges a frame, or changes the legacy pause protocol.
    qInfo() << "SCREENSHAVER PLASMA LIFECYCLE: item constructed" << this;
    connect(this, &QQuickItem::windowChanged, this,
            [this](QQuickWindow *associatedWindow) {
                qInfo() << "SCREENSHAVER PLASMA LIFECYCLE: window association"
                        << this << associatedWindow
                        << "mode=" << m_presentationMode;
                // A window association is not scene-graph readiness.
                update();
                publishPresentationObservation();
            });

    // The overlay is persistent filesystem state, but its behavior is not.
    // A live Screenshaver process writes a PID marker into XDG_RUNTIME_DIR.
    // Poll that marker so a crash, SIGKILL, or other abnormal termination
    // disables the native renderer without waiting for
    // Screenshaver to run again.
    refreshRuntimeActive();

    m_runtimeStateTimer = new QTimer(this);
    m_runtimeStateTimer->setInterval(1000);
    m_runtimeStateTimer->setTimerType(Qt::CoarseTimer);
    connect(m_runtimeStateTimer, &QTimer::timeout,
            this, &NativeOpenGLUnderlay::refreshRuntimeActive);
    m_runtimeStateTimer->start();

    // Independent GUI-thread wakeup for native wallpaper pause transitions.
    // The render thread still reads the control file itself before every
    // frame; this timer does not authorize rendering or publish an ACK.
    m_wallpaperPauseTimer = new QTimer(this);
    m_wallpaperPauseTimer->setInterval(50);
    m_wallpaperPauseTimer->setTimerType(Qt::PreciseTimer);
    connect(m_wallpaperPauseTimer, &QTimer::timeout,
            this, &NativeOpenGLUnderlay::refreshWallpaperPauseRequest);
    m_wallpaperPauseTimer->start();
    refreshWallpaperPauseRequest();
    refreshWallpaperGrantAuthorization();
    publishPresentationObservation();

    // Plasma 6 PowerDevil deliberately ignores ordinary screen-management
    // inhibitions while the screen locker is active and, by default, registers
    // a 60-second locked-screen DPMS timeout. Wake the lock presentation once
    // immediately when Screenshaver is active, then keep KDE's idle clock alive
    // with the existing 30-second heartbeat.
    m_idleHeartbeat = new QTimer(this);
    m_idleHeartbeat->setInterval(30000);
    m_idleHeartbeat->setTimerType(Qt::CoarseTimer);

    connect(m_idleHeartbeat, &QTimer::timeout,
            this, &NativeOpenGLUnderlay::simulateUserActivity);

    if (m_runtimeActive) {
        simulateUserActivity();
        m_idleHeartbeat->start();
    }
}

NativeOpenGLUnderlay::~NativeOpenGLUnderlay()
{
    // Destruction is not a native revocation ACK. Only the render-call fence
    // and verified transaction protocol may authorize legacy resumption.
    retirePresentationObservation();
    qInfo() << "SCREENSHAVER PLASMA LIFECYCLE: item destroyed" << this
            << "mode=" << m_presentationMode;
}

void NativeOpenGLUnderlay::refreshRuntimeActive()
{
    const bool active = runtimeMarkerIsLive();

    if (active == m_runtimeActive)
        return;

    m_runtimeActive = active;

    if (m_idleHeartbeat) {
        if (m_runtimeActive) {
            simulateUserActivity();
            m_idleHeartbeat->start();
        } else {
            m_idleHeartbeat->stop();
        }
    }

    publishPresentationObservation();
    emit runtimeActiveChanged();
    update();
}

void NativeOpenGLUnderlay::refreshWallpaperPauseRequest()
{
    if (m_presentationMode != QStringLiteral("wallpaper"))
        return;

    refreshWallpaperGrantAuthorization();

    publishPresentationObservation();

    // Revocation is checked by the application-owned watcher independently
    // of this item, including after this item is destroyed.

    // GUI-thread wakeup only, now unconditional for wallpaper items. The
    // render thread independently validates the full grant before Rust GL.
    {
        const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
        QByteArray grantIdentity("<unavailable>");
        if (!runtime.isEmpty()) {
            QFile grant(QDir(runtime).filePath(
                QStringLiteral("screenshaver-plasma-ownership.grant")));
            if (!grant.exists()) {
                grantIdentity = "<absent>";
            } else if (grant.open(QIODevice::ReadOnly)) {
                grantIdentity = grant.read(256);
                if (grant.error() != QFile::NoError || !grant.atEnd())
                    grantIdentity = "<invalid>";
            }
        }
        if (!m_ownershipGrantInitialized || grantIdentity != m_ownershipGrantIdentity) {
            m_ownershipGrantInitialized = true;
            m_ownershipGrantIdentity = grantIdentity;
            update();
        }
    }

    const QString requestPath = wallpaperPauseRequestPath();
    bool pauseRequested = true;
    QByteArray identity("<unavailable>");
    if (!requestPath.isEmpty() && QFileInfo(requestPath).dir().exists()) {
        QFile request(requestPath);
        if (!request.exists()) {
            pauseRequested = false;
            identity = "<absent>";
        } else if (request.open(QIODevice::ReadOnly | QIODevice::Text)) {
            identity = request.readAll().trimmed();
            if (request.error() != QFile::NoError)
                identity = "<unreadable>";
        } else {
            identity = "<unreadable>";
        }
    }

    if (!m_wallpaperPauseInitialized || pauseRequested != m_wallpaperPauseRequested ||
        identity != m_wallpaperPauseIdentity) {
        m_wallpaperPauseInitialized = true;
        m_wallpaperPauseRequested = pauseRequested;
        m_wallpaperPauseIdentity = identity;
        qInfo() << "SCREENSHAVER PLASMA PAUSE WATCHER:"
                << (pauseRequested ? "pause/token change observed; requesting render-thread check"
                                   : "request absent; requesting render-thread check");
        update();
    }
}

void NativeOpenGLUnderlay::setOwnershipMonitorOnly(bool value)
{
    if (m_ownershipMonitorOnly == value) return;
    m_ownershipMonitorOnly = value;
    // A monitor-only item must never allocate a QSGRenderNode. QML sets this
    // immediately on construction, before its first scene-graph sync.
    setFlag(ItemHasContents, !value);
    emit ownershipMonitorOnlyChanged();
    publishPresentationObservation();
    update();
}

void NativeOpenGLUnderlay::refreshWallpaperGrantAuthorization()
{
    const bool authorized = m_presentationMode == QStringLiteral("wallpaper")
        && screenshaverNativeWallpaperGrantAuthorized();
    if (authorized == m_wallpaperGrantAuthorized) return;
    m_wallpaperGrantAuthorized = authorized;
    emit wallpaperGrantAuthorizedChanged();
}

void NativeOpenGLUnderlay::simulateUserActivity()
{
    if (!m_runtimeActive)
        return;

    QDBusMessage message = QDBusMessage::createMethodCall(
        QStringLiteral("org.freedesktop.ScreenSaver"),
        QStringLiteral("/ScreenSaver"),
        QStringLiteral("org.freedesktop.ScreenSaver"),
        QStringLiteral("SimulateUserActivity"));

    QDBusConnection::sessionBus().asyncCall(message);
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

void NativeOpenGLUnderlay::setPresentationMode(const QString &mode)
{
    if (mode != QStringLiteral("screensaver") && mode != QStringLiteral("wallpaper")) {
        qWarning("SCREENSHAVER: unsupported presentation mode");
        return;
    }
    if (mode == m_presentationMode)
        return;
    m_presentationMode = mode;
    m_wallpaperPauseInitialized = false;
    m_ownershipGrantInitialized = false;
    refreshWallpaperPauseRequest();
    refreshWallpaperGrantAuthorization();
    publishPresentationObservation();
    emit presentationModeChanged();
    update();
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
    if (m_ownershipMonitorOnly) return nullptr;
    auto *node = static_cast<NativeOpenGLRenderNode *>(oldNode);

    if (!node) {
        node = new NativeOpenGLRenderNode();
        m_renderNodeObserved.store(true, std::memory_order_release);
        // Called by Qt Quick's scene-graph update path, potentially on the
        // render thread. This only proves node construction, not GL success
        // or compositor presentation. No QObject state is modified here.
        qInfo() << "SCREENSHAVER PLASMA LIFECYCLE: render node constructed"
                << "mode=" << m_presentationMode;
    }

    node->setRect(boundingRect());
    node->setPresentationMode(m_presentationMode);

    if (window()) {
        const qreal dpr = window()->effectiveDevicePixelRatio();
        node->setPixelSize(
            QSize(qMax(1, qRound(width() * dpr)),
                  qMax(1, qRound(height() * dpr))));
    }

    return node;
}

void NativeOpenGLUnderlay::publishPresentationObservation()
{
    // GUI-thread only. Atomic node flag is the sole cross-thread observation.
    // A status file is not a successful GL frame or compositor-present fence.
    if (m_presentationObservationPath.isEmpty()) return;
    const QByteArray ticks = processStartTicks();
    if (ticks.isEmpty()) return;
    const QByteArray payload =
        "v1 " + QByteArray::number(QCoreApplication::applicationPid()) + " " + ticks +
        " " + m_presentationMode.toLatin1() +
        " window=" + (window() ? QByteArray("associated") : QByteArray("absent")) +
        " monitor_only=" + (m_ownershipMonitorOnly ? QByteArray("1") : QByteArray("0")) +
        " node_ever_constructed=" +
        (m_renderNodeObserved.load(std::memory_order_acquire) ? QByteArray("1") : QByteArray("0")) +
        "\n";
    if (payload == m_lastPresentationObservation) return;
    QSaveFile file(m_presentationObservationPath);
    if (!file.open(QIODevice::WriteOnly) || file.write(payload) != payload.size() || !file.commit()) {
        qWarning() << "SCREENSHAVER PLASMA STATUS: cannot publish diagnostic record";
        return;
    }
    m_lastPresentationObservation = payload;
}

void NativeOpenGLUnderlay::retirePresentationObservation()
{
    if (m_presentationObservationPath.isEmpty()) return;
    // Per-instance filename prevents deleting another live item's status.
    // A crash can leave this file; readers must verify PID and start ticks.
    if (!QFile::remove(m_presentationObservationPath))
        qWarning() << "SCREENSHAVER PLASMA STATUS: could not retire diagnostic record";
}
