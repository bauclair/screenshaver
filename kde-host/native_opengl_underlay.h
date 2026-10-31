#pragma once

#include <QQuickItem>

class QTimer;

class NativeOpenGLUnderlay : public QQuickItem
{
    Q_OBJECT
    Q_PROPERTY(qreal time READ time WRITE setTime NOTIFY timeChanged)
    Q_PROPERTY(bool runtimeActive READ runtimeActive NOTIFY runtimeActiveChanged)
    Q_PROPERTY(QString presentationMode READ presentationMode WRITE setPresentationMode NOTIFY presentationModeChanged)

public:
    explicit NativeOpenGLUnderlay(QQuickItem *parent = nullptr);

    qreal time() const { return m_time; }
    void setTime(qreal value);

    bool runtimeActive() const { return m_runtimeActive; }
    QString presentationMode() const { return m_presentationMode; }
    void setPresentationMode(const QString &mode);

signals:
    void timeChanged();
    void runtimeActiveChanged();
    void presentationModeChanged();

protected:
    QSGNode *updatePaintNode(QSGNode *oldNode,
                             UpdatePaintNodeData *data) override;

private:
    void refreshRuntimeActive();
    void simulateUserActivity();
    bool runtimeMarkerIsLive() const;

    // This value is only a Qt Quick invalidation heartbeat. FrameRenderEngine
    // owns shader time and animation timing internally.
    qreal m_time = 0.0;
    bool m_runtimeActive = false;
    QString m_presentationMode = QStringLiteral("screensaver");
    QTimer *m_idleHeartbeat = nullptr;
    QTimer *m_runtimeStateTimer = nullptr;
};
