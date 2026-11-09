#pragma once
#include <QQuickPaintedItem>
#include <QImage>
#include <QString>
class QTimer;

// Frame-only presenter: does not load libscreenshaver or create a Rust renderer.
class NativeWallpaperPresenter : public QQuickPaintedItem {
    Q_OBJECT
    Q_PROPERTY(QString frameFile READ frameFile WRITE setFrameFile NOTIFY frameFileChanged)
    Q_PROPERTY(bool frameAvailable READ frameAvailable NOTIFY frameAvailableChanged)
    Q_PROPERTY(qulonglong frameSequence READ frameSequence NOTIFY frameSequenceChanged)
public:
    explicit NativeWallpaperPresenter(QQuickItem *parent = nullptr);
    void paint(QPainter *painter) override;
    QString frameFile() const { return m_frameFile; }
    void setFrameFile(const QString &value);
    bool frameAvailable() const { return m_frameAvailable; }
    qulonglong frameSequence() const { return m_sequence; }
signals:
    void frameFileChanged();
    void frameAvailableChanged();
    void frameSequenceChanged();
private:
    void poll();
    QString m_frameFile;
    QImage m_image;
    QTimer *m_timer = nullptr;
    bool m_frameAvailable = false;
    qulonglong m_sequence = 0;
    qint64 m_lastAckMillis = 0;
};
