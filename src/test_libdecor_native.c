/* Isolated libdecor smoke test. Does not link against Screenshaver. */
#include <libdecor.h>
#include <wayland-client.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <sys/mman.h>
#include <fcntl.h>

static struct wl_compositor *compositor;
static struct wl_shm *shm;
static struct wl_surface *surface;
static struct wl_buffer *buffer;
static struct libdecor_frame *frame;
static int running = 1;
static int configured = 0;
static int width = 720, height = 420;

static void buffer_release(void *data, struct wl_buffer *b) { (void)data; (void)b; }
static const struct wl_buffer_listener buffer_listener = { buffer_release };

static int paint(int w, int h) {
    if (w < 64 || h < 64 || w > 4096 || h > 4096) return -1;
    size_t size = (size_t)w * (size_t)h * 4;
    char name[] = "/tmp/screenshaver-libdecor-XXXXXX";
    int fd = mkstemp(name);
    if (fd < 0) return -1;
    unlink(name);
    if (ftruncate(fd, (off_t)size) < 0) { close(fd); return -1; }
    uint32_t *pixels = mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (pixels == MAP_FAILED) { close(fd); return -1; }
    for (int y = 0; y < h; ++y) for (int x = 0; x < w; ++x) {
        unsigned r = 25 + (unsigned)(x * 75 / w);
        unsigned g = 45 + (unsigned)(y * 75 / h);
        pixels[(size_t)y*w+x] = 0xff000000u | (r<<16) | (g<<8) | 145u;
    }
    struct wl_shm_pool *pool = wl_shm_create_pool(shm, fd, (int)size);
    struct wl_buffer *new_buffer = wl_shm_pool_create_buffer(pool, 0, w, h, w*4, WL_SHM_FORMAT_XRGB8888);
    wl_shm_pool_destroy(pool);
    munmap(pixels, size);
    close(fd);
    if (!new_buffer) return -1;
    wl_buffer_add_listener(new_buffer, &buffer_listener, NULL);
    /* Keep buffers until exit to avoid releasing one still used by compositor. */
    buffer = new_buffer;
    wl_surface_attach(surface, buffer, 0, 0);
    wl_surface_damage_buffer(surface, 0, 0, w, h);
    wl_surface_commit(surface);
    return 0;
}

static void configure(struct libdecor_frame *f, struct libdecor_configuration *configuration, void *user_data) {
    (void)user_data;
    int w = width, h = height;
    if (libdecor_configuration_get_content_size(configuration, f, &w, &h)) {
        if (w > 0 && h > 0) { width = w; height = h; }
    }
    struct libdecor_state *state = libdecor_state_new(width, height);
    libdecor_frame_commit(f, state, configuration);
    libdecor_state_free(state);
    if (paint(width, height) != 0) { fprintf(stderr, "Could not draw test buffer\n"); running = 0; }
    configured = 1;
    fprintf(stderr, "[LIBDECOR TEST] configure: %d x %d\n", width, height);
}
static void close_frame(struct libdecor_frame *f, void *user_data) {
    (void)f; (void)user_data;
    fprintf(stderr, "[LIBDECOR TEST] Close requested\n");
    running = 0;
}
/* The GTK decoration plugin may request a parent-surface commit when it
 * uses synchronized subsurfaces. A NULL commit callback can crash when the
 * plugin calls through the frame interface. */
static void commit_frame(struct libdecor_frame *f, void *user_data) {
    (void)f;
    (void)user_data;
    if (surface) {
        wl_surface_commit(surface);
    }
}

/* No popup windows are created by this isolated diagnostic. */
static void dismiss_popup(struct libdecor_frame *f, const char *seat_name,
                          void *user_data) {
    (void)f;
    (void)seat_name;
    (void)user_data;
}

static struct libdecor_frame_interface frame_interface = {
    .configure = configure,
    .close = close_frame,
    .commit = commit_frame,
    .dismiss_popup = dismiss_popup,
};
static void libdecor_error(struct libdecor *context, enum libdecor_error error, const char *message) {
    (void)context;
    fprintf(stderr, "[LIBDECOR TEST] error %d: %s\n", error, message ? message : "unknown");
    running = 0;
}
static struct libdecor_interface decor_interface = { .error = libdecor_error };
static void global(void *data, struct wl_registry *registry, uint32_t name, const char *interface, uint32_t version) {
    (void)data;
    if (!strcmp(interface, wl_compositor_interface.name))
        compositor = wl_registry_bind(registry, name, &wl_compositor_interface, version < 4 ? version : 4);
    if (!strcmp(interface, wl_shm_interface.name))
        shm = wl_registry_bind(registry, name, &wl_shm_interface, 1);
}
static void global_remove(void *data, struct wl_registry *registry, uint32_t name) { (void)data; (void)registry; (void)name; }
static const struct wl_registry_listener registry_listener = { global, global_remove };
int screenshaver_test_libdecor_run(void) {
    struct wl_display *display = wl_display_connect(NULL);
    if (!display) { fprintf(stderr, "No Wayland compositor connection\n"); return 1; }
    struct wl_registry *registry = wl_display_get_registry(display);
    wl_registry_add_listener(registry, &registry_listener, NULL);
    if (wl_display_roundtrip(display) < 0 || !compositor || !shm) {
        fprintf(stderr, "Missing compositor or shm global\n"); return 1;
    }
    struct libdecor *decor = libdecor_new(display, &decor_interface);
    if (!decor) { fprintf(stderr, "libdecor_new failed\n"); return 1; }
    surface = wl_compositor_create_surface(compositor);
    frame = libdecor_decorate(decor, surface, &frame_interface, NULL);
    if (!frame) { fprintf(stderr, "libdecor_decorate failed\n"); return 1; }
    libdecor_frame_set_app_id(frame, "screenshaver-libdecor-test");
    libdecor_frame_set_title(frame, "Screenshaver libdecor test - drag / resize / close");
    libdecor_frame_set_min_content_size(frame, 160, 120);
    libdecor_frame_map(frame);
    fprintf(stderr, "[LIBDECOR TEST] Window requested. Test dragging, resizing, maximize and close.\n");
    while (running) {
        if (libdecor_dispatch(decor, -1) < 0) { fprintf(stderr, "libdecor dispatch failed\n"); break; }
    }
    fprintf(stderr, "[LIBDECOR TEST] finished; configured=%d\n", configured);
    libdecor_frame_unref(frame);
    libdecor_unref(decor);
    wl_surface_destroy(surface);
    wl_registry_destroy(registry);
    wl_display_disconnect(display);
    return configured ? 0 : 1;
}
