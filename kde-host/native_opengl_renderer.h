#pragma once

#include <QSGRenderNode>

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

    QRectF m_rect;
    QSize m_pixelSize;
    QString m_presentationMode = QStringLiteral("screensaver");
    ScreenshaverKdeGlRenderer *m_renderer = nullptr;
    bool m_failed = false;
};
