import QtQuick 2.15
import org.kde.plasma.plasmoid
import ScreenshaverNativeGL 1.0

WallpaperItem {
    id: root

    // Persistent, non-rendering authorization monitor. It never constructs
    // a QSGRenderNode or submits Rust GL. The C++ probe shares the native
    // render gate's strict transaction parser and process mutex.
    NativeOpenGLUnderlay {
        id: ownershipMonitor
        ownershipMonitorOnly: true
        presentationMode: "wallpaper"
        visible: false
    }

    // Revocation unloads the rendering item through Qt Quick's normal
    // scene-graph lifecycle. The legacy Rust pause remains held regardless
    // of loader state; unloading is NOT compositor retirement proof.
    Loader {
        id: wallpaperRenderer
        anchors.fill: parent
        active: ownershipMonitor.wallpaperGrantAuthorized
        sourceComponent: Component {
            NativeOpenGLUnderlay {
                id: shader
                presentationMode: "wallpaper"

                // Invalidation heartbeat; Rust owns shader time.
                NumberAnimation {
                    target: shader
                    property: "time"
                    from: 0
                    to: 1000000
                    duration: 1000000000
                    loops: Animation.Infinite
                    running: root.visible
                }
            }
        }
    }
}
