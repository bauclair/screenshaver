#include "native_wallpaper_presenter.h"
#include <QPainter>
#include <QTimer>
#include <QFile>
#include <QByteArray>
#include <QDebug>
#include <QStandardPaths>
#include <QDir>
#include <QSaveFile>
#include <QDateTime>
#include <sys/stat.h>
#include <QtEndian>
#include <cstring>
#include <limits>
#include <sys/file.h>
#include <unistd.h>

namespace {
constexpr qint64 HeaderSize = 64;
constexpr quint32 MaxDimension = 16384;
constexpr quint64 MaxFrameBytes = 256ull * 1024 * 1024;
quint32 read32(const char *p) { return qFromLittleEndian<quint32>(reinterpret_cast<const uchar *>(p)); }
quint64 read64(const char *p) { return qFromLittleEndian<quint64>(reinterpret_cast<const uchar *>(p)); }
}

NativeWallpaperPresenter::NativeWallpaperPresenter(QQuickItem *parent)
    : QQuickPaintedItem(parent) {
    setAntialiasing(false);
    setOpaquePainting(true);
    // Default transport is resolved from the user runtime directory, never /tmp.
    // Explicit frameFile overrides remain available for standalone testing.
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (!runtime.isEmpty()) {
        m_frameFile = QDir(runtime).filePath(QStringLiteral("screenshaver-plasma.frames"));
    }
    m_timer = new QTimer(this);
    m_timer->setInterval(33); // staged polling; production event notification follows
    connect(m_timer, &QTimer::timeout, this, &NativeWallpaperPresenter::poll);
    m_timer->start();
}

void NativeWallpaperPresenter::setFrameFile(const QString &value) {
    if (m_frameFile == value) return;
    m_frameFile = value;
    m_sequence = 0;
    m_image = QImage();
    if (m_frameAvailable) { m_frameAvailable = false; emit frameAvailableChanged(); }
    emit frameFileChanged();
    emit frameSequenceChanged();
    update();
}

void NativeWallpaperPresenter::paint(QPainter *painter) {
    painter->fillRect(boundingRect(), Qt::black);
    if (!m_image.isNull()) painter->drawImage(boundingRect(), m_image);
}

void NativeWallpaperPresenter::poll() {
    if (m_frameFile.isEmpty() || !isVisible()) return;
    QFile file(m_frameFile);
    if (!file.open(QIODevice::ReadOnly)) return;
    const int fd = file.handle();
    if (fd < 0 || flock(fd, LOCK_SH | LOCK_NB) != 0) return;
    // Lock covers the header, chosen slot and image copy. No mmap data race.
    const QByteArray header = file.read(HeaderSize);
    if (header.size() != HeaderSize || std::memcmp(header.constData(), "SSPLFR01", 8) != 0
        || read32(header.constData()+8) != 1 || read32(header.constData()+12) != HeaderSize) {
        flock(fd, LOCK_UN); return;
    }
    const quint64 sequence = read64(header.constData()+16);
    const quint32 width = read32(header.constData()+24);
    const quint32 height = read32(header.constData()+28);
    const quint32 stride = read32(header.constData()+32);
    const quint32 bytes = read32(header.constData()+36);
    const quint64 expected = quint64(width) * quint64(height) * 4;
    if (!sequence || sequence == m_sequence || width == 0 || height == 0
        || width > MaxDimension || height > MaxDimension
        || stride != quint64(width)*4 || expected != bytes || expected > MaxFrameBytes
        || quint64(file.size()) < HeaderSize + 2*expected) {
        flock(fd, LOCK_UN); return;
    }
    const quint64 slot = (sequence - 1) % 2;
    if (!file.seek(HeaderSize + qint64(slot*expected))) { flock(fd, LOCK_UN); return; }
    const QByteArray pixels = file.read(qint64(expected));
    flock(fd, LOCK_UN);
    if (pixels.size() != qint64(expected)) return;
    // OpenGL ReadPixels produces bottom-up RGBA rows; mirror to Qt top-down.
    QImage snapshot(reinterpret_cast<const uchar*>(pixels.constData()),
                    int(width), int(height), int(stride), QImage::Format_RGBA8888);
    m_image = snapshot.flipped(Qt::Vertical).copy();
    m_sequence = sequence;
    // Readiness is evidence of a fully decoded frame, not merely a running producer.
    // Publish at most once per second to avoid a filesystem write per video frame.
    // Acknowledgment binds to the actual frame inode and Plasma process identity.
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    if (now - m_lastAckMillis >= 1000) {
        struct stat st {};
        if (fstat(fd, &st) == 0 && st.st_ino > 0) {
            const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
            const QString expected = QDir(runtime).filePath(QStringLiteral("screenshaver-plasma.frames"));
            if (!runtime.isEmpty() && m_frameFile == expected) {
                QSaveFile ack(QDir(runtime).filePath(QStringLiteral("screenshaver-plasma-presenter.ready")));
                ack.setDirectWriteFallback(false);
                if (ack.open(QIODevice::WriteOnly)) {
                    const QByteArray body = QByteArray("SSPLACK01\n")
                        + QByteArray::number(getpid()) + '\n'
                        + QByteArray::number(qulonglong(st.st_dev)) + '\n'
                        + QByteArray::number(qulonglong(st.st_ino)) + '\n'
                        + QByteArray::number(sequence) + '\n'
                        + QByteArray::number(now) + '\n';
                    if (ack.write(body) == body.size() && ack.commit()) m_lastAckMillis = now;
                    else ack.cancelWriting();
                }
            }
        }
    }
    if (!m_frameAvailable) { m_frameAvailable = true; emit frameAvailableChanged(); }
    emit frameSequenceChanged();
    update();
}
