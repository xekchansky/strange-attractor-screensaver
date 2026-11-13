//! JNI bridge used by the Android live wallpaper.
//!
//! The Kotlin side lives in `rust/shells/android/app/src/main/kotlin/
//! com/xekchansky/strangeattractor/NativeBridge.kt`; `tests/android_bridge.rs`
//! fails when the two sides drift apart. One [`AndroidPlayer`] belongs to one
//! rendering thread: the wallpaper engine creates it after its surface exists,
//! renders on that thread and destroys it before the surface goes away.
//!
//! The renderer is the same wgpu pipeline every other shell uses. The wallpaper
//! hands over an `android.view.Surface`; `ANativeWindow_fromSurface` turns it
//! into the handle wgpu's GLES or Vulkan backend creates its swap chain from
//! (ADR-2), so Android has no second shader or presentation path.

use std::ffi::c_void;
use std::sync::Mutex;

use attractor_core::SplitMix64;
use attractor_gpu::{GpuContext, SAMPLE_COUNT, SurfaceTarget};
use attractor_player::{Player, PlayerError, PlayerOptions, TickEvent};
use attractor_shell::selection;
use jni::JNIEnv;
use jni::objects::{JObject, JString};
use jni::sys::{jfloat, jint, jlong, jobject, jstring};

use crate::android_support::{self, seconds_from_nanos, settings_from_id};
use crate::{
    ATTRACTOR_PLAYER_ERROR, ATTRACTOR_PLAYER_INVALID_ARGUMENT, ATTRACTOR_PLAYER_OK,
    ATTRACTOR_PLAYER_RESTART_DEFERRED, ATTRACTOR_PLAYER_RESTARTED, ATTRACTOR_PLAYER_UNHEALTHY,
};

// `ANativeWindow_fromSurface` returns a new reference that the caller owns;
// `ANativeWindow_release` drops it. Both live in libandroid.so, which every
// Android process links.
#[link(name = "android")]
unsafe extern "C" {
    fn ANativeWindow_fromSurface(env: *mut jni::sys::JNIEnv, surface: jobject) -> *mut c_void;
    fn ANativeWindow_release(window: *mut c_void);
}

/// The message of the last failed [`create`], which has no handle to carry it.
static CREATE_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// One owned `ANativeWindow` reference.
///
/// `ANativeWindow_fromSurface` acquires the window, so the reference must be
/// released exactly once, after the surface built from it is gone.
struct NativeWindow(*mut c_void);

impl Drop for NativeWindow {
    fn drop(&mut self) {
        // SAFETY: the pointer came from `ANativeWindow_fromSurface`, which
        // returns an owned reference, and this is its only release.
        unsafe { ANativeWindow_release(self.0) };
    }
}

/// One wallpaper surface and the player presenting into it.
///
/// Fields drop in declaration order: the swap chain and the player go before
/// the GPU context that created them, and the window reference goes last.
pub struct AndroidPlayer {
    target: SurfaceTarget,
    player: Player,
    context: GpuContext,
    /// Held for ownership and drop order only; the target borrows the pointer.
    _window: NativeWindow,
    overlay_scale: f32,
    last_error: Option<String>,
}

impl AndroidPlayer {
    fn record(&mut self, message: impl Into<String>) -> i32 {
        self.last_error = Some(message.into());
        ATTRACTOR_PLAYER_ERROR
    }
}

/// Build the player for a wallpaper surface.
fn create_player(
    env: &JNIEnv<'_>,
    surface: JObject<'_>,
    width: jint,
    height: jint,
    seed: jlong,
    attractor: &str,
    overlay_scale: f32,
) -> Result<AndroidPlayer, String> {
    if width <= 0 || height <= 0 {
        return Err(format!("invalid frame size {width}x{height}"));
    }
    // SAFETY: the env is the caller's, the surface is a live Java object for
    // the duration of the call, and the returned reference is owned below.
    let window = unsafe { ANativeWindow_fromSurface(env.get_raw(), surface.as_raw()) };
    if window.is_null() {
        return Err("cannot get the native window behind the wallpaper surface".to_owned());
    }
    let window = NativeWindow(window);
    let (width, height) = (width as u32, height as u32);
    // SAFETY: `NativeWindow` keeps the reference alive for as long as the
    // target it is handed to, and it is dropped after the target.
    let (context, target) = unsafe { SurfaceTarget::create_android(window.0, width, height) }
        .map_err(|error| error.to_string())?;
    let catalog = selection::validated_catalog().map_err(|error| error.to_string())?;
    let settings = settings_from_id(attractor);
    let mut rng = SplitMix64::new(android_support::resolve_seed(seed));
    let chosen =
        selection::choose(&settings, &catalog, &mut rng).map_err(|error| error.to_string())?;
    // GLES cannot resolve multisampled targets on every driver, exactly like
    // the browser's WebGL2 path, so a GL-backed adapter renders single-sample.
    let sample_count = if context.supports_multisampling() {
        SAMPLE_COUNT
    } else {
        1
    };
    let options = PlayerOptions {
        width,
        height,
        overlay_scale,
        sample_count,
        ..PlayerOptions::default()
    };
    let player = Player::new(chosen, catalog, options, rng).map_err(|error| error.to_string())?;
    Ok(AndroidPlayer {
        target,
        player,
        context,
        _window: window,
        overlay_scale,
        last_error: None,
    })
}

fn render_frame(player: &mut AndroidPlayer, elapsed_nanos: jlong) -> jint {
    let status = match player.player.tick(seconds_from_nanos(elapsed_nanos)) {
        Ok(TickEvent::Advanced) => ATTRACTOR_PLAYER_OK,
        Ok(TickEvent::Restarted) => ATTRACTOR_PLAYER_RESTARTED,
        Ok(TickEvent::RestartDeferred { .. }) => ATTRACTOR_PLAYER_RESTART_DEFERRED,
        Err(PlayerError::Unhealthy(issue)) => {
            player.last_error = Some(format!("simulation health check failed: {issue:?}"));
            ATTRACTOR_PLAYER_UNHEALTHY
        }
        Err(error) => return player.record(error.to_string()),
    };
    match player
        .player
        .render_to_surface(&player.context, &mut player.target)
    {
        Ok(()) => {
            if status == ATTRACTOR_PLAYER_OK {
                player.last_error = None;
            }
            status
        }
        Err(error) => player.record(error.to_string()),
    }
}

fn resize_player(player: &mut AndroidPlayer, width: jint, height: jint) -> jint {
    if width <= 0 || height <= 0 {
        return player.record(format!("invalid frame size {width}x{height}"));
    }
    let (width, height) = (width as u32, height as u32);
    if let Err(error) = player.player.resize(width, height, player.overlay_scale) {
        return player.record(error.to_string());
    }
    if let Err(error) = player.target.resize(&player.context, width, height) {
        return player.record(error.to_string());
    }
    player.last_error = None;
    ATTRACTOR_PLAYER_OK
}

fn error_text(handle: jlong) -> String {
    if handle != 0 {
        // SAFETY: the caller passes a handle it received from `create` and has
        // not destroyed yet.
        if let Some(player) = unsafe { (handle as *mut AndroidPlayer).as_ref() }
            && let Some(message) = &player.last_error
        {
            return message.clone();
        }
    }
    CREATE_ERROR
        .lock()
        .ok()
        .and_then(|message| message.clone())
        .unwrap_or_default()
}

fn remember_create_error(message: &str) {
    log::error!("cannot create the Android attractor player: {message}");
    if let Ok(mut slot) = CREATE_ERROR.lock() {
        *slot = Some(message.to_owned());
    }
}

/// Create a player for `surface`; returns a handle, or zero on failure.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_xekchansky_strangeattractor_NativeBridge_create(
    mut env: JNIEnv,
    _this: JObject,
    surface: JObject,
    width: jint,
    height: jint,
    seed: jlong,
    attractor: JString,
    overlay_scale: jfloat,
) -> jlong {
    let attractor = match env.get_string(&attractor) {
        Ok(text) => String::from(text),
        Err(error) => {
            remember_create_error(&format!("cannot read the attractor id: {error}"));
            return 0;
        }
    };
    match create_player(
        &env,
        surface,
        width,
        height,
        seed,
        &attractor,
        overlay_scale,
    ) {
        Ok(player) => {
            if let Ok(mut slot) = CREATE_ERROR.lock() {
                *slot = None;
            }
            Box::into_raw(Box::new(player)) as jlong
        }
        Err(error) => {
            remember_create_error(&error);
            0
        }
    }
}

/// Advance the simulation by `elapsed_nanos` and present a frame.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_xekchansky_strangeattractor_NativeBridge_render(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
    elapsed_nanos: jlong,
) -> jint {
    // SAFETY: the caller passes a live handle from `create` on its rendering
    // thread and does not use it concurrently.
    let Some(player) = (unsafe { (handle as *mut AndroidPlayer).as_mut() }) else {
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    };
    render_frame(player, elapsed_nanos)
}

/// Retarget the player and its swap chain after a surface size change.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_xekchansky_strangeattractor_NativeBridge_resize(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
    width: jint,
    height: jint,
) -> jint {
    // SAFETY: as in `render`.
    let Some(player) = (unsafe { (handle as *mut AndroidPlayer).as_mut() }) else {
        return ATTRACTOR_PLAYER_INVALID_ARGUMENT;
    };
    resize_player(player, width, height)
}

/// Destroy the player and release the GPU context and the window reference.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_xekchansky_strangeattractor_NativeBridge_destroy(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) {
    if handle != 0 {
        // SAFETY: the handle came from `create` and is destroyed once; the
        // caller must not use it afterwards.
        drop(unsafe { Box::from_raw(handle as *mut AndroidPlayer) });
    }
}

/// The last error of `handle`, or of the last failed create when it is zero.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_xekchansky_strangeattractor_NativeBridge_lastError(
    env: JNIEnv,
    _this: JObject,
    handle: jlong,
) -> jstring {
    match env.new_string(error_text(handle)) {
        Ok(text) => text.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// The catalog the settings activity lists, as a JSON array.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_xekchansky_strangeattractor_NativeBridge_catalog(
    env: JNIEnv,
    _this: JObject,
) -> jstring {
    match android_support::catalog_json() {
        Ok(json) => match env.new_string(json) {
            Ok(text) => text.into_raw(),
            Err(error) => {
                log::error!("cannot pass the catalog to Android: {error}");
                std::ptr::null_mut()
            }
        },
        Err(error) => {
            log::error!("cannot list the catalog: {error}");
            std::ptr::null_mut()
        }
    }
}
