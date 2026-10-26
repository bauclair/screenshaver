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
/* Stage 3C-5B: Rust owns all GL objects; C owns EGL and presentation. */
extern int screenshaver_libdecor_rust_gl_initialize(void);
extern int screenshaver_libdecor_rust_gl_draw(int width, int height, unsigned frame);
extern void screenshaver_libdecor_rust_gl_release(void);
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
    if (screenshaver_libdecor_rust_gl_initialize() != 0) return -1;
    fprintf(stderr, "[LIBDECOR BRIDGE TEST] EGL/desktop OpenGL initialized: %s\n",
            (const char *)glGetString(GL_VERSION));
    return 0;
}

static int bridge_test_paint(struct bridge_test_window *w) {
    if (!w->configured || w->egl_surface == EGL_NO_SURFACE) return -1;
    if (screenshaver_libdecor_rust_gl_draw(w->width, w->height, w->frames) != 0) return -1;
    if (!eglSwapBuffers(w->egl_display, w->egl_surface)) return -1;
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

/* Interactive test: close the window to exit; a 120-second limit prevents
 * indefinitely hanging unattended diagnostics. */
int screenshaver_decor_bridge_run_diagnostic(void) {
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
    while (w.running) {
        if (screenshaver_decor_bridge_dispatch(w.bridge, 50) < 0) {
            w.failed = 1; break;
        }
        if (w.configured && bridge_test_paint(&w) < 0) {
            fprintf(stderr, "[LIBDECOR BRIDGE TEST] EGL swap failed (error=0x%x)\n", eglGetError());
            w.failed = 1; break;
        }
        if (time(NULL) - started >= 120) {
            fprintf(stderr, "[LIBDECOR BRIDGE TEST] timed out waiting for close\n");
            w.failed = 1; break;
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
    fprintf(stderr, "[LIBDECOR BRIDGE TEST] %s (configured=%d)\n",
            result ? "FAIL" : "PASS", w.configured);
    return result;
}
