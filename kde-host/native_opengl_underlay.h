#pragma once

#include <QQuickItem>
#include <QByteArray>
#include <atomic>

class QTimer;

class NativeOpenGLUnderlay : public QQuickItem
{
    Q_OBJECT
    Q_PROPERTY(qreal time READ time WRITE setTime NOTIFY timeChanged)
    Q_PROPERTY(bool runtimeActive READ runtimeActive NOTIFY runtimeActiveChanged)
    Q_PROPERTY(QString presentationMode READ presentationMode WRITE setPresentationMode NOTIFY presentationModeChanged)
    Q_PROPERTY(bool ownershipMonitorOnly READ ownershipMonitorOnly WRITE setOwnershipMonitorOnly NOTIFY ownershipMonitorOnlyChanged)
    Q_PROPERTY(bool wallpaperGrantAuthorized READ wallpaperGrantAuthorized NOTIFY wallpaperGrantAuthorizedChanged)

public:
    explicit NativeOpenGLUnderlay(QQuickItem *parent = nullptr);
    ~NativeOpenGLUnderlay() override;

    qreal time() const { return m_time; }
    void setTime(qreal value);

    bool runtimeActive() const { return m_runtimeActive; }
    QString presentationMode() const { return m_presentationMode; }
    void setPresentationMode(const QString &mode);
    bool ownershipMonitorOnly() const { return m_ownershipMonitorOnly; }
    void setOwnershipMonitorOnly(bool value);
    bool wallpaperGrantAuthorized() const { return m_wallpaperGrantAuthorized; }

signals:
    void timeChanged();
    void runtimeActiveChanged();
    void presentationModeChanged();
    void ownershipMonitorOnlyChanged();
    void wallpaperGrantAuthorizedChanged();

protected:
    QSGNode *updatePaintNode(QSGNode *oldNode,
                             UpdatePaintNodeData *data) override;

private:
    void refreshRuntimeActive();
    void refreshWallpaperPauseRequest();
    void refreshWallpaperGrantAuthorization();
    void simulateUserActivity();
    void publishPresentationObservation();
    void retirePresentationObservation();
    bool runtimeMarkerIsLive() const;

    // This value is only a Qt Quick invalidation heartbeat. FrameRenderEngine
    // owns shader time and animation timing internally.
    qreal m_time = 0.0;
    bool m_runtimeActive = false;
    bool m_ownershipMonitorOnly = false;
    bool m_wallpaperGrantAuthorized = false;
    QString m_presentationMode = QStringLiteral("screensaver");
    QTimer *m_idleHeartbeat = nullptr;
    QTimer *m_runtimeStateTimer = nullptr;
    QTimer *m_wallpaperPauseTimer = nullptr;
    bool m_wallpaperPauseInitialized = false;
    bool m_wallpaperPauseRequested = true;
    QByteArray m_wallpaperPauseIdentity;
    bool m_ownershipGrantInitialized = false;
    QByteArray m_ownershipGrantIdentity;
    // Render-thread observation; GUI thread owns filesystem publication.
    std::atomic_bool m_renderNodeObserved{false};
    QString m_presentationObservationPath;
    QByteArray m_lastPresentationObservation;
};
