#include "native_opengl_renderer.h"
#include "screenshaver_rust_gl_bridge.h"

#include <QByteArray>
#include <QDebug>
#include <QOpenGLContext>

namespace {

constexpr uint32_t ExpectedBridgeVersion = 3;

const void *qtGetProcAddress(const char *name)
{
    auto *context = QOpenGLContext::currentContext();
    if (!context || !name)
        return nullptr;

    const QFunctionPointer proc = context->getProcAddress(QByteArray(name));
    return reinterpret_cast<const void *>(proc);
}

} // namespace

NativeOpenGLRenderNode::NativeOpenGLRenderNode() = default;

NativeOpenGLRenderNode::~NativeOpenGLRenderNode()
{
    releaseResources();
}

void NativeOpenGLRenderNode::setRect(const QRectF &rect)
{
    m_rect = rect;
}

void NativeOpenGLRenderNode::setPixelSize(const QSize &size)
{
    m_pixelSize = size;
}

QSGRenderNode::RenderingFlags NativeOpenGLRenderNode::flags() const
{
    return BoundedRectRendering;
}

QSGRenderNode::StateFlags NativeOpenGLRenderNode::changedStates() const
{
    return BlendState
         | ScissorState
         | StencilState
         | DepthState
         | CullState
         | ViewportState;
}

QRectF NativeOpenGLRenderNode::rect() const
{
    return m_rect;
}

QString NativeOpenGLRenderNode::rustError() const
{
    const char *error = screenshaver_kde_gl_last_error();
    return error ? QString::fromUtf8(error) : QStringLiteral("unknown Screenshaver renderer error");
}

bool NativeOpenGLRenderNode::buildRenderer()
{
    if (m_failed)
        return false;

    auto *context = QOpenGLContext::currentContext();
    if (!context) {
        qCritical() << "SCREENSHAVER FRAME ENGINE: no current OpenGL context";
        m_failed = true;
        return false;
    }

    if (m_pixelSize.isEmpty()) {
        qCritical() << "SCREENSHAVER FRAME ENGINE: invalid render size" << m_pixelSize;
        m_failed = true;
        return false;
    }

    qInfo() << "SCREENSHAVER FRAME ENGINE: Qt graphics context"
            << context->format().majorVersion()
            << "."
            << context->format().minorVersion();

    const uint32_t bridgeVersion = screenshaver_kde_gl_bridge_version();
    if (bridgeVersion != ExpectedBridgeVersion) {
        qCritical() << "SCREENSHAVER FRAME ENGINE: incompatible renderer ABI; expected"
                    << ExpectedBridgeVersion << "but library reports" << bridgeVersion;
        m_failed = true;
        return false;
    }

    m_renderer = screenshaver_kde_gl_create(
        &qtGetProcAddress,
        m_pixelSize.width(),
        m_pixelSize.height());

    if (!m_renderer) {
        qCritical().noquote()
            << "SCREENSHAVER FRAME ENGINE: FrameRenderEngine creation failed:"
            << rustError();
        m_failed = true;
        return false;
    }

    qInfo() << "SCREENSHAVER FRAME ENGINE: production FrameRenderEngine created";
    return true;
}

void NativeOpenGLRenderNode::prepare()
{
    if (!m_renderer)
        buildRenderer();
}

void NativeOpenGLRenderNode::render(const RenderState *)
{
    if (!m_renderer && !buildRenderer())
        return;

    if (!QOpenGLContext::currentContext() || m_pixelSize.isEmpty())
        return;

    if (!screenshaver_kde_gl_render(
            m_renderer,
            m_pixelSize.width(),
            m_pixelSize.height())) {
        qCritical().noquote()
            << "SCREENSHAVER FRAME ENGINE: frame render failed:"
            << rustError();
        m_failed = true;
    }
}

void NativeOpenGLRenderNode::releaseResources()
{
    if (m_renderer && QOpenGLContext::currentContext()) {
        screenshaver_kde_gl_destroy(m_renderer);
        m_renderer = nullptr;
    }
}
