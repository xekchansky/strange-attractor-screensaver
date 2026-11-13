//! When a running screen saver gives the screen back.
//!
//! A screen saver starts right after the user's input has been idle, so the
//! first pointer-move event can be a leftover from the click that started it
//! (the cursor is already over the new window). Keys, buttons and the wheel
//! always end the session; pointer motion is ignored for a short grace period
//! so the saver does not disappear the moment it appears.

use std::time::Duration;

/// How long after the session starts pointer motion is ignored.
pub const POINTER_GRACE: Duration = Duration::from_millis(250);

/// One kind of input the window observes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    /// A key went down.
    Key,
    /// A mouse button went down.
    PointerButton,
    /// The mouse wheel moved.
    Wheel,
    /// The pointer moved.
    PointerMove,
}

/// Whether `event` should end the running saver.
///
/// `since_start` is the time since the first window was shown.
pub fn ends_session(event: InputEvent, since_start: Duration) -> bool {
    match event {
        InputEvent::Key | InputEvent::PointerButton | InputEvent::Wheel => true,
        InputEvent::PointerMove => since_start >= POINTER_GRACE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_buttons_and_wheel_always_end_the_session() {
        for event in [
            InputEvent::Key,
            InputEvent::PointerButton,
            InputEvent::Wheel,
        ] {
            assert!(ends_session(event, Duration::ZERO));
            assert!(ends_session(event, POINTER_GRACE));
        }
    }

    #[test]
    fn pointer_motion_is_ignored_during_the_grace_period() {
        assert!(!ends_session(InputEvent::PointerMove, Duration::ZERO));
        assert!(!ends_session(InputEvent::PointerMove, POINTER_GRACE / 2));
    }

    #[test]
    fn pointer_motion_ends_the_session_after_the_grace_period() {
        assert!(ends_session(InputEvent::PointerMove, POINTER_GRACE));
        assert!(ends_session(InputEvent::PointerMove, POINTER_GRACE * 10));
    }
}
