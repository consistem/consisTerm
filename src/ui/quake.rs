//! The drop-down terminal: a system-wide shortcut that brings the window down
//! from the top of the screen - or up from the bottom - and puts it away again,
//! the way Guake and Yakuake do on Linux.
//!
//! Done in Win32 directly, like [`crate::ui::tray`], and for the same reason:
//! eframe runs no frames for a window that is hidden, so nothing in `update`
//! could ever answer the key that is meant to bring it back. The hotkey is
//! registered to a message-only window of our own, created on the main thread,
//! so winit's message loop delivers it and nothing is polled. The window is
//! placed by Win32 as well, which needs no frame to have run either.
//!
//! Elsewhere this is a no-op: a system-wide hotkey on X11 and macOS needs a
//! library of its own, and Wayland does not let a program have one at all.

use egui::{Key, Modifiers};

use crate::config::QuakeEdge;

/// How the drop-down terminal is set up: where it comes from, how much of the
/// screen it takes, and the chord that summons it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quake {
    pub edge: QuakeEdge,
    /// Percent of the screen's height.
    pub height: u32,
    pub shortcut: Option<(Modifiers, Key)>,
    /// Keep the window above the others while it is down. Off, it comes to
    /// the front like any window brought forward and others may go over it -
    /// unless the pin button says otherwise.
    pub on_top: bool,
}

impl Quake {
    pub fn from_settings(settings: &crate::config::Settings) -> Quake {
        Quake {
            edge: settings.quake_edge,
            height: settings.quake_height,
            shortcut: settings
                .quake_shortcut
                .as_deref()
                .and_then(crate::ui::shortcut::parse_global),
            on_top: settings.quake_on_top,
        }
    }
}

/// The share of the screen the drop-down terminal may take: too short to hold
/// a prompt below it, too tall to leave anything of what it was summoned over.
pub const HEIGHT_RANGE: std::ops::RangeInclusive<u32> = 20..=90;

/// Where the window goes on a screen whose work area - the screen less the
/// taskbar - is `work` as `(left, top, right, bottom)`: across the whole of it,
/// `height` percent of it tall, against `edge`. `(x, y, width, height)`.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn dock_rect(work: (i32, i32, i32, i32), edge: QuakeEdge, height: u32) -> (i32, i32, i32, i32) {
    let (left, top, right, bottom) = work;
    let percent = height.clamp(*HEIGHT_RANGE.start(), *HEIGHT_RANGE.end()) as i64;
    let tall = ((bottom - top) as i64 * percent / 100) as i32;
    let y = match edge {
        QuakeEdge::Bottom => bottom - tall,
        QuakeEdge::Top | QuakeEdge::Off => top,
    };
    (left, y, right - left, tall)
}

/// `MOD_*` flags for `RegisterHotKey`, without `MOD_NOREPEAT`.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn hotkey_modifiers(modifiers: Modifiers) -> u32 {
    let mut flags = 0;
    if modifiers.alt {
        flags |= 0x0001;
    }
    if modifiers.ctrl || modifiers.command {
        flags |= 0x0002;
    }
    if modifiers.shift {
        flags |= 0x0004;
    }
    flags
}

/// The Windows virtual-key code for `key`, for the keys a summoning chord is
/// made of. `None` for one that has no fixed code across keyboard layouts.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn virtual_key(key: Key) -> Option<u32> {
    let name = key.name();
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.clone().next()) {
        // Letters and digits are their own ASCII codes.
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_uppercase() as u32);
        }
    }
    if let Some(n) = name.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        if (1..=24).contains(&n) {
            return Some(0x70 + n - 1);
        }
    }
    Some(match key {
        Key::Space => 0x20,
        Key::Tab => 0x09,
        Key::Enter => 0x0D,
        Key::Insert => 0x2D,
        Key::Home => 0x24,
        Key::End => 0x23,
        Key::PageUp => 0x21,
        Key::PageDown => 0x22,
        // The key left of 1 on a US keyboard, which is what Quake used.
        Key::Backtick => 0xC0,
        _ => return None,
    })
}

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU8, Ordering};

    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW,
        IsIconic, IsWindowVisible, IsZoomed, RegisterClassW, SetForegroundWindow, SetWindowPos,
        ShowWindow, GWL_EXSTYLE, HWND_MESSAGE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE,
        SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE, SW_RESTORE, WM_HOTKEY, WNDCLASSW, WS_EX_TOPMOST,
    };

    use super::Quake;
    use crate::config::QuakeEdge;

    /// The main window, and the message-only window the hotkey is sent to.
    static WINDOW: AtomicIsize = AtomicIsize::new(0);
    static SINK: AtomicIsize = AtomicIsize::new(0);
    static REGISTERED: AtomicBool = AtomicBool::new(false);
    /// What the hotkey does when it arrives: which edge, and how tall.
    static EDGE: AtomicU8 = AtomicU8::new(0);
    static HEIGHT: AtomicU32 = AtomicU32::new(40);
    static ON_TOP: AtomicBool = AtomicBool::new(false);
    /// The pin button's state, which a summoning must not undo.
    static PINNED: AtomicBool = AtomicBool::new(false);
    const HOTKEY_ID: i32 = 1;
    const CLASS: &str = "consisTermQuake";

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    fn handle(atom: &AtomicIsize) -> Option<HWND> {
        let raw = atom.load(Ordering::Relaxed);
        (raw != 0).then_some(raw as HWND)
    }

    pub fn install(hwnd: isize) {
        if hwnd == 0 || WINDOW.swap(hwnd, Ordering::Relaxed) != 0 {
            return;
        }
        let class = wide(CLASS);
        // SAFETY: the class name outlives the calls that read it, and the
        // window procedure has the signature WNDPROC asks for.
        unsafe {
            let wc = WNDCLASSW {
                lpfnWndProc: Some(on_message),
                hInstance: GetModuleHandleW(std::ptr::null()),
                lpszClassName: class.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
            let sink = CreateWindowExW(
                0,
                class.as_ptr(),
                class.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                wc.hInstance,
                std::ptr::null(),
            );
            SINK.store(sink as isize, Ordering::Relaxed);
        }
    }

    pub fn configure(quake: Quake) -> Result<(), String> {
        let Some(sink) = handle(&SINK) else {
            return Ok(());
        };
        let was_docked = docked();
        EDGE.store(
            match quake.edge {
                QuakeEdge::Off => 0,
                QuakeEdge::Top => 1,
                QuakeEdge::Bottom => 2,
            },
            Ordering::Relaxed,
        );
        HEIGHT.store(quake.height, Ordering::Relaxed);
        ON_TOP.store(quake.on_top, Ordering::Relaxed);
        // SAFETY: unregisters only the hotkey this module registered.
        if REGISTERED.swap(false, Ordering::Relaxed) {
            unsafe { UnregisterHotKey(sink, HOTKEY_ID) };
        }
        let (QuakeEdge::Top | QuakeEdge::Bottom, Some((modifiers, key))) =
            (quake.edge, quake.shortcut)
        else {
            if was_docked {
                // Back to an ordinary window: no longer kept above the rest.
                if let Some(window) = handle(&WINDOW) {
                    // SAFETY: changes only the z-order of our own window.
                    unsafe {
                        SetWindowPos(window, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE)
                    };
                }
            }
            return Ok(());
        };
        let Some(vk) = super::virtual_key(key) else {
            return Err(crate::i18n::tr("That key cannot be a system-wide shortcut.").into());
        };
        let flags = super::hotkey_modifiers(modifiers) | MOD_NOREPEAT;
        // SAFETY: registers to our own sink window, on the thread that made it.
        if unsafe { RegisterHotKey(sink, HOTKEY_ID, flags, vk) } == 0 {
            // Another program holds it - the one failure worth telling apart.
            return Err(crate::i18n::tr(
                "Another program already uses that shortcut. Choose another one.",
            )
            .into());
        }
        REGISTERED.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn docked() -> bool {
        EDGE.load(Ordering::Relaxed) != 0 && REGISTERED.load(Ordering::Relaxed)
    }

    pub fn set_pinned(pinned: bool) {
        PINNED.store(pinned, Ordering::Relaxed);
    }

    pub fn auto_pinned() -> bool {
        let Some(window) = handle(&WINDOW) else {
            return false;
        };
        // Asked of the window rather than worked out from the settings: the
        // pin button can take the window off the top after the drop-down put
        // it there, and the mark must not outlive the state it describes.
        // SAFETY: queries on our own window's handle.
        let topmost = unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST != 0;
        docked()
            && ON_TOP.load(Ordering::Relaxed)
            && unsafe { IsWindowVisible(window) } != 0
            && topmost
    }

    /// In front: put it away. Anywhere else - hidden, behind another window,
    /// minimized - bring it down over whatever is there, on the screen the
    /// pointer is on.
    fn toggle() {
        let Some(window) = handle(&WINDOW) else {
            return;
        };
        let edge = match EDGE.load(Ordering::Relaxed) {
            1 => QuakeEdge::Top,
            2 => QuakeEdge::Bottom,
            _ => return,
        };
        // SAFETY: plain calls on our own window's handle and on structures
        // initialised here.
        unsafe {
            let shown = IsWindowVisible(window) != 0 && IsIconic(window) == 0;
            if shown && GetForegroundWindow() == window {
                ShowWindow(window, SW_HIDE);
                return;
            }
            let mut at = POINT { x: 0, y: 0 };
            GetCursorPos(&mut at);
            let monitor = MonitorFromPoint(at, MONITOR_DEFAULTTONEAREST);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return;
            }
            let work = info.rcWork;
            let (x, y, w, h) = super::dock_rect(
                (work.left, work.top, work.right, work.bottom),
                edge,
                HEIGHT.load(Ordering::Relaxed),
            );
            // A maximized or minimized window keeps its own idea of where it
            // goes; it has to be an ordinary one to be placed.
            if IsIconic(window) != 0 || IsZoomed(window) != 0 {
                ShowWindow(window, SW_RESTORE);
            }
            // Above everything only when asked to be, by this setting or by
            // the pin; otherwise brought to the front as an ordinary window.
            let level = if ON_TOP.load(Ordering::Relaxed) || PINNED.load(Ordering::Relaxed) {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            };
            SetWindowPos(window, level, x, y, w, h, SWP_SHOWWINDOW);
            // Allowed: a hotkey hands its thread the right to take the
            // foreground.
            SetForegroundWindow(window);
        }
    }

    unsafe extern "system" fn on_message(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_HOTKEY && wparam as i32 == HOTKEY_ID {
            toggle();
            return 0;
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Quake;

    pub fn install(_hwnd: isize) {}
    pub fn configure(_quake: Quake) -> Result<(), String> {
        Ok(())
    }
    pub fn docked() -> bool {
        false
    }
    pub fn set_pinned(_pinned: bool) {}
    pub fn auto_pinned() -> bool {
        false
    }
}

/// Whether this platform has a drop-down terminal to offer.
pub const SUPPORTED: bool = cfg!(windows);

/// Gets the drop-down terminal ready for the main window `cc` was made for.
pub fn install(cc: &eframe::CreationContext<'_>) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let hwnd = match cc.window_handle().map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::Win32(handle)) => handle.hwnd.get(),
        _ => 0,
    };
    imp::install(hwnd);
}

/// Registers the shortcut `quake` names, dropping the one registered before.
/// An error is the reason it could not be, worded for the user.
pub fn configure(quake: Quake) -> Result<(), String> {
    imp::configure(quake)
}

/// Tells the drop-down what the pin button says, so bringing the window down
/// keeps it above the others when the user has pinned it.
pub fn set_pinned(pinned: bool) {
    imp::set_pinned(pinned)
}

/// Whether the window is above the others because the drop-down put it there,
/// rather than the pin.
pub fn auto_pinned() -> bool {
    imp::auto_pinned()
}

/// Whether the window is a drop-down terminal now. Its size and place are then
/// the screen's, and must not be saved as the window's own.
pub fn docked() -> bool {
    imp::docked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_drop_down_spans_the_screen_against_the_edge_it_comes_from() {
        // A 1920x1080 screen with a 40 px taskbar along the bottom.
        let work = (0, 0, 1920, 1040);
        assert_eq!(dock_rect(work, QuakeEdge::Top, 40), (0, 0, 1920, 416));
        assert_eq!(dock_rect(work, QuakeEdge::Bottom, 50), (0, 520, 1920, 520));
        // A second screen to the left of the first.
        assert_eq!(
            dock_rect((-1280, 0, 0, 1024), QuakeEdge::Top, 50),
            (-1280, 0, 1280, 512)
        );
    }

    #[test]
    fn a_hand_edited_height_is_held_to_the_range_offered() {
        let work = (0, 0, 1000, 1000);
        assert_eq!(dock_rect(work, QuakeEdge::Top, 0).3, 200);
        assert_eq!(dock_rect(work, QuakeEdge::Top, 500).3, 900);
    }

    #[test]
    fn the_usual_summoning_keys_have_a_virtual_key_code() {
        assert_eq!(virtual_key(Key::F12), Some(0x7B));
        assert_eq!(virtual_key(Key::F1), Some(0x70));
        assert_eq!(virtual_key(Key::A), Some(b'A' as u32));
        assert_eq!(virtual_key(Key::Num1), Some(b'1' as u32));
        assert_eq!(virtual_key(Key::Backtick), Some(0xC0));
        assert_eq!(virtual_key(Key::ArrowUp), None);
        assert_eq!(
            hotkey_modifiers(Modifiers::CTRL | Modifiers::SHIFT),
            0x0002 | 0x0004
        );
    }
}
