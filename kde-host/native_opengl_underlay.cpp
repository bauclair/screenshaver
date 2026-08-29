#include "native_opengl_underlay.h"
#include "native_opengl_renderer.h"

#include <QCoreApplication>
#include <QEvent>
#include <QKeyEvent>
#include <QQuickWindow>

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
