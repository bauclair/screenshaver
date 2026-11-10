import QtQuick 2.15
import org.kde.plasma.plasmoid
import "ScreenshaverNativeGL" as ScreenshaverGL

WallpaperItem {
    id: root

    // Plasma owns the desktop surface; Rust renders into a separate frame file.
    // No legacy OpenGL underlay or native ownership-grant protocol is instantiated.
    ScreenshaverGL.NativeWallpaperPresenter {
        id: wallpaperPresenter
        anchors.fill: parent
        visible: root.visible
    }
}
