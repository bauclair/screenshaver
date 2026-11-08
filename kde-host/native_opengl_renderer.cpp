#include "native_opengl_renderer.h"
#include "screenshaver_rust_gl_bridge.h"

#include <QByteArray>
#include <QDebug>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSaveFile>
#include <QTextStream>
#include <QCoreApplication>
#include <QOpenGLContext>
#include <QOpenGLExtraFunctions>
#include <QMutex>
#include <QMutexLocker>
#include <QSet>
#include <QHash>
#include <QMap>

namespace {

constexpr uint32_t ExpectedBridgeVersion = 4;

// All wallpaper nodes in this Plasma process share this serialization fence.
// The GUI watcher can acknowledge revocation only after any in-flight Rust GL
// submission has returned. This is intentionally not a compositor-present ACK.
QMutex nativeOwnershipMutex;
// Guarded by nativeOwnershipMutex. Tokens are single-use within this Plasma
// process. A fresh Plasma process requires a fresh coordinator transaction.
QByteArray lastAuthorizedToken;
QSet<QByteArray> retiredOwnershipTokens;
// Diagnostic bookkeeping only, protected by nativeOwnershipMutex.
// Recording a token here never authorizes a frame or a legacy resume.
QHash<const NativeOpenGLRenderNode *, QByteArray> nodeObservedTokens;
QByteArray plasmaProcessStartTicks();
// Diagnostic ledger: one entry per node that completed an authorized frame.
// An entry is never removed during this Plasma process; retired entries remain
// visible to prevent a destroyed node from silently disappearing from accounting.
// This does NOT enumerate unobserved Qt nodes or certify compositor quiescence.
struct NodeLedgerEntry { QByteArray token; bool retired = false; };
QMap<quint64, NodeLedgerEntry> nodeLedger;
// Lifetime IDs never depend on allocator address reuse. Assigned once when
// each node is constructed; the pointer map is only an in-process lookup.
QHash<const NativeOpenGLRenderNode *, quint64> nodeIdentity;
quint64 nextNodeIdentity = 1;
// A closed census is NOT proof that Qt or KWin retired a presentation.
// Late wallpaper nodes permanently invalidate that transaction's snapshot.
QSet<QByteArray> closedNodeCensuses;
QSet<QByteArray> invalidNodeCensuses;
// Increment 09E: enumerate live Qt render nodes, including nodes that have
// never submitted an authorized GL frame. Protected by nativeOwnershipMutex.
// This is a process-local census, not a compositor or display inventory.
QSet<quint64> liveWallpaperNodes;

quint64 identityForNodeLocked(const NativeOpenGLRenderNode *node)
{
    return nodeIdentity.value(node, 0);
}

void publishNodeSnapshotLocked(const QByteArray &token);

void publishCensusStateLocked(const QByteArray &token)
{
    if (token.isEmpty()) return;
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    const QByteArray ticks = plasmaProcessStartTicks();
    if (runtime.isEmpty() || ticks.isEmpty()) return;
    const QByteArray pid = QByteArray::number(QCoreApplication::applicationPid());
    const QString path = QDir(runtime).filePath(
        QStringLiteral("screenshaver-plasma-node-census.%1.%2.%3.observed")
            .arg(QString::fromLatin1(pid), QString::fromLatin1(ticks),
                 QString::fromLatin1(token)));
    const QByteArray state = invalidNodeCensuses.contains(token) ? "invalid" :
        (closedNodeCensuses.contains(token) ? "closed" : "open");
    const QByteArray payload = "v1 census " + token + " " + pid + " " + ticks +
        " " + state + "\n";
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly) ||
        file.write(payload) != payload.size() || !file.commit())
        qWarning() << "SCREENSHAVER PLASMA RETIREMENT: census status write failed";
    publishNodeSnapshotLocked(token);
}

void invalidateClosedCensusForLateNodeLocked()
{
    for (const QByteArray &token : closedNodeCensuses) {
        if (invalidNodeCensuses.contains(token)) continue;
        invalidNodeCensuses.insert(token);
        publishCensusStateLocked(token);
        qWarning() << "SCREENSHAVER PLASMA RETIREMENT: late node invalidated closed census";
    }
}

void publishNodeLedgerLocked(const QByteArray &token);

void closeNodeCensusLocked(const QByteArray &token)
{
    if (token.isEmpty() || closedNodeCensuses.contains(token)) return;
    closedNodeCensuses.insert(token);
    publishNodeLedgerLocked(token);
    publishCensusStateLocked(token);
    publishNodeSnapshotLocked(token);
}


void publishNodeLedgerLocked(const QByteArray &token);

void enrollLiveNodesLocked(const QByteArray &token)
{
    if (token.isEmpty()) return;
    bool changed = false;
    for (quint64 id : liveWallpaperNodes) {
        if (id == 0) continue;
        auto it = nodeLedger.find(id);
        if (it == nodeLedger.end()) {
            nodeLedger.insert(id, NodeLedgerEntry{token, false});
            changed = true;
        } else if (it.value().token != token) {
            // A node retained across distinct transactions cannot silently
            // overwrite its earlier transaction entry.
            qWarning() << "SCREENSHAVER PLASMA RETIREMENT: cross-transaction node enrollment rejected";
        }
    }
    if (changed) publishNodeLedgerLocked(token);
    publishCensusStateLocked(token);
}


void publishNodeLedgerLocked(const QByteArray &token)
{
    if (token.isEmpty()) return;
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    const QByteArray ticks = plasmaProcessStartTicks();
    if (runtime.isEmpty() || ticks.isEmpty()) return;
    const QByteArray pid = QByteArray::number(QCoreApplication::applicationPid());
    const QString path = QDir(runtime).filePath(
        QStringLiteral("screenshaver-plasma-node-ledger.%1.%2.%3.observed")
            .arg(QString::fromLatin1(pid), QString::fromLatin1(ticks),
                 QString::fromLatin1(token)));
    QByteArray payload = "v1 nodes " + token + " " + pid + " " + ticks + "\n";
    for (auto it = nodeLedger.cbegin(); it != nodeLedger.cend(); ++it) {
        if (it.value().token != token) continue;
        payload += QByteArray::number(static_cast<qulonglong>(it.key())) +
            (it.value().retired ? " retired\n" : " active\n");
    }
    // Status is diagnostic and never grants ownership. Atomic replacement
    // prevents a reader from seeing a partially written ledger.
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly) ||
        file.write(payload) != payload.size() || !file.commit())
        qWarning() << "SCREENSHAVER PLASMA RETIREMENT: node ledger write failed";
    publishNodeSnapshotLocked(token);
}


// Increment 09H: one QSaveFile publication contains both the census state
// and the complete node ledger at the same mutex-protected instant.
// Diagnostic only. This is not proof of compositor quiescence.
void publishNodeSnapshotLocked(const QByteArray &token)
{
    if (token.isEmpty()) return;
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    const QByteArray ticks = plasmaProcessStartTicks();
    if (runtime.isEmpty() || ticks.isEmpty()) return;
    const QByteArray pid = QByteArray::number(QCoreApplication::applicationPid());
    const QString path = QDir(runtime).filePath(
        QStringLiteral("screenshaver-plasma-node-snapshot.%1.%2.%3.observed")
            .arg(QString::fromLatin1(pid), QString::fromLatin1(ticks),
                 QString::fromLatin1(token)));
    const QByteArray state = invalidNodeCensuses.contains(token) ? "invalid" :
        (closedNodeCensuses.contains(token) ? "closed" : "open");
    QByteArray payload = "v1 snapshot " + token + " " + pid + " " + ticks +
        " " + state + "\n";
    for (auto it = nodeLedger.cbegin(); it != nodeLedger.cend(); ++it) {
        if (it.value().token != token) continue;
        payload += QByteArray::number(static_cast<qulonglong>(it.key())) +
            (it.value().retired ? " retired\n" : " active\n");
    }
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly) ||
        file.write(payload) != payload.size() || !file.commit())
        qWarning() << "SCREENSHAVER PLASMA RETIREMENT: node snapshot write failed";
}

void publishNodeRetirementObservation(const NativeOpenGLRenderNode *node,
                                      const QByteArray &token)
{
    if (!node || token.isEmpty()) return;
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    const QByteArray ticks = plasmaProcessStartTicks();
    if (runtime.isEmpty() || ticks.isEmpty()) return;
    // The protocol's node identity is a nonzero canonical decimal integer.
    const quint64 identity = identityForNodeLocked(node);
    if (identity == 0) return;
    const QString path = QDir(runtime).filePath(
        QStringLiteral("screenshaver-plasma-scene-retirement.observed"));
    const QByteArray payload = "v1 scene-retired " + token + " " +
        QByteArray::number(QCoreApplication::applicationPid()) + " " + ticks + " " +
        QByteArray::number(static_cast<qulonglong>(identity)) + "\n";
    // Diagnostic only: do not replace another node's observation.
    // The fixed-path contract currently supports only one observation;
    // multi-monitor aggregation must be designed before production use.
    if (QFileInfo::exists(path)) return;
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly) ||
        file.write(payload) != payload.size() || !file.commit()) {
        qWarning() << "SCREENSHAVER PLASMA RETIREMENT: diagnostic write failed";
    }
}


bool acceptOwnershipTokenLocked(const QByteArray &token)
{
    if (token.isEmpty() || retiredOwnershipTokens.contains(token))
        return false;
    if (!lastAuthorizedToken.isEmpty() && lastAuthorizedToken != token) {
        retiredOwnershipTokens.insert(lastAuthorizedToken);
        qInfo() << "SCREENSHAVER PLASMA OWNERSHIP: superseded transaction retired";
    }
    if (lastAuthorizedToken != token) {
        lastAuthorizedToken = token;
        enrollLiveNodesLocked(token);
        qInfo() << "SCREENSHAVER PLASMA OWNERSHIP: transaction observed" << token;
    }
    return true;
}



// Phase 10C-3B-3B-7H: wallpaper authorization is mandatory regardless of
// Plasma's startup environment. A grant is still issued only by the Rust
// coordinator; this native gate does not itself initiate a handoff.
// Screensaver presentation remains independent of this gate.
QString nativeOwnershipGrantPath()
{
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (runtime.isEmpty())
        return {};
    return QDir(runtime).filePath(QStringLiteral("screenshaver-plasma-ownership.grant"));
}

// Defined after plasmaProcessStartTicks(), below.
QByteArray nativeWallpaperGrantToken();
bool nativeWallpaperGrantValid();

// Isolated, opt-in diagnostic. Never create production ownership evidence.
// The marker is read at each frame, so deleting it immediately disables testing.
// Do not enable while Screenshaver's external wallpaper renderer is running.
QString nativeWallpaperTestMode()
{
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (runtime.isEmpty()) return {};
    const QString path = QDir(runtime).filePath(
        QStringLiteral("screenshaver-plasma-native-render-test.enable"));
    const QFileInfo info(path);
    if (!info.isFile() || info.isSymLink() || info.size() > 32) return {};
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) return {};
    const QByteArray mode = file.readAll().trimmed();
    if (mode == "color") return QStringLiteral("color");
    if (mode == "rust") return QStringLiteral("rust");
    return {};
}

bool nativeWallpaperDiagnostic(const QString &mode)
{
    return mode == QStringLiteral("wallpaper") && !nativeWallpaperTestMode().isEmpty();
}

bool nativeWallpaperOwnershipDenied(const QString &mode)
{
    return mode == QStringLiteral("wallpaper")
        && !nativeWallpaperDiagnostic(mode) && !nativeWallpaperGrantValid();
}

// Linux /proc/<pid>/stat field 22: process start time in clock ticks.
// Paired with PID, this distinguishes successive Plasma process instances.
QByteArray plasmaProcessStartTicks()
{
    QFile statFile(QStringLiteral("/proc/self/stat"));
    if (!statFile.open(QIODevice::ReadOnly))
        return {};
    const QByteArray stat = statFile.readAll();
    const qsizetype closingParen = stat.lastIndexOf(')');
    if (closingParen < 0)
        return {};
    const QList<QByteArray> fields = stat.mid(closingParen + 1).simplified().split(' ');
    // Fields after comm begin at field 3 (state); starttime is field 22.
    if (fields.size() <= 19)
        return {};
    bool valid = false;
    fields.at(19).toULongLong(&valid);
    return valid ? fields.at(19) : QByteArray{};
}

// A single ASCII line: "v1 <hex-token> <pid> <start-ticks>".
// Re-read at each render-thread check: removing or replacing the grant
// denies the NEXT frame. No filesystem marker is accepted as proof that
// the legacy renderer has paused; that proof belongs to the Rust coordinator.
QByteArray nativeWallpaperGrantToken()
{
    const QString path = nativeOwnershipGrantPath();
    if (path.isEmpty())
        return {};

    // Match the Rust coordinator's fail-closed regular-file requirement.
    // QFile::open() alone follows a substituted symbolic link. This check is
    // defensive, not an atomic filesystem transaction with the coordinator.
    const QFileInfo info(path);
    if (!info.isFile() || info.isSymLink())
        return {};

    QFile file(path);
    if (!file.open(QIODevice::ReadOnly))
        return {};
    const QByteArray contents = file.read(257);
    if (file.error() != QFile::NoError || !file.atEnd() ||
        contents.isEmpty() || contents.size() > 256 || !contents.endsWith('\n'))
        return {};
    // The wire record is exactly one ASCII line with no embedded control
    // characters. Do not normalize whitespace or accept extra lines.
    for (const char c : contents) {
        if (static_cast<unsigned char>(c) > 0x7f || c == '\r' || c == '\0')
            return {};
    }
    const QByteArray line = contents.left(contents.size() - 1);
    if (line.contains('\n') || line.contains('\t'))
        return {};
    const QList<QByteArray> parts = line.split(' ');
    if (parts.size() != 4 || parts.at(0) != "v1")
        return {};
    const QByteArray &token = parts.at(1);
    if (token.size() != 32)
        return {};
    for (const char c : token) {
        if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')))
            return {};
    }
    const QByteArray expectedPid = QByteArray::number(QCoreApplication::applicationPid());
    const QByteArray startTicks = plasmaProcessStartTicks();
    // Comparing canonical decimal text also rejects leading zeros and signs.
    return (!startTicks.isEmpty()
        && parts.at(2) == expectedPid
        && parts.at(3) == startTicks) ? token : QByteArray{};
}

// Called only while nativeOwnershipMutex is held when the gate is enabled.
// The initial prepare() call is also serialized below.
bool nativeWallpaperGrantValid()
{
    const QByteArray token = nativeWallpaperGrantToken();
    return !token.isEmpty() && !retiredOwnershipTokens.contains(token);
}

QString nativeOwnershipRevokedPath()
{
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    return runtime.isEmpty() ? QString{} :
        QDir(runtime).filePath(QStringLiteral("screenshaver-plasma-ownership.revoked"));
}

bool publishNativeRevocation(const QByteArray &token)
{
    const QString path = nativeOwnershipRevokedPath();
    const QByteArray ticks = plasmaProcessStartTicks();
    if (path.isEmpty() || ticks.isEmpty() || token.isEmpty())
        return false;
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text))
        return false;
    const QByteArray payload = "v1 " + token + " " +
        QByteArray::number(QCoreApplication::applicationPid()) + " " + ticks + "\n";
    if (file.write(payload) != payload.size()) {
        file.cancelWriting();
        return false;
    }
    return file.commit();
}

// Native pause testing is strictly opt-in and confined to a dedicated
// diagnostic directory. The production legacy pause marker must NEVER be
// consumed by the Plasma renderer: it belongs exclusively to the legacy
// Wayland wallpaper transaction.
QString wallpaperPauseRequestPath()
{
    const QString diagnostic = qEnvironmentVariable("SCREENSHAVER_PLASMA_PAUSE_DIAG_DIR");
    if (diagnostic.isEmpty() || !QDir(diagnostic).exists())
        return {};
    return QDir(diagnostic).filePath(QStringLiteral("screenshaver-wallpaper.pause"));
}

// ACK publication is intentionally restricted to the isolated diagnostic
// directory. Production wallpaper ownership has not been handed over.
QString diagnosticControlPath(const QString &name)
{
    const QString dir = qEnvironmentVariable("SCREENSHAVER_PLASMA_PAUSE_DIAG_DIR");
    if (dir.isEmpty() || !QDir(dir).exists())
        return {};
    return QDir(dir).filePath(name);
}

// Missing is different from unreadable. An unreadable request fails closed.
bool readControlToken(const QString &path, QByteArray *token, bool *present)
{
    *present = false;
    token->clear();
    if (path.isEmpty())
        return false;
    QFile file(path);
    if (!file.exists())
        return true;
    *present = true;
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text))
        return false;
    *token = file.readAll().trimmed();
    return file.error() == QFile::NoError && !token->isEmpty();
}

bool publishDiagnosticAck(const QString &name, const QByteArray &token)
{
    if (token.isEmpty() || token.contains('\n') || token.contains('\r'))
        return false;
    const QString path = diagnosticControlPath(name);
    if (path.isEmpty())
        return false;
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text))
        return false;
    const QByteArray data = token + '\n';
    if (file.write(data) != data.size()) {
        file.cancelWriting();
        return false;
    }
    return file.commit();
}

const void *qtGetProcAddress(const char *name)
{
    auto *context = QOpenGLContext::currentContext();
    if (!context || !name)
        return nullptr;

    const QFunctionPointer proc = context->getProcAddress(QByteArray(name));
    return reinterpret_cast<const void *>(proc);
}

} // namespace

// Called by the existing GUI-thread 50ms wakeup. Holding the process-wide
// mutex guarantees all earlier GL submissions in this process have returned.
// The render thread independently validates authorization for each new frame.
void screenshaverCheckNativeWallpaperRevocation()
{
    QMutexLocker lock(&nativeOwnershipMutex);
    const QByteArray current = nativeWallpaperGrantToken();
    const QString ackPath = nativeOwnershipRevokedPath();
    if (!current.isEmpty()) {
        if (acceptOwnershipTokenLocked(current)) {
            if (!ackPath.isEmpty()) QFile::remove(ackPath);
        } else {
            // A reused transaction is never reauthorized in this process.
            if (!ackPath.isEmpty()) QFile::remove(ackPath);
            qWarning() << "SCREENSHAVER PLASMA OWNERSHIP: retired token rejected";
        }
        return;
    }
    if (lastAuthorizedToken.isEmpty())
        return;

    const QByteArray withdrawn = lastAuthorizedToken;
    // No native render call is in flight while this mutex is held. The
    // coordinator must still validate grant absence after reading this ACK.
    if (publishNativeRevocation(withdrawn)) {
        // A grant can change independently of our mutex. Recheck immediately
        // and withdraw our ACK if any grant is present. This cannot replace
        // the coordinator's final post-ACK absence verification.
        if (!nativeWallpaperGrantToken().isEmpty()) {
            if (!ackPath.isEmpty()) QFile::remove(ackPath);
            qWarning() << "SCREENSHAVER PLASMA OWNERSHIP: grant changed during revocation";
            return;
        }
        closeNodeCensusLocked(withdrawn);
        retiredOwnershipTokens.insert(withdrawn);
        lastAuthorizedToken.clear();
        qInfo() << "SCREENSHAVER PLASMA OWNERSHIP: native revocation acknowledged" << withdrawn;
    }
}

// Read-only authorization probe for the QML loader controller. The same
// process-wide mutex and strict grant parser used by the render path apply.
// This does not accept a token, create a node, or acknowledge ownership.
bool screenshaverNativeWallpaperGrantAuthorized()
{
    QMutexLocker lock(&nativeOwnershipMutex);
    return nativeWallpaperGrantValid();
}

NativeOpenGLRenderNode::NativeOpenGLRenderNode()
{
    QMutexLocker ownershipLock(&nativeOwnershipMutex);
    // Zero is reserved as invalid; never recycle an ID in this process.
    if (nextNodeIdentity == 0) {
        qCritical() << "SCREENSHAVER PLASMA RETIREMENT: node identity exhausted";
        invalidateClosedCensusForLateNodeLocked();
        return;
    }
    nodeIdentity.insert(this, nextNodeIdentity++);
}

NativeOpenGLRenderNode::~NativeOpenGLRenderNode()
{
    releaseResources();
    clearWallpaperReadyMarker();
    clearNativeActivationAck();
    // Node destruction is a Qt scene-graph observation, NOT compositor
    // retirement. Publish only after the node's own cleanup has returned.
    QMutexLocker ownershipLock(&nativeOwnershipMutex);
    const QByteArray token = nodeObservedTokens.take(this);
    const quint64 nodeId = identityForNodeLocked(this);
    liveWallpaperNodes.remove(nodeId);
    // Nodes enrolled before rendering must also retire when destroyed.
    auto enrolled = nodeLedger.find(nodeId);
    if (enrolled != nodeLedger.end() && !enrolled.value().retired) {
        enrolled.value().retired = true;
        publishNodeLedgerLocked(enrolled.value().token);
    }
    if (!token.isEmpty()) {
        auto it = nodeLedger.find(nodeId);
        if (it != nodeLedger.end() && it.value().token == token) {
            it.value().retired = true;
            publishNodeLedgerLocked(token);
        }
    }
    publishNodeRetirementObservation(this, token);
    nodeIdentity.remove(this);
}

void NativeOpenGLRenderNode::setRect(const QRectF &rect)
{
    m_rect = rect;
}

void NativeOpenGLRenderNode::setPixelSize(const QSize &size)
{
    m_pixelSize = size;
}

void NativeOpenGLRenderNode::setPresentationMode(const QString &mode)
{
    // Mode changes must not destroy live GL objects on the GUI thread.
    // Phase 3A fixes mode before the first render; runtime mode switching
    // requires a separate, render-thread-owned lifecycle transition.
    if (m_renderer || m_failed) {
        if (mode != m_presentationMode)
            qWarning() << "SCREENSHAVER: cannot change presentation mode after renderer initialization";
        return;
    }
    if (mode != m_presentationMode) {
        QMutexLocker ownershipLock(&nativeOwnershipMutex);
        const quint64 nodeId = identityForNodeLocked(this);
        if (mode == QStringLiteral("wallpaper")) {
            // Even after revocation, new nodes make a previously closed
            // census incomplete. This never reopens or authorizes a grant.
            invalidateClosedCensusForLateNodeLocked();
            if (nodeId != 0) liveWallpaperNodes.insert(nodeId);
            if (!lastAuthorizedToken.isEmpty())
                enrollLiveNodesLocked(lastAuthorizedToken);
        } else {
            liveWallpaperNodes.remove(nodeId);
            // A node that changes away from wallpaper cannot be silently
            // omitted from a transaction to which it was already enrolled.
            auto it = nodeLedger.find(nodeId);
            if (it != nodeLedger.end() && !it.value().retired) {
                it.value().retired = true;
                publishNodeLedgerLocked(it.value().token);
            }
        }
    }
    m_presentationMode = mode;
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

    m_renderer = screenshaver_kde_gl_create_for_mode(
        &qtGetProcAddress,
        m_pixelSize.width(),
        m_pixelSize.height(),
        m_presentationMode == QStringLiteral("wallpaper") ? 1u : 0u);

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
    if (nativeWallpaperDiagnostic(m_presentationMode)) {
        static bool logged = false;
        if (!logged) { qInfo() << "SCREENSHAVER NATIVE TEST: prepare entered"; logged = true; }
        // Color diagnostic needs no Rust engine or grant.
        if (nativeWallpaperTestMode() == QStringLiteral("color")) return;
    }
    const bool gatedWallpaper = m_presentationMode == QStringLiteral("wallpaper");
    QMutexLocker ownershipLock(gatedWallpaper ? &nativeOwnershipMutex : nullptr);
    if (nativeWallpaperOwnershipDenied(m_presentationMode)) {
        clearWallpaperReadyMarker();
        clearNativeActivationAck();
        return;
    }
    if (!m_renderer)
        buildRenderer();
}

void NativeOpenGLRenderNode::render(const RenderState *)
{
    const QString testMode = m_presentationMode == QStringLiteral("wallpaper")
        ? nativeWallpaperTestMode() : QString{};
    if (!testMode.isEmpty()) {
        static bool logged = false;
        if (!logged) { qInfo() << "SCREENSHAVER NATIVE TEST: render entered, mode" << testMode; logged = true; }
    }
    if (testMode == QStringLiteral("color")) {
        auto *context = QOpenGLContext::currentContext();
        if (!context || m_pixelSize.isEmpty()) {
            static bool warned = false;
            if (!warned) { qWarning() << "SCREENSHAVER NATIVE TEST: no GL context or size" << m_pixelSize; warned = true; }
            return;
        }
        auto *gl = context->extraFunctions();
        // Preserve the framebuffer and viewport Qt already bound. Restrict the
        // diagnostic clear to the current viewport; restore modified GL state.
        GLint viewport[4] = {0,0,0,0};
        GLint scissor[4] = {0,0,0,0};
        GLfloat previousColor[4] = {0,0,0,0};
        const GLboolean wasScissored = gl->glIsEnabled(GL_SCISSOR_TEST);
        gl->glGetIntegerv(GL_VIEWPORT, viewport);
        gl->glGetIntegerv(GL_SCISSOR_BOX, scissor);
        gl->glGetFloatv(GL_COLOR_CLEAR_VALUE, previousColor);
        gl->glEnable(GL_SCISSOR_TEST);
        gl->glScissor(viewport[0], viewport[1], viewport[2], viewport[3]);
        gl->glClearColor(0.08f, 0.55f, 0.35f, 1.0f);
        gl->glClear(GL_COLOR_BUFFER_BIT);
        gl->glClearColor(previousColor[0], previousColor[1], previousColor[2], previousColor[3]);
        gl->glScissor(scissor[0], scissor[1], scissor[2], scissor[3]);
        if (!wasScissored) gl->glDisable(GL_SCISSOR_TEST);
        return; // No production readiness or activation ACK in test mode.
    }
    // Serializes all wallpaper render nodes against process-wide revocation.
    // No GL submission can remain in flight when the watcher acquires this lock.
    const bool gatedWallpaper = m_presentationMode == QStringLiteral("wallpaper");
    QMutexLocker ownershipLock(gatedWallpaper ? &nativeOwnershipMutex : nullptr);
    QByteArray frameGrantToken;
    if (gatedWallpaper && testMode.isEmpty()) {
        const QByteArray token = nativeWallpaperGrantToken();
        if (!token.isEmpty()) {
            if (acceptOwnershipTokenLocked(token)) {
                frameGrantToken = token;
                const QString ackPath = nativeOwnershipRevokedPath();
                if (!ackPath.isEmpty()) QFile::remove(ackPath);
            }
        }
    }

    // Check on the render thread before pause processing or Rust GL calls.
    // A deny-only gate must never publish readiness or a resume ACK.
    if (nativeWallpaperOwnershipDenied(m_presentationMode)) {
        clearWallpaperReadyMarker();
        clearNativeActivationAck();
        return;
    }
    // The render thread is the sole ACK authority. No ACK is emitted from
    // the GUI-thread watcher, and no production ACK is emitted in this phase.
    if (m_presentationMode == QStringLiteral("wallpaper")
        && !wallpaperPauseRequestPath().isEmpty()) {
        const QString requestPath = wallpaperPauseRequestPath();
        QByteArray token;
        bool present = false;
        const bool valid = readControlToken(requestPath, &token, &present);
        const bool pauseRequested = !valid || present;

        if (pauseRequested) {
            if (!m_pauseRequestObserved || (valid && token != m_heldPauseToken)) {
                clearWallpaperReadyMarker();
                clearNativeActivationAck();
                m_pauseRequestObserved = true;
                qInfo() << "SCREENSHAVER PLASMA PAUSE: frame submission suspended";
            }
            // An unreadable or malformed request must fail closed without
            // retaining authorization from an earlier transaction. In
            // particular, an old resume-token must not resume that transaction.
            if (!valid) {
                m_heldPauseToken.clear();
                m_pauseAckToken.clear();
            } else if (present && token != m_heldPauseToken) {
                m_heldPauseToken = token;
                m_pauseAckToken.clear();
            }
            if (valid && present && m_pauseAckToken != token &&
                !diagnosticControlPath(QStringLiteral("screenshaver-wallpaper.paused")).isEmpty()) {
                QByteArray current;
                bool stillPresent = false;
                if (readControlToken(requestPath, &current, &stillPresent) &&
                    stillPresent && current == token &&
                    publishDiagnosticAck(QStringLiteral("screenshaver-wallpaper.paused"), token)) {
                    m_pauseAckToken = token;
                    qInfo() << "SCREENSHAVER PLASMA PAUSE: token acknowledged";
                }
            }
            return;
        }
        if (m_pauseRequestObserved) {
            m_pauseRequestObserved = false;
            qInfo() << "SCREENSHAVER PLASMA PAUSE: request absent; frame submission permitted";
        }
    }
    if (!m_renderer && !buildRenderer())
        return;

    if (!QOpenGLContext::currentContext() || m_pixelSize.isEmpty())
        return;

    // A grant may have been withdrawn while preparing this frame.
    if (nativeWallpaperOwnershipDenied(m_presentationMode) ||
        (gatedWallpaper && testMode.isEmpty() && (frameGrantToken.isEmpty() ||
            nativeWallpaperGrantToken() != frameGrantToken))) {
        clearWallpaperReadyMarker();
        clearNativeActivationAck();
        return;
    }

    if (!screenshaver_kde_gl_render(
            m_renderer,
            m_pixelSize.width(),
            m_pixelSize.height())) {
        qCritical().noquote()
            << "SCREENSHAVER FRAME ENGINE: frame render failed:"
            << rustError();
        m_failed = true;
        clearWallpaperReadyMarker();
        clearNativeActivationAck();
        return;
    }

    // Filesystem authorization may be withdrawn while Rust executes its GL
    // call, even though our mutex excludes the in-process revocation watcher.
    // Do not publish a successful-frame marker (or a diagnostic resume ACK)
    // for a transaction that is no longer authorized. This is NOT a guarantee
    // that an already-submitted frame was not displayed by the compositor.
    if (gatedWallpaper && testMode.isEmpty() && (frameGrantToken.isEmpty() ||
        nativeWallpaperGrantToken() != frameGrantToken ||
        retiredOwnershipTokens.contains(frameGrantToken))) {
        clearWallpaperReadyMarker();
        clearNativeActivationAck();
        qWarning() << "SCREENSHAVER PLASMA OWNERSHIP: grant withdrawn during Rust render; readiness suppressed";
        return;
    }

    if (gatedWallpaper && testMode.isEmpty() && !frameGrantToken.isEmpty()) {
        nodeObservedTokens.insert(this, frameGrantToken);
        const quint64 nodeId = identityForNodeLocked(this);
        auto it = nodeLedger.find(nodeId);
        if (it == nodeLedger.end() && nodeId != 0) {
            nodeLedger.insert(nodeId, NodeLedgerEntry{frameGrantToken, false});
            publishNodeLedgerLocked(frameGrantToken);
        } else if (it.value().token != frameGrantToken) {
            qWarning() << "SCREENSHAVER PLASMA RETIREMENT: node token collision; observation incomplete";
        }
    }

    // Phase 7K-2: this is a transaction-scoped GL execution ACK, NOT a
    // compositor presentation ACK. The post-render grant recheck above must
    // succeed before this record can be published.
    if (gatedWallpaper && testMode.isEmpty())
        publishNativeActivationAck(frameGrantToken);

    if (m_presentationMode == QStringLiteral("wallpaper") && testMode.isEmpty()) {
        if (!m_readyPublished)
            publishWallpaperReadyMarker();

        // A successful Rust render is required before resume ACK. Verify
        // both the withdrawn request and the matching resume token.
        if (!m_heldPauseToken.isEmpty() && m_pauseAckToken == m_heldPauseToken) {
            QByteArray requestToken, resumeToken;
            bool requestPresent = false, resumePresent = false;
            const QString resumePath = diagnosticControlPath(
                QStringLiteral("screenshaver-wallpaper.resume-token"));
            if (!resumePath.isEmpty() &&
                readControlToken(wallpaperPauseRequestPath(), &requestToken, &requestPresent) &&
                !requestPresent &&
                readControlToken(resumePath, &resumeToken, &resumePresent) &&
                resumePresent && resumeToken == m_heldPauseToken &&
                publishDiagnosticAck(QStringLiteral("screenshaver-wallpaper.resumed"), resumeToken)) {
                qInfo() << "SCREENSHAVER PLASMA PAUSE: resumed frame acknowledged";
                m_heldPauseToken.clear();
                m_pauseAckToken.clear();
            }
        }
    }
}

void NativeOpenGLRenderNode::releaseResources()
{
    if (m_renderer && QOpenGLContext::currentContext()) {
        screenshaver_kde_gl_destroy(m_renderer);
        m_renderer = nullptr;
    }
}

// A per-render-node marker prevents one monitor from deleting another's state.
// This confirms a successful Rust render call, NOT compositor presentation.
void NativeOpenGLRenderNode::publishWallpaperReadyMarker()
{
    const QString runtimeDir = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (runtimeDir.isEmpty())
        return;

    if (m_readyPath.isEmpty()) {
        m_readyPath = QDir(runtimeDir).filePath(
            QStringLiteral("screenshaver-plasma-frame-%1-%2.ready")
                .arg(QCoreApplication::applicationPid())
                .arg(reinterpret_cast<quintptr>(this), 0, 16));
    }

    QSaveFile file(m_readyPath);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        qWarning() << "SCREENSHAVER: cannot publish Plasma frame marker:" << file.errorString();
        return;
    }
    const QByteArray startTicks = plasmaProcessStartTicks();
    if (startTicks.isEmpty()) {
        qWarning() << "SCREENSHAVER: cannot identify Plasma process instance";
        file.cancelWriting();
        return;
    }
    const QByteArray content = QByteArray::number(QCoreApplication::applicationPid())
        + " " + startTicks + "\n";
    if (file.write(content) != content.size() || !file.commit()) {
        qWarning() << "SCREENSHAVER: Plasma frame marker write failed:" << file.errorString();
        return;
    }
    m_readyPublished = true;
    qInfo() << "SCREENSHAVER PLASMA FRAME READY:" << m_readyPath;
}

void NativeOpenGLRenderNode::clearWallpaperReadyMarker()
{
    if (m_readyPublished && !m_readyPath.isEmpty())
        QFile::remove(m_readyPath);
    m_readyPublished = false;
}

// Phase 7K-2: per-node activation evidence. The filename is unique to this
// render node, so a different monitor cannot replace or retire its record.
// Rust must validate the token, PID and start ticks AND current grant; this
// file alone never grants ownership or proves compositor presentation.
void NativeOpenGLRenderNode::publishNativeActivationAck(const QByteArray &token)
{
    if (token.isEmpty() || retiredOwnershipTokens.contains(token))
        return;
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    const QByteArray ticks = plasmaProcessStartTicks();
    if (runtime.isEmpty() || ticks.isEmpty())
        return;
    if (m_activationAckPath.isEmpty()) {
        m_activationAckPath = QDir(runtime).filePath(
            QStringLiteral("screenshaver-plasma-activation-%1-%2.ack")
                .arg(QCoreApplication::applicationPid())
                .arg(reinterpret_cast<quintptr>(this), 0, 16));
    }
    if (m_activationAckToken == token)
        return;
    // Never retain evidence for a superseded transaction.
    clearNativeActivationAck();
    QSaveFile file(m_activationAckPath);
    const QByteArray payload = "v1 " + token + " " +
        QByteArray::number(QCoreApplication::applicationPid()) + " " + ticks + "\n";
    if (!file.open(QIODevice::WriteOnly) ||
        file.write(payload) != payload.size() || !file.commit()) {
        qWarning() << "SCREENSHAVER PLASMA OWNERSHIP: activation ACK publication failed";
        return;
    }
    m_activationAckToken = token;
    qInfo() << "SCREENSHAVER PLASMA OWNERSHIP: native GL activation acknowledged" << token;
}

void NativeOpenGLRenderNode::clearNativeActivationAck()
{
    if (!m_activationAckPath.isEmpty() && !m_activationAckToken.isEmpty()) {
        if (!QFile::remove(m_activationAckPath))
            qWarning() << "SCREENSHAVER PLASMA OWNERSHIP: activation ACK retirement failed";
    }
    m_activationAckToken.clear();
}
