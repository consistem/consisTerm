//! Small looping pictures of what a feature does, for the settings pages - the
//! way GNOME's Settings shows a gesture on its Touchpad page and GNOME Tour
//! shows each thing it introduces.
//!
//! Painted, not played back: each scene is drawn from the theme's own colours
//! at whatever scale the window is at, its words go through [`tr`] like any
//! other, and there is no file to ship or to fall out of step with the
//! interface. A scene is a function of one number, how far through its loop it
//! is, so a still one - with animations switched off - is the same drawing at
//! the moment that explains it best.
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
    /// The title bar moving round the window, the tabs a column at the sides.
    TitleBar,
}

impl Scene {
    /// How long one loop takes. Long enough to read what is typed.
    fn period(self) -> f64 {
        match self {
            Scene::TitleBar => 8.0,
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
            Scene::TitleBar => 0.4,
        }
    }
}

/// Draws `scene` across the width it is given, moving when `animate` is set.
pub fn show(ui: &mut Ui, scene: Scene, animate: bool) {
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
        Scene::DropDown => drop_down(&painter, rect, &ink, t),
        Scene::TitleBar => title_bar(&painter, rect, &ink, t),
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

fn drop_down(painter: &Painter, rect: Rect, ink: &Ink, t: f32) {
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
    // The key: down for a moment on the way in and on the way out.
    let pressed = (0.06..0.12).contains(&t) || (0.62..0.68).contains(&t);
    let key = Rect::from_min_size(
        rect.right_bottom() - Vec2::new(46.0, 30.0),
        Vec2::new(36.0, 22.0),
    );
    painter.rect(
        key.translate(Vec2::new(0.0, if pressed { 2.0 } else { 0.0 })),
        Rounding::same(4.0),
        if pressed { ink.accent } else { ink.screen },
        Stroke::new(1.0_f32, ink.line),
    );
    painter.text(
        key.center() + Vec2::new(0.0, if pressed { 2.0 } else { 0.0 }),
        Align2::CENTER_CENTER,
        "F12",
        FontId::proportional(10.0),
        if pressed { ink.accent_text } else { ink.text },
    );
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

fn title_bar(painter: &Painter, rect: Rect, ink: &Ink, t: f32) {
    // Top, right, bottom, left: a quarter of the loop each, with the move
    // between two of them in the last fifth of the quarter.
    let sides = [0usize, 1, 2, 3];
    let quarter = (t * 4.0).floor() as usize % 4;
    let into = (t * 4.0).fract();
    let k = ease(into, 0.8, 1.0);
    let from = sides[quarter];
    let to = sides[(quarter + 1) % 4];
    let window = Rect::from_center_size(
        rect.center(),
        Vec2::new(rect.height() * 1.5, rect.height() * 0.9),
    );
    painter.rect_filled(window, Rounding::same(6.0), ink.screen);
    for (side, alpha) in [(from, 1.0 - k), (to, k)] {
        if alpha <= 0.0 {
            continue;
        }
        bar_on(painter, window, side, ink, alpha);
    }
}

/// The bar of a window against `side` - 0 top, 1 right, 2 bottom, 3 left -
/// with its tabs: a row along the top or bottom, a column down a side.
fn bar_on(painter: &Painter, window: Rect, side: usize, ink: &Ink, alpha: f32) {
    let thick = window.height() * 0.16;
    let column = window.width() * 0.3;
    let bar = match side {
        0 => Rect::from_min_size(window.min, Vec2::new(window.width(), thick)),
        2 => Rect::from_min_size(
            window.left_bottom() - Vec2::new(0.0, thick),
            Vec2::new(window.width(), thick),
        ),
        1 => Rect::from_min_size(
            window.right_top() - Vec2::new(column, 0.0),
            Vec2::new(column, window.height()),
        ),
        _ => Rect::from_min_size(window.min, Vec2::new(column, window.height())),
    };
    painter.rect_filled(bar, Rounding::same(6.0), faded(ink.chrome, alpha));
    for i in 0..3 {
        let tab = if side.is_multiple_of(2) {
            Rect::from_min_size(
                bar.min + Vec2::new(6.0 + i as f32 * window.width() * 0.22, 3.0),
                Vec2::new(window.width() * 0.2, thick - 6.0),
            )
        } else {
            Rect::from_min_size(
                bar.min + Vec2::new(4.0, thick + i as f32 * thick * 0.9),
                Vec2::new(column - 8.0, thick * 0.75),
            )
        };
        let fill = if i == 0 { ink.accent } else { ink.popup };
        painter.rect_filled(tab, Rounding::same(3.0), faded(fill, alpha));
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
            Scene::TitleBar,
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
                            Scene::DropDown => drop_down(&painter, rect, &ink, t),
                            Scene::TitleBar => title_bar(&painter, rect, &ink, t),
                        }
                    }
                });
            });
        }
    }

    #[test]
    fn typing_reaches_the_whole_text_and_no_further() {
        assert_eq!(typed(0.0, 0.1, 0.5, 10), 0);
        assert_eq!(typed(0.3, 0.1, 0.5, 10), 5);
        assert_eq!(typed(0.9, 0.1, 0.5, 10), 10);
    }
}
