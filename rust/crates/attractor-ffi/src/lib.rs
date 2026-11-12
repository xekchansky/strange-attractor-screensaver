//! C ABI over the player, used by the macOS `.saver` and the Android JNI
//! bridge.
//!
//! The interface is deliberately small: a host creates one player per
//! surface, ticks it with the wall-clock frame interval, renders into a pixel
//! buffer, resizes the surface and destroys the player. Every entry point
//! takes and returns plain C types; [`AttractorPlayerOptions`] is the only
//! struct, and it is passed by pointer.
//!
//! The matching header lives in `include/attractor.h`. It is maintained by
//! hand; `tests/header.rs` fails when the declarations and the exported
//! symbols drift apart.
//!
//! # Safety
//!
//! Every function that takes a pointer is `unsafe`: the pointers must be
//! valid for their documented use, and a player may only be used from one
//! thread at a time. A destroyed player pointer must not be used again.

use std::error::Error;
use std::ffi::{CString, c_char, c_void};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use attractor_core::catalog::{self, CatalogError};
use attractor_core::{RandomizerError, SplitMix64, randomizer};
use attractor_gpu::{GpuContext, GpuError, SurfaceTarget};
use attractor_player::{Player, PlayerError, PlayerOptions, RestartOutcome, TickEvent};

/// The call succeeded.
pub const ATTRACTOR_PLAYER_OK: i32 = 0;
/// A degenerate run was replaced with a fresh selection.
pub const ATTRACTOR_PLAYER_RESTARTED: i32 = 1;
/// The restart limiter blocks a restart; retry after the reported delay.
pub const ATTRACTOR_PLAYER_RESTART_DEFERRED: i32 = 2;
/// The health monitor rejected the run (only without automatic healing).
pub const ATTRACTOR_PLAYER_UNHEALTHY: i32 = 3;
/// The call failed; `attractor_player_last_error` explains why.
pub const ATTRACTOR_PLAYER_ERROR: i32 = 4;
/// A null or otherwise unusable argument was passed.
pub const ATTRACTOR_PLAYER_INVALID_ARGUMENT: i32 = -1;

/// Construction settings for one player.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttractorPlayerOptions {
    /// Seed for the attractor and the camera; zero picks a clock-based seed.
    pub seed: u64,
    /// Surface width in pixels.
    pub width: u32,
    /// Surface height in pixels.
    pub height: u32,
    /// Overlay font and margin multiplier (a Retina surface passes 2).
    pub overlay_scale: f32,
    /// Non-zero selects wgpu's `noop` backend, used by tests and CI only.
    pub use_noop_backend: u32,
}

/// The opaque player handle.
///
/// Fields drop in declaration order, so the surface and the player (which
/// owns GPU resources) are released before the context that created them.
pub struct AttractorPlayer {
    surface: Option<SurfaceTarget>,
    player: Player,
    context: GpuContext,
    last_error: Option<CString>,
}

impl AttractorPlayer {
    fn clear_error(&mut self) {
        self.last_error = None;
    }

    fn set_error(&mut self, message: impl fmt::Display) {
        self.last_error = CString::new(message.to_string()).ok();
    }
}

/// Create a player with a random attractor from the bundled catalog.
///
/// Returns null when the options are null, the GPU context cannot be created
/// or the catalog has no runnable descriptor. Failures are logged through the
/// `log` crate; there is no handle to carry a message when creation fails.
///
/// # Safety
///
/// `options` must point to a readable [`AttractorPlayerOptions`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn attractor_player_create(
    options: *const AttractorPlayerOptions,
) -> *mut AttractorPlayer {
    let Some(options) = (unsafe { options.as_ref() }) else {
        return std::ptr::null_mut();
    };
    match create_player(options) {
        Ok(player) => Box::into_raw(Box::new(player)),
        Err(error) => {
            log::error!("cannot create the attractor player: {error}");
            std::ptr::null_mut()
        }
    }
}

/// Destroy a player and release its GPU resources. Accepts null.
///
/// # Safety
///
/// `player` must come from [`attractor_player_create`] and must not be used
/// afterwards. Destroying it twice is undefined behaviour.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn attractor_player_destroy(player: *mut AttractorPlayer) {
    if !player.is_null() {
        drop(unsafe { Box::from_raw(player) });
    }
}

/// Advance one frame by `dt` seconds of wall-clock time.
///
/// When the restart limiter blocks a deferred restart,
/// `retry_after_seconds` (when not null) receives how long to wait before the
/// next attempt. The status is one of the `ATTRACTOR_PLAYER_*` constants.
///
/// # Safety
///
/// `player` must be a live handle; `retry_after_seconds` must be null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn attractor_player_tick(
    player: *mut AttractorPlayer,
    dt: f64,
    retry_after_seconds: *mut f64,
) -> i32 {
    let Some(player) = (unsafe { player.as_mut() }) else {
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    };
    match player.player.tick(dt) {
        Ok(TickEvent::Advanced) => {
            player.clear_error();
            ATTRACTOR_PLAYER_OK
        }
        Ok(TickEvent::Restarted) => {
            player.clear_error();
            ATTRACTOR_PLAYER_RESTARTED
        }
        Ok(TickEvent::RestartDeferred {
            retry_after_seconds: retry,
        }) => {
            if let Some(out) = unsafe { retry_after_seconds.as_mut() } {
                *out = retry;
            }
            player.clear_error();
            ATTRACTOR_PLAYER_RESTART_DEFERRED
        }
        Err(PlayerError::Unhealthy(issue)) => {
            player.set_error(format!("simulation health check failed: {issue:?}"));
            ATTRACTOR_PLAYER_UNHEALTHY
        }
        Err(error) => {
            player.set_error(error);
            ATTRACTOR_PLAYER_ERROR
        }
    }
}

/// Render the current frame into `pixels` as tightly packed RGBA8 rows.
///
/// `pixels_len` must be at least `width * height * 4` bytes, where the width
/// and height come from [`attractor_player_width`] and
/// [`attractor_player_height`]. The buffer is fully overwritten on success.
///
/// # Safety
///
/// `player` must be a live handle; `pixels` must be writable for
/// `pixels_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn attractor_player_render(
    player: *mut AttractorPlayer,
    pixels: *mut u8,
    pixels_len: usize,
) -> i32 {
    let Some(player) = (unsafe { player.as_mut() }) else {
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    };
    let (width, height) = player.player.size();
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4));
    let Some(expected) = expected else {
        player.set_error(format!(
            "the frame size {width}x{height} does not fit a buffer"
        ));
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    };
    if pixels.is_null() || pixels_len < expected {
        player.set_error(format!(
            "the pixel buffer holds {pixels_len} bytes, {expected} are required"
        ));
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    }
    match player.player.render(&player.context) {
        Ok(frame) => {
            debug_assert_eq!(frame.len(), expected);
            unsafe { std::ptr::copy_nonoverlapping(frame.as_ptr(), pixels, expected) };
            player.clear_error();
            ATTRACTOR_PLAYER_OK
        }
        Err(error) => {
            player.set_error(error);
            ATTRACTOR_PLAYER_ERROR
        }
    }
}

/// Attach a native presentation layer to the player.
///
/// On macOS `layer` is a `CAMetalLayer *` owned by the host view; the layer
/// must outlive the player. Once attached, [`attractor_player_draw`] presents
/// frames instead of reading them back.
///
/// # Safety
///
/// `player` must be a live handle and `layer` must be a valid `CAMetalLayer`
/// that outlives the player.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn attractor_player_attach_layer(
    player: *mut AttractorPlayer,
    layer: *mut c_void,
) -> i32 {
    let Some(player) = (unsafe { player.as_mut() }) else {
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    };
    if layer.is_null() {
        player.set_error("the presentation layer is null");
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    }
    let (width, height) = player.player.size();
    // SAFETY: the caller guarantees the layer outlives the player.
    match unsafe { SurfaceTarget::from_metal_layer(&player.context, layer, width, height) } {
        Ok(surface) => {
            player.surface = Some(surface);
            player.clear_error();
            ATTRACTOR_PLAYER_OK
        }
        Err(error) => {
            player.set_error(error);
            ATTRACTOR_PLAYER_ERROR
        }
    }
}

/// Draw the current frame into the attached layer and present it.
///
/// Returns [`ATTRACTOR_PLAYER_ERROR`] when no layer was attached; the surface
/// may skip a frame (a timeout or an occluded layer) and still report success.
///
/// # Safety
///
/// `player` must be a live handle with a layer attached.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn attractor_player_draw(player: *mut AttractorPlayer) -> i32 {
    let Some(player) = (unsafe { player.as_mut() }) else {
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    };
    if player.surface.is_none() {
        player.set_error("no presentation layer is attached");
        return ATTRACTOR_PLAYER_ERROR;
    }
    let result = {
        let target = player
            .surface
            .as_mut()
            .expect("the surface presence was checked above");
        player.player.render_to_surface(&player.context, target)
    };
    match result {
        Ok(()) => {
            player.clear_error();
            ATTRACTOR_PLAYER_OK
        }
        Err(error) => {
            player.set_error(error);
            ATTRACTOR_PLAYER_ERROR
        }
    }
}

/// Retarget the player at a new surface size.
///
/// # Safety
///
/// `player` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn attractor_player_resize(
    player: *mut AttractorPlayer,
    width: u32,
    height: u32,
    overlay_scale: f32,
) -> i32 {
    let Some(player) = (unsafe { player.as_mut() }) else {
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    };
    if width == 0 || height == 0 {
        player.set_error(format!("invalid frame size {width}x{height}"));
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    }
    match player.player.resize(width, height, overlay_scale) {
        Ok(()) => {
            player.clear_error();
            ATTRACTOR_PLAYER_OK
        }
        Err(error) => {
            player.set_error(error);
            ATTRACTOR_PLAYER_ERROR
        }
    }
}

/// Replace the current run with a fresh random selection.
///
/// Returns `ATTRACTOR_PLAYER_RESTARTED`, or `ATTRACTOR_PLAYER_RESTART_DEFERRED`
/// when the restart limiter blocks it (the next [`attractor_player_tick`]
/// retries automatically).
///
/// # Safety
///
/// `player` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn attractor_player_restart(player: *mut AttractorPlayer) -> i32 {
    let Some(player) = (unsafe { player.as_mut() }) else {
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    };
    match player.player.restart() {
        Ok(RestartOutcome::Restarted) => {
            player.clear_error();
            ATTRACTOR_PLAYER_RESTARTED
        }
        Ok(RestartOutcome::Blocked { .. }) => {
            player.clear_error();
            ATTRACTOR_PLAYER_RESTART_DEFERRED
        }
        Err(error) => {
            player.set_error(error);
            ATTRACTOR_PLAYER_ERROR
        }
    }
}

/// Current surface width in pixels, or zero for a null handle.
///
/// # Safety
///
/// `player` must be null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn attractor_player_width(player: *const AttractorPlayer) -> u32 {
    unsafe { player.as_ref() }
        .map(|player| player.player.size().0)
        .unwrap_or(0)
}

/// Current surface height in pixels, or zero for a null handle.
///
/// # Safety
///
/// `player` must be null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn attractor_player_height(player: *const AttractorPlayer) -> u32 {
    unsafe { player.as_ref() }
        .map(|player| player.player.size().1)
        .unwrap_or(0)
}

/// The last failure message, or null when the last call succeeded.
///
/// The pointer stays valid until the next call that touches the player and is
/// owned by the player; do not free it. The string is UTF-8 and
/// NUL-terminated.
///
/// # Safety
///
/// `player` must be null or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn attractor_player_last_error(
    player: *const AttractorPlayer,
) -> *const c_char {
    unsafe { player.as_ref() }
        .and_then(|player| player.last_error.as_ref())
        .map_or(std::ptr::null(), |error| error.as_ptr())
}

/// A failure while creating a player.
#[derive(Debug)]
enum CreateError {
    Catalog(CatalogError),
    EmptyCatalog,
    Player(PlayerError),
}

impl fmt::Display for CreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(formatter, "failed to load the catalog: {error}"),
            Self::EmptyCatalog => {
                write!(formatter, "the attractor catalog has no runnable entries")
            }
            Self::Player(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for CreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::EmptyCatalog => None,
            Self::Player(error) => Some(error),
        }
    }
}

impl From<PlayerError> for CreateError {
    fn from(error: PlayerError) -> Self {
        Self::Player(error)
    }
}

impl From<GpuError> for CreateError {
    fn from(error: GpuError) -> Self {
        Self::Player(PlayerError::Gpu(error))
    }
}

impl From<RandomizerError> for CreateError {
    fn from(error: RandomizerError) -> Self {
        Self::Player(PlayerError::Randomizer(error))
    }
}

fn create_player(options: &AttractorPlayerOptions) -> Result<AttractorPlayer, CreateError> {
    let context = if options.use_noop_backend != 0 {
        GpuContext::new_noop()?
    } else {
        GpuContext::new()?
    };

    let validation = catalog::bundled_validation().map_err(CreateError::Catalog)?;
    for issue in &validation.issues {
        log::warn!(
            "dropping attractor '{}' ({}): {}",
            issue.descriptor_id,
            issue.field,
            issue.message
        );
    }
    let descriptors = validation.valid;
    if descriptors.is_empty() {
        return Err(CreateError::EmptyCatalog);
    }

    let seed = if options.seed == 0 {
        time_seed()
    } else {
        options.seed
    };
    let mut rng = SplitMix64::new(seed);
    let selection = randomizer::select_from_catalog(&descriptors, &mut rng)?;

    let player = Player::new(
        selection,
        descriptors,
        PlayerOptions {
            heal_automatically: true,
            width: options.width,
            height: options.height,
            overlay_scale: options.overlay_scale,
            ..PlayerOptions::default()
        },
        rng,
    )?;

    Ok(AttractorPlayer {
        surface: None,
        player,
        context,
        last_error: None,
    })
}

/// A seed that differs between runs when the caller passes zero.
fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15)
        ^ u64::from(std::process::id())
}
