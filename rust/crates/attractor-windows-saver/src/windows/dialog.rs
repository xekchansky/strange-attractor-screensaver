//! The `/c` settings window.
//!
//! The saver exposes one setting today: which attractor runs. Windows users
//! expect the `Settings` link of the Screen Saver page to open something, so
//! the switch opens a small native window with a combo box listing the catalog
//! plus a "random" entry. The choice is written to the same JSON file an
//! administrator can edit by hand.

use std::cell::RefCell;
use std::path::PathBuf;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, TRUE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{DEFAULT_GUI_FONT, GetStockObject};
use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRect, BS_DEFPUSHBUTTON, BS_PUSHBUTTON, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL,
    CBS_DROPDOWNLIST, CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics,
    GetWindowRect, MB_ICONERROR, MB_OK, MessageBoxW, PostQuitMessage, SM_CXSCREEN, SM_CYSCREEN,
    SW_SHOW, SendMessageW, SetForegroundWindow, ShowWindow, WM_CLOSE, WM_COMMAND, WM_DESTROY,
    WM_SETFONT, WS_CAPTION, WS_CHILD, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

use super::{
    DIALOG_CLASS, PRODUCT_NAME, WindowsError, dialog_background, enable_dpi_awareness,
    module_instance, register_window_class, run_message_loop,
};
use crate::args::WindowHandle;
use crate::selection;
use crate::settings::Settings;
use crate::text::wide;

/// Command id of the OK button.
const OK_ID: usize = 1;
/// Command id of the Cancel button.
const CANCEL_ID: usize = 2;
/// Command id of the attractor combo box.
const COMBO_ID: usize = 3;

/// Client size of the window at 100 % scaling.
const CLIENT_WIDTH: i32 = 420;
/// Client height at 100 % scaling.
const CLIENT_HEIGHT: i32 = 190;

/// What the window procedure needs to read the combo box and save the file.
struct DialogState {
    /// Combo box index → attractor id; index 0 is "random".
    ids: Vec<Option<String>>,
    combo: HWND,
    path: PathBuf,
}

thread_local! {
    /// The open settings window, if any.
    static DIALOG: RefCell<Option<DialogState>> = const { RefCell::new(None) };
}

/// Show the settings window and save the chosen attractor.
pub fn run(owner: Option<WindowHandle>) -> Result<(), WindowsError> {
    let Some(path) = Settings::environment_path() else {
        return Err(WindowsError::new(
            "%APPDATA% is not set, so the settings file has no home",
        ));
    };
    let settings = match Settings::load(&path) {
        Ok(settings) => settings,
        Err(error) => {
            // A corrupt file must not block the settings window, which is how
            // the user repairs it.
            log::warn!("cannot read the settings: {error}");
            Settings::default()
        }
    };
    let catalog =
        selection::validated_catalog().map_err(|error| WindowsError::new(error.to_string()))?;

    enable_dpi_awareness();
    let instance = module_instance()?;
    register_window_class(DIALOG_CLASS, Some(dialog_procedure), dialog_background())?;

    let owner = owner.map(|handle| handle.value() as isize as HWND);
    let dpi = unsafe {
        match owner {
            Some(owner) => GetDpiForWindow(owner),
            None => GetDpiForSystem(),
        }
    };
    let scale = if dpi == 0 { 1.0 } else { f64::from(dpi) / 96.0 };
    let style = WS_CAPTION | WS_SYSMENU;
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: scaled(CLIENT_WIDTH, scale),
        bottom: scaled(CLIENT_HEIGHT, scale),
    };
    unsafe {
        AdjustWindowRect(&mut rect, style, 0);
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let (left, top) = centered_position(owner, width, height);

    let class = wide(DIALOG_CLASS);
    let title = wide(PRODUCT_NAME);
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            style,
            left,
            top,
            width,
            height,
            owner.unwrap_or(std::ptr::null_mut()),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return Err(WindowsError::last("CreateWindowExW"));
    }

    DIALOG.with(|cell| {
        *cell.borrow_mut() = Some(DialogState {
            ids: std::iter::once(None)
                .chain(catalog.iter().map(|descriptor| Some(descriptor.id.clone())))
                .collect(),
            combo: std::ptr::null_mut(),
            path,
        });
    });

    let combo = create_children(hwnd, &catalog, &settings, scale);
    DIALOG.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            state.combo = combo;
        }
    });

    unsafe {
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
    }
    run_message_loop();
    DIALOG.with(|cell| {
        *cell.borrow_mut() = None;
    });
    unsafe {
        DestroyWindow(hwnd);
    }
    Ok(())
}

/// Create the label, the combo box and the buttons, returning the combo.
fn create_children(
    parent: HWND,
    catalog: &[attractor_core::AttractorDescriptor],
    settings: &Settings,
    scale: f64,
) -> HWND {
    let instance = super::module_instance().unwrap_or(std::ptr::null_mut());
    let font = unsafe { GetStockObject(DEFAULT_GUI_FONT) };

    let label_text = wide("Attractor:");
    let label = unsafe {
        create_child(
            "Static",
            label_text.as_ptr(),
            parent,
            WS_CHILD | WS_VISIBLE,
            bounds(16, 18, 380, 20, scale),
            0,
            instance,
        )
    };

    let combo = unsafe {
        create_child(
            "ComboBox",
            std::ptr::null(),
            parent,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | (CBS_DROPDOWNLIST as u32),
            bounds(16, 42, 388, 220, scale),
            COMBO_ID,
            instance,
        )
    };

    for child in [label, combo] {
        if !child.is_null() {
            unsafe {
                SendMessageW(child, WM_SETFONT, font as WPARAM, TRUE as LPARAM);
            }
        }
    }
    fill_combo(combo, catalog, settings);

    let ok_text = wide("OK");
    let ok = unsafe {
        create_child(
            "Button",
            ok_text.as_ptr(),
            parent,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_DEFPUSHBUTTON as u32),
            bounds(246, 150, 76, 28, scale),
            OK_ID,
            instance,
        )
    };
    let cancel_text = wide("Cancel");
    let cancel = unsafe {
        create_child(
            "Button",
            cancel_text.as_ptr(),
            parent,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            bounds(330, 150, 76, 28, scale),
            CANCEL_ID,
            instance,
        )
    };
    for child in [ok, cancel] {
        if !child.is_null() {
            unsafe {
                SendMessageW(child, WM_SETFONT, font as WPARAM, TRUE as LPARAM);
            }
        }
    }

    combo
}

/// Placement of an interactive control at 96-DPI coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Bounds {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

/// Scale a 96-DPI layout rectangle to physical pixels.
fn bounds(x: i32, y: i32, width: i32, height: i32, scale: f64) -> Bounds {
    Bounds {
        x: scaled(x, scale),
        y: scaled(y, scale),
        width: scaled(width, scale),
        height: scaled(height, scale),
    }
}

/// Create one child control with `id` as its command identifier.
unsafe fn create_child(
    class: &str,
    text: *const u16,
    parent: HWND,
    style: u32,
    bounds: Bounds,
    id: usize,
    instance: windows_sys::Win32::Foundation::HINSTANCE,
) -> HWND {
    let class = wide(class);
    unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            text,
            style,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            parent,
            id as isize as *mut std::ffi::c_void,
            instance,
            std::ptr::null(),
        )
    }
}

/// Fill the combo with "random" and every catalog attractor, then preselect
/// the saved choice.
fn fill_combo(combo: HWND, catalog: &[attractor_core::AttractorDescriptor], settings: &Settings) {
    if combo.is_null() {
        return;
    }
    let random = wide("Random each time");
    unsafe {
        SendMessageW(combo, CB_ADDSTRING, 0, random.as_ptr() as LPARAM);
    }
    for descriptor in catalog {
        let name = wide(&descriptor.name);
        unsafe {
            SendMessageW(combo, CB_ADDSTRING, 0, name.as_ptr() as LPARAM);
        }
    }
    let selected = settings
        .attractor
        .as_deref()
        .and_then(|id| catalog.iter().position(|descriptor| descriptor.id == id))
        .map_or(0, |index| index + 1);
    unsafe {
        SendMessageW(combo, CB_SETCURSEL, selected, 0);
    }
}

/// Save the combo's choice and close the window when it worked.
fn accept(hwnd: HWND) {
    let saved = DIALOG.with(|cell| {
        let mut guard = cell.borrow_mut();
        let state = guard.as_mut()?;
        let index = unsafe { SendMessageW(state.combo, CB_GETCURSEL, 0, 0) };
        let attractor = usize::try_from(index)
            .ok()
            .and_then(|index| state.ids.get(index).cloned().flatten());
        let settings = Settings { attractor };
        Some((settings.save(&state.path), state.path.clone()))
    });
    match saved {
        Some((Ok(()), _)) => {
            unsafe { DestroyWindow(hwnd) };
        }
        Some((Err(error), path)) => {
            let text = wide(&format!(
                "Cannot save the settings to {}: {error}",
                path.display()
            ));
            let title = wide(PRODUCT_NAME);
            unsafe {
                MessageBoxW(hwnd, text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
            }
        }
        None => {}
    }
}

/// The settings window's procedure.
unsafe extern "system" fn dialog_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_COMMAND => {
            let id = wparam & 0xFFFF;
            if id == OK_ID {
                accept(hwnd);
                0
            } else if id == CANCEL_ID {
                unsafe { DestroyWindow(hwnd) };
                0
            } else {
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
        }
        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

/// Position `width`×`height` in the center of the owner, or of the screen.
fn centered_position(owner: Option<HWND>, width: i32, height: i32) -> (i32, i32) {
    let (left, top, owner_width, owner_height) = match owner {
        Some(owner) => {
            let mut rect = RECT::default();
            if unsafe { GetWindowRect(owner, &mut rect) } == 0 {
                screen_bounds()
            } else {
                (
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                )
            }
        }
        None => screen_bounds(),
    };
    (
        left + (owner_width - width) / 2,
        top + (owner_height - height) / 2,
    )
}

/// The primary screen's rectangle.
fn screen_bounds() -> (i32, i32, i32, i32) {
    unsafe {
        (
            0,
            0,
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
        )
    }
}

/// A 96-DPI layout coordinate in physical pixels.
fn scaled(value: i32, scale: f64) -> i32 {
    (f64::from(value) * scale).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_scales_with_the_dpi() {
        assert_eq!(scaled(100, 1.0), 100);
        assert_eq!(scaled(100, 1.5), 150);
        assert_eq!(scaled(100, 1.25), 125);
    }
}
