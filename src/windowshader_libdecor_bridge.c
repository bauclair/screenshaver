/* Screenshaver Stage 3C-5B: isolated native libdecor connection.
 * The existing borrowed-display API remains inert and compatible with Stage 3B.
 * The new isolated API owns its wl_display and libdecor context, and creates
 * no wl_surface, libdecor_frame, or EGL objects. Production does not call it.
 * All calls for one instance must occur on the same thread, serially.
 */
#include <libdecor.h>
#include <wayland-client.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>
#include <sys/mman.h>
#include <fcntl.h>
#include <time.h>
#include <EGL/egl.h>
#define GL_GLEXT_PROTOTYPES 1
#include <GL/gl.h>
#include <GL/glext.h>
#include <wayland-egl.h>
#include <math.h>

struct screenshaver_decor_bridge {
    struct wl_display *display;
    struct libdecor *context;
    int owns_display;
    int may_dispatch_display;
};

static void bridge_error(struct libdecor *context,
                         enum libdecor_error error, const char *message) {
    (void)context;
    /* No per-context userdata in this callback: log, do not guess owner. */
    fprintf(stderr, "[WINDOWSHADER LIBDECOR] native error %d: %s\n",
            (int)error, message ? message : "unspecified");
}

static struct libdecor_interface bridge_interface = {
    .error = bridge_error,
};

/* Legacy API: borrows Rust's connection; native dispatch is forbidden. */
struct screenshaver_decor_bridge *screenshaver_decor_bridge_create(
    struct wl_display *display) {
    if (!display) return NULL;
    struct screenshaver_decor_bridge *bridge = calloc(1, sizeof(*bridge));
    if (!bridge) return NULL;
    bridge->display = display;
    bridge->context = libdecor_new(display, &bridge_interface);
    if (!bridge->context) { free(bridge); return NULL; }
    return bridge;
}

/* Isolated diagnostic API: establishes its own Wayland connection.
 * Does not create a surface or assign any XDG role. */
struct screenshaver_decor_bridge *screenshaver_decor_bridge_create_isolated(void) {
    struct screenshaver_decor_bridge *bridge = calloc(1, sizeof(*bridge));
    if (!bridge) return NULL;
    bridge->display = wl_display_connect(NULL);
    if (!bridge->display) {
        fprintf(stderr, "[WINDOWSHADER LIBDECOR] isolated connect failed\n");
        free(bridge);
        return NULL;
    }
    bridge->owns_display = 1;
    bridge->context = libdecor_new(bridge->display, &bridge_interface);
    if (!bridge->context) {
        fprintf(stderr, "[WINDOWSHADER LIBDECOR] isolated libdecor_new failed\n");
        wl_display_disconnect(bridge->display);
        free(bridge);
        return NULL;
    }
    bridge->may_dispatch_display = 1;
    fprintf(stderr, "[WINDOWSHADER LIBDECOR] isolated context initialized\n");
    return bridge;
}

/* The owner must stop dispatching before calling destroy. */
void screenshaver_decor_bridge_destroy(struct screenshaver_decor_bridge *bridge) {
    if (!bridge) return;
    if (bridge->context) libdecor_unref(bridge->context);
    if (bridge->owns_display && bridge->display)
        wl_display_disconnect(bridge->display);
    free(bridge);
}

/* Preserve Stage 3A's Rust ABI handshake. */
int screenshaver_decor_bridge_abi_version(void) { return 1; }

/* Return -2 for borrowed display, -1 for invalid bridge/dispatch failure.
 * For an isolated bridge, libdecor_dispatch's nonnegative return is passed
 * through. Use timeout_ms=0 for a nonblocking diagnostic iteration; avoid
 * indefinite waits so callers can process shutdown requests. */
int screenshaver_decor_bridge_dispatch(struct screenshaver_decor_bridge *bridge,
                                      int timeout_ms) {
    if (!bridge || !bridge->context) return -1;
    if (!bridge->may_dispatch_display || !bridge->owns_display) return -2;
    if (timeout_ms < 0) return -3; /* bounded dispatch only */
    int result = libdecor_dispatch(bridge->context, timeout_ms);
    if (result < 0)
        fprintf(stderr, "[WINDOWSHADER LIBDECOR] isolated dispatch failed: %d\n", result);
    return result;
}

int screenshaver_decor_bridge_dispatch_is_safe(
    const struct screenshaver_decor_bridge *bridge) {
    return bridge && bridge->context && bridge->owns_display &&
           bridge->may_dispatch_display;
}


/* Stage 3C-3: windowed diagnostic on the bridge-owned Wayland connection.
 * Production windowshader role selection and EGL rendering remain unchanged.
 * All Wayland/libdecor calls are serialized on this diagnostic's caller thread.
 */
struct bridge_test_window {
    struct screenshaver_decor_bridge *bridge;
    struct wl_registry *registry;
    struct wl_compositor *compositor;
    struct wl_surface *surface;
    struct libdecor_frame *frame;
    EGLDisplay egl_display;
    EGLContext egl_context;
    EGLSurface egl_surface;
    struct wl_egl_window *egl_window;
    unsigned frames;
    int width, height;
    int configured, running, failed;
};

static void bridge_test_global(void *data, struct wl_registry *registry,
                               uint32_t name, const char *interface, uint32_t version) {
    struct bridge_test_window *w = data;
    if (!strcmp(interface, wl_compositor_interface.name) && !w->compositor)
        w->compositor = wl_registry_bind(registry, name, &wl_compositor_interface,
                                         version < 4 ? version : 4);
}
static void bridge_test_global_remove(void *data, struct wl_registry *registry, uint32_t name) {
    (void)data; (void)registry; (void)name;
}
static const struct wl_registry_listener bridge_test_registry_listener = {
    .global = bridge_test_global, .global_remove = bridge_test_global_remove
};
/* Stage 3C-6D2C-5: opt-in swap-interval experiment; Rust owns GL objects. */
extern int screenshaver_libdecor_rust_gl_initialize(void);
extern int screenshaver_libdecor_rust_gl_draw(int width, int height, unsigned frame);
extern void screenshaver_libdecor_rust_gl_release(void);
extern int screenshaver_libdecor_rust_gl_replace(const char *name);
void *screenshaver_libdecor_gl_proc_address(const char *name) {
    return (void *)eglGetProcAddress(name);
}

static int bridge_egl_initialize(struct bridge_test_window *w) {
    static const EGLint config_attrs[] = {
        EGL_SURFACE_TYPE, EGL_WINDOW_BIT, EGL_RENDERABLE_TYPE, EGL_OPENGL_BIT,
        EGL_RED_SIZE, 8, EGL_GREEN_SIZE, 8, EGL_BLUE_SIZE, 8, EGL_NONE
    };
    static const EGLint context_attrs[] = {
        EGL_CONTEXT_MAJOR_VERSION, 3, EGL_CONTEXT_MINOR_VERSION, 3,
        EGL_CONTEXT_OPENGL_PROFILE_MASK, EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT, EGL_NONE
    };
    w->egl_display = eglGetDisplay((EGLNativeDisplayType)w->bridge->display);
    if (w->egl_display == EGL_NO_DISPLAY || !eglInitialize(w->egl_display, NULL, NULL)) return -1;
    if (!eglBindAPI(EGL_OPENGL_API)) return -1;
    EGLConfig config;
    EGLint count = 0;
    if (!eglChooseConfig(w->egl_display, config_attrs, &config, 1, &count) || count != 1) return -1;
    w->egl_context = eglCreateContext(w->egl_display, config, EGL_NO_CONTEXT, context_attrs);
    if (w->egl_context == EGL_NO_CONTEXT) return -1;
    w->egl_window = wl_egl_window_create(w->surface, w->width, w->height);
    if (!w->egl_window) return -1;
    w->egl_surface = eglCreateWindowSurface(w->egl_display, config,
                                             (EGLNativeWindowType)w->egl_window, NULL);
    if (w->egl_surface == EGL_NO_SURFACE ||
        !eglMakeCurrent(w->egl_display, w->egl_surface, w->egl_surface, w->egl_context)) return -1;
    /* Stage 3C-6D2C-5: opt-in EGL swap-interval comparison.
     * Leave the established default untouched unless explicitly requested.
     * This is diagnostic only: a successful request does not guarantee the
     * compositor or EGL driver will avoid blocking in eglSwapBuffers(). */
    const char *swap_interval = getenv("SCREENSHAVER_LIBDECOR_DIAG_SWAP_INTERVAL");
    if (swap_interval && swap_interval[0]) {
        if (strcmp(swap_interval, "0") != 0 && strcmp(swap_interval, "1") != 0) {
            fprintf(stderr, "[LIBDECOR EGL DIAG] invalid swap interval '%s' (expected 0 or 1)\n",
                    swap_interval);
            return -1;
        }
        int requested = swap_interval[0] - '0';
        EGLBoolean accepted = eglSwapInterval(w->egl_display, requested);
        if (!accepted) {
            EGLint error = eglGetError();
            fprintf(stderr, "[LIBDECOR EGL DIAG] eglSwapInterval(%d) rejected (EGL error=0x%x)\n",
                    requested, error);
            return -1;
        }
        fprintf(stderr, "[LIBDECOR EGL DIAG] eglSwapInterval(%d) accepted; effective presentation pacing is driver/compositor-dependent\n",
                requested);
    } else {
        fprintf(stderr, "[LIBDECOR EGL DIAG] swap interval unchanged (EGL default)\n");
    }
    if (screenshaver_libdecor_rust_gl_initialize() != 0) return -1;
    fprintf(stderr, "[LIBDECOR BRIDGE TEST] EGL/desktop OpenGL initialized: %s\n",
            (const char *)glGetString(GL_VERSION));
    return 0;
}

/* Stage 3C-6D2C-4: timestamps pair native diagnostics with the runtime log. */
static void bridge_handoff_stamp(const char *operation, long duration_ms) {
    struct timespec wall, mono;
    struct tm local;
    clock_gettime(CLOCK_REALTIME, &wall);
    clock_gettime(CLOCK_MONOTONIC, &mono);
    localtime_r(&wall.tv_sec, &local);
    fprintf(stderr,
            "[LIBDECOR HANDOFF DIAG] %04d-%02d-%02d %02d:%02d:%02d.%03ld monotonic=%lld.%09ld %s %ldms\n",
            local.tm_year + 1900, local.tm_mon + 1, local.tm_mday,
            local.tm_hour, local.tm_min, local.tm_sec, wall.tv_nsec / 1000000L,
            (long long)mono.tv_sec, mono.tv_nsec, operation, duration_ms);
}

/* Only the diagnostic's owning thread sets and calls these borrowed callbacks. */
static int (*bridge_diagnostic_pause_status)(void *, int) = NULL;
static void *bridge_diagnostic_pause_userdata = NULL;

static int bridge_test_paint(struct bridge_test_window *w) {
    if (!w->configured || w->egl_surface == EGL_NO_SURFACE) return -1;
    struct timespec draw_begin, draw_end, swap_begin, swap_end;
    clock_gettime(CLOCK_MONOTONIC, &draw_begin);
    int draw_result = screenshaver_libdecor_rust_gl_draw(w->width, w->height, w->frames);
    clock_gettime(CLOCK_MONOTONIC, &draw_end);
    long draw_ms = (draw_end.tv_sec - draw_begin.tv_sec) * 1000L
                 + (draw_end.tv_nsec - draw_begin.tv_nsec) / 1000000L;
    if (draw_ms >= 250) bridge_handoff_stamp("Rust GL draw blocked", draw_ms);
    if (draw_result != 0) return -1;
    /* Phase 2/3 only update diagnostics; they never grant pause acknowledgment. */
    if (bridge_diagnostic_pause_status) (void)bridge_diagnostic_pause_status(bridge_diagnostic_pause_userdata, 2);
    clock_gettime(CLOCK_MONOTONIC, &swap_begin);
    int swap_result = eglSwapBuffers(w->egl_display, w->egl_surface);
    clock_gettime(CLOCK_MONOTONIC, &swap_end);
    if (bridge_diagnostic_pause_status) (void)bridge_diagnostic_pause_status(bridge_diagnostic_pause_userdata, 3);
    long swap_ms = (swap_end.tv_sec - swap_begin.tv_sec) * 1000L
                 + (swap_end.tv_nsec - swap_begin.tv_nsec) / 1000000L;
    if (swap_ms >= 250) bridge_handoff_stamp("eglSwapBuffers blocked", swap_ms);
    if (!swap_result) return -1;
    ++w->frames;
    return 0;
}

static void bridge_test_configure(struct libdecor_frame *frame,
                                  struct libdecor_configuration *configuration, void *data) {
    struct bridge_test_window *w = data;
    int width = w->width, height = w->height;
    if (libdecor_configuration_get_content_size(configuration, frame, &width, &height)
        && width > 0 && height > 0) { w->width = width; w->height = height; }
    struct libdecor_state *state = libdecor_state_new(w->width, w->height);
    if (!state) { w->failed = 1; w->running = 0; return; }
    libdecor_frame_commit(frame, state, configuration);
    libdecor_state_free(state);
    if (w->width < 64 || w->height < 64 || w->width > 4096 || w->height > 4096) {
        w->failed = 1; w->running = 0; return;
    }
    if (w->egl_window) wl_egl_window_resize(w->egl_window, w->width, w->height, 0, 0);
    w->configured = 1;
    fprintf(stderr, "[LIBDECOR BRIDGE TEST] configure: %d x %d\n", w->width, w->height);
}
static void bridge_test_close(struct libdecor_frame *frame, void *data) {
    (void)frame;
    struct bridge_test_window *w = data;
    fprintf(stderr, "[LIBDECOR BRIDGE TEST] close requested\n");
    w->running = 0;
}
static void bridge_test_commit(struct libdecor_frame *frame, void *data) {
    (void)frame;
    struct bridge_test_window *w = data;
    if (w->surface) wl_surface_commit(w->surface);
}
static void bridge_test_dismiss_popup(struct libdecor_frame *frame,
                                      const char *seat_name, void *data) {
    (void)frame; (void)seat_name; (void)data;
}
static struct libdecor_frame_interface bridge_test_frame_interface = {
    .configure = bridge_test_configure, .close = bridge_test_close,
    .commit = bridge_test_commit, .dismiss_popup = bridge_test_dismiss_popup
};

/* Interactive diagnostic: continue until explicit shutdown or window close. */
int screenshaver_decor_bridge_run_diagnostic_with_pause_and_fps(
    int (*should_stop)(void *), void *stop_userdata,
    int (*replace_policy)(void *), void *policy_userdata,
    int (*pause_status)(void *, int), void *pause_userdata,
    unsigned configured_fps) {
    bridge_diagnostic_pause_status = pause_status;
    bridge_diagnostic_pause_userdata = pause_userdata;
    struct bridge_test_window w = {0};
    w.width = 720; w.height = 420; w.running = 1;
    w.egl_display = EGL_NO_DISPLAY;
    w.egl_context = EGL_NO_CONTEXT;
    w.egl_surface = EGL_NO_SURFACE;
    int result = 1;
    fprintf(stderr, "[LIBDECOR BRIDGE TEST] connecting isolated native bridge\n");
    w.bridge = screenshaver_decor_bridge_create_isolated();
    if (!w.bridge || !screenshaver_decor_bridge_dispatch_is_safe(w.bridge)) goto cleanup;
    w.registry = wl_display_get_registry(w.bridge->display);
    if (!w.registry || wl_registry_add_listener(w.registry, &bridge_test_registry_listener, &w) < 0)
        goto cleanup;
    if (wl_display_roundtrip(w.bridge->display) < 0 || !w.compositor) {
        fprintf(stderr, "[LIBDECOR BRIDGE TEST] compositor unavailable\n");
        goto cleanup;
    }
    w.surface = wl_compositor_create_surface(w.compositor);
    if (!w.surface) goto cleanup;
    w.frame = libdecor_decorate(w.bridge->context, w.surface,
                               &bridge_test_frame_interface, &w);
    if (!w.frame) goto cleanup;
    libdecor_frame_set_app_id(w.frame, "screenshaver-libdecor-bridge");
    libdecor_frame_set_title(w.frame, "Screenshaver libdecor bridge - drag / resize / close");
    libdecor_frame_set_min_content_size(w.frame, 160, 120);
    if (bridge_egl_initialize(&w) < 0) {
        fprintf(stderr, "[LIBDECOR BRIDGE TEST] EGL setup failed (error=0x%x)\n", eglGetError());
        goto cleanup;
    }
    libdecor_frame_map(w.frame);
    fprintf(stderr, "[LIBDECOR BRIDGE TEST] window requested; drag, resize, then close\n");
    time_t started = time(NULL);
    /* Stage 3C-6D2C-6: bounded, opt-in diagnostic frame pacing.
     * Rust supplies policy-resolved FPS; diagnostic override is optional. */
    const char *fps_env = getenv("SCREENSHAVER_LIBDECOR_DIAG_FPS");
    unsigned diagnostic_fps = configured_fps;
    if (diagnostic_fps < 1 || diagnostic_fps > 240) {
        fprintf(stderr, "[LIBDECOR FPS] invalid configured FPS %u (expected 1..240)\n", diagnostic_fps);
        w.failed = 1;
        goto cleanup;
    }
    fprintf(stderr, "[LIBDECOR FPS] Rust-resolved policy FPS: %u\n", diagnostic_fps);
    if (fps_env && fps_env[0]) {
        char *end = NULL;
        unsigned long parsed = strtoul(fps_env, &end, 10);
        if (!end || *end != '\0' || parsed < 1 || parsed > 240) {
            fprintf(stderr, "[LIBDECOR FPS DIAG] invalid FPS '%s' (expected 1..240)\n", fps_env);
            w.failed = 1;
            goto cleanup;
        }
        diagnostic_fps = (unsigned)parsed;
        fprintf(stderr, "[LIBDECOR FPS DIAG] diagnostic frame pacing enabled: %u FPS\n", diagnostic_fps);
    } else {
        fprintf(stderr, "[LIBDECOR FPS DIAG] no diagnostic override; using Rust-resolved %u FPS\n", diagnostic_fps);
    }
    /* Stage 3C-6D2B: explicitly opted-in one-shot policy transition.
     * The callback runs on the EGL owner thread, between draw calls. */
    const char *policy_test = getenv("SCREENSHAVER_LIBDECOR_TEST_POLICY_TRANSITION");
    int policy_attempted = 0;
    /* Diagnostic only: one manually requested replacement after ten seconds.
     * This is not the policy scheduler and does not alter the native window. */
    const char *replacement = getenv("SCREENSHAVER_LIBDECOR_REPLACEMENT_SHADER");
    int replacement_attempted = 0;
    /* Stage 3C-6D2C-3: native loop diagnostics, no behavior change. */
    struct timespec diagnostic_last;
    clock_gettime(CLOCK_MONOTONIC, &diagnostic_last);
    while (w.running) {
        if (should_stop && should_stop(stop_userdata)) {
            fprintf(stderr, "[LIBDECOR BRIDGE TEST] external shutdown requested\n");
            break;
        }
        struct timespec dispatch_begin, dispatch_end;
        clock_gettime(CLOCK_MONOTONIC, &dispatch_begin);
        int dispatch_result = screenshaver_decor_bridge_dispatch(w.bridge, 50);
        clock_gettime(CLOCK_MONOTONIC, &dispatch_end);
        long dispatch_ms = (dispatch_end.tv_sec - dispatch_begin.tv_sec) * 1000L
                         + (dispatch_end.tv_nsec - dispatch_begin.tv_nsec) / 1000000L;
        if (dispatch_ms >= 250)
            bridge_handoff_stamp("libdecor dispatch blocked", dispatch_ms);
        if (dispatch_result < 0) {
            w.failed = 1; break;
        }
        long heartbeat_ms = (dispatch_end.tv_sec - diagnostic_last.tv_sec) * 1000L
                          + (dispatch_end.tv_nsec - diagnostic_last.tv_nsec) / 1000000L;
        if (heartbeat_ms >= 5000) {
            bridge_handoff_stamp("native loop heartbeat", heartbeat_ms);
            fprintf(stderr, "[LIBDECOR HANDOFF DIAG] frames=%u configured=%d\n",
                    w.frames, w.configured);
            diagnostic_last = dispatch_end;
        }
        /* Keep dispatch and shutdown live while paused, but never swap or
         * request a replacement until presentation has resumed. */
        int paused = pause_status ? pause_status(pause_userdata, 0) : 0;
        if (paused) continue;
        if (!policy_attempted && policy_test && policy_test[0] == '1' &&
            w.configured && w.frames > 0 && time(NULL) - started >= 10) {
            policy_attempted = 1;
            if (replace_policy) {
                fprintf(stderr, "[LIBDECOR POLICY TEST] requesting next database policy\n");
                if (replace_policy(policy_userdata) != 0)
                    fprintf(stderr, "[LIBDECOR POLICY TEST] transition failed; previous policy retained\n");
            }
        }
        if (!replacement_attempted && replacement && replacement[0] &&
            w.configured && w.frames > 0 && time(NULL) - started >= 10) {
            replacement_attempted = 1;
            fprintf(stderr, "[LIBDECOR REPLACE] attempting managed shader: %s\n", replacement);
            if (screenshaver_libdecor_rust_gl_replace(replacement) != 0)
                fprintf(stderr, "[LIBDECOR REPLACE] previous shader remains active\n");
        }
        struct timespec paint_begin, paint_end;
        if (w.configured) {
            clock_gettime(CLOCK_MONOTONIC, &paint_begin);
            int paint_result = bridge_test_paint(&w);
            clock_gettime(CLOCK_MONOTONIC, &paint_end);
            long paint_ms = (paint_end.tv_sec - paint_begin.tv_sec) * 1000L
                          + (paint_end.tv_nsec - paint_begin.tv_nsec) / 1000000L;
            if (paint_ms >= 250)
                bridge_handoff_stamp("total draw/swap blocked", paint_ms);
            if (paint_result < 0) {
                fprintf(stderr, "[LIBDECOR BRIDGE TEST] EGL swap failed (error=0x%x)\n", eglGetError());
                w.failed = 1; break;
            }
        }
        if (w.configured && pause_status) (void)pause_status(pause_userdata, 1);
        /* Sleep only the remaining frame budget. The pause callback runs before
         * this bounded sleep and on the next iteration; never sleep while paused.
         * Avoid accumulated deadlines or catch-up bursts after a handoff. */
        if (diagnostic_fps && w.configured) {
            struct timespec now;
            clock_gettime(CLOCK_MONOTONIC, &now);
            long long elapsed_ns = (long long)(now.tv_sec - paint_begin.tv_sec) * 1000000000LL
                                 + (long long)(now.tv_nsec - paint_begin.tv_nsec);
            long long remaining_ns = 1000000000LL / diagnostic_fps - elapsed_ns;
            if (remaining_ns > 0) {
                struct timespec delay = {
                    .tv_sec = (time_t)(remaining_ns / 1000000000LL),
                    .tv_nsec = (long)(remaining_ns % 1000000000LL)
                };
                (void)nanosleep(&delay, NULL);
            }
        }
    }
    result = (w.configured && w.frames > 0 && !w.failed) ? 0 : 1;
cleanup:
    /* Frame must be released before the surface; all proxies before disconnect. */
    if (w.frame) libdecor_frame_unref(w.frame);
    if (w.egl_display != EGL_NO_DISPLAY) {
        if (w.egl_context != EGL_NO_CONTEXT && w.egl_surface != EGL_NO_SURFACE)
            eglMakeCurrent(w.egl_display, w.egl_surface, w.egl_surface, w.egl_context);
        if (w.egl_context != EGL_NO_CONTEXT && w.egl_surface != EGL_NO_SURFACE)
            screenshaver_libdecor_rust_gl_release();
        eglMakeCurrent(w.egl_display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
        if (w.egl_surface != EGL_NO_SURFACE) eglDestroySurface(w.egl_display, w.egl_surface);
        if (w.egl_context != EGL_NO_CONTEXT) eglDestroyContext(w.egl_display, w.egl_context);
        eglTerminate(w.egl_display);
    }
    if (w.egl_window) wl_egl_window_destroy(w.egl_window);
    if (w.surface) wl_surface_destroy(w.surface);
    if (w.compositor) wl_compositor_destroy(w.compositor);
    if (w.registry) wl_registry_destroy(w.registry);
    screenshaver_decor_bridge_destroy(w.bridge);
    bridge_diagnostic_pause_status = NULL;
    bridge_diagnostic_pause_userdata = NULL;
    fprintf(stderr, "[LIBDECOR BRIDGE TEST] %s (configured=%d)\n",
            result ? "FAIL" : "PASS", w.configured);
    return result;
}


/* Legacy diagnostic entrypoint: no Rust policy context. Preserve the
 * previous unpaced behavior unless a diagnostic FPS override is supplied. */
int screenshaver_decor_bridge_run_diagnostic_with_pause(
    int (*should_stop)(void *), void *stop_userdata,
    int (*replace_policy)(void *), void *policy_userdata,
    int (*pause_status)(void *, int), void *pause_userdata) {
    /* Legacy calls retain the old diagnostic default of 60 FPS. */
    return screenshaver_decor_bridge_run_diagnostic_with_pause_and_fps(
        should_stop, stop_userdata, replace_policy, policy_userdata,
        pause_status, pause_userdata, 60);
}

/* Keep the prior C entrypoint and standalone diagnostic behavior intact. */
int screenshaver_decor_bridge_run_diagnostic_with_policy(
    int (*should_stop)(void *), void *stop_userdata,
    int (*replace_policy)(void *), void *policy_userdata) {
    return screenshaver_decor_bridge_run_diagnostic_with_pause(
        should_stop, stop_userdata, replace_policy, policy_userdata, NULL, NULL);
}

int screenshaver_decor_bridge_run_diagnostic_with_stop(
    int (*should_stop)(void *), void *stop_userdata) {
    return screenshaver_decor_bridge_run_diagnostic_with_policy(
        should_stop, stop_userdata, NULL, NULL);
}

/* Preserve standalone diagnostic callers without an external stop source. */
int screenshaver_decor_bridge_run_diagnostic(void) {
    return screenshaver_decor_bridge_run_diagnostic_with_stop(NULL, NULL);
}


/* Stage 3C-6B1: bounded, non-rendering native lifecycle probe.
 * This owns a separate connection and a libdecor surface role. It intentionally
 * creates no EGL objects and does not enter the production shader loop.
 * A compositor may not display the surface without an attached buffer.
 */
int screenshaver_decor_bridge_probe_native_session(void) {
    struct bridge_test_window w = {0};
    w.width = 720; w.height = 420; w.running = 1;
    w.egl_display = EGL_NO_DISPLAY;
    w.egl_context = EGL_NO_CONTEXT;
    w.egl_surface = EGL_NO_SURFACE;
    int result = -1;
    fprintf(stderr, "[LIBDECOR SESSION PROBE] starting native ownership test\n");
    w.bridge = screenshaver_decor_bridge_create_isolated();
    if (!w.bridge || !screenshaver_decor_bridge_dispatch_is_safe(w.bridge)) goto done;
    w.registry = wl_display_get_registry(w.bridge->display);
    if (!w.registry || wl_registry_add_listener(w.registry, &bridge_test_registry_listener, &w) < 0)
        goto done;
    if (wl_display_roundtrip(w.bridge->display) < 0 || !w.compositor) goto done;
    w.surface = wl_compositor_create_surface(w.compositor);
    if (!w.surface) goto done;
    w.frame = libdecor_decorate(w.bridge->context, w.surface,
                               &bridge_test_frame_interface, &w);
    if (!w.frame) goto done;
    libdecor_frame_set_app_id(w.frame, "screenshaver-libdecor-session-probe");
    libdecor_frame_set_title(w.frame, "Screenshaver native session probe");
    libdecor_frame_set_min_content_size(w.frame, 160, 120);
    libdecor_frame_map(w.frame);
    /* The configure callback commits the libdecor configuration. A lack of
     * configure without an EGL buffer is not treated as rendering success. */
    struct timespec begin, now;
    clock_gettime(CLOCK_MONOTONIC, &begin);
    do {
        if (screenshaver_decor_bridge_dispatch(w.bridge, 50) < 0 || w.failed) goto done;
        if (!w.running) break;
        clock_gettime(CLOCK_MONOTONIC, &now);
    } while ((now.tv_sec - begin.tv_sec) * 1000L +
             (now.tv_nsec - begin.tv_nsec) / 1000000L < 3000L);
    result = 0;
done:
    if (w.frame) libdecor_frame_unref(w.frame);
    if (w.surface) wl_surface_destroy(w.surface);
    if (w.compositor) wl_compositor_destroy(w.compositor);
    if (w.registry) wl_registry_destroy(w.registry);
    screenshaver_decor_bridge_destroy(w.bridge);
    fprintf(stderr, "[LIBDECOR SESSION PROBE] %s (configured=%d, no EGL)\n",
            result == 0 ? "PASS" : "FAIL", w.configured);
    return result;
}
