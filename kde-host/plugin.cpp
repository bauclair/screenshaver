#include "native_opengl_underlay.h"
#include "native_wallpaper_presenter.h"

#include <QQmlExtensionPlugin>
#include <qqml.h>

class ScreenshaverNativeGLPlugin final : public QQmlExtensionPlugin
{
    Q_OBJECT
    Q_PLUGIN_METADATA(IID QQmlExtensionInterface_iid)

public:
    void registerTypes(const char *uri) override
    {
        qmlRegisterType<NativeOpenGLUnderlay>(
            uri,
            1,
            0,
            "NativeOpenGLUnderlay");
        qmlRegisterType<NativeWallpaperPresenter>(uri, 1, 0, "NativeWallpaperPresenter");
    }
};

#include "plugin.moc"
