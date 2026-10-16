//! The Super Nintendo, after snes.css (devMiguelCarrero/snes.css, MIT).
//!
//! Every control of the library is one box: a fill, the six-pixel line of
//! `--border-image` round it - a 30 px sheet sliced at 12, laid 6 px outside
//! the box, its corners cut to a single step - and, on what stands up off the
//! page, `generate-shadow-shine`: a strip of white at .3 along the inside of
//! the top and down the left, a strip of black at .2 along the bottom and up
//! the right, each running into its corner as a stair. Under the pointer the
//! two deepen to .4 and .3; held, they change places.
//!
//! The library's pixel is 6 px at the size a web page sets it, which would
//! make a title bar the height of a toolbar; here it is two, everything else
//! in proportion, and the icons - its checkbox, the radio's diamond, the
//! select's caret - one CSS pixel to a cell, as a 16 px label would draw them.
//!
//! snes.css has no window, so the frame is a container's, and the title bar is
//! the console's purple carrying its buttons in the library's colours.
//! The dark reading is the same boxes on Dusk, the library's own ink.

use egui::{pos2, vec2, Color32, Pos2, Rect, Shape};

use super::prim::{self, fill, hex, rgba};
use super::{Colors, Dir, Face, Fonts, Glyph, Kit, Metrics, St, Style};

pub struct Snes;

/// `$pixel-regular`, in CSS pixels as this skin draws it.
const PX: f32 = 2.0;

/// The line of `--border-image`.
const LINE: Color32 = hex(0x181010);
/// `$color-dusk`, the library's ink; `$color-text-hover`.
const DUSK: Color32 = hex(0x2c3e50);
const DUSK_HOVER: Color32 = hex(0x566573);
const PLUMBER: Color32 = hex(0xf22561);
const NATURE: Color32 = hex(0x4bb244);
const SUNSHINE: Color32 = hex(0xf2c019);
const OCEAN: Color32 = hex(0x4eb6d9);
const PHANTOM: Color32 = hex(0x9b5de5);
const GALAXY: Color32 = hex(0x5a7d9a);
const GREY: Color32 = hex(0xe5e5e5);
const AGED: Color32 = hex(0xfcf4d9);
const WHITE: Color32 = hex(0xffffff);
/// `$color-disabled`.
const DISABLED: Color32 = hex(0xb5ced1);

/// The page, the fields and the ink of a reading.
struct Paper {
    page: Color32,
    field: Color32,
    ink: Color32,
    soft: Color32,
}

fn paper(dark: bool) -> Paper {
    if dark {
        Paper {
            page: DUSK,
            field: hex(0x1b2631),
            ink: AGED,
            soft: hex(0xb5ced1),
        }
    } else {
        Paper {
            page: GREY,
            field: WHITE,
            ink: DUSK,
            soft: DUSK_HOVER,
        }
    }
}

/// `$button-shine` and `$button-shadow` for a control's state, or none for
/// one that lies flat.
fn glow(st: St) -> (Color32, Color32) {
    let (shine, shadow) = if st.hovered && !st.disabled {
        (rgba(255, 255, 255, 0.4), rgba(0, 0, 0, 0.3))
    } else {
        (rgba(255, 255, 255, 0.3), rgba(0, 0, 0, 0.2))
    };
    // Held, and disabled, the light comes from below.
    if (st.pressed && !st.disabled) || st.disabled {
        (shadow, shine)
    } else {
        (shine, shadow)
    }
}

/// One of the library's boxes in `rect`: the fill, the stepped line round
/// it - inside `rect`, the line being 6 px outside a box in the CSS - and the
/// shine and shadow when `lit` gives them.
fn boxed(k: &Kit, rect: Rect, body: Color32, lit: Option<(Color32, Color32)>) -> Vec<Shape> {
    let p = PX * k.u();
    let outer = prim::snap_rect(rect, k.ppp);
    let r = outer.shrink(p);
    if r.width() < 2.0 * p || r.height() < 2.0 * p {
        return vec![fill(outer, body)];
    }
    let span = |x0: f32, y0: f32, x1: f32, y1: f32, c: Color32| {
        fill(Rect::from_min_max(pos2(x0, y0), pos2(x1, y1)), c)
    };
    let (l, t, rt, b) = (r.left(), r.top(), r.right(), r.bottom());
    let mut out = vec![fill(r, body)];
    if let Some((shine, shadow)) = lit {
        // `::after`, the shine, turned half a turn: the top row, then the
        // stair down the left.
        out.push(span(l + p, t, rt - p, t + p, shine));
        out.push(span(l, t + p, l + 2.0 * p, t + 2.0 * p, shine));
        out.push(span(l, t + 2.0 * p, l + p, b - p, shine));
        // `::before`, the shadow: the bottom row and the stair up the right.
        out.push(span(l + p, b - p, rt - p, b, shadow));
        out.push(span(rt - 2.0 * p, b - 2.0 * p, rt, b - p, shadow));
        out.push(span(rt - p, t + p, rt, b - 2.0 * p, shadow));
    }
    // The corner cell of the sheet, inside the box's corner...
    for (x, y) in [(l, t), (rt - p, t), (l, b - p), (rt - p, b - p)] {
        out.push(span(x, y, x + p, y + p, LINE));
    }
    // ...and its edges, outside it, stopping short of the corners.
    out.push(span(l + p, t - p, rt - p, t, LINE));
    out.push(span(l + p, b, rt - p, b + p, LINE));
    out.push(span(l - p, t + p, l, b - p, LINE));
    out.push(span(rt, t + p, rt + p, b - p, LINE));
    out
}

/// `color-mix(in srgb, currentColor, white|black n%)`.
fn mix(c: Color32, with: Color32, amount: f32) -> Color32 {
    c.lerp_to_gamma(with, amount)
}

/// A sprite of the library's: digit rows, `0` for none and each other digit
/// the colour at that place of `colours` - `1` the first.
fn icon(k: &Kit, rect: Rect, rows: &[&str], colours: &[Color32]) -> Vec<Shape> {
    let ink: Vec<(char, Color32)> = colours
        .iter()
        .enumerate()
        .map(|(i, &c)| (char::from(b'1' + i as u8), c))
        .collect();
    let at = prim::centred(rect, rows, k.ppp);
    prim::sprite(at, rows, &ink, k.ppp)
}

/// `checkbox.scss`: `1` black, `2` white, `3` the colour, `4` it lighter,
/// `5` it darker.
const CHECKBOX: [&str; 13] = [
    "000000000000000",
    "001111111101110",
    "014444443312221",
    "143113333122221",
    "131221331222210",
    "131222112222100",
    "131222222221000",
    "133122222211000",
    "133312222131000",
    "133331221331000",
    "133333113351000",
    "013355555510000",
    "001111111100000",
];

/// `checkbox-empty.scss`.
const CHECKBOX_EMPTY: [&str; 13] = [
    "000000000000000",
    "001111111100000",
    "010000000010000",
    "100000000001000",
    "100000000001000",
    "100000000001000",
    "100000000001000",
    "100000000001000",
    "100000000001000",
    "100000000001000",
    "100000000001000",
    "010000000010000",
    "001111111100000",
];

/// `diamond.scss`: `1` black, `2` white, `3` the colour, `4` lighter, `5`
/// and `6` darker.
const DIAMOND: [&str; 13] = [
    "0055555555566600",
    "0534422224443560",
    "5344222222444356",
    "6355333333335531",
    "6356333555665531",
    "6536544556666351",
    "0635634356663510",
    "0013553366635100",
    "0001353366351000",
    "0000135353510000",
    "0000013335100000",
    "0000001351000000",
    "0000000110000000",
];

/// `caret.scss`, pointing down: `1` black, `2` white (unused), `3` the
/// colour, `4` and `5` lighter, `6`, `7` and `8` darker.
const CARET: [&str; 11] = [
    "11000000000011",
    "15100000000181",
    "14510000001831",
    "14451000018331",
    "16445100183371",
    "01644518833710",
    "00164458337100",
    "00016443371000",
    "00001643710000",
    "00000167100000",
    "00000011000000",
];

/// The caret's colours for `colour`.
fn caret_colours(colour: Color32) -> [Color32; 8] {
    [
        hex(0x000000),
        WHITE,
        colour,
        mix(colour, WHITE, 0.15),
        mix(colour, WHITE, 0.35),
        mix(colour, Color32::BLACK, 0.10),
        mix(colour, Color32::BLACK, 0.35),
        mix(colour, Color32::BLACK, 0.15),
    ]
}

/// `rows` turned to point `dir`, from pointing down.
fn turned(rows: &[&str], dir: Dir) -> Vec<String> {
    let grid: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
    let (h, w) = (grid.len(), grid.first().map_or(0, Vec::len));
    match dir {
        Dir::Down => rows.iter().map(|r| (*r).to_owned()).collect(),
        Dir::Up => rows.iter().rev().map(|r| (*r).to_owned()).collect(),
        Dir::Right => (0..w)
            .map(|x| (0..h).map(|y| grid[y][x]).collect())
            .collect(),
        Dir::Left => (0..w)
            .map(|x| (0..h).rev().map(|y| grid[y][x]).collect())
            .collect(),
    }
}

impl Style for Snes {
    fn colors(&self, dark: bool) -> Colors {
        let p = paper(dark);
        Colors {
            face: p.page,
            text: p.ink,
            text_disabled: if dark { hex(0x7f8c8d) } else { hex(0x8a9aa5) },
            text_disabled_shadow: None,
            field: p.field,
            field_text: p.ink,
            select: OCEAN,
            select_text: DUSK,
            title_text: WHITE,
            title_text_inactive: hex(0xe0d4f5),
            title_shadow: Some(hex(0x3b1f6b)),
            link: if dark { OCEAN } else { hex(0x1f6f99) },
            tooltip_text: DUSK,
            subtle: p.soft,
        }
    }

    fn metrics(&self) -> Metrics {
        Metrics {
            // Press Start 2P is drawn on an 8 px grid: a font pixel is a
            // screen pixel at 8, and anything between blurs it.
            font_size: 8.0,
            title_font_size: 8.0,
            title_height: 24.0,
            title_button: vec2(16.0, 16.0),
            title_button_gap: 6.0,
            close_gap: 6.0,
            // The line and the step inside it.
            window_border: 2.0 * PX,
            window_radius: 0.0,
            scrollbar: 16.0,
            scroll_arrow: 16.0,
            checkbox: 15.0,
            radio: 16.0,
            button_min: vec2(64.0, 20.0),
            button_pad_x: 10.0,
            field_height: 20.0,
            tab_height: 22.0,
            select_arrow: 18.0,
            menu_item_height: 18.0,
            slider_thumb: vec2(16.0, 14.0),
            slider_track: 8.0,
            label_gap: 8.0,
        }
    }

    fn fonts(&self) -> Fonts {
        // `$main-font: 'Press Start 2P'`, every control and label in it -
        // with a line gap of its own, and so, as the OFL asks of a changed
        // face, under another name (assets/fonts/press-start-2p).
        let face = Face::Bundled(include_bytes!(
            "../../../assets/fonts/press-start-2p/SNESPixel8px-Regular.ttf"
        ));
        Fonts {
            body: Some(face),
            bold: Some(face),
            title: Some(face),
        }
    }

    fn title_centred(&self) -> bool {
        false
    }

    fn button_text(&self, k: &Kit, st: St) -> Color32 {
        // `.snes-button { color: $main-color }`, on every button colour.
        let _ = k;
        if st.disabled {
            DUSK_HOVER
        } else {
            DUSK
        }
    }

    fn button(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `.snes-button`: Nature green; the default one, Enter's, in Ocean.
        let body = if st.disabled {
            DISABLED
        } else if st.default {
            OCEAN
        } else {
            NATURE
        };
        let mut out = boxed(k, rect, body, Some(glow(st)));
        if st.focused {
            out.extend(self.focus(k, rect.shrink(4.0 * PX * k.u())));
        }
        out
    }

    fn field(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `.snes-input`: white in the line, flat; disabled, `$color-disabled`.
        let p = paper(k.look.dark);
        let body = if st.disabled { DISABLED } else { p.field };
        let mut out = boxed(k, rect, body, None);
        if st.focused {
            // The caret the field takes is its mark of focus; the line
            // turns Phantom, the colour the library gives a select's arrow.
            let u = k.u();
            let r = prim::snap_rect(rect, k.ppp).shrink(PX * u);
            out.push(fill(
                Rect::from_min_max(pos2(r.left(), r.bottom() - u), r.right_bottom()),
                PHANTOM,
            ));
        }
        out
    }

    fn checkbox(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let colour = if st.disabled { DISABLED } else { NATURE };
        let ink = paper(k.look.dark).ink;
        if st.on {
            let colours = [
                hex(0x000000),
                WHITE,
                colour,
                mix(colour, WHITE, 0.55),
                mix(colour, Color32::BLACK, 0.35),
            ];
            icon(k, rect, &CHECKBOX, &colours)
        } else {
            // The empty box is drawn in the label's ink.
            icon(k, rect, &CHECKBOX_EMPTY, &[ink])
        }
    }

    fn radio(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // The diamond shows only when chosen; unchosen, its outline in the
        // disabled grey says where it would be.
        let colour = if st.disabled {
            DISABLED
        } else if st.on {
            NATURE
        } else {
            DISABLED
        };
        let colours = [
            hex(0x000000),
            WHITE,
            colour,
            mix(colour, WHITE, 0.55),
            mix(colour, Color32::BLACK, 0.30),
            mix(colour, Color32::BLACK, 0.70),
        ];
        if st.on {
            icon(k, rect, &DIAMOND, &colours)
        } else {
            let hollow: Vec<String> = DIAMOND
                .iter()
                .map(|r| {
                    r.chars()
                        .map(|c| if c == '0' { '0' } else { '1' })
                        .collect()
                })
                .collect();
            let rows: Vec<&str> = hollow.iter().map(String::as_str).collect();
            icon(k, rect, &rows, &[colour])
        }
    }

    fn select(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `.snes-input:has(select)`: the field, and the caret in Phantom.
        let mut out = self.field(
            k,
            rect,
            St {
                focused: false,
                ..st
            },
        );
        let m = self.metrics();
        let arrow = Rect::from_min_max(
            pos2(
                rect.right() - m.select_arrow * k.u() - PX * k.u(),
                rect.top(),
            ),
            pos2(rect.right() - PX * k.u(), rect.bottom()),
        );
        let colour = if st.disabled { DISABLED } else { PHANTOM };
        out.extend(icon(k, arrow, &CARET, &caret_colours(colour)));
        out
    }

    fn select_text(&self, k: &Kit, st: St) -> Color32 {
        if st.disabled {
            DUSK_HOVER
        } else {
            paper(k.look.dark).ink
        }
    }

    fn arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        let rows = turned(&CARET, dir);
        let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
        let colour = if st.disabled { DISABLED } else { PHANTOM };
        // The caret is 14 cells; a scroll arrow has room for 7, so every
        // other cell of it is drawn.
        let small: Vec<String> = rows
            .iter()
            .step_by(2)
            .map(|r| r.chars().step_by(2).collect())
            .collect();
        let small: Vec<&str> = small.iter().map(String::as_str).collect();
        let fits = rect.width() >= 18.0 * k.u() && rect.height() >= 18.0 * k.u();
        let rows = if fits { rows } else { small };
        icon(k, rect, &rows, &caret_colours(colour))
    }

    fn slider_track(&self, k: &Kit, rect: Rect, _st: St) -> Vec<Shape> {
        // `.snes-range`: a Plumber-red bar in the line, lit.
        let h = (self.metrics().slider_track + 2.0 * PX) * k.u();
        let bar = Rect::from_center_size(rect.center(), vec2(rect.width(), h));
        boxed(k, bar, PLUMBER, Some(glow(St::default())))
    }

    fn slider_thumb(&self, k: &Kit, centre: Pos2, st: St) -> Vec<Shape> {
        // `::-webkit-slider-thumb`: aged yellow, darkened under the pointer.
        let size = self.metrics().slider_thumb * k.u();
        let body = if st.hovered || st.pressed {
            mix(AGED, Color32::BLACK, 0.10)
        } else {
            AGED
        };
        boxed(k, Rect::from_center_size(centre, size), body, None)
    }

    fn tab(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // A tab is a box on the page; the shown one aged yellow with the
        // `.snes-container-title` underline in Plumber red.
        let p = paper(k.look.dark);
        let body = if st.on {
            if k.look.dark {
                hex(0x34495e)
            } else {
                AGED
            }
        } else {
            p.page
        };
        let mut out = boxed(k, rect, body, Some(glow(St { on: false, ..st })));
        if st.on {
            let u = k.u();
            let r = prim::snap_rect(rect, k.ppp).shrink(2.0 * PX * u);
            out.push(fill(
                Rect::from_min_max(pos2(r.left(), r.bottom() - PX * u), r.right_bottom()),
                PLUMBER,
            ));
        }
        out
    }

    fn tab_text(&self, k: &Kit, st: St) -> Color32 {
        if st.on && !k.look.dark {
            DUSK
        } else {
            paper(k.look.dark).ink
        }
    }

    fn tab_panel(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        boxed(k, rect, paper(k.look.dark).page, Some(glow(St::default())))
    }

    fn group(&self, k: &Kit, rect: Rect, legend: Option<Rect>) -> Vec<Shape> {
        // `.snes-container`: transparent inside its line, lit; the legend
        // breaks the line, on the page.
        let mut out = boxed(k, rect, Color32::TRANSPARENT, Some(glow(St::default())));
        if let Some(legend) = legend {
            let u = k.u();
            let gap = prim::snap_rect(legend.expand2(vec2(3.0 * u, 0.0)), k.ppp);
            let top = prim::snap_rect(rect, k.ppp).top();
            out.push(fill(
                Rect::from_min_max(pos2(gap.left(), top), pos2(gap.right(), top + 2.0 * PX * u)),
                // The page the legend sits on, the user's if they gave one.
                k.colors().face,
            ));
        }
        out
    }

    fn window(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let _ = st;
        boxed(k, rect, paper(k.look.dark).page, Some(glow(St::default())))
    }

    fn title_bar(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // The console's purple - or the user's border colour, the accent
        // every skin's title bar takes - lit as a button is; an inactive
        // window's in Galaxy blue.
        let body = if st.inactive_window {
            GALAXY
        } else {
            k.look.paint.accent.unwrap_or(PHANTOM)
        };
        let r = prim::snap_rect(rect, k.ppp);
        let (shine, shadow) = glow(St::default());
        let p = PX * k.u();
        vec![
            fill(r, body),
            fill(
                Rect::from_min_max(r.min, pos2(r.right(), r.top() + p)),
                shine,
            ),
            fill(
                Rect::from_min_max(pos2(r.left(), r.bottom() - p), r.max),
                shadow,
            ),
        ]
    }

    fn title_button(&self, k: &Kit, rect: Rect, glyph: Glyph, st: St) -> Vec<Shape> {
        // Small `.snes-button`s in the library's colours: Plumber to close,
        // Sunshine to minimize, Nature to maximize, Ocean for the app's own.
        let body = if st.inactive_window {
            DISABLED
        } else {
            match glyph {
                Glyph::Close => PLUMBER,
                Glyph::Minimize => SUNSHINE,
                Glyph::Maximize | Glyph::Restore => NATURE,
                _ => OCEAN,
            }
        };
        let mut out = boxed(k, rect, body, Some(glow(st)));
        let ink = k.glyph_ink(DUSK);
        let inner = prim::snap_rect(rect, k.ppp).shrink(PX * k.u());
        out.extend(super::title_glyph(k, inner, glyph, ink, st));
        out
    }

    fn scroll_track(&self, k: &Kit, rect: Rect, _vertical: bool, _st: St) -> Vec<Shape> {
        boxed(k, rect, paper(k.look.dark).field, None)
    }

    fn scroll_thumb(&self, k: &Kit, rect: Rect, _vertical: bool, st: St) -> Vec<Shape> {
        // A button in Galaxy blue.
        boxed(k, rect, GALAXY, Some(glow(st)))
    }

    fn scroll_arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        let mut out = boxed(k, rect, paper(k.look.dark).page, Some(glow(st)));
        out.extend(self.arrow(k, rect.shrink(PX * k.u()), dir, st));
        out
    }

    fn scroll_corner(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        vec![fill(prim::snap_rect(rect, k.ppp), paper(k.look.dark).page)]
    }

    fn menu(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        boxed(k, rect, paper(k.look.dark).field, Some(glow(St::default())))
    }

    fn menu_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        if (st.hovered || st.on) && !st.disabled {
            vec![fill(prim::snap_rect(rect, k.ppp), k.colors().select)]
        } else {
            Vec::new()
        }
    }

    fn separator(&self, k: &Kit, rect: Rect, vertical: bool) -> Vec<Shape> {
        let p = PX * k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let line = if vertical {
            Rect::from_center_size(r.center(), vec2(p, r.height()))
        } else {
            Rect::from_center_size(r.center(), vec2(r.width(), p))
        };
        vec![fill(prim::snap_rect(line, k.ppp), paper(k.look.dark).ink)]
    }

    fn tooltip(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        boxed(k, rect, AGED, None)
    }

    fn list(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        boxed(k, rect, paper(k.look.dark).field, None)
    }

    fn list_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        if st.on {
            vec![fill(r, k.colors().select)]
        } else if st.hovered {
            let wash = if k.look.dark {
                rgba(255, 255, 255, 0.10)
            } else {
                AGED
            };
            vec![fill(r, wash)]
        } else {
            Vec::new()
        }
    }

    fn list_text(&self, k: &Kit, st: St) -> Color32 {
        if st.on {
            DUSK
        } else {
            paper(k.look.dark).ink
        }
    }

    fn progress(&self, k: &Kit, rect: Rect, fraction: f32) -> Vec<Shape> {
        let mut out = boxed(k, rect, paper(k.look.dark).field, None);
        let inner = prim::snap_rect(rect, k.ppp).shrink(2.0 * PX * k.u());
        let done = Rect::from_min_size(
            inner.min,
            vec2(
                prim::snap(inner.width() * fraction.clamp(0.0, 1.0), k.ppp),
                inner.height(),
            ),
        );
        if done.width() > 0.0 {
            out.push(fill(done, NATURE));
        }
        out
    }

    fn status_bar(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let r = prim::snap_rect(rect, k.ppp);
        vec![
            fill(r, paper(k.look.dark).page),
            fill(
                Rect::from_min_max(r.min, pos2(r.right(), r.top() + PX * k.u())),
                LINE,
            ),
        ]
    }

    fn status_field(&self, _k: &Kit, _rect: Rect) -> Vec<Shape> {
        Vec::new()
    }

    fn focus(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        prim::dotted(rect, PHANTOM, k.ppp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::skin::{Look, Skin};

    fn kit() -> Kit {
        Kit::bare(
            Look {
                skin: Skin::Snes,
                ..Default::default()
            },
            1.0,
        )
    }

    /// Every sprite's rows are as wide as each other, as the SCSS matrices
    /// they were copied from are.
    #[test]
    fn every_sprite_is_a_rectangle() {
        for rows in [&CHECKBOX[..], &CHECKBOX_EMPTY[..], &DIAMOND[..], &CARET[..]] {
            let w = rows[0].len();
            assert!(rows.iter().all(|r| r.len() == w));
        }
    }

    /// The box's line sits outside the fill, a pixel of the library away,
    /// and the corner of the fill is the line's step.
    #[test]
    fn a_box_is_lined_outside_its_fill_and_stepped_at_its_corners() {
        let k = kit();
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(40.0, 20.0));
        let shapes = boxed(&k, rect, NATURE, None);
        let at = |x: f32, y: f32| {
            shapes
                .iter()
                .rev()
                .find_map(|s| match s {
                    Shape::Rect(r) if r.rect.contains(pos2(x, y)) => Some(r.fill),
                    _ => None,
                })
                .unwrap_or(Color32::TRANSPARENT)
        };
        assert_eq!(at(20.0, 0.5), LINE, "the top line");
        assert_eq!(at(0.5, 0.5), Color32::TRANSPARENT, "outside the corner");
        assert_eq!(at(2.5, 2.5), LINE, "the corner's step");
        assert_eq!(at(20.0, 10.0), NATURE, "the fill");
    }

    #[test]
    fn a_held_button_turns_its_light_over() {
        let rest = glow(St::default());
        let held = glow(St {
            pressed: true,
            ..St::default()
        });
        assert_eq!(rest.0, held.1);
        assert_eq!(rest.1, held.0);
    }

    #[test]
    fn a_caret_turns_every_way_and_keeps_its_cells() {
        let cells = |rows: &[String]| {
            rows.iter()
                .map(|r| r.chars().filter(|&c| c != '0').count())
                .sum::<usize>()
        };
        let down = turned(&CARET, Dir::Down);
        for dir in [Dir::Up, Dir::Left, Dir::Right] {
            assert_eq!(cells(&turned(&CARET, dir)), cells(&down), "{dir:?}");
        }
    }
}
