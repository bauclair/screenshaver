#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef const void *(*ScreenshaverGlProcLoader)(const char *name);

typedef struct ScreenshaverKdeGlRenderer ScreenshaverKdeGlRenderer;

ScreenshaverKdeGlRenderer *screenshaver_kde_gl_create(
    ScreenshaverGlProcLoader loader,
    int32_t width,
    int32_t height);

// 0 = screensaver (existing behavior); 1 = wallpaper (reserved until
// wallpaper policy/runtime initialization is implemented).
ScreenshaverKdeGlRenderer *screenshaver_kde_gl_create_for_mode(
    ScreenshaverGlProcLoader loader,
    int32_t width,
    int32_t height,
    uint32_t mode);

bool screenshaver_kde_gl_render(
    ScreenshaverKdeGlRenderer *renderer,
    int32_t width,
    int32_t height);

void screenshaver_kde_gl_destroy(
    ScreenshaverKdeGlRenderer *renderer);

const char *screenshaver_kde_gl_last_error(void);
uint32_t screenshaver_kde_gl_bridge_version(void);

#ifdef __cplusplus
}
#endif
