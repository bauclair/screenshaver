#pragma once

#include <QSGRenderNode>
#include <QByteArray>

struct ScreenshaverKdeGlRenderer;

class NativeOpenGLRenderNode final : public QSGRenderNode
{
public:
    NativeOpenGLRenderNode();
    ~NativeOpenGLRenderNode() override;

    void setRect(const QRectF &rect);
    void setPixelSize(const QSize &size);
    void setPresentationMode(const QString &mode);

    RenderingFlags flags() const override;
    StateFlags changedStates() const override;
    QRectF rect() const override;
    void prepare() override;
    void render(const RenderState *state) override;
    void releaseResources() override;

private:
    bool buildRenderer();
    QString rustError() const;
    void publishWallpaperReadyMarker();
    void clearWallpaperReadyMarker();
    void publishNativeActivationAck(const QByteArray &token);
    void clearNativeActivationAck();

    QRectF m_rect;
    QSize m_pixelSize;
    QString m_presentationMode = QStringLiteral("screensaver");
    ScreenshaverKdeGlRenderer *m_renderer = nullptr;
    bool m_failed = false;
    QString m_readyPath;
    bool m_readyPublished = false;
    QString m_activationAckPath;
    QByteArray m_activationAckToken;
    bool m_pauseRequestObserved = false;
    QByteArray m_heldPauseToken;
    QByteArray m_pauseAckToken;
};
