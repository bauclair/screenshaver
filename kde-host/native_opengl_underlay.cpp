#include "native_opengl_underlay.h"
#include "native_opengl_renderer.h"

#include <QQuickWindow>

NativeOpenGLUnderlay::NativeOpenGLUnderlay(QQuickItem *parent)
    : QQuickItem(parent)
{
    setFlag(ItemHasContents, true);
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
