//! Winamp: the 2.91 skin every copy shipped with, and Winamp 5 Classified.
//!
//! Neither is a CSS library, so neither is ported value for value the way
//! 98.css is. Each control is drawn after the skin's own bitmaps - webamp.org's
//! copy of base 2.91, and Classified's `.wsz` - with every colour sampled off
//! them: the transport buttons for a button, the shuffle and repeat lights for
//! a checkbox, the seek bar's thumb for a slider's, the playlist for a list
//! and a field. No pixel of either skin is shipped: Classified's readme
//! reserves its rights, and 2.91's bitmaps are Nullsoft's.
//!
//! Classic is slate and steel: a dark slate face, grey-blue bevelled buttons
//! with a dark outline, gold rails either side of the title, and the
//! playlist's green on black. Classified is the Winamp 5 look made a classic
//! skin: brushed silver, glossy rounded buttons, a navy title bar with silver
//! rails, and its playlist's white on navy.
//!
//! Neither had a dark reading of its own - Classic is dark already - so both
//! draw the same either way.

use egui::{pos2, vec2, Color32, Pos2, Rect, Shape};

use super::prim::{self, fill, hex, inset, Inset};
use super::{title_glyph, Colors, Dir, Glyph, Kit, Metrics, St, Style};

/// Winamp 2.91.
pub struct Winamp;

/// Winamp 5 Classified.
pub struct Classified;

// --- Classic: sampled off base-2.91's MAIN, CBUTTONS, TITLEBAR, PLEDIT -----

/// The main window's slate, its shadow, and the light catching its edges.
const SLATE: Color32 = hex(0x39395a);
const SLATE_DARK: Color32 = hex(0x181829);
const SLATE_LIGHT: Color32 = hex(0x5a5a7a);
/// The title bar's band, from its top row to its foot.
const BAR_TOP: Color32 = hex(0x31314d);
const BAR_FOOT: Color32 = hex(0x232336);
/// The transport buttons: their lit top, face, foot, edges and glyph.
const STEEL_TOP: Color32 = hex(0xe2eef4);
const STEEL: Color32 = hex(0xbdced6);
const STEEL_FOOT: Color32 = hex(0x97a8b9);
const STEEL_LIGHT: Color32 = hex(0xefffff);
const STEEL_SHADE: Color32 = hex(0x7b8494);
const STEEL_EDGE: Color32 = hex(0x181829);
const STEEL_INK: Color32 = hex(0x1e1f25);
/// The title bar's gold: the WINAMP lettering and the rails beside it.
const GOLD: Color32 = hex(0xecce7a);
const GOLD_MID: Color32 = hex(0xc8b072);
const GOLD_DARK: Color32 = hex(0xa3946a);
const GOLD_DEEP: Color32 = hex(0x45413c);
/// The playlist: `Normal=#00FF00` on `NormalBG=#000000`, `SelectedBG=#0000C6`.
const DISPLAY: Color32 = hex(0x000000);
const PHOSPHOR: Color32 = hex(0x00ff00);
const PHOSPHOR_DIM: Color32 = hex(0x007a00);
const PLAY_SELECT: Color32 = hex(0x0000c6);
/// The shuffle light, lit and dark.
const LED_GREEN: Color32 = hex(0x18e018);
const LED_GREEN_OFF: Color32 = hex(0x1f3a26);

// --- Classified: sampled off its main, cbuttons, titlebar, pledit ---------

/// The brushed silver down the main window's side, top to foot.
const SILVER_TOP: Color32 = hex(0xdcdde4);
const SILVER: Color32 = hex(0xc3c8d3);
const SILVER_FOOT: Color32 = hex(0x9aa1ae);
/// The rounded buttons: white gloss over the upper half, silver under it.
const GLOSS: Color32 = hex(0xffffff);
const GLOSS_MID: Color32 = hex(0xe2e5ec);
const GLOSS_LOW: Color32 = hex(0xb9bfcb);
const GLOSS_FOOT: Color32 = hex(0x8d95a3);
const GLOSS_EDGE: Color32 = hex(0x5a616e);
/// The title bar's navy and the silver rails along it.
const NAVY_TOP: Color32 = hex(0x233458);
const NAVY_DEEP: Color32 = hex(0x0b1427);
const NAVY_SHINE: Color32 = hex(0x6b768e);
const RAIL_LIGHT: Color32 = hex(0xb3b9c9);
const RAIL_DARK: Color32 = hex(0x596580);
/// The playlist: `NormalBG=#1C3D7D`, `Normal=#FFFFFF`, `SelectedBG=#6788C9`.
const NAVY: Color32 = hex(0x1c3d7d);
const NAVY_SELECT: Color32 = hex(0x6788c9);
/// The repeat light, lit and dark - lit a stronger blue than the bitmap's
/// glow, which is read at a glance only beside its own dark state.
const LED_BLUE: Color32 = hex(0x3f86ff);
const LED_BLUE_OFF: Color32 = hex(0x5b6372);

/// `a` to `b` by `t`, in the gamma space the bitmaps were drawn in.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    a.lerp_to_gamma(b, t.clamp(0.0, 1.0))
}

/// `rect` filled top to foot from `top` to `foot`, its corners rounded by
/// `radius` CSS pixels, inside one-pixel `rings`.
fn shaded(
    k: &Kit,
    rect: Rect,
    radius: f32,
    top: Color32,
    foot: Color32,
    rings: &[Color32],
) -> Vec<Shape> {
    let rows = (rect.height() / k.u()).max(1.0);
    prim::boxed(
        rect,
        [radius; 4],
        &move |y| mix(top, foot, y / rows),
        rings,
        k.ppp,
    )
}

/// A glossy pill, as Classified's buttons are: white over the upper half,
/// silver under it, the line between them sharp.
fn gloss(k: &Kit, rect: Rect, radius: f32, pressed: bool, rings: &[Color32]) -> Vec<Shape> {
    let rows = (rect.height() / k.u()).max(2.0);
    let (a, b, c, d) = if pressed {
        (GLOSS_LOW, GLOSS_MID, GLOSS_MID, GLOSS_LOW)
    } else {
        (GLOSS, GLOSS_MID, GLOSS_LOW, GLOSS_FOOT)
    };
    prim::boxed(
        rect,
        [radius; 4],
        &move |y| {
            let half = rows / 2.0;
            if y < half {
                mix(a, b, y / half)
            } else {
                mix(c, d, (y - half) / half)
            }
        },
        rings,
        k.ppp,
    )
}

/// Two-tone lines round `rect`, engraved into the face - `dark` outside,
/// `light` a pixel in - stopping either side of `legend`.
fn engraved(
    k: &Kit,
    rect: Rect,
    legend: Option<Rect>,
    dark: Color32,
    light: Color32,
) -> Vec<Shape> {
    let u = k.u();
    let r = prim::snap_rect(rect, k.ppp);
    let gap = legend.map(|l| (l.left() - 2.0 * u, l.right() + 2.0 * u));
    let mut out = Vec::new();
    let top = |y: f32, colour: Color32, out: &mut Vec<Shape>| {
        let line =
            |x0: f32, x1: f32| fill(Rect::from_min_max(pos2(x0, y), pos2(x1, y + u)), colour);
        match gap {
            Some((a, b)) => {
                out.push(line(r.left(), a.max(r.left())));
                out.push(line(b.min(r.right()), r.right()));
            }
            None => out.push(line(r.left(), r.right())),
        }
    };
    top(r.top(), dark, &mut out);
    top(r.top() + u, light, &mut out);
    let span = |x0: f32, y0: f32, x1: f32, y1: f32, c: Color32| {
        fill(Rect::from_min_max(pos2(x0, y0), pos2(x1, y1)), c)
    };
    out.push(span(r.left(), r.top(), r.left() + u, r.bottom(), dark));
    out.push(span(
        r.left() + u,
        r.top() + u,
        r.left() + 2.0 * u,
        r.bottom() - u,
        light,
    ));
    out.push(span(
        r.right() - 2.0 * u,
        r.top(),
        r.right() - u,
        r.bottom() - u,
        dark,
    ));
    out.push(span(r.right() - u, r.top(), r.right(), r.bottom(), light));
    out.push(span(
        r.left(),
        r.bottom() - 2.0 * u,
        r.right() - u,
        r.bottom() - u,
        dark,
    ));
    out.push(span(r.left(), r.bottom() - u, r.right(), r.bottom(), light));
    out
}

/// Rails along a title bar, either side of where the title goes: rows of
/// `colours` from the top, the bar's width less a margin each end. The
/// buttons and the title's backing are drawn over them.
fn rails(k: &Kit, rect: Rect, colours: &[Color32]) -> Vec<Shape> {
    let u = k.u();
    let r = prim::snap_rect(rect, k.ppp);
    let height = colours.len() as f32 * u;
    let top = ((r.center().y - height / 2.0) * k.ppp).round() / k.ppp;
    colours
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let y = top + i as f32 * u;
            fill(
                Rect::from_min_max(
                    pos2(r.left() + 4.0 * u, y),
                    pos2(r.right() - 4.0 * u, y + u),
                ),
                c,
            )
        })
        .collect()
}

/// A light in the middle of `rect` - the shuffle and repeat buttons' - lit
/// in `on`, dark in `off`, with a glint when lit.
fn light(k: &Kit, rect: Rect, lit: bool, on: Color32, off: Color32, round: bool) -> Vec<Shape> {
    let u = k.u();
    let side = (rect.width().min(rect.height()) * 0.45).max(3.0 * u);
    let dot = prim::snap_rect(
        Rect::from_center_size(rect.center(), vec2(side, side)),
        k.ppp,
    );
    let colour = if lit { on } else { off };
    let mut out = if round {
        vec![Shape::circle_filled(dot.center(), side / 2.0, colour)]
    } else {
        vec![fill(dot, colour)]
    };
    if lit {
        out.push(fill(
            Rect::from_min_size(dot.min + vec2(u, u), vec2(u, u)),
            mix(on, Color32::WHITE, 0.7),
        ));
    }
    out
}

/// A pixel-art arrow in `ink`, centred in `rect`.
fn arrow_in(k: &Kit, rect: Rect, dir: Dir, ink: Color32, pressed: bool) -> Vec<Shape> {
    let rows: &[&str] = match dir {
        Dir::Up => &["..#..", ".###.", "#####"],
        Dir::Down => &["#####", ".###.", "..#.."],
        Dir::Left => &["..#", ".##", "###", ".##", "..#"],
        Dir::Right => &["#..", "##.", "###", "##.", "#.."],
    };
    let shift = if pressed { k.u() } else { 0.0 };
    prim::sprite(
        prim::centred(rect, rows, k.ppp) + vec2(shift, shift),
        rows,
        &[('#', ink)],
        k.ppp,
    )
}

fn metrics_for(rounded: bool) -> Metrics {
    Metrics {
        font_size: 12.0,
        title_font_size: 11.0,
        // Winamp's own bar is 14 px; the app's carries more buttons than
        // close and shade, and they have to be aimed at.
        title_height: 18.0,
        title_button: vec2(13.0, 12.0),
        title_button_gap: 2.0,
        window_border: 3.0,
        window_radius: if rounded { 2.0 } else { 0.0 },
        // The playlist's bar is a thin trough with a thumb and no arrows.
        scrollbar: 12.0,
        scroll_arrow: 0.0,
        checkbox: 13.0,
        radio: 13.0,
        button_min: vec2(60.0, 20.0),
        button_pad_x: 10.0,
        field_height: 20.0,
        tab_height: 20.0,
        select_arrow: 16.0,
        menu_item_height: 20.0,
        slider_thumb: if rounded {
            vec2(18.0, 10.0)
        } else {
            vec2(14.0, 10.0)
        },
        slider_track: 4.0,
        label_gap: 6.0,
    }
}

impl Style for Winamp {
    fn colors(&self, _dark: bool) -> Colors {
        Colors {
            face: SLATE,
            text: hex(0xdfe3f0),
            text_disabled: hex(0x6a6a8a),
            text_disabled_shadow: None,
            field: DISPLAY,
            field_text: PHOSPHOR,
            select: PLAY_SELECT,
            select_text: hex(0xffffff),
            title_text: hex(0xffffff),
            title_text_inactive: hex(0x8a8aa8),
            title_shadow: None,
            link: hex(0x7fb2ff),
            tooltip_text: PHOSPHOR,
            subtle: hex(0x9a9ab8),
        }
    }

    fn metrics(&self) -> Metrics {
        metrics_for(false)
    }

    fn pressed_shift(&self) -> f32 {
        1.0
    }

    fn title_centred(&self) -> bool {
        true
    }

    fn title_backing(&self, k: &Kit, text: Rect, _st: St) -> Vec<Shape> {
        // The rails stop short of WINAMP, a few pixels clear either side.
        vec![fill(
            text.expand2(vec2(4.0 * k.u(), 0.0)),
            mix(BAR_TOP, BAR_FOOT, 0.5),
        )]
    }

    fn button_text(&self, _k: &Kit, st: St) -> Color32 {
        if st.disabled {
            STEEL_SHADE
        } else {
            STEEL_INK
        }
    }

    fn button(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // CBUTTONS: steel, lit from the top, inside a dark outline, with a
        // light edge top and left and a shade bottom and right. Held, the
        // face drops and only the shade is left at the top.
        let r = prim::snap_rect(rect, k.ppp);
        let pressed = st.pressed && !st.disabled;
        let (top, foot) = if pressed {
            (STEEL_FOOT, STEEL)
        } else if st.disabled {
            (STEEL, STEEL_FOOT)
        } else {
            (STEEL_TOP, STEEL_FOOT)
        };
        let mut out = shaded(k, r, 0.0, top, foot, &[STEEL_EDGE]);
        let bevel: &[Inset] = if pressed {
            &[(1, 1, STEEL_SHADE)]
        } else {
            &[(1, 1, STEEL_LIGHT), (-1, -1, STEEL_SHADE)]
        };
        out.extend(inset(r.shrink(k.u()), bevel, k.ppp));
        if st.default && !pressed {
            out.push(prim::outline(
                r.expand(k.u()),
                egui::Rounding::ZERO,
                GOLD_DARK,
                k.ppp,
            ));
        }
        out
    }

    fn field(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // The song display: black, sunk into the slate.
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![fill(r, if st.disabled { SLATE_DARK } else { DISPLAY })];
        out.extend(inset(
            r,
            &[(1, 1, SLATE_DARK), (-1, -1, SLATE_LIGHT)],
            k.ppp,
        ));
        if st.focused {
            out.push(prim::outline(
                r.shrink(k.u()),
                egui::Rounding::ZERO,
                PHOSPHOR_DIM,
                k.ppp,
            ));
        }
        out
    }

    fn checkbox(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // The shuffle button's light: a small steel button, lit green.
        let mut out = self.button(k, rect, St { on: false, ..st });
        out.extend(light(k, rect, st.on, LED_GREEN, LED_GREEN_OFF, false));
        out
    }

    fn radio(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let mut out = self.button(k, rect, St { on: false, ..st });
        out.extend(light(k, rect, st.on, LED_GREEN, LED_GREEN_OFF, true));
        out
    }

    fn arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        let ink = if st.disabled { STEEL_SHADE } else { STEEL_INK };
        arrow_in(k, rect, dir, ink, st.pressed)
    }

    fn slider_track(&self, k: &Kit, rect: Rect, _st: St) -> Vec<Shape> {
        // The volume bar's groove, a green line lit along it.
        let h = self.metrics().slider_track * k.u();
        let r = prim::snap_rect(
            Rect::from_center_size(rect.center(), vec2(rect.width(), h)),
            k.ppp,
        );
        let mut out = vec![fill(r, hex(0x0a160a))];
        out.extend(inset(
            r,
            &[(1, 1, SLATE_DARK), (-1, -1, SLATE_LIGHT)],
            k.ppp,
        ));
        out.push(fill(
            Rect::from_center_size(r.center(), vec2(r.width() - 4.0 * k.u(), k.u())),
            hex(0x18a018),
        ));
        out
    }

    fn slider_thumb(&self, k: &Kit, centre: Pos2, st: St) -> Vec<Shape> {
        // POSBAR's thumb: a gold bar in a dark edge.
        let size = self.metrics().slider_thumb * k.u();
        let r = prim::snap_rect(Rect::from_center_size(centre, size), k.ppp);
        let (top, foot) = if st.pressed {
            (GOLD_DARK, GOLD_MID)
        } else {
            (GOLD, GOLD_DARK)
        };
        let mut out = shaded(k, r, 0.0, top, foot, &[GOLD_DEEP]);
        out.extend(inset(r.shrink(k.u()), &[(1, 1, hex(0xfff2c4))], k.ppp));
        out
    }

    fn tab(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // A steel button; the shown one lit, the rest a step darker.
        let r = prim::snap_rect(rect, k.ppp);
        let (top, foot) = if st.on {
            (STEEL_TOP, STEEL)
        } else {
            (STEEL, STEEL_FOOT)
        };
        let mut out = shaded(k, r, 0.0, top, foot, &[STEEL_EDGE]);
        out.extend(inset(r.shrink(k.u()), &[(1, 1, STEEL_LIGHT)], k.ppp));
        out
    }

    fn tab_text(&self, _k: &Kit, _st: St) -> Color32 {
        STEEL_INK
    }

    fn tab_panel(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![fill(r, SLATE)];
        out.extend(inset(
            r,
            &[(1, 1, SLATE_LIGHT), (-1, -1, SLATE_DARK)],
            k.ppp,
        ));
        out
    }

    fn group(&self, k: &Kit, rect: Rect, legend: Option<Rect>) -> Vec<Shape> {
        engraved(k, rect, legend, SLATE_DARK, SLATE_LIGHT)
    }

    fn legend_text(&self, _k: &Kit) -> Color32 {
        GOLD
    }

    fn window(&self, k: &Kit, rect: Rect, _st: St) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = shaded(k, r, 0.0, hex(0x3e3e62), hex(0x2a2a44), &[hex(0x0e0e18)]);
        out.extend(inset(
            r.shrink(k.u()),
            &[(1, 1, SLATE_LIGHT), (-1, -1, SLATE_DARK)],
            k.ppp,
        ));
        out
    }

    fn title_bar(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // TITLEBAR: a dark band, and two gold rails along it - the lit one
        // over the dull one - grey on a window in the background.
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = shaded(k, r, 0.0, BAR_TOP, BAR_FOOT, &[]);
        let colours: &[Color32] = if st.inactive_window {
            &[
                hex(0x8a8aa8),
                hex(0x5a5a7a),
                hex(0x31314d),
                hex(0x8a8aa8),
                hex(0x5a5a7a),
            ]
        } else {
            &[GOLD, GOLD_MID, GOLD_DEEP, GOLD, GOLD_DARK]
        };
        out.extend(rails(k, r, colours));
        out
    }

    fn title_button(&self, k: &Kit, rect: Rect, glyph: Glyph, st: St) -> Vec<Shape> {
        // The close and shade buttons: a dark square in a gold frame, the
        // mark in gold.
        let r = prim::snap_rect(rect, k.ppp);
        let held = st.pressed && !st.disabled;
        let face = if held { GOLD_DEEP } else { BAR_FOOT };
        let frame = if st.inactive_window {
            hex(0x5a5a7a)
        } else {
            GOLD_DARK
        };
        let mut out = vec![fill(r, face)];
        out.push(prim::outline(r, egui::Rounding::ZERO, frame, k.ppp));
        let ink = match k.button_colour(glyph) {
            Some(c) if !st.inactive_window => c,
            _ if st.inactive_window => hex(0x8a8aa8),
            _ if st.hovered => hex(0xfff2c4),
            _ => GOLD,
        };
        out.extend(title_glyph(k, r, glyph, ink, st));
        out
    }

    fn scroll_track(&self, k: &Kit, rect: Rect, _vertical: bool, _st: St) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![fill(r, SLATE_DARK)];
        out.extend(inset(r, &[(-1, -1, SLATE_LIGHT)], k.ppp));
        out
    }

    fn scroll_thumb(&self, k: &Kit, rect: Rect, _vertical: bool, st: St) -> Vec<Shape> {
        // PLEDIT's thumb: gold, like the seek bar's.
        let r = prim::snap_rect(rect.shrink(k.u()), k.ppp);
        let (top, foot) = if st.pressed {
            (GOLD_DARK, GOLD_MID)
        } else {
            (GOLD, GOLD_DARK)
        };
        let mut out = shaded(k, r, 0.0, top, foot, &[GOLD_DEEP]);
        out.extend(inset(r.shrink(k.u()), &[(1, 1, hex(0xfff2c4))], k.ppp));
        out
    }

    fn scroll_corner(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        vec![fill(prim::snap_rect(rect, k.ppp), SLATE)]
    }

    fn menu(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![fill(r, BAR_FOOT)];
        out.extend(inset(
            r,
            &[(1, 1, SLATE_LIGHT), (-1, -1, hex(0x0e0e18))],
            k.ppp,
        ));
        out
    }

    fn menu_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        if st.hovered || st.on {
            vec![fill(prim::snap_rect(rect, k.ppp), PLAY_SELECT)]
        } else {
            Vec::new()
        }
    }

    fn separator(&self, k: &Kit, rect: Rect, vertical: bool) -> Vec<Shape> {
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        if vertical {
            let x = r.center().x;
            vec![
                fill(
                    Rect::from_min_max(pos2(x, r.top()), pos2(x + u, r.bottom())),
                    SLATE_DARK,
                ),
                fill(
                    Rect::from_min_max(pos2(x + u, r.top()), pos2(x + 2.0 * u, r.bottom())),
                    SLATE_LIGHT,
                ),
            ]
        } else {
            let y = r.center().y;
            vec![
                fill(
                    Rect::from_min_max(pos2(r.left(), y), pos2(r.right(), y + u)),
                    SLATE_DARK,
                ),
                fill(
                    Rect::from_min_max(pos2(r.left(), y + u), pos2(r.right(), y + 2.0 * u)),
                    SLATE_LIGHT,
                ),
            ]
        }
    }

    fn tooltip(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        vec![
            fill(r, DISPLAY),
            prim::outline(r, egui::Rounding::ZERO, PHOSPHOR_DIM, k.ppp),
        ]
    }

    fn list_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        if st.on {
            vec![fill(r, PLAY_SELECT)]
        } else if st.hovered {
            vec![fill(r, PLAY_SELECT.gamma_multiply(0.45))]
        } else {
            Vec::new()
        }
    }

    fn list_text(&self, _k: &Kit, st: St) -> Color32 {
        // The playlist's current entry is white, the rest green.
        if st.on {
            hex(0xffffff)
        } else {
            PHOSPHOR
        }
    }

    fn progress(&self, k: &Kit, rect: Rect, fraction: f32) -> Vec<Shape> {
        let mut out = self.field(k, rect, St::default());
        let inner = prim::snap_rect(rect.shrink(2.0 * k.u()), k.ppp);
        let done = Rect::from_min_size(
            inner.min,
            vec2(inner.width() * fraction.clamp(0.0, 1.0), inner.height()),
        );
        out.extend(shaded(k, done, 0.0, LED_GREEN, hex(0x0a7a0a), &[]));
        out
    }

    fn status_bar(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![fill(r, SLATE)];
        out.extend(inset(r, &[(1, 1, SLATE_LIGHT)], k.ppp));
        out
    }

    fn status_field(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        inset(
            prim::snap_rect(rect, k.ppp),
            &[(1, 1, SLATE_DARK), (-1, -1, SLATE_LIGHT)],
            k.ppp,
        )
    }

    fn focus(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        prim::dotted(rect, GOLD, k.ppp)
    }
}

impl Style for Classified {
    fn colors(&self, _dark: bool) -> Colors {
        Colors {
            face: SILVER,
            text: hex(0x000000),
            text_disabled: hex(0x6b7381),
            text_disabled_shadow: None,
            field: NAVY,
            field_text: hex(0xffffff),
            select: NAVY_SELECT,
            select_text: hex(0xffffff),
            title_text: hex(0xffffff),
            title_text_inactive: hex(0x9aa2b4),
            title_shadow: None,
            link: NAVY,
            tooltip_text: hex(0xffffff),
            subtle: hex(0x3b414d),
        }
    }

    fn metrics(&self) -> Metrics {
        metrics_for(true)
    }

    fn title_centred(&self) -> bool {
        true
    }

    fn title_backing(&self, k: &Kit, text: Rect, _st: St) -> Vec<Shape> {
        vec![fill(
            text.expand2(vec2(4.0 * k.u(), 0.0)),
            mix(NAVY_TOP, NAVY_DEEP, 0.5),
        )]
    }

    fn button_text(&self, _k: &Kit, st: St) -> Color32 {
        if st.disabled {
            hex(0x7d8593)
        } else {
            NAVY_DEEP
        }
    }

    fn button(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // The rounded transport buttons: white gloss over silver, in a
        // grey edge; the default one blue, as the lit EQ and PL are.
        let r = prim::snap_rect(rect, k.ppp);
        let radius = (r.height() / k.u() / 2.0).min(6.0);
        let pressed = st.pressed && !st.disabled;
        if st.default && !pressed {
            return shaded(k, r, radius, hex(0xdbe6ff), NAVY_SELECT, &[hex(0x354566)]);
        }
        gloss(k, r, radius, pressed, &[GLOSS_EDGE])
    }

    fn field(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // The playlist's navy, sunk into the silver.
        let r = prim::snap_rect(rect, k.ppp);
        let body = if st.disabled { SILVER_FOOT } else { NAVY };
        let edge = if st.focused { NAVY_SELECT } else { NAVY_DEEP };
        let mut out = prim::boxed(r, [2.0; 4], &move |_| body, &[edge], k.ppp);
        out.extend(inset(r.shrink(k.u()), &[(1, 1, hex(0x16284e))], k.ppp));
        out
    }

    fn checkbox(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // The repeat button's light: a silver bead, lit blue.
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = gloss(k, r, r.height() / k.u() / 2.0, false, &[GLOSS_EDGE]);
        out.extend(light(k, r, st.on, LED_BLUE, LED_BLUE_OFF, true));
        out
    }

    fn radio(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        self.checkbox(k, rect, st)
    }

    fn arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        let ink = if st.disabled {
            hex(0x7d8593)
        } else {
            NAVY_DEEP
        };
        arrow_in(k, rect, dir, ink, st.pressed)
    }

    fn slider_track(&self, k: &Kit, rect: Rect, _st: St) -> Vec<Shape> {
        // The volume bar: a navy groove with a blue line along it.
        let h = self.metrics().slider_track * k.u();
        let r = prim::snap_rect(
            Rect::from_center_size(rect.center(), vec2(rect.width(), h)),
            k.ppp,
        );
        let mut out = prim::boxed(r, [2.0; 4], &|_| NAVY_DEEP, &[GLOSS_EDGE], k.ppp);
        out.push(fill(
            Rect::from_center_size(r.center(), vec2(r.width() - 4.0 * k.u(), k.u())),
            NAVY_SELECT,
        ));
        out
    }

    fn slider_thumb(&self, k: &Kit, centre: Pos2, st: St) -> Vec<Shape> {
        // POSBAR's thumb: a silver pill.
        let size = self.metrics().slider_thumb * k.u();
        let r = prim::snap_rect(Rect::from_center_size(centre, size), k.ppp);
        gloss(k, r, r.height() / k.u() / 2.0, st.pressed, &[GLOSS_EDGE])
    }

    fn tab(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        if st.on {
            gloss(k, r, 4.0, false, &[GLOSS_EDGE])
        } else {
            shaded(k, r, 4.0, GLOSS_LOW, GLOSS_FOOT, &[GLOSS_EDGE])
        }
    }

    fn tab_text(&self, _k: &Kit, _st: St) -> Color32 {
        NAVY_DEEP
    }

    fn tab_panel(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        prim::boxed(r, [3.0; 4], &|_| SILVER, &[GLOSS_EDGE], k.ppp)
    }

    fn group(&self, k: &Kit, rect: Rect, legend: Option<Rect>) -> Vec<Shape> {
        engraved(k, rect, legend, hex(0x7d8593), hex(0xeef0f4))
    }

    fn legend_text(&self, _k: &Kit) -> Color32 {
        NAVY
    }

    fn window(&self, k: &Kit, rect: Rect, _st: St) -> Vec<Shape> {
        // The brushed silver of the main window, in the navy edge its frame
        // ends in, with the white the silver catches just inside it.
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = shaded(k, r, 2.0, SILVER_TOP, SILVER_FOOT, &[NAVY_DEEP]);
        out.extend(inset(r.shrink(k.u()), &[(1, 1, hex(0xffffff))], k.ppp));
        out
    }

    fn title_bar(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // TITLEBAR: navy, a lighter row along its top, and silver rails.
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = shaded(k, r, 0.0, NAVY_TOP, NAVY_DEEP, &[]);
        out.push(fill(
            Rect::from_min_max(r.min, pos2(r.right(), r.top() + k.u())),
            NAVY_SHINE,
        ));
        let colours: &[Color32] = if st.inactive_window {
            &[RAIL_DARK, hex(0x2a3654), RAIL_DARK]
        } else {
            &[RAIL_LIGHT, RAIL_DARK, RAIL_LIGHT]
        };
        out.extend(rails(k, r, colours));
        out
    }

    fn title_button(&self, k: &Kit, rect: Rect, glyph: Glyph, st: St) -> Vec<Shape> {
        // Small silver squares, edged in navy, the mark in navy.
        let r = prim::snap_rect(rect, k.ppp);
        let held = st.pressed && !st.disabled;
        let (top, foot) = if held {
            (hex(0x9aa2b4), hex(0xe4e6ec))
        } else {
            (hex(0xe4e6ec), hex(0x9aa2b4))
        };
        let mut out = shaded(k, r, 1.0, top, foot, &[NAVY_DEEP]);
        let ink = match k.button_colour(glyph) {
            Some(c) if !st.inactive_window => c,
            _ if st.inactive_window => hex(0x596580),
            _ => NAVY_DEEP,
        };
        out.extend(title_glyph(k, r, glyph, ink, st));
        out
    }

    fn scroll_track(&self, k: &Kit, rect: Rect, _vertical: bool, _st: St) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        prim::boxed(r, [2.0; 4], &|_| hex(0x16284e), &[NAVY_DEEP], k.ppp)
    }

    fn scroll_thumb(&self, k: &Kit, rect: Rect, vertical: bool, st: St) -> Vec<Shape> {
        let r = prim::snap_rect(rect.shrink(k.u()), k.ppp);
        let across = if vertical { r.width() } else { r.height() };
        gloss(k, r, across / k.u() / 2.0, st.pressed, &[GLOSS_EDGE])
    }

    fn scroll_corner(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        vec![fill(prim::snap_rect(rect, k.ppp), SILVER)]
    }

    fn menu(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        prim::boxed(r, [2.0; 4], &|_| hex(0xe8eaef), &[GLOSS_EDGE], k.ppp)
    }

    fn menu_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        if st.hovered || st.on {
            vec![fill(prim::snap_rect(rect, k.ppp), NAVY_SELECT)]
        } else {
            Vec::new()
        }
    }

    fn tooltip(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        prim::boxed(r, [2.0; 4], &|_| NAVY, &[NAVY_DEEP], k.ppp)
    }

    fn list_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        if st.on {
            vec![fill(r, NAVY_SELECT)]
        } else if st.hovered {
            vec![fill(r, NAVY_SELECT.gamma_multiply(0.45))]
        } else {
            Vec::new()
        }
    }

    fn list_text(&self, _k: &Kit, st: St) -> Color32 {
        // White, selected or not: the playlist's selection is a fill behind
        // the text, and `Current=#BBD2FF` marks the entry playing, which a
        // list here has none of.
        let _ = st;
        hex(0xffffff)
    }

    fn progress(&self, k: &Kit, rect: Rect, fraction: f32) -> Vec<Shape> {
        let mut out = self.field(k, rect, St::default());
        let inner = prim::snap_rect(rect.shrink(2.0 * k.u()), k.ppp);
        let done = Rect::from_min_size(
            inner.min,
            vec2(inner.width() * fraction.clamp(0.0, 1.0), inner.height()),
        );
        out.extend(shaded(k, done, 1.0, LED_BLUE, hex(0x4f6fb0), &[]));
        out
    }

    fn status_bar(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = shaded(k, r, 0.0, SILVER, SILVER_FOOT, &[]);
        out.push(fill(
            Rect::from_min_max(r.min, pos2(r.right(), r.top() + k.u())),
            hex(0xffffff),
        ));
        out
    }

    fn status_field(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        inset(
            prim::snap_rect(rect, k.ppp),
            &[(1, 1, hex(0x7d8593)), (-1, -1, hex(0xffffff))],
            k.ppp,
        )
    }

    fn focus(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        prim::dotted(rect, NAVY, k.ppp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::skin::{Look, Skin};

    fn kit(skin: Skin) -> Kit {
        Kit::bare(
            Look {
                skin,
                ..Default::default()
            },
            1.0,
        )
    }

    /// Every control both skins draw comes out as something, in every state
    /// a widget can be in.
    #[test]
    fn every_control_draws_in_every_state() {
        let styles: [(&dyn Style, Skin); 2] = [
            (&Winamp, Skin::Winamp),
            (&Classified, Skin::WinampClassified),
        ];
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(80.0, 22.0));
        for (style, skin) in styles {
            let k = kit(skin);
            for st in [
                St::default(),
                St {
                    hovered: true,
                    ..St::default()
                },
                St {
                    pressed: true,
                    ..St::default()
                },
                St {
                    on: true,
                    ..St::default()
                },
                St {
                    disabled: true,
                    ..St::default()
                },
                St {
                    inactive_window: true,
                    ..St::default()
                },
            ] {
                assert!(!style.button(&k, rect, st).is_empty());
                assert!(!style.field(&k, rect, st).is_empty());
                assert!(!style.checkbox(&k, rect, st).is_empty());
                assert!(!style.tab(&k, rect, st).is_empty());
                assert!(!style.title_bar(&k, rect, st).is_empty());
                assert!(!style.title_button(&k, rect, Glyph::Close, st).is_empty());
                assert!(!style.scroll_thumb(&k, rect, true, st).is_empty());
                assert!(!style.slider_thumb(&k, rect.center(), st).is_empty());
            }
            assert!(!style.window(&k, rect, St::default()).is_empty());
            assert!(!style.progress(&k, rect, 0.5).is_empty());
            assert!(!style.group(&k, rect, Some(rect.shrink(20.0))).is_empty());
        }
    }

    /// Text reads on what it is drawn on: labels on the face, a field's text
    /// in the field, a button's label on the button.
    #[test]
    fn the_text_reads_on_what_it_is_drawn_on() {
        use crate::config::theme::contrast;
        for (style, face) in [(&Winamp as &dyn Style, STEEL), (&Classified, GLOSS_MID)] {
            let c = style.colors(false);
            assert!(contrast(c.text, c.face) >= 4.5, "{:?}", c.text);
            assert!(contrast(c.field_text, c.field) >= 4.5);
            assert!(contrast(c.select_text, c.select) >= 3.0);
            let k = kit(Skin::Winamp);
            assert!(contrast(style.button_text(&k, St::default()), face) >= 4.5);
        }
    }

    /// The title's rails are broken where the title goes, by a backing the
    /// bar's own colour.
    #[test]
    fn the_title_is_set_into_its_rails() {
        let k = kit(Skin::Winamp);
        let text = Rect::from_min_size(Pos2::new(100.0, 2.0), vec2(60.0, 12.0));
        assert!(!Winamp.title_backing(&k, text, St::default()).is_empty());
        assert!(Winamp.title_centred() && Classified.title_centred());
    }
}
