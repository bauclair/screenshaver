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
#include <dlfcn.h>

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
    struct wl_seat *input_seat;
    struct wl_pointer *input_pointer;
    struct wl_shm *cursor_shm;
    struct wl_surface *cursor_surface;
    void *cursor_library;
    void *cursor_theme;
    uint32_t pointer_enter_serial;
    int pointer_cursor_kind; /* -1 unknown, 0 arrow, 1 hand, 2 horizontal resize */
    int pointer_on_surface;
    /* Stage 28C2: no pointer callback runs outside the owning dispatch thread. */
    double pointer_x, pointer_y;
    struct wl_surface *surface;
    struct libdecor_frame *frame;
    EGLDisplay egl_display;
    EGLContext egl_context;
    EGLSurface egl_surface;
    struct wl_egl_window *egl_window;
    struct wl_callback *workspace_frame_callback;
    unsigned workspace_frame_requests;
    unsigned workspace_frame_done;
    struct timespec workspace_last_done_at;
    int workspace_have_done;
    unsigned frames;
    int width, height;
    int normal_width, normal_height;
    int maximized, maximized_known;
    int configured, running, failed;
    int diagnostic_mode;
};

/* Stage 15: compositor frame callbacks gate rotation eligibility.
 * A callback is not a universal visibility guarantee, but Mango withholds
 * callbacks while the surface is hidden. Drawing remains independent. */
static void bridge_workspace_frame_done(void *data, struct wl_callback *callback,
                                        uint32_t callback_time) {
    struct bridge_test_window *w = data;
    (void)callback_time;
    if (w->workspace_frame_callback == callback)
        w->workspace_frame_callback = NULL;
    ++w->workspace_frame_done;
    clock_gettime(CLOCK_MONOTONIC, &w->workspace_last_done_at);
    w->workspace_have_done = 1;
    wl_callback_destroy(callback);
}
static const struct wl_callback_listener bridge_workspace_frame_listener = {
    .done = bridge_workspace_frame_done
};

static int (*bridge_lyrics_pointer_event)(double, double, int) = NULL;
/* Rust owns lyrics hit-testing and MPRIS dispatch on this same native thread. */
extern int screenshaver_libdecor_rust_lyrics_pointer(double, double, int);
/* Stage 28C2B: cursor theme is loaded at runtime to avoid changing Cargo's
 * existing native link configuration. Wayland owns the cursor wl_surface;
 * the native libdecor connection remains the sole event dispatcher. */
struct bridge_cursor_image { uint32_t width, height, hotspot_x, hotspot_y, delay; };
struct bridge_cursor { unsigned image_count; struct bridge_cursor_image **images; char *name; };
typedef void *(*bridge_theme_load_fn)(const char *, int, struct wl_shm *);
typedef void (*bridge_theme_destroy_fn)(void *);
typedef struct bridge_cursor *(*bridge_theme_get_fn)(void *, const char *);
typedef struct wl_buffer *(*bridge_image_buffer_fn)(struct bridge_cursor_image *);

static void bridge_cursor_cleanup(struct bridge_test_window *w) {
    if (w->cursor_surface) { wl_surface_destroy(w->cursor_surface); w->cursor_surface = NULL; }
    if (w->cursor_theme && w->cursor_library) {
        bridge_theme_destroy_fn destroy = (bridge_theme_destroy_fn)dlsym(w->cursor_library, "wl_cursor_theme_destroy");
        if (destroy) destroy(w->cursor_theme);
        w->cursor_theme = NULL;
    }
    if (w->cursor_library) { dlclose(w->cursor_library); w->cursor_library = NULL; }
}

static void bridge_cursor_apply(struct bridge_test_window *w, int kind) {
    if (!w->input_pointer || !w->pointer_on_surface || !w->pointer_enter_serial) return;
    if (w->pointer_cursor_kind == kind) return;
    if (!w->cursor_library || !w->cursor_theme || !w->cursor_surface) return;
    bridge_theme_get_fn get = (bridge_theme_get_fn)dlsym(w->cursor_library, "wl_cursor_theme_get_cursor");
    bridge_image_buffer_fn buffer = (bridge_image_buffer_fn)dlsym(w->cursor_library, "wl_cursor_image_get_buffer");
    if (!get || !buffer) return;
    const char *const *names;
    static const char *arrow[] = {"left_ptr", "default", NULL};
    static const char *hand[] = {"pointer", "hand1", "hand2", NULL};
    static const char *resize[] = {"ew-resize", "sb_h_double_arrow", "size_hor", NULL};
    names = kind == 1 ? hand : (kind == 2 ? resize : arrow);
    struct bridge_cursor *cursor = NULL;
    for (unsigned i = 0; names[i]; ++i) {
        cursor = get(w->cursor_theme, names[i]);
        if (cursor && cursor->image_count && cursor->images && cursor->images[0]) break;
        cursor = NULL;
    }
    if (!cursor) return;
    struct bridge_cursor_image *img = cursor->images[0];
    struct wl_buffer *buf = buffer(img);
    if (!buf) return;
    wl_surface_attach(w->cursor_surface, buf, 0, 0);
    wl_surface_damage(w->cursor_surface, 0, 0, (int)img->width, (int)img->height);
    wl_surface_commit(w->cursor_surface);
    wl_pointer_set_cursor(w->input_pointer, w->pointer_enter_serial,
                          w->cursor_surface, (int)img->hotspot_x, (int)img->hotspot_y);
    w->pointer_cursor_kind = kind;
    if (w->diagnostic_mode)
        fprintf(stderr, "[LIBDECOR CURSOR] %s\n", kind == 1 ? "hand" : kind == 2 ? "horizontal-resize" : "arrow");
}

static void bridge_cursor_refresh(struct bridge_test_window *w) {
    if (!w->pointer_on_surface) return;
    int region = bridge_lyrics_pointer_event ? bridge_lyrics_pointer_event(w->pointer_x, w->pointer_y, 0) : 0;
    bridge_cursor_apply(w, region == 1 ? 1 : region == 2 ? 2 : 0);
}

static void bridge_cursor_initialize(struct bridge_test_window *w) {
    if (!w->cursor_shm || !w->compositor || w->cursor_library) return;
    w->cursor_library = dlopen("libwayland-cursor.so.0", RTLD_NOW | RTLD_LOCAL);
    if (!w->cursor_library) {
        fprintf(stderr, "[LIBDECOR CURSOR] libwayland-cursor unavailable; leaving compositor cursor unchanged\n");
        return;
    }
    bridge_theme_load_fn load = (bridge_theme_load_fn)dlsym(w->cursor_library, "wl_cursor_theme_load");
    if (!load) { bridge_cursor_cleanup(w); return; }
    w->cursor_theme = load(NULL, 24, w->cursor_shm);
    if (!w->cursor_theme) { bridge_cursor_cleanup(w); return; }
    w->cursor_surface = wl_compositor_create_surface(w->compositor);
    if (!w->cursor_surface) { bridge_cursor_cleanup(w); return; }
    w->pointer_cursor_kind = -1;
    fprintf(stderr, "[LIBDECOR CURSOR] theme ready\n");
}

/* Stage 28C2: the callback is installed only for the native production session.
 * 1=left-button press, 0=pointer motion. Returns 0 when not handled. */
void screenshaver_decor_bridge_set_lyrics_pointer_event(
    int (*callback)(double, double, int)) {
    bridge_lyrics_pointer_event = callback;
}

/* Stage 28C1: pointer acquisition and surface-local event diagnostics only.
 * The native connection is the sole dispatcher. No MPRIS actions are issued. */
static void bridge_pointer_enter(void *data, struct wl_pointer *pointer,
                                 uint32_t serial, struct wl_surface *surface,
                                 wl_fixed_t x, wl_fixed_t y) {
    struct bridge_test_window *w = data;
    (void)pointer;
    w->pointer_enter_serial = serial;
    w->pointer_cursor_kind = -1;
    w->pointer_on_surface = (surface == w->surface);
    if (w->pointer_on_surface) {
        w->pointer_x = wl_fixed_to_double(x);
        w->pointer_y = wl_fixed_to_double(y);
        if (w->diagnostic_mode)
            fprintf(stderr, "[LIBDECOR POINTER] enter x=%.1f y=%.1f\n", w->pointer_x, w->pointer_y);
        bridge_cursor_refresh(w);
    }
}
static void bridge_pointer_leave(void *data, struct wl_pointer *pointer,
                                 uint32_t serial, struct wl_surface *surface) {
    struct bridge_test_window *w = data;
    (void)pointer; (void)serial;
    if (surface == w->surface) {
        w->pointer_on_surface = 0;
        w->pointer_enter_serial = 0;
        w->pointer_cursor_kind = -1;
        if (w->diagnostic_mode)
            fprintf(stderr, "[LIBDECOR POINTER] leave\n");
    }
}
static void bridge_pointer_motion(void *data, struct wl_pointer *pointer,
                                  uint32_t time, wl_fixed_t x, wl_fixed_t y) {
    struct bridge_test_window *w = data;
    (void)pointer; (void)time;
    if (w->pointer_on_surface) {
        w->pointer_x = wl_fixed_to_double(x);
        w->pointer_y = wl_fixed_to_double(y);
        bridge_cursor_refresh(w);
    }
}
static void bridge_pointer_button(void *data, struct wl_pointer *pointer,
                                  uint32_t serial, uint32_t time,
                                  uint32_t button, uint32_t state) {
    struct bridge_test_window *w = data;
    (void)pointer; (void)serial; (void)time;
    if (w->pointer_on_surface && button == 0x110 && state == WL_POINTER_BUTTON_STATE_PRESSED
        && bridge_lyrics_pointer_event) {
        (void)bridge_lyrics_pointer_event(w->pointer_x, w->pointer_y, 1);
    }
    if (w->pointer_on_surface && w->diagnostic_mode)
        fprintf(stderr, "[LIBDECOR POINTER] button=%u state=%u x=%.1f y=%.1f\n",
                button, state, w->pointer_x, w->pointer_y);
}
static void bridge_pointer_axis(void *data, struct wl_pointer *pointer,
                                uint32_t time, uint32_t axis, wl_fixed_t value) {
    (void)data; (void)pointer; (void)time; (void)axis; (void)value;
}
/* wl_pointer version 5 can emit frame/axis metadata events even when
 * Screenshaver does not use scrolling. Every advertised event needs a handler. */
static void bridge_pointer_frame(void *data, struct wl_pointer *pointer) {
    (void)data; (void)pointer;
}
static void bridge_pointer_axis_source(void *data, struct wl_pointer *pointer,
                                       uint32_t source) {
    (void)data; (void)pointer; (void)source;
}
static void bridge_pointer_axis_stop(void *data, struct wl_pointer *pointer,
                                     uint32_t time, uint32_t axis) {
    (void)data; (void)pointer; (void)time; (void)axis;
}
static void bridge_pointer_axis_discrete(void *data, struct wl_pointer *pointer,
                                         uint32_t axis, int32_t discrete) {
    (void)data; (void)pointer; (void)axis; (void)discrete;
}
static const struct wl_pointer_listener bridge_pointer_listener = {
    .enter = bridge_pointer_enter, .leave = bridge_pointer_leave,
    .motion = bridge_pointer_motion, .button = bridge_pointer_button,
    .axis = bridge_pointer_axis,
    .frame = bridge_pointer_frame,
    .axis_source = bridge_pointer_axis_source,
    .axis_stop = bridge_pointer_axis_stop,
    .axis_discrete = bridge_pointer_axis_discrete
};
static void bridge_seat_capabilities(void *data, struct wl_seat *seat, uint32_t capabilities) {
    struct bridge_test_window *w = data;
    if ((capabilities & WL_SEAT_CAPABILITY_POINTER) && !w->input_pointer) {
        w->input_pointer = wl_seat_get_pointer(seat);
        if (w->input_pointer && wl_pointer_add_listener(w->input_pointer, &bridge_pointer_listener, w) == 0)
            fprintf(stderr, "[LIBDECOR POINTER] acquired\n");
        else {
            if (w->input_pointer) wl_pointer_destroy(w->input_pointer);
            w->input_pointer = NULL;
            fprintf(stderr, "[LIBDECOR POINTER] acquisition failed\n");
        }
    } else if (!(capabilities & WL_SEAT_CAPABILITY_POINTER) && w->input_pointer) {
        if (wl_pointer_get_version(w->input_pointer) >= 3) wl_pointer_release(w->input_pointer);
        else wl_pointer_destroy(w->input_pointer);
        w->input_pointer = NULL;
        w->pointer_on_surface = 0;
        fprintf(stderr, "[LIBDECOR POINTER] capability removed\n");
    }
}
static void bridge_seat_name(void *data, struct wl_seat *seat, const char *name) {
    (void)data; (void)seat; (void)name;
}
static const struct wl_seat_listener bridge_seat_listener = {
    .capabilities = bridge_seat_capabilities, .name = bridge_seat_name
};

static void bridge_test_global(void *data, struct wl_registry *registry,
                               uint32_t name, const char *interface, uint32_t version) {
    struct bridge_test_window *w = data;
    if (!strcmp(interface, wl_shm_interface.name) && !w->cursor_shm)
        w->cursor_shm = wl_registry_bind(registry, name, &wl_shm_interface, 1);
    if (!strcmp(interface, wl_seat_interface.name) && !w->input_seat) {
        w->input_seat = wl_registry_bind(registry, name, &wl_seat_interface,
                                         version < 5 ? version : 5);
        if (w->input_seat && wl_seat_add_listener(w->input_seat, &bridge_seat_listener, w) < 0) {
            wl_seat_destroy(w->input_seat);
            w->input_seat = NULL;
        }
    }
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
    const char *swap_interval = w->diagnostic_mode ? getenv("SCREENSHAVER_LIBDECOR_DIAG_SWAP_INTERVAL") : NULL;
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
    } else if (!w->diagnostic_mode) {
        /* Production parity with the Stage 28 validated launch: keep the
         * Wayland event dispatcher responsive rather than letting EGL's
         * default swap interval introduce driver-dependent blocking.  The
         * existing policy FPS budget still controls application pacing. */
        if (!eglSwapInterval(w->egl_display, 0)) {
            fprintf(stderr, "[LIBDECOR EGL] production swap interval 0 rejected (error=0x%x); retaining driver default\n", eglGetError());
        } else {
            fprintf(stderr, "[LIBDECOR EGL] production swap interval 0 accepted\n");
        }
    } else {
        fprintf(stderr, "[LIBDECOR EGL DIAG] swap interval unchanged (EGL default)\n");
    }
    if (screenshaver_libdecor_rust_gl_initialize() != 0) return -1;
    fprintf(stderr, "[%s] EGL/desktop OpenGL initialized: %s\n",
            w->diagnostic_mode ? "LIBDECOR BRIDGE TEST" : "LIBDECOR WINDOWSHADER",
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
    enum libdecor_window_state window_state;
    if (libdecor_configuration_get_window_state(configuration, &window_state)) {
        w->maximized = !!(window_state & LIBDECOR_WINDOW_STATE_MAXIMIZED);
        w->maximized_known = 1;
    }
    if (!w->maximized && w->width >= 64 && w->height >= 64) {
        w->normal_width = w->width;
        w->normal_height = w->height;
    }
    struct libdecor_state *state = libdecor_state_new(w->width, w->height);
    if (!state) { w->failed = 1; w->running = 0; return; }
    libdecor_frame_commit(frame, state, configuration);
    libdecor_state_free(state);
    if (w->width < 64 || w->height < 64 || w->width > 4096 || w->height > 4096) {
        w->failed = 1; w->running = 0; return;
    }
    if (w->egl_window) wl_egl_window_resize(w->egl_window, w->width, w->height, 0, 0);
    w->configured = 1;
    if (w->diagnostic_mode)
        fprintf(stderr, "[LIBDECOR BRIDGE TEST] configure: %d x %d\n", w->width, w->height);
}
static void bridge_test_close(struct libdecor_frame *frame, void *data) {
    (void)frame;
    struct bridge_test_window *w = data;
    fprintf(stderr, "[%s] close requested\n",
            w->diagnostic_mode ? "LIBDECOR BRIDGE TEST" : "LIBDECOR WINDOWSHADER");
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

/* Stage 17: shared diagnostic lifecycle. Rendering loops remain independent. */
static void bridge_diagnostic_window_defaults(struct bridge_test_window *w) {
    memset(w, 0, sizeof(*w));
    w->width = 720; w->height = 420; w->running = 1;
    w->egl_display = EGL_NO_DISPLAY;
    w->egl_context = EGL_NO_CONTEXT;
    w->egl_surface = EGL_NO_SURFACE;
}

static int bridge_diagnostic_window_open(struct bridge_test_window *w, int production_windowshader, const char *production_title) {
    fprintf(stderr, "[%s] connecting native bridge\n",
            w->diagnostic_mode ? "LIBDECOR BRIDGE TEST" : "LIBDECOR WINDOWSHADER");
    w->bridge = screenshaver_decor_bridge_create_isolated();
    if (!w->bridge || !screenshaver_decor_bridge_dispatch_is_safe(w->bridge)) return -1;
    w->registry = wl_display_get_registry(w->bridge->display);
    if (!w->registry || wl_registry_add_listener(w->registry, &bridge_test_registry_listener, w) < 0)
        return -1;
    if (wl_display_roundtrip(w->bridge->display) < 0 || !w->compositor) {
        fprintf(stderr, "[%s] compositor unavailable\n",
                w->diagnostic_mode ? "LIBDECOR BRIDGE TEST" : "LIBDECOR WINDOWSHADER");
        return -1;
    }
    bridge_cursor_initialize(w);
    w->surface = wl_compositor_create_surface(w->compositor);
    if (!w->surface) return -1;
    w->frame = libdecor_decorate(w->bridge->context, w->surface,
                                &bridge_test_frame_interface, w);
    if (!w->frame) return -1;
    libdecor_frame_set_app_id(w->frame, production_windowshader ? "screenshaver" : "screenshaver-libdecor-bridge");
    libdecor_frame_set_title(w->frame, production_windowshader ? (production_title ? production_title : "P: Windowshader") : "Screenshaver libdecor bridge - drag / resize / close");
    libdecor_frame_set_min_content_size(w->frame, 160, 120);
    if (bridge_egl_initialize(w) < 0) {
        fprintf(stderr, "[%s] EGL setup failed (error=0x%x)\n",
                w->diagnostic_mode ? "LIBDECOR BRIDGE TEST" : "LIBDECOR WINDOWSHADER", eglGetError());
        return -1;
    }
    libdecor_frame_map(w->frame);
    if (w->diagnostic_mode)
        fprintf(stderr, "[LIBDECOR BRIDGE TEST] window requested; drag, resize, then close\n");
    else
        fprintf(stderr, "[LIBDECOR WINDOWSHADER] window requested\n");
    return 0;
}

static int bridge_diagnostic_fps_initialize(struct bridge_test_window *w,
                                             unsigned configured_fps,
                                             const char *fps_env,
                                             unsigned *diagnostic_fps) {
    *diagnostic_fps = configured_fps;
    if (*diagnostic_fps < 1 || *diagnostic_fps > 240) {
        fprintf(stderr, "[LIBDECOR FPS] invalid configured FPS %u (expected 1..240)\n", *diagnostic_fps);
        w->failed = 1;
        return -1;
    }
    fprintf(stderr, "[LIBDECOR FPS] Rust-resolved policy FPS: %u\n", *diagnostic_fps);
    if (fps_env && fps_env[0]) {
        char *end = NULL;
        unsigned long parsed = strtoul(fps_env, &end, 10);
        if (!end || *end != '\0' || parsed < 1 || parsed > 240) {
            fprintf(stderr, "[LIBDECOR FPS DIAG] invalid FPS '%s' (expected 1..240)\n", fps_env);
            w->failed = 1;
            return -1;
        }
        *diagnostic_fps = (unsigned)parsed;
        fprintf(stderr, "[LIBDECOR FPS DIAG] diagnostic frame pacing enabled: %u FPS\n", *diagnostic_fps);
    } else {
        fprintf(stderr, "[LIBDECOR FPS DIAG] no diagnostic override; using Rust-resolved %u FPS\n", *diagnostic_fps);
    }
    return 0;
}

static void bridge_diagnostic_window_close(struct bridge_test_window *w,
                                            int result, int workspace_diag) {
    /* Preserve the existing destruction order and completion diagnostics. */
    if (w->frame) libdecor_frame_unref(w->frame);
    if (w->egl_display != EGL_NO_DISPLAY) {
        if (w->egl_context != EGL_NO_CONTEXT && w->egl_surface != EGL_NO_SURFACE)
            eglMakeCurrent(w->egl_display, w->egl_surface, w->egl_surface, w->egl_context);
        if (w->egl_context != EGL_NO_CONTEXT && w->egl_surface != EGL_NO_SURFACE)
            screenshaver_libdecor_rust_gl_release();
        eglMakeCurrent(w->egl_display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
        if (w->egl_surface != EGL_NO_SURFACE) eglDestroySurface(w->egl_display, w->egl_surface);
        if (w->egl_context != EGL_NO_CONTEXT) eglDestroyContext(w->egl_display, w->egl_context);
        eglTerminate(w->egl_display);
    }
    if (w->egl_window) wl_egl_window_destroy(w->egl_window);
    if (w->workspace_frame_callback) {
        wl_callback_destroy(w->workspace_frame_callback);
        w->workspace_frame_callback = NULL;
    }
    if (workspace_diag)
        fprintf(stderr, "[LIBDECOR WORKSPACE DIAG] final frame_requests=%u frame_done=%u (visibility unknown)\n",
                w->workspace_frame_requests, w->workspace_frame_done);
    bridge_cursor_cleanup(w);
    if (w->input_pointer) {
        if (wl_pointer_get_version(w->input_pointer) >= 3) wl_pointer_release(w->input_pointer);
        else wl_pointer_destroy(w->input_pointer);
        w->input_pointer = NULL;
    }
    if (w->input_seat) {
        if (wl_seat_get_version(w->input_seat) >= 5) wl_seat_release(w->input_seat);
        else wl_seat_destroy(w->input_seat);
        w->input_seat = NULL;
    }
    if (w->cursor_shm) wl_shm_destroy(w->cursor_shm);
    if (w->surface) wl_surface_destroy(w->surface);
    if (w->compositor) wl_compositor_destroy(w->compositor);
    if (w->registry) wl_registry_destroy(w->registry);
    screenshaver_decor_bridge_destroy(w->bridge);
    bridge_lyrics_pointer_event = NULL;
    bridge_diagnostic_pause_status = NULL;
    bridge_diagnostic_pause_userdata = NULL;
    fprintf(stderr, "[%s] %s (configured=%d)\n",
            w->diagnostic_mode ? "LIBDECOR BRIDGE TEST" : "LIBDECOR WINDOWSHADER",
            result ? "FAIL" : "PASS", w->configured);
}

/* Stage 18: common post-transition FPS resolution.  Preserve the older
 * rotation-specific diagnostics while sharing validation and cadence updates. */
static void bridge_diagnostic_update_policy_fps(
    int (*resolve_fps)(void *, unsigned *), void *policy_userdata,
    const char *fps_env, unsigned *diagnostic_fps, int rotation_path) {
    if (!resolve_fps) return;
    unsigned next_fps = 0;
    if (resolve_fps(policy_userdata, &next_fps) == 0 &&
        next_fps >= 1 && next_fps <= 240) {
        if (!fps_env || !fps_env[0]) {
            *diagnostic_fps = next_fps;
            fprintf(stderr, "[LIBDECOR FPS] active cadence updated to %u FPS\n", *diagnostic_fps);
        } else if (rotation_path) {
            fprintf(stderr, "[LIBDECOR FPS DIAG] override retained at %u FPS\n", *diagnostic_fps);
        } else {
            fprintf(stderr, "[LIBDECOR FPS DIAG] override retained at %u FPS (policy resolves %u FPS)\n",
                    *diagnostic_fps, next_fps);
        }
    } else if (rotation_path) {
        fprintf(stderr, "[LIBDECOR FPS] resolution failed; previous cadence retained\n");
    } else {
        fprintf(stderr, "[LIBDECOR FPS] replacement FPS resolution failed; retaining %u FPS\n", *diagnostic_fps);
    }
}

/* Stage 20: identical one-shot diagnostic transition in both native loops.
 * This is deliberately independent of automatic callback-driven rotation. */
static void bridge_diagnostic_one_shot_policy_transition(
    struct bridge_test_window *w, time_t started,
    const char *policy_test, int *policy_attempted,
    int (*replace_policy)(void *), void *policy_userdata,
    int (*resolve_fps)(void *, unsigned *), const char *fps_env,
    unsigned *diagnostic_fps) {
    if (*policy_attempted || !policy_test || policy_test[0] != '1' ||
        !w->configured || w->frames == 0 || time(NULL) - started < 10)
        return;
    *policy_attempted = 1;
    if (replace_policy) {
        fprintf(stderr, "[LIBDECOR POLICY TEST] requesting next database policy\n");
        if (replace_policy(policy_userdata) != 0) {
            fprintf(stderr, "[LIBDECOR POLICY TEST] transition failed or unchanged; retaining FPS\n");
        } else if (resolve_fps) {
            bridge_diagnostic_update_policy_fps(
                resolve_fps, policy_userdata, fps_env, diagnostic_fps, 0);
        }
    }
}

/* Stage 21: shared opt-in managed-shader replacement diagnostic.
 * This is independent of policy rotation and is attempted at most once. */
static void bridge_diagnostic_managed_shader_replacement(
    struct bridge_test_window *w, time_t started,
    const char *replacement, int *replacement_attempted) {
    if (*replacement_attempted || !replacement || !replacement[0] ||
        !w->configured || w->frames == 0 || time(NULL) - started < 10)
        return;
    *replacement_attempted = 1;
    fprintf(stderr, "[LIBDECOR REPLACE] attempting managed shader: %s\n", replacement);
    if (screenshaver_libdecor_rust_gl_replace(replacement) != 0)
        fprintf(stderr, "[LIBDECOR REPLACE] previous shader remains active\n");
}

/* Interactive diagnostic: continue until explicit shutdown or window close. */
/* Stage 19: shared diagnostic bookkeeping; no scheduler or gate ownership. */
static int bridge_diagnostic_dispatch_heartbeat(struct bridge_test_window *w,
                                                struct timespec *diagnostic_last) {
    struct timespec dispatch_begin, dispatch_end;
    clock_gettime(CLOCK_MONOTONIC, &dispatch_begin);
    int dispatch_result = screenshaver_decor_bridge_dispatch(w->bridge, 50);
    bridge_cursor_refresh(w);
    clock_gettime(CLOCK_MONOTONIC, &dispatch_end);
    long dispatch_ms = (dispatch_end.tv_sec - dispatch_begin.tv_sec) * 1000L
                     + (dispatch_end.tv_nsec - dispatch_begin.tv_nsec) / 1000000L;
    if (dispatch_ms >= 250)
        bridge_handoff_stamp("libdecor dispatch blocked", dispatch_ms);
    if (dispatch_result < 0) return -1;
    long heartbeat_ms = (dispatch_end.tv_sec - diagnostic_last->tv_sec) * 1000L
                      + (dispatch_end.tv_nsec - diagnostic_last->tv_nsec) / 1000000L;
    if (heartbeat_ms >= 5000) {
        /* Routine heartbeat output belongs to the isolated diagnostic mode.
         * Production still dispatches events, refreshes the cursor, and
         * records heartbeat timing; blocking-dispatch warnings remain active. */
        if (w->diagnostic_mode) {
            bridge_handoff_stamp("native loop heartbeat", heartbeat_ms);
            fprintf(stderr, "[LIBDECOR HANDOFF DIAG] frames=%u configured=%d\n",
                    w->frames, w->configured);
        }
        *diagnostic_last = dispatch_end;
    }
    return 0;
}

static void bridge_diagnostic_log_paint_duration(const struct timespec *begin,
                                                  const struct timespec *end) {
    long paint_ms = (end->tv_sec - begin->tv_sec) * 1000L
                  + (end->tv_nsec - begin->tv_nsec) / 1000000L;
    if (paint_ms >= 250)
        bridge_handoff_stamp("total draw/swap blocked", paint_ms);
}

static void bridge_diagnostic_sleep_frame_budget(unsigned diagnostic_fps,
                                                  int configured,
                                                  const struct timespec *paint_begin) {
    if (!diagnostic_fps || !configured) return;
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    long long elapsed_ns = (long long)(now.tv_sec - paint_begin->tv_sec) * 1000000000LL
                         + (long long)(now.tv_nsec - paint_begin->tv_nsec);
    long long remaining_ns = 1000000000LL / diagnostic_fps - elapsed_ns;
    if (remaining_ns > 0) {
        struct timespec delay = {
            .tv_sec = (time_t)(remaining_ns / 1000000000LL),
            .tv_nsec = (long)(remaining_ns % 1000000000LL)
        };
        (void)nanosleep(&delay, NULL);
    }
}

int screenshaver_decor_bridge_run_diagnostic_with_dynamic_fps(
    int (*should_stop)(void *), void *stop_userdata,
    int (*replace_policy)(void *), void *policy_userdata,
    int (*pause_status)(void *, int), void *pause_userdata,
    unsigned configured_fps,
    int (*resolve_fps)(void *, unsigned *)) {
    bridge_diagnostic_pause_status = pause_status;
    bridge_diagnostic_pause_userdata = pause_userdata;
    struct bridge_test_window w;
    bridge_diagnostic_window_defaults(&w);
    int result = 1;
    if (bridge_diagnostic_window_open(&w, 0, NULL) < 0) goto cleanup;
    time_t started = time(NULL);
    /* Stage 3C-6D2C-6: bounded, opt-in diagnostic frame pacing.
     * Rust supplies policy-resolved FPS; diagnostic override is optional. */
    const char *fps_env = getenv("SCREENSHAVER_LIBDECOR_DIAG_FPS");
    unsigned diagnostic_fps;
    if (bridge_diagnostic_fps_initialize(&w, configured_fps, fps_env,
                                         &diagnostic_fps) < 0) goto cleanup;
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
        if (bridge_diagnostic_dispatch_heartbeat(&w, &diagnostic_last) < 0) {
            w.failed = 1; break;
        }
        /* Keep dispatch and shutdown live while paused, but never swap or
         * request a replacement until presentation has resumed. */
        int paused = pause_status ? pause_status(pause_userdata, 0) : 0;
        if (paused) continue;
        bridge_diagnostic_one_shot_policy_transition(
            &w, started, policy_test, &policy_attempted,
            replace_policy, policy_userdata, resolve_fps, fps_env,
            &diagnostic_fps);
        bridge_diagnostic_managed_shader_replacement(
            &w, started, replacement, &replacement_attempted);
        struct timespec paint_begin, paint_end;
        if (w.configured) {
            clock_gettime(CLOCK_MONOTONIC, &paint_begin);
            int paint_result = bridge_test_paint(&w);
            clock_gettime(CLOCK_MONOTONIC, &paint_end);
            bridge_diagnostic_log_paint_duration(&paint_begin, &paint_end);
            if (paint_result < 0) {
                fprintf(stderr, "[LIBDECOR BRIDGE TEST] EGL swap failed (error=0x%x)\n", eglGetError());
                w.failed = 1; break;
            }
        }
        if (w.configured && pause_status) (void)pause_status(pause_userdata, 1);
        /* Sleep only the remaining frame budget. The pause callback runs before
         * this bounded sleep and on the next iteration; never sleep while paused.
         * Avoid accumulated deadlines or catch-up bursts after a handoff. */
        bridge_diagnostic_sleep_frame_budget(diagnostic_fps, w.configured, &paint_begin);
    }
    result = (w.configured && w.frames > 0 && !w.failed) ? 0 : 1;
cleanup:
    bridge_diagnostic_window_close(&w, result, 0);
    return result;
}


/* Preserve the prior entrypoint and callback ABI for existing callers. */
int screenshaver_decor_bridge_run_diagnostic_with_pause_and_fps(
    int (*should_stop)(void *), void *stop_userdata,
    int (*replace_policy)(void *), void *policy_userdata,
    int (*pause_status)(void *, int), void *pause_userdata,
    unsigned configured_fps) {
    return screenshaver_decor_bridge_run_diagnostic_with_dynamic_fps(
        should_stop, stop_userdata, replace_policy, policy_userdata,
        pause_status, pause_userdata, configured_fps, NULL);
}
static int bridge_run_windowshader_with_rotation(
    int diagnostic_mode,
    int (*should_stop)(void *), void *stop_userdata,
    int (*replace_policy)(void *), void *policy_userdata,
    int (*pause_status)(void *, int), void *pause_userdata,
    unsigned configured_fps,
    int (*resolve_fps)(void *, unsigned *),
    uint64_t rotation_interval_ns,
    int initial_width, int initial_height, int initial_maximized,
    int *final_width, int *final_height, int *final_maximized,
    const char *initial_title, const char *(*current_title)(void *)) {
    /* Stage 23: dimensions are exchanged only for the native rotation path. */
    if (final_width) *final_width = 0;
    if (final_height) *final_height = 0;
    if (final_maximized) *final_maximized = -1;
    bridge_diagnostic_pause_status = pause_status;
    bridge_diagnostic_pause_userdata = pause_userdata;
    struct bridge_test_window w;
    bridge_diagnostic_window_defaults(&w);
    w.diagnostic_mode = diagnostic_mode;
    /* Connect the pointer listener to the Rust lyrics interaction handler.
     * Both production and rotation diagnostics use the same native surface.
     * The close path clears this callback before the next session. */
    screenshaver_decor_bridge_set_lyrics_pointer_event(
        screenshaver_libdecor_rust_lyrics_pointer);
    fprintf(stderr, "[LIBDECOR LYRICS INPUT] native pointer callback connected\n");
    if (initial_width >= 64 && initial_width <= 4096 &&
        initial_height >= 64 && initial_height <= 4096) {
        w.width = initial_width;
        w.height = initial_height;
        w.normal_width = initial_width;
        w.normal_height = initial_height;
        fprintf(stderr, "[LIBDECOR GEOMETRY] restored requested size: %d x %d\n",
                w.width, w.height);
    }
    int result = 1;
    if (bridge_diagnostic_window_open(&w, 1, initial_title) < 0) goto cleanup;
    if (initial_maximized) {
        libdecor_frame_set_maximized(w.frame);
        fprintf(stderr, "[LIBDECOR MAXIMIZE] requested saved maximized state\n");
    }
    time_t started = time(NULL);
    /* Stage 3C-6D2C-6: bounded, opt-in diagnostic frame pacing.
     * Rust supplies policy-resolved FPS; diagnostic override is optional. */
    const char *fps_env = diagnostic_mode ? getenv("SCREENSHAVER_LIBDECOR_DIAG_FPS") : NULL;
    unsigned diagnostic_fps;
    if (bridge_diagnostic_fps_initialize(&w, configured_fps, fps_env,
                                         &diagnostic_fps) < 0) goto cleanup;
    /* Stage 3C-6D2B: explicitly opted-in one-shot policy transition.
     * The callback runs on the EGL owner thread, between draw calls. */
    const char *policy_test = diagnostic_mode ? getenv("SCREENSHAVER_LIBDECOR_TEST_POLICY_TRANSITION") : NULL;
    int policy_attempted = 0;
    /* Diagnostic only: one manually requested replacement after ten seconds.
     * This is not the policy scheduler and does not alter the native window. */
    const char *replacement = diagnostic_mode ? getenv("SCREENSHAVER_LIBDECOR_REPLACEMENT_SHADER") : NULL;
    int replacement_attempted = 0;
    /* Stage 3C-6D2C-3: native loop diagnostics, no behavior change. */
    struct timespec diagnostic_last;
    clock_gettime(CLOCK_MONOTONIC, &diagnostic_last);
    /* Active-presentation clock: screensaver pauses do not consume rotation time. */
    struct timespec rotation_tick;
    clock_gettime(CLOCK_MONOTONIC, &rotation_tick);
    uint64_t active_rotation_ns = 0;
    int was_paused = 0;
    /* Stage 15: count only bounded intervals between delivered callbacks.
     * Never credit the first callback after a long compositor stall. */
    unsigned rotation_last_done = 0;
    struct timespec rotation_last_callback_tick = rotation_tick;
    int rotation_have_callback_tick = 0;
    /* Stage 13: observation only. This is NOT a compositor visibility signal. */
    const char *workspace_diag_env = diagnostic_mode ? getenv("SCREENSHAVER_LIBDECOR_DIAG_WORKSPACE") : NULL;
    int workspace_diag = workspace_diag_env && workspace_diag_env[0] == '1';
    struct timespec workspace_last;
    clock_gettime(CLOCK_MONOTONIC, &workspace_last);
    unsigned workspace_last_frames = 0;
    unsigned workspace_last_done = 0;
    unsigned long long workspace_rotations = 0;
    /* Stage 16: never stop dispatching while presentation callbacks stall.
     * A pending callback and >=500 ms without delivery gate only GL drawing.
     * Require at least one delivered callback first, to avoid startup deadlock. */
    int draw_suspended_for_callback = 0;
    const int64_t draw_stall_threshold_ns = 500000000LL;
    if (workspace_diag)
        fprintf(stderr, "[LIBDECOR WORKSPACE DIAG] enabled; frame/swap activity is not proof of compositor visibility\n");
    fprintf(stderr, "[LIBDECOR ROTATION] interval_ns=%llu (%s)\n",
            (unsigned long long)rotation_interval_ns,
            rotation_interval_ns ? "enabled" : "disabled");
    while (w.running) {
        if (should_stop && should_stop(stop_userdata)) {
            fprintf(stderr, "[%s] external shutdown requested\n",
                    diagnostic_mode ? "LIBDECOR BRIDGE TEST" : "LIBDECOR WINDOWSHADER");
            break;
        }
        if (bridge_diagnostic_dispatch_heartbeat(&w, &diagnostic_last) < 0) {
            w.failed = 1; break;
        }
        /* Keep dispatch and shutdown live while paused, but never swap or
         * request a replacement until presentation has resumed. */
        int paused = pause_status ? pause_status(pause_userdata, 0) : 0;
        struct timespec rotation_now;
        clock_gettime(CLOCK_MONOTONIC, &rotation_now);
        if (paused) {
            was_paused = 1;
            rotation_have_callback_tick = 0;
            rotation_last_done = w.workspace_frame_done;
            continue;
        }
        if (was_paused) {
            was_paused = 0;
            rotation_have_callback_tick = 0;
            rotation_last_done = w.workspace_frame_done;
            fprintf(stderr, "[LIBDECOR ROTATION] resumed; paused time excluded\n");
        }
        /* Only callback delivery authorizes rotation-clock progress.  This
         * prevents hidden-workspace render/swap loops from advancing rotation.
         * A callback following >250 ms without one is treated as a return
         * from suspension, not as a catch-up interval. */
        int rotation_frame_authorized = 0;
        if (w.workspace_frame_done != rotation_last_done) {
            rotation_frame_authorized = 1;
            rotation_last_done = w.workspace_frame_done;
            if (rotation_have_callback_tick && w.configured && w.frames > 0 && rotation_interval_ns) {
                int64_t callback_delta_ns =
                    (int64_t)(rotation_now.tv_sec - rotation_last_callback_tick.tv_sec) * 1000000000LL +
                    (int64_t)(rotation_now.tv_nsec - rotation_last_callback_tick.tv_nsec);
                if (callback_delta_ns > 0 && callback_delta_ns <= 250000000LL) {
                    uint64_t eligible_ns = (uint64_t)callback_delta_ns;
                    active_rotation_ns = UINT64_MAX - active_rotation_ns < eligible_ns
                                       ? UINT64_MAX : active_rotation_ns + eligible_ns;
                } else if (workspace_diag && callback_delta_ns > 250000000LL) {
                    fprintf(stderr, "[LIBDECOR ROTATION] callback gap %lld ms excluded\n",
                            (long long)(callback_delta_ns / 1000000LL));
                }
            }
            rotation_last_callback_tick = rotation_now;
            rotation_have_callback_tick = 1;
        }
        if (workspace_diag) {
            int64_t elapsed_ns = (int64_t)(rotation_now.tv_sec - workspace_last.tv_sec) * 1000000000LL
                               + (int64_t)(rotation_now.tv_nsec - workspace_last.tv_nsec);
            if (elapsed_ns >= 5000000000LL) {
                fprintf(stderr,
                        "[LIBDECOR WORKSPACE DIAG] frames_delta=%u frames_total=%u frame_done_delta=%u frame_done_total=%u frame_pending=%d paused=%d configured=%d rotation_elapsed_ms=%llu rotation_interval_ms=%llu rotation_attempts=%llu (visibility unknown)\n",
                        w.frames - workspace_last_frames, w.frames,
                        w.workspace_frame_done - workspace_last_done, w.workspace_frame_done,
                        w.workspace_frame_callback != NULL, paused, w.configured,
                        (unsigned long long)(active_rotation_ns / 1000000ULL),
                        (unsigned long long)(rotation_interval_ns / 1000000ULL),
                        workspace_rotations);
                workspace_last = rotation_now;
                workspace_last_frames = w.frames;
                workspace_last_done = w.workspace_frame_done;
            }
        }
        bridge_diagnostic_one_shot_policy_transition(
            &w, started, policy_test, &policy_attempted,
            replace_policy, policy_userdata, resolve_fps, fps_env,
            &diagnostic_fps);
        if (rotation_interval_ns && (!policy_test || policy_test[0] != '1') &&
            w.configured && w.frames > 0 && rotation_frame_authorized &&
            active_rotation_ns >= rotation_interval_ns) {
            ++workspace_rotations;
            if (workspace_diag)
                fprintf(stderr, "[LIBDECOR WORKSPACE DIAG] rotation_due attempt=%llu frames=%u (visibility unknown)\n",
                        workspace_rotations, w.frames);
            active_rotation_ns = 0; /* bounded retry: do not spin on failures */
            if (replace_policy) {
                fprintf(stderr, "[LIBDECOR ROTATION] interval elapsed; requesting next policy\n");
                int change = replace_policy(policy_userdata);
                if (change == 0) {
                    fprintf(stderr, "[LIBDECOR ROTATION] replacement confirmed\n");
                    if (current_title) {
                        const char *title = current_title(policy_userdata);
                        if (title && title[0]) {
                            libdecor_frame_set_title(w.frame, title);
                            fprintf(stderr, "[LIBDECOR TITLE] updated: %s\n", title);
                        }
                    }
                    if (resolve_fps) {
                        bridge_diagnostic_update_policy_fps(
                            resolve_fps, policy_userdata, fps_env, &diagnostic_fps, 1);
                    }
                } else {
                    fprintf(stderr, "[LIBDECOR ROTATION] %s; previous shader retained\n",
                            change == 1 ? "selection unchanged" : "replacement failed");
                }
            }
        }
        bridge_diagnostic_managed_shader_replacement(
            &w, started, replacement, &replacement_attempted);
        /* Stage 16: gate drawing only after sustained compositor callback
         * starvation. Keep Wayland dispatch, stop and pause processing alive. */
        int draw_callback_stalled = 0;
        if (w.configured && w.workspace_have_done && w.workspace_frame_callback) {
            struct timespec stall_now;
            clock_gettime(CLOCK_MONOTONIC, &stall_now);
            int64_t since_done_ns =
                (int64_t)(stall_now.tv_sec - w.workspace_last_done_at.tv_sec) * 1000000000LL +
                (int64_t)(stall_now.tv_nsec - w.workspace_last_done_at.tv_nsec);
            draw_callback_stalled = since_done_ns >= draw_stall_threshold_ns;
        }
        if (draw_callback_stalled != draw_suspended_for_callback) {
            draw_suspended_for_callback = draw_callback_stalled;
            fprintf(stderr, "[LIBDECOR DRAW GATE] %s (pending=%d frame_done=%u)\n",
                    draw_callback_stalled ? "suspended: callback stalled" : "resumed: callback delivery",
                    w.workspace_frame_callback != NULL, w.workspace_frame_done);
        }
        struct timespec paint_begin, paint_end;
        clock_gettime(CLOCK_MONOTONIC, &paint_begin);
        if (w.configured && !draw_callback_stalled) {
            if (w.surface && !w.workspace_frame_callback) {
                struct wl_callback *cb = wl_surface_frame(w.surface);
                if (cb) {
                    w.workspace_frame_callback = cb;
                    wl_callback_add_listener(cb, &bridge_workspace_frame_listener, &w);
                    ++w.workspace_frame_requests;
                }
            }
            int paint_result = bridge_test_paint(&w);
            clock_gettime(CLOCK_MONOTONIC, &paint_end);
            bridge_diagnostic_log_paint_duration(&paint_begin, &paint_end);
            if (paint_result < 0) {
                fprintf(stderr, "[%s] EGL swap failed (error=0x%x)\n",
                        diagnostic_mode ? "LIBDECOR BRIDGE TEST" : "LIBDECOR WINDOWSHADER", eglGetError());
                w.failed = 1; break;
            }
        }
        if (w.configured && pause_status) (void)pause_status(pause_userdata, 1);
        /* Sleep only the remaining frame budget. The pause callback runs before
         * this bounded sleep and on the next iteration; never sleep while paused.
         * Avoid accumulated deadlines or catch-up bursts after a handoff. */
        bridge_diagnostic_sleep_frame_budget(diagnostic_fps, w.configured, &paint_begin);
    }
    result = (w.configured && w.frames > 0 && !w.failed) ? 0 : 1;
    if (result == 0) {
        if (final_width) *final_width = w.normal_width > 0 ? w.normal_width : w.width;
        if (final_height) *final_height = w.normal_height > 0 ? w.normal_height : w.height;
        if (final_maximized && w.maximized_known) *final_maximized = w.maximized;
        fprintf(stderr, "[LIBDECOR MAXIMIZE] final state=%s normal_size=%d x %d\n",
                w.maximized_known ? (w.maximized ? "maximized" : "normal") : "unknown",
                w.normal_width, w.normal_height);
    }
cleanup:
    bridge_diagnostic_window_close(&w, result, workspace_diag);
    return result;
}


/* Stage 29: production entrypoint. Diagnostic-only environment overrides are
 * deliberately ignored; policy FPS, rotation, geometry and callbacks are
 * supplied by Rust. Legacy diagnostic entrypoints remain available. */
int screenshaver_decor_bridge_run_windowshader(
    int (*should_stop)(void *), void *stop_userdata,
    int (*replace_policy)(void *), void *policy_userdata,
    int (*pause_status)(void *, int), void *pause_userdata,
    unsigned configured_fps,
    int (*resolve_fps)(void *, unsigned *),
    uint64_t rotation_interval_ns,
    int initial_width, int initial_height, int initial_maximized,
    int *final_width, int *final_height, int *final_maximized,
    const char *initial_title, const char *(*current_title)(void *)) {
    return bridge_run_windowshader_with_rotation(0,
        should_stop, stop_userdata, replace_policy, policy_userdata,
        pause_status, pause_userdata, configured_fps, resolve_fps,
        rotation_interval_ns, initial_width, initial_height, initial_maximized,
        final_width, final_height, final_maximized, initial_title, current_title);
}

int screenshaver_decor_bridge_run_diagnostic_with_rotation(
    int (*should_stop)(void *), void *stop_userdata,
    int (*replace_policy)(void *), void *policy_userdata,
    int (*pause_status)(void *, int), void *pause_userdata,
    unsigned configured_fps,
    int (*resolve_fps)(void *, unsigned *),
    uint64_t rotation_interval_ns,
    int initial_width, int initial_height, int initial_maximized,
    int *final_width, int *final_height, int *final_maximized,
    const char *initial_title, const char *(*current_title)(void *)) {
    return bridge_run_windowshader_with_rotation(1,
        should_stop, stop_userdata, replace_policy, policy_userdata,
        pause_status, pause_userdata, configured_fps, resolve_fps,
        rotation_interval_ns, initial_width, initial_height, initial_maximized,
        final_width, final_height, final_maximized, initial_title, current_title);
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
    bridge_cursor_initialize(&w);
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
