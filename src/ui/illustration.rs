//! Small looping pictures of what a feature does, for the settings pages - the
//! way GNOME's Settings shows a gesture on its Touchpad page and GNOME Tour
//! shows each thing it introduces.
//!
//! Painted, not played back: each scene is drawn from the theme's own colours
//! at whatever scale the window is at, its words go through [`tr`] like any
//! other, and there is no file to ship or to fall out of step with the
//! interface. A scene is a function of one number, how far through its loop it
//! is, so a still one is the same drawing at the moment that explains it best.
//!
//! The title bar's picture is not a scene but a preview: the layout the
//! settings beside it are set to, redrawn as they change.
//!
//! A scene asks for its next frame only while it is on screen. An idle window
//! with no Settings open costs nothing, which `app/frame.rs` depends on.

use std::time::Duration;

use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

use crate::i18n::tr;

/// What a scene shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    /// The pointer on a piece of a global, and the tooltip naming it.
    GlobalTooltip,
    /// A subscript typed a letter at a time, the list narrowing under it.
    Autocomplete,
    /// A macro from the right-click menu, its parameter, and the line sent.
    Macros,
    /// IRIS, PowerShell and Git Bash in tabs of one window.
    Shells,
    /// A key pressed and the terminal coming down from the top of the screen.
    DropDown,
}

impl Scene {
    /// How long one loop takes. Long enough to read what is typed.
    fn period(self) -> f64 {
        match self {
            Scene::Shells => 6.0,
            _ => 7.0,
        }
    }

    /// Where a still scene stops: the moment that says the most.
    fn still_at(self) -> f32 {
        match self {
            Scene::GlobalTooltip => 0.7,
            Scene::Autocomplete => 0.62,
            Scene::Macros => 0.66,
            Scene::Shells => 0.5,
            Scene::DropDown => 0.5,
        }
    }
}

/// Draws `scene` across the width it is given, moving when `animate` is set.
pub fn show(ui: &mut Ui, scene: Scene, animate: bool) {
    show_with_key(ui, scene, animate, None);
}

/// [`show`], with the key the scene presses written on its key cap: the
/// shortcut actually set, so the picture does not promise one that is not.
/// `None` for no shortcut, and the picture then presses no key.
pub fn show_with_key(ui: &mut Ui, scene: Scene, animate: bool, key: Option<&str>) {
    let width = ui.available_width().min(420.0);
    let size = Vec2::new(width, (width * 0.46).round());
    // Centred in the row, with the card's padding round it.
    let (outer, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), size.y), Sense::hover());
    let rect = Rect::from_center_size(outer.center(), size);
    if !ui.is_rect_visible(rect) {
        return;
    }
    // Nobody is watching a minimized window: it keeps the frame it has.
    let minimized = ui.input(|i| i.viewport().minimized.unwrap_or(false));
    let t = if animate && !minimized {
        let now = ui.input(|i| i.time);
        // A frame every 33 ms while it is in view, and none once it is not:
        // nothing else asks for this one.
        ui.ctx().request_repaint_after(Duration::from_millis(33));
        ((now % scene.period()) / scene.period()) as f32
    } else {
        scene.still_at()
    };
    let ink = Ink::of(ui);
    let painter = ui.painter_at(rect);
    match scene {
        Scene::GlobalTooltip => global_tooltip(&painter, rect, &ink, t),
        Scene::Autocomplete => autocomplete(&painter, rect, &ink, t),
        Scene::Macros => macros(&painter, rect, &ink, t),
        Scene::Shells => shells(&painter, rect, &ink, t),
        Scene::DropDown => drop_down(&painter, rect, &ink, t, key),
    }
}

/// What the title bar is set to, as far as a picture of it can show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarLayout {
    pub position: crate::config::BarPosition,
    /// The tabs share the buttons' row - along the top or bottom only.
    pub tabs_in_bar: bool,
    pub fixed_tabs: bool,
    pub close_left: bool,
    /// The title bar's own scale, which makes the bar thicker.
    pub scale: f32,
}

impl BarLayout {
    pub fn of(settings: &crate::config::Settings) -> BarLayout {
        BarLayout {
            position: settings.bar_position,
            tabs_in_bar: settings.tabs_in_title_bar,
            fixed_tabs: settings.tab_width == crate::config::TabWidth::Fixed,
            close_left: settings.tab_close_side == crate::config::TabCloseSide::Left,
            scale: settings.title_bar_scale,
        }
    }
}

/// A window laid out as `layout` says: where the bar is, whether the tabs
/// share its row, how wide they are and which end their cross is at.
pub fn title_bar_preview(ui: &mut Ui, layout: BarLayout) {
    let width = ui.available_width().min(420.0);
    let size = Vec2::new(width, (width * 0.46).round());
    let (outer, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), size.y), Sense::hover());
    let rect = Rect::from_center_size(outer.center(), size);
    if !ui.is_rect_visible(rect) {
        return;
    }
    let ink = Ink::of(ui);
    let painter = ui.painter_at(rect);
    let window = Rect::from_center_size(
        rect.center(),
        Vec2::new(rect.height() * 1.6, rect.height() * 0.92),
    );
    paint_layout(&painter, window, &ink, layout);
}

fn paint_layout(painter: &Painter, window: Rect, ink: &Ink, layout: BarLayout) {
    use crate::config::BarPosition;
    painter.rect_filled(window, Rounding::same(6.0), ink.screen);
    let row = (window.height() * 0.13 * layout.scale.clamp(1.0, 2.0)).max(10.0);
    let names = ["IRIS", "USER", "%SYS"];
    // A prompt on the screen, so it reads as a terminal and the bar as round it.
    let mono_font = FontId::monospace((window.height() * 0.08).clamp(7.0, 11.0));
    let prompt = |screen: Rect| {
        painter.text(
            screen.min + Vec2::new(8.0, 6.0),
            Align2::LEFT_TOP,
            "USER>",
            mono_font.clone(),
            ink.weak,
        );
    };
    match layout.position {
        BarPosition::Left | BarPosition::Right => {
            let column = window.width() * 0.3;
            let left = layout.position == BarPosition::Left;
            let bar = if left {
                Rect::from_min_size(window.min, Vec2::new(column, window.height()))
            } else {
                Rect::from_min_size(
                    window.right_top() - Vec2::new(column, 0.0),
                    Vec2::new(column, window.height()),
                )
            };
            painter.rect_filled(bar, Rounding::same(6.0), ink.chrome);
            dots(
                painter,
                Rect::from_min_size(bar.min, Vec2::new(column, row)),
                ink,
                !left,
            );
            for (i, name) in names.iter().enumerate() {
                let tab = Rect::from_min_size(
                    bar.min + Vec2::new(4.0, row + 2.0 + i as f32 * (row + 2.0)),
                    Vec2::new(column - 8.0, row),
                );
                paint_tab(painter, tab, name, i == 0, layout.close_left, ink);
            }
            let screen = if left {
                Rect::from_min_max(Pos2::new(bar.right(), window.top()), window.max)
            } else {
                Rect::from_min_max(window.min, Pos2::new(bar.left(), window.bottom()))
            };
            prompt(screen);
        }
        BarPosition::Top | BarPosition::Bottom => {
            let top = layout.position == BarPosition::Top;
            let rows = if layout.tabs_in_bar { 1.0 } else { 2.0 };
            let thick = row * rows;
            let bar = if top {
                Rect::from_min_size(window.min, Vec2::new(window.width(), thick))
            } else {
                Rect::from_min_size(
                    window.left_bottom() - Vec2::new(0.0, thick),
                    Vec2::new(window.width(), thick),
                )
            };
            painter.rect_filled(bar, Rounding::same(6.0), ink.chrome);
            // The buttons' row, and the tabs' - one and the same when the tabs
            // share it, in which case the buttons take its right-hand end.
            let (buttons_row, tabs_row) = if layout.tabs_in_bar {
                (bar, bar)
            } else {
                let first = Rect::from_min_size(bar.min, Vec2::new(bar.width(), row));
                let second = first.translate(Vec2::new(0.0, row));
                // The buttons stay on the outer edge of the window.
                if top {
                    (first, second)
                } else {
                    (second, first)
                }
            };
            let buttons_w = dots(painter, buttons_row, ink, true);
            if !layout.tabs_in_bar {
                // The session line between the buttons.
                painter.text(
                    buttons_row.left_center() + Vec2::new(8.0, 0.0),
                    Align2::LEFT_CENTER,
                    "IRIS · 1234 · 100x30",
                    FontId::proportional(row * 0.5),
                    ink.weak,
                );
            }
            let end = if layout.tabs_in_bar {
                tabs_row.right() - buttons_w - 4.0
            } else {
                tabs_row.right() - 4.0
            };
            let start = tabs_row.left() + 4.0;
            let shared = (end - start) / names.len() as f32;
            let mut x = start;
            for (i, name) in names.iter().enumerate() {
                let w = if layout.fixed_tabs {
                    (shared * 0.62).min(70.0)
                } else {
                    shared
                };
                let tab = Rect::from_min_size(
                    Pos2::new(x, tabs_row.top() + 2.0),
                    Vec2::new(w - 2.0, tabs_row.height() - 4.0),
                );
                paint_tab(painter, tab, name, i == 0, layout.close_left, ink);
                x += w;
            }
            let screen = if top {
                Rect::from_min_max(Pos2::new(window.left(), bar.bottom()), window.max)
            } else {
                Rect::from_min_max(window.min, Pos2::new(window.right(), bar.top()))
            };
            prompt(screen);
        }
    }
}

/// The window buttons as three dots at the end of `row`, the right-hand one
/// when `right`. Returns the width they take.
fn dots(painter: &Painter, row: Rect, ink: &Ink, right: bool) -> f32 {
    let r = (row.height() * 0.14).clamp(2.0, 3.5);
    let gap = r * 3.6;
    for i in 0..3 {
        let x = if right {
            row.right() - 8.0 - i as f32 * gap
        } else {
            row.left() + 8.0 + i as f32 * gap
        };
        painter.circle_filled(Pos2::new(x, row.center().y), r, ink.weak);
    }
    16.0 + 2.0 * gap
}

/// One tab: filled when it is the one shown, its name, and on the shown one
/// the cross at whichever end the setting puts it.
fn paint_tab(
    painter: &Painter,
    tab: Rect,
    name: &str,
    selected: bool,
    close_left: bool,
    ink: &Ink,
) {
    if selected {
        painter.rect_filled(tab, Rounding::same(3.0), ink.screen);
    } else {
        painter.rect_filled(tab, Rounding::same(3.0), faded(ink.popup, 0.6));
    }
    let font = FontId::proportional((tab.height() * 0.5).clamp(6.0, 11.0));
    painter.text(
        tab.center(),
        Align2::CENTER_CENTER,
        name,
        font,
        if selected { ink.text } else { ink.weak },
    );
    if selected {
        let arm = (tab.height() * 0.14).clamp(1.5, 3.0);
        let x = if close_left {
            tab.left() + 4.0 + arm
        } else {
            tab.right() - 4.0 - arm
        };
        let c = Pos2::new(x, tab.center().y);
        let stroke = Stroke::new(1.0_f32, ink.accent);
        painter.line_segment([c + Vec2::new(-arm, -arm), c + Vec2::new(arm, arm)], stroke);
        painter.line_segment([c + Vec2::new(-arm, arm), c + Vec2::new(arm, -arm)], stroke);
    }
}

/// The colours a scene is drawn in, from the active theme.
struct Ink {
    /// The terminal inside a scene.
    screen: Color32,
    /// The window round it, its bar, and a popup over it.
    chrome: Color32,
    popup: Color32,
    text: Color32,
    weak: Color32,
    accent: Color32,
    accent_text: Color32,
    /// What the user typed, as against what IRIS printed.
    typed: Color32,
    line: Color32,
}

impl Ink {
    fn of(ui: &Ui) -> Ink {
        let v = ui.visuals();
        Ink {
            screen: v.extreme_bg_color,
            chrome: v.window_fill,
            popup: v.widgets.inactive.bg_fill,
            text: v.text_color(),
            weak: v.weak_text_color(),
            accent: v.selection.bg_fill,
            accent_text: v.selection.stroke.color,
            typed: v.hyperlink_color,
            line: v.widgets.noninteractive.bg_stroke.color,
        }
    }
}

// ---------------------------------------------------------------------------
// Timing
// ---------------------------------------------------------------------------

/// 0 before `from`, 1 after `to`, eased between.
fn ease(t: f32, from: f32, to: f32) -> f32 {
    let x = ((t - from) / (to - from)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// 1 while `t` is between `from` and `to`, fading in and out at the ends.
fn during(t: f32, from: f32, to: f32) -> f32 {
    let fade = 0.03;
    ease(t, from, from + fade) * (1.0 - ease(t, to - fade, to))
}

/// How many of `count` characters are typed by `t`, typing from `from` to `to`.
fn typed(t: f32, from: f32, to: f32, count: usize) -> usize {
    ((((t - from) / (to - from)).clamp(0.0, 1.0)) * count as f32).floor() as usize
}

fn lerp(a: Pos2, b: Pos2, k: f32) -> Pos2 {
    a + (b - a) * k
}

fn faded(colour: Color32, alpha: f32) -> Color32 {
    colour.gamma_multiply(alpha.clamp(0.0, 1.0))
}

// ---------------------------------------------------------------------------
// Pieces every scene is made of
// ---------------------------------------------------------------------------

/// A window: its bar with one tab per name, the active one in the screen's
/// colour, and the screen under it. Returns the screen.
fn window(painter: &Painter, rect: Rect, ink: &Ink, tabs: &[&str], active: usize) -> Rect {
    let bar_h = (rect.height() * 0.13).max(12.0);
    painter.rect_filled(rect, Rounding::same(6.0), ink.chrome);
    let screen = Rect::from_min_max(rect.min + Vec2::new(0.0, bar_h), rect.max);
    painter.rect_filled(
        screen,
        Rounding {
            nw: 0.0,
            ne: 0.0,
            sw: 6.0,
            se: 6.0,
        },
        ink.screen,
    );
    let font = FontId::proportional(bar_h * 0.55);
    let tab_w = (rect.width() * 0.24).min(120.0);
    for (i, name) in tabs.iter().enumerate() {
        let tab = Rect::from_min_size(
            rect.min + Vec2::new(8.0 + i as f32 * tab_w, 0.0),
            Vec2::new(tab_w, bar_h),
        );
        if i == active {
            painter.rect_filled(tab, Rounding::ZERO, ink.screen);
        }
        let colour = if i == active { ink.text } else { ink.weak };
        painter.text(
            tab.center(),
            Align2::CENTER_CENTER,
            *name,
            font.clone(),
            colour,
        );
    }
    // The window buttons, as dots: enough to say "window" at this size.
    for i in 0..3 {
        painter.circle_filled(
            Pos2::new(
                rect.right() - 10.0 - i as f32 * 10.0,
                rect.top() + bar_h / 2.0,
            ),
            2.5,
            ink.weak,
        );
    }
    screen
}

/// Monospace text, one character per cell. `cell` is the cell's width.
struct Mono {
    font: FontId,
    cell: f32,
    row: f32,
}

impl Mono {
    fn for_screen(painter: &Painter, screen: Rect) -> Mono {
        let size = (screen.height() / 7.5).clamp(7.0, 13.0);
        let font = FontId::monospace(size);
        let cell = painter.ctx().fonts(|f| f.glyph_width(&font, 'M'));
        Mono {
            font,
            cell,
            row: size * 1.45,
        }
    }

    /// Where row `row`, column `col` starts on `screen`.
    fn at(&self, screen: Rect, col: usize, row: usize) -> Pos2 {
        screen.min + Vec2::new(8.0 + col as f32 * self.cell, 6.0 + row as f32 * self.row)
    }

    fn text(&self, painter: &Painter, at: Pos2, text: &str, colour: Color32) {
        painter.text(at, Align2::LEFT_TOP, text, self.font.clone(), colour);
    }
}

/// The pointer, its tip at `at`.
fn pointer(painter: &Painter, at: Pos2, ink: &Ink, pressed: bool) {
    let s = 11.0;
    let points = vec![
        at,
        at + Vec2::new(0.0, s),
        at + Vec2::new(s * 0.28, s * 0.74),
        at + Vec2::new(s * 0.68, s * 0.7),
    ];
    let fill = if pressed { ink.accent } else { Color32::WHITE };
    painter.add(egui::Shape::convex_polygon(
        points,
        fill,
        Stroke::new(1.0_f32, Color32::BLACK),
    ));
}

/// A popup box with `lines`, the one at `selected` highlighted. Returns it.
fn popup(
    painter: &Painter,
    top_left: Pos2,
    lines: &[&str],
    selected: Option<usize>,
    font: &FontId,
    ink: &Ink,
    alpha: f32,
) -> Rect {
    let row = font.size * 1.5;
    let width = lines
        .iter()
        .map(|l| {
            painter.ctx().fonts(|f| {
                f.layout_no_wrap(l.to_string(), font.clone(), ink.text)
                    .size()
                    .x
            })
        })
        .fold(0.0, f32::max)
        + 16.0;
    let rect = Rect::from_min_size(top_left, Vec2::new(width, row * lines.len() as f32 + 6.0));
    painter.rect(
        rect,
        Rounding::same(4.0),
        faded(ink.popup, alpha),
        Stroke::new(1.0_f32, faded(ink.line, alpha)),
    );
    for (i, line) in lines.iter().enumerate() {
        let y = rect.top() + 3.0 + i as f32 * row;
        if selected == Some(i) {
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(rect.left() + 2.0, y), Vec2::new(width - 4.0, row)),
                Rounding::same(3.0),
                faded(ink.accent, alpha),
            );
        }
        let colour = if selected == Some(i) {
            ink.accent_text
        } else {
            ink.text
        };
        painter.text(
            Pos2::new(rect.left() + 8.0, y + row / 2.0),
            Align2::LEFT_CENTER,
            *line,
            font.clone(),
            faded(colour, alpha),
        );
    }
    rect
}

// ---------------------------------------------------------------------------
// The scenes
// ---------------------------------------------------------------------------

fn global_tooltip(painter: &Painter, rect: Rect, ink: &Ink, t: f32) {
    let screen = window(painter, rect, ink, &["IRIS"], 0);
    let mono = Mono::for_screen(painter, screen);
    mono.text(painter, mono.at(screen, 0, 0), "USER>", ink.weak);
    mono.text(painter, mono.at(screen, 5, 0), "zw ^CLI(1)", ink.typed);
    let line = "^CLI(1)=\"Maria^SP^1985\"";
    mono.text(painter, mono.at(screen, 0, 1), line, ink.text);
    mono.text(painter, mono.at(screen, 0, 2), "USER>", ink.weak);
    // The piece being pointed at: "SP", the second.
    let col = line.find("SP").unwrap_or(0);
    let piece = Rect::from_min_size(
        mono.at(screen, col, 1) - Vec2::new(1.0, 1.0),
        Vec2::new(mono.cell * 2.0 + 2.0, mono.row),
    );
    let reach = ease(t, 0.05, 0.35);
    let start = screen.right_bottom() - Vec2::new(20.0, 16.0);
    let tip = lerp(start, piece.center() + Vec2::new(2.0, 2.0), reach);
    let lit = during(t, 0.38, 0.92);
    painter.rect_filled(piece, Rounding::same(2.0), faded(ink.accent, lit * 0.6));
    let shown = during(t, 0.45, 0.92);
    if shown > 0.0 {
        let font = FontId::proportional(mono.font.size);
        popup(
            painter,
            piece.left_bottom() + Vec2::new(0.0, 6.0),
            &[tr("State · %String(2)"), tr("Values: SP, RJ, MG")],
            None,
            &font,
            ink,
            shown,
        );
    }
    pointer(painter, tip, ink, false);
}

fn autocomplete(painter: &Painter, rect: Rect, ink: &Ink, t: f32) {
    let screen = window(painter, rect, ink, &["IRIS"], 0);
    let mono = Mono::for_screen(painter, screen);
    let full = "w ^mtemp(\"CC";
    let accept = 0.78;
    let mut line: String = full
        .chars()
        .take(typed(t, 0.05, 0.5, full.chars().count()))
        .collect();
    if t >= accept {
        line = format!("{full}02\"");
    }
    mono.text(painter, mono.at(screen, 0, 0), "USER>", ink.weak);
    mono.text(painter, mono.at(screen, 5, 0), &line, ink.typed);
    // The caret, blinking.
    let caret = mono.at(screen, 5 + line.chars().count(), 0);
    if (t * 14.0) as i32 % 2 == 0 || t < accept {
        painter.rect_filled(
            Rect::from_min_size(caret, Vec2::new(mono.cell * 0.15 + 1.0, mono.row * 0.8)),
            Rounding::ZERO,
            ink.text,
        );
    }
    // The list opens once a subscript has begun, and narrows as it is typed.
    let typed_sub = line
        .split('(')
        .nth(1)
        .map(|s| s.trim_start_matches('"'))
        .unwrap_or("");
    if line.contains('(') && t < accept {
        let all = ["\"CA10\"", "\"CB07\"", "\"CC01\"", "\"CC02\""];
        let shown: Vec<&str> = all
            .iter()
            .copied()
            .filter(|s| s.trim_matches('"').starts_with(typed_sub))
            .collect();
        let selected = if t > 0.62 {
            shown.len().checked_sub(1)
        } else {
            Some(0)
        };
        let at = mono.at(screen, 5 + line.find('(').unwrap_or(0) + 1, 1);
        popup(
            painter,
            at,
            &shown,
            selected,
            &mono.font,
            ink,
            ease(t, 0.18, 0.24),
        );
    }
}

fn macros(painter: &Painter, rect: Rect, ink: &Ink, t: f32) {
    let screen = window(painter, rect, ink, &["IRIS"], 0);
    let mono = Mono::for_screen(painter, screen);
    let sent = t > 0.78;
    mono.text(painter, mono.at(screen, 0, 0), "USER>", ink.weak);
    if sent {
        mono.text(painter, mono.at(screen, 5, 0), "ZN \"DEV\"", ink.typed);
        mono.text(painter, mono.at(screen, 0, 1), "DEV>", ink.weak);
    }
    let font = FontId::proportional(mono.font.size);
    let click = mono.at(screen, 14, 0) + Vec2::new(0.0, mono.row * 0.5);
    let tip = lerp(
        screen.right_bottom() - Vec2::new(20.0, 14.0),
        click,
        ease(t, 0.03, 0.18),
    );
    let menu = during(t, 0.2, 0.56);
    if menu > 0.0 {
        let groups = popup(
            painter,
            click + Vec2::new(4.0, 4.0),
            &[tr("Copy"), tr("Macros")],
            Some(1),
            &font,
            ink,
            menu,
        );
        let lit = (t > 0.4).then_some(1);
        popup(
            painter,
            Pos2::new(groups.right() + 2.0, groups.top()),
            &[
                tr("Show global"),
                tr("Switch namespace"),
                tr("Kill a global"),
            ],
            lit,
            &font,
            ink,
            ease(t, 0.28, 0.32) * menu,
        );
    }
    let ask = during(t, 0.56, 0.78);
    if ask > 0.0 {
        let dialog = Rect::from_center_size(
            screen.center(),
            Vec2::new(screen.width() * 0.55, mono.row * 2.6),
        );
        painter.rect(
            dialog,
            Rounding::same(6.0),
            faded(ink.popup, ask),
            Stroke::new(1.0_f32, faded(ink.line, ask)),
        );
        painter.text(
            dialog.left_top() + Vec2::new(10.0, 8.0),
            Align2::LEFT_TOP,
            tr("Namespace"),
            font.clone(),
            faded(ink.weak, ask),
        );
        let field = Rect::from_min_size(
            dialog.left_top() + Vec2::new(10.0, mono.row * 1.2),
            Vec2::new(dialog.width() - 20.0, mono.row),
        );
        painter.rect(
            field,
            Rounding::same(3.0),
            faded(ink.screen, ask),
            Stroke::new(1.0_f32, faded(ink.accent, ask)),
        );
        let value: String = "DEV".chars().take(typed(t, 0.6, 0.7, 3)).collect();
        painter.text(
            field.left_center() + Vec2::new(6.0, 0.0),
            Align2::LEFT_CENTER,
            value,
            mono.font.clone(),
            faded(ink.text, ask),
        );
    }
    pointer(
        painter,
        tip,
        ink,
        (0.18..0.21).contains(&t) || (0.5..0.54).contains(&t),
    );
}

fn shells(painter: &Painter, rect: Rect, ink: &Ink, t: f32) {
    let names = ["IRIS", "PowerShell", "Git Bash"];
    let count = 1 + usize::from(t > 0.33) + usize::from(t > 0.66);
    let active = count - 1;
    let screen = window(painter, rect, ink, &names[..count], active);
    let mono = Mono::for_screen(painter, screen);
    let (prompt, command, answer) = match active {
        0 => ("USER>", "w $ZV", "IRIS for Windows 2024.1"),
        1 => ("PS C:\\>", "Get-Location", "C:\\Users\\you"),
        _ => ("$", "git status", "On branch main"),
    };
    let local = (t * 3.0).fract();
    let shown: String = command
        .chars()
        .take(typed(local, 0.15, 0.5, command.chars().count()))
        .collect();
    let width = prompt.chars().count() + 1;
    mono.text(painter, mono.at(screen, 0, 0), prompt, ink.weak);
    mono.text(painter, mono.at(screen, width, 0), &shown, ink.typed);
    if local > 0.6 {
        mono.text(painter, mono.at(screen, 0, 1), answer, ink.text);
        mono.text(painter, mono.at(screen, 0, 2), prompt, ink.weak);
    }
}

fn drop_down(painter: &Painter, rect: Rect, ink: &Ink, t: f32, key: Option<&str>) {
    // The desktop, with an ordinary window on it the terminal comes down over.
    painter.rect_filled(rect, Rounding::same(6.0), ink.chrome);
    let other = Rect::from_min_size(
        rect.min + Vec2::new(rect.width() * 0.12, rect.height() * 0.3),
        Vec2::new(rect.width() * 0.5, rect.height() * 0.55),
    );
    painter.rect(
        other,
        Rounding::same(4.0),
        ink.popup,
        Stroke::new(1.0_f32, ink.line),
    );
    // The key: down for a moment on the way in and on the way out, and as
    // wide as the chord written on it.
    if let Some(label) = key {
        let pressed = (0.06..0.12).contains(&t) || (0.62..0.68).contains(&t);
        let font = FontId::proportional(10.0);
        let colour = if pressed { ink.accent_text } else { ink.text };
        let text = painter.layout_no_wrap(label.to_string(), font, colour);
        let size = Vec2::new((text.size().x + 16.0).max(36.0), 22.0);
        let cap = Rect::from_min_size(rect.right_bottom() - size - Vec2::new(10.0, 8.0), size)
            .translate(Vec2::new(0.0, if pressed { 2.0 } else { 0.0 }));
        painter.rect(
            cap,
            Rounding::same(4.0),
            if pressed { ink.accent } else { ink.screen },
            Stroke::new(1.0_f32, ink.line),
        );
        painter.galley(cap.center() - text.size() / 2.0, text, colour);
    }
    let down = ease(t, 0.1, 0.32) * (1.0 - ease(t, 0.66, 0.86));
    if down > 0.0 {
        let height = rect.height() * 0.45;
        let term = Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() - height + height * down),
            Vec2::new(rect.width(), height),
        );
        let painter = painter.with_clip_rect(rect);
        painter.rect_filled(term, Rounding::same(4.0), ink.screen);
        let mono = Mono::for_screen(&painter, term.expand2(Vec2::new(0.0, term.height())));
        mono.text(&painter, term.min + Vec2::new(8.0, 6.0), "USER>", ink.weak);
        painter.hline(
            term.x_range(),
            term.bottom(),
            Stroke::new(2.0_f32, ink.accent),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scene_draws_still_and_at_every_point_of_its_loop() {
        let ctx = egui::Context::default();
        for scene in [
            Scene::GlobalTooltip,
            Scene::Autocomplete,
            Scene::Macros,
            Scene::Shells,
            Scene::DropDown,
        ] {
            for animate in [false, true] {
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| show(ui, scene, animate));
                });
            }
            // Every phase, not only the one the clock happens to be at.
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 184.0));
                    let ink = Ink::of(ui);
                    let painter = ui.painter_at(rect);
                    for step in 0..=40 {
                        let t = step as f32 / 40.0;
                        match scene {
                            Scene::GlobalTooltip => global_tooltip(&painter, rect, &ink, t),
                            Scene::Autocomplete => autocomplete(&painter, rect, &ink, t),
                            Scene::Macros => macros(&painter, rect, &ink, t),
                            Scene::Shells => shells(&painter, rect, &ink, t),
                            Scene::DropDown => {
                                drop_down(&painter, rect, &ink, t, Some("Ctrl+Shift+F12"));
                                drop_down(&painter, rect, &ink, t, None);
                            }
                        }
                    }
                });
            });
        }
    }

    #[test]
    fn the_title_bar_preview_draws_every_layout_the_settings_can_make() {
        use crate::config::BarPosition;
        let ctx = egui::Context::default();
        for position in BarPosition::ALL {
            for tabs_in_bar in [false, true] {
                for fixed_tabs in [false, true] {
                    for close_left in [false, true] {
                        let layout = BarLayout {
                            position,
                            tabs_in_bar,
                            fixed_tabs,
                            close_left,
                            scale: 1.5,
                        };
                        let _ = ctx.run(egui::RawInput::default(), |ctx| {
                            egui::CentralPanel::default()
                                .show(ctx, |ui| title_bar_preview(ui, layout));
                        });
                    }
                }
            }
        }
    }

    #[test]
    fn typing_reaches_the_whole_text_and_no_further() {
        assert_eq!(typed(0.0, 0.1, 0.5, 10), 0);
        assert_eq!(typed(0.3, 0.1, 0.5, 10), 5);
        assert_eq!(typed(0.9, 0.1, 0.5, 10), 10);
    }
}
