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
//! Its size is its own - a share of the screen's width and height, centred
//! on the edge - and not the window's: dragged away from the edge, the window
//! goes back to the size it had as an ordinary window, and is one again until
//! the shortcut next brings it down.
//!
//! Elsewhere this is a no-op: a system-wide hotkey on X11 and macOS needs a
//! library of its own, and Wayland does not let a program have one at all.

use egui::{Key, Modifiers};

use crate::config::{QuakeAnimation, QuakeEdge};

/// How the drop-down terminal is set up: where it comes from, how much of the
/// screen it takes, how it rolls in, and the chord that summons it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quake {
    pub edge: QuakeEdge,
    /// Percent of the screen's height.
    pub height: u32,
    /// Percent of the screen's width, centred on it.
    pub width: u32,
    pub animation: QuakeAnimation,
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
            width: settings.quake_width,
            animation: settings.quake_animation,
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

/// The share of the screen's width: from a column a prompt still fits in to
/// the whole of it, Guake's default.
pub const WIDTH_RANGE: std::ops::RangeInclusive<u32> = 30..=100;

/// Where the window goes on a screen whose work area - the screen less the
/// taskbar - is `work` as `(left, top, right, bottom)`: `width` percent of it
/// wide and centred, `height` percent of it tall, against `edge`.
/// `(x, y, width, height)`.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn dock_rect(
    work: (i32, i32, i32, i32),
    edge: QuakeEdge,
    width: u32,
    height: u32,
) -> (i32, i32, i32, i32) {
    let (left, top, right, bottom) = work;
    let tall_percent = height.clamp(*HEIGHT_RANGE.start(), *HEIGHT_RANGE.end()) as i64;
    let wide_percent = width.clamp(*WIDTH_RANGE.start(), *WIDTH_RANGE.end()) as i64;
    let tall = ((bottom - top) as i64 * tall_percent / 100) as i32;
    let wide = ((right - left) as i64 * wide_percent / 100) as i32;
    let x = left + (right - left - wide) / 2;
    let y = match edge {
        QuakeEdge::Bottom => bottom - tall,
        QuakeEdge::Top | QuakeEdge::Off => top,
    };
    (x, y, wide, tall)
}

/// How far into its roll the window is, `elapsed` of `millis` in: eased out,
/// so it arrives quickly and settles rather than stopping dead.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn rolled(elapsed: u128, millis: u32) -> f32 {
    if millis == 0 {
        return 1.0;
    }
    let k = (elapsed as f32 / millis as f32).clamp(0.0, 1.0);
    1.0 - (1.0 - k).powi(3)
}

/// How much of a window `tall` high shows `k` of the way into its roll, and
/// where it is: `(y offset from where it ends up, the band of it that shows,
/// as top and bottom in its own coordinates)`. Rolling from the top edge, the
/// window slides down from above it and its bottom shows first; from the
/// bottom edge, it slides up and its top shows first. The band is what is
/// clipped to, so a window sliding in from above never shows on a screen
/// above this one.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn roll_frame(edge: QuakeEdge, tall: i32, k: f32) -> (i32, i32, i32) {
    let shown = ((tall as f32) * k.clamp(0.0, 1.0)).round() as i32;
    match edge {
        QuakeEdge::Bottom => (tall - shown, 0, shown),
        QuakeEdge::Top | QuakeEdge::Off => (shown - tall, tall - shown, tall),
    }
}

/// Where a window dragged off its edge goes back to its free size: under the
/// pointer at `pointer_x`, as far along it as the pointer was along the
/// docked one, with its top where the docked one's top was dragged to.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn detached_at(
    docked: (i32, i32, i32, i32),
    pointer_x: i32,
    free: (i32, i32),
) -> (i32, i32, i32, i32) {
    let (x, y, w, _) = docked;
    let along = if w > 0 {
        ((pointer_x - x) as f32 / w as f32).clamp(0.0, 1.0)
    } else {
        0.5
    };
    let left = pointer_x - (free.0 as f32 * along).round() as i32;
    (left, y, free.0, free.1)
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
    use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU32, AtomicU8, Ordering};
    use std::sync::Mutex;
    use std::time::Instant;

    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateRectRgn, GetMonitorInfoW, MonitorFromPoint, SetWindowRgn, MONITORINFO,
        MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT,
    };
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, EnumThreadWindows, GetCursorPos, GetForegroundWindow,
        GetWindowLongPtrW, GetWindowRect, IsIconic, IsWindow, IsWindowVisible, IsZoomed, KillTimer,
        RegisterClassW, SetForegroundWindow, SetTimer, SetWindowPos, ShowWindow, GWL_EXSTYLE,
        HWND_MESSAGE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_RESTORE, SW_SHOWNOACTIVATE, WM_EXITSIZEMOVE,
        WM_HOTKEY, WM_SHOWWINDOW, WM_TIMER, WNDCLASSW, WS_EX_TOPMOST,
    };

    use super::Quake;
    use crate::config::QuakeEdge;

    /// The main window, and the message-only window the hotkey is sent to.
    static WINDOW: AtomicIsize = AtomicIsize::new(0);
    static SINK: AtomicIsize = AtomicIsize::new(0);
    static REGISTERED: AtomicBool = AtomicBool::new(false);
    /// What the hotkey does when it arrives: which edge, and how big.
    static EDGE: AtomicU8 = AtomicU8::new(0);
    static HEIGHT: AtomicU32 = AtomicU32::new(40);
    static WIDTH: AtomicU32 = AtomicU32::new(100);
    static ROLL_MILLIS: AtomicU32 = AtomicU32::new(200);
    static ON_TOP: AtomicBool = AtomicBool::new(false);
    /// The pin button's state, which a summoning must not undo.
    static PINNED: AtomicBool = AtomicBool::new(false);
    /// The window is against its edge, at the drop-down's size - rather than
    /// never brought down yet, or dragged off the edge since.
    static DOWN: AtomicBool = AtomicBool::new(false);
    /// Where it was put against the edge, to tell a drag away from it.
    static DOCK: Mutex<Option<(i32, i32, i32, i32)>> = Mutex::new(None);
    /// The size it has as an ordinary window, to go back to off the edge.
    static FREE_W: AtomicI32 = AtomicI32::new(0);
    static FREE_H: AtomicI32 = AtomicI32::new(0);
    /// The roll under way, if one is.
    static ROLL: Mutex<Option<Roll>> = Mutex::new(None);
    /// The app's other windows - Settings, a detached manager - put away with
    /// the main one, to bring back with it.
    static PUT_AWAY: Mutex<Vec<isize>> = Mutex::new(Vec::new());
    const HOTKEY_ID: i32 = 1;
    const ROLL_TIMER: usize = 1;
    /// How long a window coming back from hidden is held fully clipped before
    /// it starts to roll in: two frames, for eframe to paint it. Rolled in at
    /// once, the first steps showed whatever the hidden window last held, or
    /// nothing, and the roll flickered.
    const PAINT_FIRST: std::time::Duration = std::time::Duration::from_millis(34);
    const SUBCLASS_ID: usize = 1;
    const CLASS: &str = "consisTermQuake";

    /// A roll in or out, moved on by a timer on the sink window - so the
    /// message loop keeps turning and the window keeps painting while it
    /// moves, which a loop sleeping between steps would stop.
    #[derive(Clone, Copy)]
    struct Roll {
        started: Instant,
        millis: u32,
        rect: (i32, i32, i32, i32),
        edge: QuakeEdge,
        coming_in: bool,
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    fn handle(atom: &AtomicIsize) -> Option<HWND> {
        let raw = atom.load(Ordering::Relaxed);
        (raw != 0).then_some(raw as HWND)
    }

    fn window_rect(window: HWND) -> (i32, i32, i32, i32) {
        let mut r = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: reads our own window's rectangle into a local.
        unsafe { GetWindowRect(window, &mut r) };
        (r.left, r.top, r.right - r.left, r.bottom - r.top)
    }

    /// Remembers the window's size as an ordinary window, unless it is not
    /// one just now - minimized or maximized, whose sizes are not its own.
    fn remember_free_size(window: HWND) {
        // SAFETY: queries on our own window's handle.
        if unsafe { IsIconic(window) != 0 || IsZoomed(window) != 0 } {
            return;
        }
        let (_, _, w, h) = window_rect(window);
        if w > 0 && h > 0 {
            FREE_W.store(w, Ordering::Relaxed);
            FREE_H.store(h, Ordering::Relaxed);
        }
    }

    pub fn install(hwnd: isize) {
        if hwnd == 0 || WINDOW.swap(hwnd, Ordering::Relaxed) != 0 {
            return;
        }
        let class = wide(CLASS);
        // SAFETY: the class name outlives the calls that read it, and the
        // window procedures have the signatures WNDPROC and SUBCLASSPROC ask
        // for. The subclass sees the main window's messages before winit
        // does, and passes every one of them on.
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
            SetWindowSubclass(hwnd as HWND, Some(on_main_message), SUBCLASS_ID, 0);
        }
        // Known from the start, in case the first time it is brought down is
        // from the tray, with no ordinary window on screen to measure then.
        remember_free_size(hwnd as HWND);
    }

    pub fn configure(quake: Quake) -> Result<(), String> {
        let Some(sink) = handle(&SINK) else {
            return Ok(());
        };
        let was_armed = armed();
        EDGE.store(
            match quake.edge {
                QuakeEdge::Off => 0,
                QuakeEdge::Top => 1,
                QuakeEdge::Bottom => 2,
            },
            Ordering::Relaxed,
        );
        HEIGHT.store(quake.height, Ordering::Relaxed);
        WIDTH.store(quake.width, Ordering::Relaxed);
        ROLL_MILLIS.store(quake.animation.millis(), Ordering::Relaxed);
        ON_TOP.store(quake.on_top, Ordering::Relaxed);
        // SAFETY: unregisters only the hotkey this module registered.
        if REGISTERED.swap(false, Ordering::Relaxed) {
            unsafe { UnregisterHotKey(sink, HOTKEY_ID) };
        }
        let (QuakeEdge::Top | QuakeEdge::Bottom, Some((modifiers, key))) =
            (quake.edge, quake.shortcut)
        else {
            DOWN.store(false, Ordering::Relaxed);
            if was_armed {
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

    /// The shortcut is set up to bring the window down.
    fn armed() -> bool {
        EDGE.load(Ordering::Relaxed) != 0 && REGISTERED.load(Ordering::Relaxed)
    }

    pub fn docked() -> bool {
        armed() && DOWN.load(Ordering::Relaxed)
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

    /// Above everything only when asked to be, by the setting or by the pin;
    /// otherwise brought to the front as an ordinary window.
    fn level() -> HWND {
        if ON_TOP.load(Ordering::Relaxed) || PINNED.load(Ordering::Relaxed) {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        }
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
        // A press in the middle of a roll lands it first, then is read as
        // usual: pressed again on the way in, it goes back the way it came.
        finish_roll(window);
        // SAFETY: plain calls on our own window's handle and on structures
        // initialised here.
        unsafe {
            let shown = IsWindowVisible(window) != 0 && IsIconic(window) == 0;
            if shown && GetForegroundWindow() == window {
                if DOWN.load(Ordering::Relaxed) {
                    let rect = window_rect(window);
                    start_roll(window, rect, edge, false);
                } else {
                    // Dragged off the edge: put away as the window it became.
                    remember_free_size(window);
                    ShowWindow(window, SW_HIDE);
                    put_away_the_others(window);
                }
                return;
            }
            if shown && !DOWN.load(Ordering::Relaxed) {
                remember_free_size(window);
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
            let rect = super::dock_rect(
                (work.left, work.top, work.right, work.bottom),
                edge,
                WIDTH.load(Ordering::Relaxed),
                HEIGHT.load(Ordering::Relaxed),
            );
            // A maximized or minimized window keeps its own idea of where it
            // goes; it has to be an ordinary one to be placed.
            if IsIconic(window) != 0 || IsZoomed(window) != 0 {
                ShowWindow(window, SW_RESTORE);
            }
            if let Ok(mut dock) = DOCK.lock() {
                *dock = Some(rect);
            }
            DOWN.store(true, Ordering::Relaxed);
            start_roll(window, rect, edge, true);
            bring_back_the_others();
            // Allowed: a hotkey hands its thread the right to take the
            // foreground.
            SetForegroundWindow(window);
        }
    }

    /// Starts `window` rolling into `rect` from `edge`, or out of it to that
    /// edge - or, with no animation set, puts it there at once.
    fn start_roll(window: HWND, rect: (i32, i32, i32, i32), edge: QuakeEdge, coming_in: bool) {
        let millis = ROLL_MILLIS.load(Ordering::Relaxed);
        let started = if coming_in {
            Instant::now() + PAINT_FIRST
        } else {
            Instant::now()
        };
        let roll = Roll {
            started,
            millis,
            rect,
            edge,
            coming_in,
        };
        if millis == 0 {
            land(window, roll);
            return;
        }
        let (x, y, w, h) = rect;
        // SAFETY: places our own window, clips it to a region the system
        // takes ownership of, and starts a timer on our own sink window.
        unsafe {
            if coming_in {
                // Placed with nothing of it showing, then rolled in.
                let (dy, from, to) = super::roll_frame(edge, h, 0.0);
                SetWindowRgn(window, CreateRectRgn(0, from, w, to), 0);
                SetWindowPos(window, level(), x, y + dy, w, h, SWP_SHOWWINDOW);
            }
            if let Ok(mut slot) = ROLL.lock() {
                *slot = Some(roll);
            }
            if let Some(sink) = handle(&SINK) {
                SetTimer(sink, ROLL_TIMER, 10, None);
            }
        }
    }

    /// One step of the roll under way, from the timer.
    fn step_roll() {
        let Some(window) = handle(&WINDOW) else {
            return;
        };
        let Some(roll) = ROLL.lock().ok().and_then(|slot| *slot) else {
            stop_timer();
            return;
        };
        // Zero until `started`, which a roll in sets a little ahead.
        let k = super::rolled(roll.started.elapsed().as_millis(), roll.millis);
        if k >= 1.0 {
            finish_roll(window);
            return;
        }
        let shown = if roll.coming_in { k } else { 1.0 - k };
        let (x, y, w, h) = roll.rect;
        let (dy, from, to) = super::roll_frame(roll.edge, h, shown);
        // SAFETY: moves our own window and hands the system its new region.
        // Not redrawn by the region change: what is inside it is already
        // painted, and invalidating it each step is what made it flash.
        unsafe {
            SetWindowRgn(window, CreateRectRgn(0, from, w, to), 0);
            SetWindowPos(
                window,
                std::ptr::null_mut(),
                x,
                y + dy,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    /// Lands the roll under way, if there is one, where it was going.
    fn finish_roll(window: HWND) {
        let roll = ROLL.lock().ok().and_then(|mut slot| slot.take());
        stop_timer();
        if let Some(roll) = roll {
            land(window, roll);
        }
    }

    fn stop_timer() {
        if let Some(sink) = handle(&SINK) {
            // SAFETY: kills only the timer this module set on its own window.
            unsafe { KillTimer(sink, ROLL_TIMER) };
        }
    }

    /// Where a roll ends: down against the edge, whole, or put away.
    fn land(window: HWND, roll: Roll) {
        let (x, y, w, h) = roll.rect;
        // SAFETY: places, unclips and shows or hides our own window.
        unsafe {
            if roll.coming_in {
                SetWindowPos(window, level(), x, y, w, h, SWP_SHOWWINDOW);
                SetWindowRgn(window, std::ptr::null_mut(), 1);
            } else {
                ShowWindow(window, SW_HIDE);
                put_away_the_others(window);
                // Unclipped while nobody can see it, so it comes back whole
                // whatever brings it back - the tray, a second launch.
                SetWindowRgn(window, std::ptr::null_mut(), 0);
                SetWindowPos(
                    window,
                    std::ptr::null_mut(),
                    x,
                    y,
                    w,
                    h,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }

    /// Hides the app's other visible windows along with the main one.
    ///
    /// They are drawn inside the main window's frame - Settings is an
    /// immediate viewport - and a hidden window is given no frames, so a
    /// Settings window left on screen while the terminal was put away stopped
    /// answering anything until it came back. Going away together, they come
    /// back together.
    fn put_away_the_others(main: HWND) {
        unsafe extern "system" fn each(hwnd: HWND, main: LPARAM) -> BOOL {
            if hwnd as isize != main && IsWindowVisible(hwnd) != 0 {
                if let Ok(mut list) = PUT_AWAY.lock() {
                    list.push(hwnd as isize);
                }
                ShowWindow(hwnd, SW_HIDE);
            }
            1
        }
        // SAFETY: walks only this thread's own top-level windows, which are
        // the app's; the callback has the signature WNDENUMPROC asks for.
        unsafe { EnumThreadWindows(GetCurrentThreadId(), Some(each), main as LPARAM) };
    }

    /// Shows again, without taking the keyboard, what `put_away_the_others`
    /// hid - those of them that still exist.
    fn bring_back_the_others() {
        let list = PUT_AWAY
            .lock()
            .map(|mut l| std::mem::take(&mut *l))
            .unwrap_or_default();
        for hwnd in list {
            // SAFETY: a stale handle is refused by the call, not acted on.
            unsafe {
                if IsWindow(hwnd as HWND) != 0 {
                    ShowWindow(hwnd as HWND, SW_SHOWNOACTIVATE);
                }
            }
        }
    }

    /// A move or a resize of the main window has ended. Dragged away from the
    /// edge it was brought down against, it is an ordinary window again, at
    /// the size it had as one. A resize - from any edge, which can move its
    /// corner too - keeps it the drop-down it is, at the size it was given.
    fn moved_or_sized(window: HWND) {
        if !DOWN.load(Ordering::Relaxed) || ROLL.lock().is_ok_and(|r| r.is_some()) {
            return;
        }
        let Some(dock) = DOCK.lock().ok().and_then(|d| *d) else {
            return;
        };
        let now = window_rect(window);
        if (now.2, now.3) != (dock.2, dock.3) {
            // Resized where it is. What it was resized to is now where it is
            // docked, or a later drag off the edge would never be told from
            // a resize again.
            if let Ok(mut slot) = DOCK.lock() {
                *slot = Some(now);
            }
            return;
        }
        if (now.0, now.1) == (dock.0, dock.1) {
            return;
        }
        DOWN.store(false, Ordering::Relaxed);
        let free = (
            FREE_W.load(Ordering::Relaxed),
            FREE_H.load(Ordering::Relaxed),
        );
        if free.0 <= 0 || free.1 <= 0 {
            return;
        }
        let mut at = POINT { x: 0, y: 0 };
        // SAFETY: reads the pointer, and places our own window.
        unsafe {
            GetCursorPos(&mut at);
            let (x, y, w, h) = super::detached_at(now, at.x, free);
            let level = if PINNED.load(Ordering::Relaxed) {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            };
            SetWindowPos(window, level, x, y, w, h, SWP_NOACTIVATE);
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
        if msg == WM_TIMER && wparam == ROLL_TIMER {
            step_roll();
            return 0;
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    unsafe extern "system" fn on_main_message(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        if msg == WM_EXITSIZEMOVE {
            moved_or_sized(hwnd);
        }
        // Every `ShowWindow` of the main window - the tray's restore, closing
        // to the tray, a second launch - takes the others with it, not only
        // the drop-down's own: brought back by the tray, a Settings window
        // put away with the drop-down stayed hidden while the app went on
        // believing it open. `lparam` is zero for a `ShowWindow` call, and not
        // for an owner minimizing.
        if msg == WM_SHOWWINDOW && lparam == 0 {
            if wparam != 0 {
                bring_back_the_others();
            } else {
                put_away_the_others(hwnd);
            }
        }
        DefSubclassProc(hwnd, msg, wparam, lparam)
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
        assert_eq!(dock_rect(work, QuakeEdge::Top, 100, 40), (0, 0, 1920, 416));
        assert_eq!(
            dock_rect(work, QuakeEdge::Bottom, 100, 50),
            (0, 520, 1920, 520)
        );
        // A second screen to the left of the first.
        assert_eq!(
            dock_rect((-1280, 0, 0, 1024), QuakeEdge::Top, 100, 50),
            (-1280, 0, 1280, 512)
        );
    }

    #[test]
    fn a_narrower_drop_down_is_centred_on_its_edge() {
        let work = (0, 0, 1920, 1040);
        assert_eq!(dock_rect(work, QuakeEdge::Top, 50, 40), (480, 0, 960, 416));
    }

    #[test]
    fn a_hand_edited_size_is_held_to_the_range_offered() {
        let work = (0, 0, 1000, 1000);
        assert_eq!(dock_rect(work, QuakeEdge::Top, 100, 0).3, 200);
        assert_eq!(dock_rect(work, QuakeEdge::Top, 100, 500).3, 900);
        assert_eq!(dock_rect(work, QuakeEdge::Top, 0, 40).2, 300);
        assert_eq!(dock_rect(work, QuakeEdge::Top, 500, 40).2, 1000);
    }

    #[test]
    fn a_roll_eases_from_nothing_to_all_and_off_lands_at_once() {
        assert_eq!(rolled(0, 200), 0.0);
        assert!(rolled(100, 200) > 0.5, "eased out, so past half by halfway");
        assert_eq!(rolled(200, 200), 1.0);
        assert_eq!(rolled(900, 200), 1.0);
        assert_eq!(rolled(0, 0), 1.0);
    }

    #[test]
    fn rolling_from_the_top_shows_the_bottom_first_and_never_rises_above_the_edge() {
        // Nothing of it, then half, then all of it.
        assert_eq!(roll_frame(QuakeEdge::Top, 400, 0.0), (-400, 400, 400));
        assert_eq!(roll_frame(QuakeEdge::Top, 400, 0.5), (-200, 200, 400));
        assert_eq!(roll_frame(QuakeEdge::Top, 400, 1.0), (0, 0, 400));
        // Whatever shows is the part below the edge: offset plus band top is
        // where the shown part starts, and that is always the edge itself.
        for k in [0.1, 0.3, 0.7] {
            let (dy, from, _) = roll_frame(QuakeEdge::Top, 400, k);
            assert_eq!(dy + from, 0);
        }
    }

    #[test]
    fn rolling_from_the_bottom_shows_the_top_first() {
        assert_eq!(roll_frame(QuakeEdge::Bottom, 400, 0.0), (400, 0, 0));
        assert_eq!(roll_frame(QuakeEdge::Bottom, 400, 0.25), (300, 0, 100));
        assert_eq!(roll_frame(QuakeEdge::Bottom, 400, 1.0), (0, 0, 400));
    }

    #[test]
    fn a_window_dragged_off_its_edge_takes_its_free_size_under_the_pointer() {
        // Docked across a 1920 px screen, dragged down by a point a quarter of
        // the way along it.
        let docked = (0, 300, 1920, 416);
        let (x, y, w, h) = detached_at(docked, 480, (1000, 600));
        assert_eq!((w, h), (1000, 600));
        assert_eq!(y, 300);
        assert_eq!(
            x,
            480 - 250,
            "the pointer is a quarter of the way along it still"
        );
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
