/* C ABI of the attractor player.
 *
 * This header is maintained by hand alongside src/lib.rs; tests/header.rs
 * fails when a declaration and an exported symbol drift apart. The player is
 * one instance per surface: create one handle per display, drive it from the
 * render callback, and destroy it when the surface goes away.
 *
 * Every function that takes a player handle assumes the handle is alive and
 * used from one thread at a time. A destroyed handle must not be used again.
 */

#ifndef ATTRACTOR_H
#define ATTRACTOR_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque player handle. */
typedef struct AttractorPlayer AttractorPlayer;

/* Construction settings for one player. */
typedef struct AttractorPlayerOptions {
    /* Seed for the attractor and the camera; zero picks a clock-based seed. */
    uint64_t seed;
    /* Surface width in pixels. */
    uint32_t width;
    /* Surface height in pixels. */
    uint32_t height;
    /* Overlay font and margin multiplier (a Retina surface passes 2). */
    float overlay_scale;
    /* Non-zero selects wgpu's noop backend, used by tests and CI only. */
    uint32_t use_noop_backend;
} AttractorPlayerOptions;

/* Status codes returned by the int32_t entry points. */
typedef enum AttractorPlayerStatus {
    ATTRACTOR_PLAYER_OK = 0,
    ATTRACTOR_PLAYER_RESTARTED = 1,
    ATTRACTOR_PLAYER_RESTART_DEFERRED = 2,
    ATTRACTOR_PLAYER_UNHEALTHY = 3,
    ATTRACTOR_PLAYER_ERROR = 4,
    ATTRACTOR_PLAYER_INVALID_ARGUMENT = -1
} AttractorPlayerStatus;

/* Create a player with a random attractor from the bundled catalog.
 * Returns NULL when options is NULL, no GPU is available, or the catalog has
 * no runnable entry. */
AttractorPlayer *attractor_player_create(const AttractorPlayerOptions *options);

/* Destroy a player and release its GPU resources. Accepts NULL. */
void attractor_player_destroy(AttractorPlayer *player);

/* Advance one frame by dt seconds of wall-clock time.
 * When the restart limiter defers a restart, retry_after_seconds (optional)
 * receives the delay before the next attempt. */
int32_t attractor_player_tick(AttractorPlayer *player, double dt,
                              double *retry_after_seconds);

/* Render the current frame into pixels as tightly packed RGBA8 rows.
 * pixels_len must be at least width * height * 4 bytes. */
int32_t attractor_player_render(AttractorPlayer *player, uint8_t *pixels,
                                size_t pixels_len);

/* Retarget the player at a new surface size. */
int32_t attractor_player_resize(AttractorPlayer *player, uint32_t width,
                                uint32_t height, float overlay_scale);

/* Replace the current run with a fresh random selection.
 * Returns ATTRACTOR_PLAYER_RESTART_DEFERRED when the limiter blocks it; the
 * next tick retries automatically. */
int32_t attractor_player_restart(AttractorPlayer *player);

/* Current surface size in pixels, or zero for a NULL handle. */
uint32_t attractor_player_width(const AttractorPlayer *player);
uint32_t attractor_player_height(const AttractorPlayer *player);

/* Last failure message as a NUL-terminated UTF-8 string, or NULL when the
 * last call succeeded. Owned by the player; valid until the next call. */
const char *attractor_player_last_error(const AttractorPlayer *player);

#ifdef __cplusplus
}
#endif

#endif /* ATTRACTOR_H */
