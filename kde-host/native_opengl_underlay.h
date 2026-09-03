#pragma once

#include <QQuickItem>

class QTimer;

class NativeOpenGLUnderlay : public QQuickItem
{
    Q_OBJECT
    Q_PROPERTY(qreal time READ time WRITE setTime NOTIFY timeChanged)
    Q_PROPERTY(bool runtimeActive READ runtimeActive NOTIFY runtimeActiveChanged)

public:
    explicit NativeOpenGLUnderlay(QQuickItem *parent = nullptr);

    qreal time() const { return m_time; }
    void setTime(qreal value);

    bool runtimeActive() const { return m_runtimeActive; }

signals:
    void timeChanged();
    void runtimeActiveChanged();

protected:
    QSGNode *updatePaintNode(QSGNode *oldNode,
                             UpdatePaintNodeData *data) override;

private:
    void refreshRuntimeActive();
    bool runtimeMarkerIsLive() const;

    // This value is only a Qt Quick invalidation heartbeat. FrameRenderEngine
    // owns shader time and animation timing internally.
    qreal m_time = 0.0;
    bool m_runtimeActive = false;
    QTimer *m_idleHeartbeat = nullptr;
    QTimer *m_runtimeStateTimer = nullptr;
};
