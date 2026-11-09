import QtQuick 2.15
import org.kde.plasma.plasmoid
import ScreenshaverNativeGL 1.0

WallpaperItem {
    id: root

    // Plasma owns the desktop surface; Rust renders into a separate frame file.
    // No legacy OpenGL underlay or native ownership-grant protocol is instantiated.
    NativeWallpaperPresenter {
        id: wallpaperPresenter
        anchors.fill: parent
        visible: root.visible
    }
}
