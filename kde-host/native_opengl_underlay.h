#pragma once

#include <QQuickItem>

class NativeOpenGLUnderlay : public QQuickItem
{
    Q_OBJECT
    Q_PROPERTY(qreal time READ time WRITE setTime NOTIFY timeChanged)

public:
    explicit NativeOpenGLUnderlay(QQuickItem *parent = nullptr);

    qreal time() const { return m_time; }
    void setTime(qreal value);

signals:
    void timeChanged();

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

    QSGNode *updatePaintNode(QSGNode *oldNode,
                             UpdatePaintNodeData *data) override;

private:
    // This value is only a Qt Quick invalidation heartbeat. FrameRenderEngine
    // owns shader time and animation timing internally.
    qreal m_time = 0.0;
};
