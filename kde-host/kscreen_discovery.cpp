// One-shot KDE output enumeration. No background worker or global mutable state.
#include <QGuiApplication>
#include <QCoreApplication>
#include <QRect>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QByteArray>
#include <QString>
#include <KScreen/GetConfigOperation>
#include <KScreen/Config>
#include <KScreen/Output>
#include <KScreen/Mode>
#include <cstring>
#include <algorithm>
#include <cstdio>
#include <exception>
#include <QElapsedTimer>
#include <QThread>

extern "C" int screenshaver_kscreen_discover(char *destination, size_t capacity) noexcept {
    if (!destination || capacity < 2) return -1;
    destination[0] = '\0';
    try {
        int argc = 1;
        char app_name[] = "screenshaver-kscreen";
        char *argv[] = {app_name, nullptr};
        // The Rust supervisor has no Qt application. This local application
        // permits libkscreen's synchronous operation to process Qt events.
        if (QCoreApplication::instance()) return -2;
        QGuiApplication app(argc, argv);
        std::fprintf(stderr,
            "[KSCREEN NATIVE] Qt=%s platform=%s dbus=%s thread=%p\n",
            qVersion(), qgetenv("QT_QPA_PLATFORM").constData(),
            qgetenv("DBUS_SESSION_BUS_ADDRESS").isEmpty() ? "unset" : "set",
            static_cast<void *>(QThread::currentThread()));
        std::fflush(stderr);
        QElapsedTimer elapsed;
        elapsed.start();
        KScreen::GetConfigOperation operation;
        const bool completed = operation.exec();
        const auto config = operation.config();
        std::fprintf(stderr,
            "[KSCREEN NATIVE] operation.exec=%s config=%s elapsed_ms=%lld\n",
            completed ? "true" : "false", config ? "present" : "null",
            static_cast<long long>(elapsed.elapsed()));
        std::fflush(stderr);
        if (!completed) {
            std::fprintf(stderr, "[KSCREEN NATIVE] configuration request failed: %s\n",
                         operation.errorString().toLocal8Bit().constData());
            std::fflush(stderr);
            return -3;
        }
        if (!config) return -6;
        QJsonArray displays;
        const auto outputs = config->outputs();
        std::fprintf(stderr, "[KSCREEN NATIVE] output_count=%lld\n", static_cast<long long>(outputs.size()));
        for (auto it = outputs.cbegin(); it != outputs.cend(); ++it) {
            const auto &output = it.value();
            if (!output) continue;
            QJsonObject display;
            display.insert("id", static_cast<int>(output->id()));
            display.insert("connector", output->name());
            display.insert("connected", output->isConnected());
            display.insert("enabled", output->isEnabled());
            display.insert("scale", output->scale());
            display.insert("x", output->pos().x());
            display.insert("y", output->pos().y());
            const auto geometry = output->geometry();
            display.insert("logical_width", geometry.width());
            display.insert("logical_height", geometry.height());
            const auto mode = output->currentMode();
            display.insert("physical_width", mode ? mode->size().width() : 0);
            display.insert("physical_height", mode ? mode->size().height() : 0);
            display.insert("refresh_hz", mode ? mode->refreshRate() : 0.0);
            displays.append(display);
        }
        const QByteArray bytes = QJsonDocument(displays).toJson(QJsonDocument::Compact);
        if (static_cast<size_t>(bytes.size()) + 1 > capacity) return -4;
        std::memcpy(destination, bytes.constData(), static_cast<size_t>(bytes.size()));
        destination[bytes.size()] = '\0';
        return 0;
    } catch (const std::exception &e) {
        std::fprintf(stderr, "[KSCREEN NATIVE] exception: %s\n", e.what());
        return -5;
    } catch (...) {
        std::fprintf(stderr, "[KSCREEN NATIVE] unknown C++ exception\n");
        return -5;
    }
}

// Standalone discovery entry point. Keep diagnostic output on stderr and
// emit exactly one JSON array to stdout for the Rust supervisor.
#ifdef SCREENSHAVER_KSCREEN_STANDALONE
int main() {
    char result[65536] = {};
    const int status = screenshaver_kscreen_discover(result, sizeof(result));
    if (status != 0) {
        std::fprintf(stderr, "[KSCREEN NATIVE] discovery failed with status %d\n", status);
        return 1;
    }
    if (std::puts(result) == EOF) return 1;
    return 0;
}
#endif
