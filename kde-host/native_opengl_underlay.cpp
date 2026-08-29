#include "native_opengl_underlay.h"
#include "native_opengl_renderer.h"

#include <QCoreApplication>
#include <QDBusConnection>
#include <QDBusMessage>
#include <QDBusPendingCall>
#include <QEvent>
#include <QKeyEvent>
#include <QQuickWindow>
#include <QTimer>

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

    // Plasma 6 PowerDevil deliberately ignores ordinary screen-management
    // inhibitions while the screen locker is active and, by default, registers
    // a 60-second locked-screen DPMS timeout.  Keep KDE's idle clock alive for
    // exactly the lifetime of this lock-screen QML item.  This uses KDE's
    // standard org.freedesktop.ScreenSaver.SimulateUserActivity() entry point;
    // it does not synthesize a keyboard or pointer event, so it does not reveal
    // the authentication UI.
    auto *idleHeartbeat = new QTimer(this);
    idleHeartbeat->setInterval(30000);
    idleHeartbeat->setTimerType(Qt::CoarseTimer);

    connect(idleHeartbeat, &QTimer::timeout, this, [] {
        QDBusMessage message = QDBusMessage::createMethodCall(
            QStringLiteral("org.freedesktop.ScreenSaver"),
            QStringLiteral("/ScreenSaver"),
            QStringLiteral("org.freedesktop.ScreenSaver"),
            QStringLiteral("SimulateUserActivity"));

        QDBusConnection::sessionBus().asyncCall(message);
    });

    idleHeartbeat->start();
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
