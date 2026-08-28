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
