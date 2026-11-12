//! Smoke tests through the C entry points, exactly as a shell would call them.

use std::ffi::CStr;
use std::ptr;

use attractor_ffi::{
    ATTRACTOR_PLAYER_ERROR, ATTRACTOR_PLAYER_INVALID_ARGUMENT, ATTRACTOR_PLAYER_OK,
    ATTRACTOR_PLAYER_RESTART_DEFERRED, ATTRACTOR_PLAYER_RESTARTED, AttractorPlayerOptions,
    attractor_player_attach_layer, attractor_player_create, attractor_player_destroy,
    attractor_player_draw, attractor_player_height, attractor_player_last_error,
    attractor_player_render, attractor_player_resize, attractor_player_restart,
    attractor_player_tick, attractor_player_width,
};
use attractor_gpu::lit_pixels;

const FRAME_INTERVAL: f64 = 1.0 / 60.0;

fn options(use_noop_backend: bool) -> AttractorPlayerOptions {
    AttractorPlayerOptions {
        seed: 7,
        width: 64,
        height: 48,
        overlay_scale: 1.0,
        use_noop_backend: u32::from(use_noop_backend),
    }
}

#[test]
fn the_noop_player_ticks_resizes_restarts_and_renders() {
    let options = options(true);
    let player = unsafe { attractor_player_create(&options) };
    assert!(!player.is_null(), "the noop player must be created");
    assert_eq!(unsafe { attractor_player_width(player) }, 64);
    assert_eq!(unsafe { attractor_player_height(player) }, 48);

    assert_eq!(
        unsafe { attractor_player_tick(player, FRAME_INTERVAL, ptr::null_mut()) },
        ATTRACTOR_PLAYER_OK
    );
    assert_eq!(
        unsafe { attractor_player_resize(player, 128, 96, 2.0) },
        ATTRACTOR_PLAYER_OK
    );
    assert_eq!(unsafe { attractor_player_width(player) }, 128);
    assert_eq!(unsafe { attractor_player_height(player) }, 96);
    assert_eq!(
        unsafe { attractor_player_restart(player) },
        ATTRACTOR_PLAYER_RESTARTED
    );

    let mut pixels = vec![0_u8; 128 * 96 * 4];
    assert_eq!(
        unsafe { attractor_player_render(player, pixels.as_mut_ptr(), pixels.len()) },
        ATTRACTOR_PLAYER_OK
    );
    unsafe { attractor_player_destroy(player) };
}

#[test]
fn null_and_invalid_arguments_are_rejected() {
    assert!(unsafe { attractor_player_create(ptr::null()) }.is_null());

    let mut zero = options(true);
    zero.width = 0;
    assert!(
        unsafe { attractor_player_create(&zero) }.is_null(),
        "a zero-width surface must not create a player"
    );

    assert_eq!(
        unsafe { attractor_player_tick(ptr::null_mut(), FRAME_INTERVAL, ptr::null_mut()) },
        ATTRACTOR_PLAYER_INVALID_ARGUMENT
    );
    assert_eq!(
        unsafe { attractor_player_render(ptr::null_mut(), ptr::null_mut(), 0) },
        ATTRACTOR_PLAYER_INVALID_ARGUMENT
    );
    assert_eq!(
        unsafe { attractor_player_resize(ptr::null_mut(), 10, 10, 1.0) },
        ATTRACTOR_PLAYER_INVALID_ARGUMENT
    );
    assert_eq!(
        unsafe { attractor_player_restart(ptr::null_mut()) },
        ATTRACTOR_PLAYER_INVALID_ARGUMENT
    );
    assert_eq!(
        unsafe { attractor_player_attach_layer(ptr::null_mut(), ptr::null_mut()) },
        ATTRACTOR_PLAYER_INVALID_ARGUMENT
    );
    assert_eq!(
        unsafe { attractor_player_draw(ptr::null_mut()) },
        ATTRACTOR_PLAYER_INVALID_ARGUMENT
    );
    assert_eq!(unsafe { attractor_player_width(ptr::null()) }, 0);
    assert_eq!(unsafe { attractor_player_height(ptr::null()) }, 0);
    assert!(unsafe { attractor_player_last_error(ptr::null()) }.is_null());
    unsafe { attractor_player_destroy(ptr::null_mut()) };
}

#[test]
fn the_last_error_explains_a_rejected_buffer() {
    let options = options(true);
    let player = unsafe { attractor_player_create(&options) };
    assert!(!player.is_null());

    let mut pixels = vec![0_u8; 3];
    assert_eq!(
        unsafe { attractor_player_render(player, pixels.as_mut_ptr(), pixels.len()) },
        ATTRACTOR_PLAYER_INVALID_ARGUMENT
    );
    let message = unsafe { CStr::from_ptr(attractor_player_last_error(player)) }
        .to_str()
        .expect("the error is UTF-8");
    assert!(
        message.contains("bytes"),
        "the error must mention the buffer size: {message}"
    );

    // A successful call clears the message again.
    let mut full = vec![0_u8; 64 * 48 * 4];
    assert_eq!(
        unsafe { attractor_player_render(player, full.as_mut_ptr(), full.len()) },
        ATTRACTOR_PLAYER_OK
    );
    assert!(unsafe { attractor_player_last_error(player) }.is_null());
    unsafe { attractor_player_destroy(player) };
}

#[test]
fn restarting_through_the_limiter_reports_deferral() {
    let options = AttractorPlayerOptions {
        seed: 3,
        ..options(true)
    };
    let player = unsafe { attractor_player_create(&options) };
    assert!(!player.is_null());

    for _ in 0..3 {
        assert_eq!(
            unsafe { attractor_player_restart(player) },
            ATTRACTOR_PLAYER_RESTARTED
        );
    }
    assert_eq!(
        unsafe { attractor_player_restart(player) },
        ATTRACTOR_PLAYER_RESTART_DEFERRED
    );

    // Once the limiter window has elapsed, restarts are allowed again.
    for _ in 0..120 {
        assert_eq!(
            unsafe { attractor_player_tick(player, 0.1, ptr::null_mut()) },
            ATTRACTOR_PLAYER_OK
        );
    }
    assert_eq!(
        unsafe { attractor_player_restart(player) },
        ATTRACTOR_PLAYER_RESTARTED
    );
    unsafe { attractor_player_destroy(player) };
}

#[test]
fn a_native_player_renders_ink() {
    let options = AttractorPlayerOptions {
        seed: 42,
        width: 320,
        height: 240,
        overlay_scale: 1.0,
        use_noop_backend: 0,
    };
    let player = unsafe { attractor_player_create(&options) };
    if player.is_null() {
        eprintln!("skipping the native render test: no GPU adapter");
        return;
    }
    for _ in 0..240 {
        assert_eq!(
            unsafe { attractor_player_tick(player, FRAME_INTERVAL, ptr::null_mut()) },
            ATTRACTOR_PLAYER_OK
        );
    }
    let mut pixels = vec![0_u8; 320 * 240 * 4];
    assert_eq!(
        unsafe { attractor_player_render(player, pixels.as_mut_ptr(), pixels.len()) },
        ATTRACTOR_PLAYER_OK
    );
    assert!(lit_pixels(&pixels) > 0, "the ribbon must draw ink");
    unsafe { attractor_player_destroy(player) };
}

#[test]
fn a_missing_presentation_layer_is_reported() {
    let options = options(true);
    let player = unsafe { attractor_player_create(&options) };
    assert!(!player.is_null());

    assert_eq!(
        unsafe { attractor_player_attach_layer(player, ptr::null_mut()) },
        ATTRACTOR_PLAYER_INVALID_ARGUMENT
    );
    let message = unsafe { CStr::from_ptr(attractor_player_last_error(player)) }
        .to_str()
        .expect("the error is UTF-8");
    assert!(
        message.contains("layer"),
        "the error must mention the layer: {message}"
    );

    assert_eq!(
        unsafe { attractor_player_draw(player) },
        ATTRACTOR_PLAYER_ERROR
    );
    let message = unsafe { CStr::from_ptr(attractor_player_last_error(player)) }
        .to_str()
        .expect("the error is UTF-8");
    assert!(
        message.contains("no presentation layer"),
        "the error must explain the missing layer: {message}"
    );
    unsafe { attractor_player_destroy(player) };
}

#[test]
fn a_zero_extent_resize_is_rejected_with_a_message() {
    let options = options(true);
    let player = unsafe { attractor_player_create(&options) };
    assert!(!player.is_null());
    assert_eq!(
        unsafe { attractor_player_resize(player, 0, 48, 1.0) },
        ATTRACTOR_PLAYER_INVALID_ARGUMENT
    );
    let message = unsafe { CStr::from_ptr(attractor_player_last_error(player)) }
        .to_str()
        .expect("the error is UTF-8");
    assert!(
        message.contains("invalid frame size"),
        "unexpected error text: {message}"
    );
    assert_eq!(unsafe { attractor_player_width(player) }, 64);
    unsafe { attractor_player_destroy(player) };
}
