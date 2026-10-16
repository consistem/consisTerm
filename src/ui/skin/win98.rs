//! Windows 98, after 98.css (jdan/98.css, MIT).
//!
//! Every border is one of the library's inset-shadow stacks, painted through
//! [`prim::inset`]; every icon is one of its pixel-art SVGs, transcribed
//! pixel for pixel. The CSS variable each colour stands for is named beside
//! it, so a value can be checked against `style.css`.
//!
//! The dark reading is not 98's - Windows 98 had none. It keeps every bevel
//! and swaps the greys for the charcoal of the "Windows 98 Dark" theme, so
//! the raised and the sunken still read the same way round.

use egui::{pos2, vec2, Color32, Pos2, Rect, Shape};

use super::prim::{self, fill, hex, inset, sprite, Axis, Inset};
use super::{Colors, Dir, Face, Fonts, Glyph, Kit, Metrics, St, Style};

pub struct Win98;

/// The greys every bevel is cut from.
#[derive(Clone, Copy)]
struct Greys {
    /// `--surface`: a window, a button.
    surface: Color32,
    /// `--button-highlight`: the lit edge, and the inside of a field.
    highlight: Color32,
    /// `--button-face`: the half-lit edge just inside the lit one.
    face: Color32,
    /// `--button-shadow`.
    shadow: Color32,
    /// `--window-frame`: the outermost dark edge.
    frame: Color32,
    text: Color32,
}

fn greys(dark: bool) -> Greys {
    if dark {
        Greys {
            surface: hex(0x3c3c3c),
            highlight: hex(0x6e6e6e),
            face: hex(0x505050),
            shadow: hex(0x262626),
            frame: hex(0x000000),
            text: hex(0xe8e8e8),
        }
    } else {
        Greys {
            surface: hex(0xc0c0c0),
            highlight: hex(0xffffff),
            face: hex(0xdfdfdf),
            shadow: hex(0x808080),
            frame: hex(0x0a0a0a),
            text: hex(0x222222),
        }
    }
}

/// `--dialog-blue` to `--dialog-blue-light`, and their inactive greys.
const DIALOG_BLUE: Color32 = hex(0x000080);
const DIALOG_BLUE_LIGHT: Color32 = hex(0x1084d0);
const DIALOG_GRAY: Color32 = hex(0x808080);
const DIALOG_GRAY_LIGHT: Color32 = hex(0xb5b5b5);
/// The dark reading's title bar: Windows 2000's deeper blue, as the theme
/// already uses for its selection.
const DARK_BLUE: Color32 = hex(0x0a246a);
const DARK_BLUE_LIGHT: Color32 = hex(0x3a6ea5);

/// The two colours of the scroll track's dither, `icon/scrollbar-background.svg`.
pub fn dither_colours(dark: bool) -> (Color32, Color32) {
    let g = greys(dark);
    (g.surface, g.highlight)
}

/// `--border-raised-outer`, `--border-raised-inner`: a button at rest.
fn raised(g: Greys) -> [Inset; 4] {
    [
        (-1, -1, g.frame),
        (1, 1, g.highlight),
        (-2, -2, g.shadow),
        (2, 2, g.face),
    ]
}

/// `--border-sunken-outer`, `--border-sunken-inner`: a button held down.
fn sunken(g: Greys) -> [Inset; 4] {
    [
        (-1, -1, g.highlight),
        (1, 1, g.frame),
        (-2, -2, g.face),
        (2, 2, g.shadow),
    ]
}

/// `--border-window-outer`, `--border-window-inner`: a window, a menu - and
/// the scroll and drop-down buttons, whose SVGs draw the same rings.
fn window_rings(g: Greys) -> [Inset; 4] {
    [
        (-1, -1, g.frame),
        (1, 1, g.face),
        (-2, -2, g.shadow),
        (2, 2, g.highlight),
    ]
}

/// `--border-field`: a text field, a checkbox, a list.
fn field_rings(g: Greys) -> [Inset; 4] {
    [
        (-1, -1, g.highlight),
        (1, 1, g.shadow),
        (-2, -2, g.face),
        (2, 2, g.frame),
    ]
}

/// The etched line of a group box, `icon/groupbox-border.svg`.
fn etched(g: Greys) -> [Inset; 4] {
    [
        (-1, -1, g.highlight),
        (1, 1, g.shadow),
        (-2, -2, g.shadow),
        (2, 2, g.highlight),
    ]
}

/// `icon/checkmark.svg`.
const CHECKMARK: [&str; 7] = [
    "......#", ".....##", "#...###", "##.###.", "#####..", ".###...", "..#....",
];

/// `icon/radio-border.svg`: G `#808080`, B `--window-frame`, L
/// `--button-face`, W `--button-highlight`, I the inside.
const RADIO: [&str; 12] = [
    "....GGGG....",
    "..GGBBBBGG..",
    ".GBBIIIIBBW.",
    ".GBIIIIIILW.",
    "GBIIIIIIIILW",
    "GBIIIIIIIILW",
    "GBIIIIIIIILW",
    "GBIIIIIIIILW",
    ".GBIIIIIILW.",
    ".GLLIIIILLW.",
    "..WWLLLLWW..",
    "....WWWW....",
];

/// `icon/radio-dot.svg`.
const RADIO_DOT: [&str; 4] = [".##.", "####", "####", ".##."];

/// The scroll and drop-down arrows, `icon/button-*.svg`, at the place each
/// sits in its 16 by 17 button.
fn arrow_rows(dir: Dir) -> (&'static [&'static str], Pos2) {
    match dir {
        Dir::Up => (
            &["...#...", "..###..", ".#####.", "#######"],
            pos2(4.0, 6.0),
        ),
        Dir::Down => (
            &["#######", ".#####.", "..###..", "...#..."],
            pos2(4.0, 6.0),
        ),
        Dir::Left => (
            &["...#", "..##", ".###", "####", ".###", "..##", "...#"],
            pos2(5.0, 4.0),
        ),
        Dir::Right => (
            &["#...", "##..", "###.", "####", "###.", "##..", "#..."],
            pos2(6.0, 4.0),
        ),
    }
}

/// The title-bar marks, `icon/*.svg`, with the offset `style.css` gives each
/// in its 16 by 14 button (`background-position`).
fn glyph_rows(glyph: Glyph) -> (&'static [&'static str], Pos2) {
    match glyph {
        Glyph::Minimize => (&["######", "######"], pos2(4.0, 9.0)),
        Glyph::Maximize => (
            &[
                "#########",
                "#########",
                "#.......#",
                "#.......#",
                "#.......#",
                "#.......#",
                "#.......#",
                "#.......#",
                "#########",
            ],
            pos2(3.0, 2.0),
        ),
        Glyph::Restore => (
            &[
                "..######", "..######", "..#....#", "######.#", "######.#", "#....###", "#....#..",
                "#....#..", "######..",
            ],
            pos2(3.0, 2.0),
        ),
        Glyph::Close => (
            &[
                "##....##", ".##..##.", "..####..", "...##...", "..####..", ".##..##.", "##....##",
            ],
            pos2(4.0, 3.0),
        ),
        Glyph::Help => (
            &[
                ".####.", "##..##", "##..##", "...##.", "..##..", "..##..", "......", "..##..",
                "..##..",
            ],
            pos2(5.0, 2.0),
        ),
        // The app's own three, drawn in the same hand: whole pixels, two
        // thick where a stroke would be.
        Glyph::Gear => (
            &[
                "...##...", ".#.##.#.", "..####..", "###..###", "###..###", "..####..", ".#.##.#.",
                "...##...",
            ],
            pos2(4.0, 3.0),
        ),
        Glyph::Plus => (
            &[
                "...##...", "...##...", "...##...", "########", "########", "...##...", "...##...",
                "...##...",
            ],
            pos2(4.0, 3.0),
        ),
        Glyph::Pin => (
            &[
                "..####..", "..#..#..", "..#..#..", ".######.", "...##...", "...##...", "...##...",
            ],
            pos2(4.0, 3.0),
        ),
        Glyph::Pinned => (
            &[
                "..####..", "..####..", "..####..", ".######.", "...##...", "...##...", "...##...",
            ],
            pos2(4.0, 3.0),
        ),
    }
}

/// A sprite at its CSS offset in `rect` when `rect` is the size the CSS drew
/// it in, centred otherwise - a title bar scaled up keeps its marks in the
/// middle of the buttons.
fn place(k: &Kit, rect: Rect, rows: &[&str], offset: Pos2, native: egui::Vec2) -> Pos2 {
    let u = k.u();
    if (rect.width() - native.x * u).abs() < 0.5 && (rect.height() - native.y * u).abs() < 0.5 {
        let r = prim::snap_rect(rect, k.ppp);
        pos2(r.left() + offset.x * u, r.top() + offset.y * u)
    } else {
        prim::centred(rect, rows, k.ppp)
    }
}

fn shifted(at: Pos2, st: St, u: f32) -> Pos2 {
    if st.pressed && !st.disabled {
        at + vec2(u, u)
    } else {
        at
    }
}

impl Win98 {
    /// A grey glyph with the white emboss 98 gave anything disabled.
    fn ink(k: &Kit, at: Pos2, rows: &[&str], st: St) -> Vec<Shape> {
        let g = greys(k.look.dark);
        if st.disabled {
            let mut out = sprite(at + vec2(k.u(), k.u()), rows, &[('#', g.highlight)], k.ppp);
            out.extend(sprite(at, rows, &[('#', g.shadow)], k.ppp));
            out
        } else {
            sprite(at, rows, &[('#', g.text)], k.ppp)
        }
    }
}

impl Style for Win98 {
    fn colors(&self, dark: bool) -> Colors {
        let g = greys(dark);
        Colors {
            face: g.surface,
            text: g.text,
            text_disabled: g.shadow,
            text_disabled_shadow: Some(g.highlight),
            field: if dark { hex(0x1e1e1e) } else { g.highlight },
            field_text: g.text,
            select: if dark { DARK_BLUE } else { DIALOG_BLUE },
            select_text: hex(0xffffff),
            title_text: hex(0xffffff),
            title_text_inactive: if dark { hex(0xa0a0a0) } else { hex(0xc0c0c0) },
            title_shadow: None,
            link: if dark { hex(0x8ab4ff) } else { hex(0x0000ff) },
            tooltip_text: hex(0x000000),
            subtle: if dark { hex(0xa0a0a0) } else { hex(0x505050) },
        }
    }

    fn metrics(&self) -> Metrics {
        Metrics {
            font_size: 11.0,
            title_font_size: 11.0,
            // 3 px of padding above and below the 14 px buttons.
            title_height: 20.0,
            title_button: vec2(16.0, 14.0),
            title_button_gap: 0.0,
            close_gap: 2.0,
            // Two rings of border, then the one pixel of padding left showing.
            window_border: 3.0,
            window_radius: 0.0,
            scrollbar: 16.0,
            scroll_arrow: 16.0,
            checkbox: 13.0,
            radio: 12.0,
            button_min: vec2(75.0, 23.0),
            button_pad_x: 12.0,
            field_height: 21.0,
            tab_height: 24.0,
            select_arrow: 16.0,
            menu_item_height: 18.0,
            slider_thumb: vec2(11.0, 21.0),
            slider_track: 4.0,
            label_gap: 6.0,
        }
    }

    fn fonts(&self) -> Fonts {
        Fonts {
            body: Some(Face::Bundled(include_bytes!(
                "../../../assets/fonts/ms-sans-serif/MS Sans Serif.ttf"
            ))),
            bold: Some(Face::Bundled(include_bytes!(
                "../../../assets/fonts/ms-sans-serif-bold/MS Sans Serif Bold.ttf"
            ))),
            title: None,
        }
    }

    fn pressed_shift(&self) -> f32 {
        1.0
    }

    fn button(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let g = greys(k.look.dark);
        let mut out = vec![fill(prim::snap_rect(rect, k.ppp), g.surface)];
        let pressed = st.pressed && !st.disabled;
        let rings: Vec<Inset> = match (st.default, pressed) {
            (false, false) => raised(g).to_vec(),
            (false, true) => sunken(g).to_vec(),
            // `--default-button-border-raised-*`: one more ring of frame.
            (true, false) => vec![
                (-2, -2, g.frame),
                (1, 1, g.frame),
                (2, 2, g.highlight),
                (-3, -3, g.shadow),
                (3, 3, g.face),
            ],
            (true, true) => vec![
                (2, 2, g.frame),
                (-1, -1, g.frame),
                (-2, -2, g.highlight),
                (3, 3, g.shadow),
                (-3, -3, g.face),
            ],
        };
        out.extend(inset(rect, &rings, k.ppp));
        if st.focused {
            // `outline: 1px dotted; outline-offset: -4px`.
            out.extend(prim::dotted(prim::inner(rect, 4.0, k.ppp), g.text, k.ppp));
        }
        out
    }

    fn field(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let g = greys(k.look.dark);
        let c = k.colors();
        let inside = if st.disabled { g.surface } else { c.field };
        let mut out = vec![fill(prim::snap_rect(rect, k.ppp), inside)];
        out.extend(inset(rect, &field_rings(g), k.ppp));
        out
    }

    fn checkbox(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let g = greys(k.look.dark);
        let c = k.colors();
        let r = prim::snap_rect(rect, k.ppp);
        // `:active` and `[disabled]` turn the inside to the surface grey.
        let inside = if st.disabled || (st.pressed && !st.disabled) {
            g.surface
        } else {
            c.field
        };
        let mut out = vec![fill(r, inside)];
        out.extend(inset(r, &field_rings(g), k.ppp));
        if st.on {
            let ink = if st.disabled { g.shadow } else { g.text };
            let at = pos2(r.left() + 3.0 * k.u(), r.top() + 3.0 * k.u());
            out.extend(sprite(at, &CHECKMARK, &[('#', ink)], k.ppp));
        }
        out
    }

    fn radio(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let g = greys(k.look.dark);
        let c = k.colors();
        let at = prim::centred(rect, &RADIO, k.ppp);
        // `:active` and `[disabled]` draw `radio-border-disabled.svg`, whose
        // only difference is the grey inside.
        let inside = if st.disabled || st.pressed {
            g.surface
        } else {
            c.field
        };
        let mut out = sprite(
            at,
            &RADIO,
            &[
                ('G', g.shadow),
                ('B', g.frame),
                ('L', g.face),
                ('W', g.highlight),
                ('I', inside),
            ],
            k.ppp,
        );
        if st.on {
            let ink = if st.disabled { g.shadow } else { g.text };
            let dot = pos2(at.x + 4.0 * k.u(), at.y + 4.0 * k.u());
            out.extend(sprite(dot, &RADIO_DOT, &[('#', ink)], k.ppp));
        }
        out
    }

    fn select(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let mut out = self.field(k, rect, st);
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        // `background-position: top 2px right 2px`, the button 16 by 17.
        let button = Rect::from_min_size(
            pos2(r.right() - 2.0 * u - 16.0 * u, r.top() + 2.0 * u),
            vec2(16.0 * u, (r.height() - 4.0 * u).min(17.0 * u)),
        );
        out.extend(self.scroll_arrow(k, button, Dir::Down, st));
        if st.focused && !st.disabled {
            // `select:focus` fills the value's line in the dialog blue.
            let c = k.colors();
            let line = Rect::from_min_max(
                pos2(r.left() + 3.0 * u, r.top() + 3.0 * u),
                pos2(button.left() - u, r.bottom() - 3.0 * u),
            );
            out.push(fill(line, c.select));
        }
        out
    }

    fn select_text(&self, k: &Kit, st: St) -> Color32 {
        let c = k.colors();
        if st.disabled {
            c.text_disabled
        } else if st.focused {
            c.select_text
        } else {
            c.field_text
        }
    }

    fn arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        let (rows, offset) = arrow_rows(dir);
        let at = place(k, rect, rows, offset, vec2(16.0, 17.0));
        Self::ink(k, shifted(at, st, k.u()), rows, st)
    }

    fn slider_track(&self, k: &Kit, rect: Rect, _st: St) -> Vec<Shape> {
        // The 2 px black track with grey right and bottom edges, and the
        // box-shadows round it: darkgrey above and to the left, white below
        // and to the right. Four pixels from top to bottom.
        let g = greys(k.look.dark);
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let top = prim::snap(r.center().y - 2.0 * u, k.ppp);
        let outer = Rect::from_min_max(pos2(r.left(), top), pos2(r.right(), top + 4.0 * u));
        let darkgrey = if k.look.dark {
            hex(0x2e2e2e)
        } else {
            hex(0xa9a9a9)
        };
        let mut out = vec![fill(outer, g.highlight)];
        out.push(fill(
            Rect::from_min_max(outer.min, pos2(outer.right() - u, outer.bottom() - u)),
            darkgrey,
        ));
        let track = outer.shrink(u);
        out.push(fill(track, g.shadow));
        out.push(fill(
            Rect::from_min_max(track.min, pos2(track.right() - u, track.bottom() - u)),
            hex(0x000000),
        ));
        out
    }

    fn slider_thumb(&self, k: &Kit, centre: Pos2, st: St) -> Vec<Shape> {
        // `icon/indicator-horizontal.svg`: a pointed thumb, 11 by 21.
        const THUMB: [&str; 21] = [
            "WWWWWWWWWWK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WCCCCCCCCDK",
            "WWCCCCCCDDK",
            ".WWCCCCDDK.",
            "..WWCCDDK..",
            "...WWDDK...",
            ".....K.....",
        ];
        let g = greys(k.look.dark);
        let face = if st.pressed || st.disabled {
            g.face
        } else if k.look.dark {
            g.surface
        } else {
            hex(0xc0c7c8)
        };
        let at = prim::centred(
            Rect::from_center_size(centre, vec2(11.0, 21.0) * k.u()),
            &THUMB,
            k.ppp,
        );
        sprite(
            at,
            &THUMB,
            &[
                ('W', g.highlight),
                ('C', face),
                ('D', if k.look.dark { g.shadow } else { hex(0x87888f) }),
                ('K', g.frame),
            ],
            k.ppp,
        )
    }

    fn tab(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `--border-tab`. Its 3 px top corners round off one pixel at each
        // end of the top edge, which shapes laid over a backdrop cannot take
        // away again; the tab is left square-cornered.
        let g = greys(k.look.dark);
        let u = k.u();
        let mut r = prim::snap_rect(rect, k.ppp);
        if st.on {
            // `margin-top: -2px; padding-bottom: 2px`: the selected tab
            // stands taller and runs down over the panel's edge.
            r = Rect::from_min_max(
                pos2(r.left(), r.top() - 2.0 * u),
                pos2(r.right(), r.bottom() + 2.0 * u),
            );
        }
        let mut out = vec![fill(r, g.surface)];
        let rings: [Inset; 4] = [
            (-1, 0, g.frame),
            (1, 1, g.face),
            (-2, 0, g.shadow),
            (2, 2, g.highlight),
        ];
        out.extend(inset(r, &rings, k.ppp));
        if st.focused {
            out.extend(prim::dotted(r.shrink(4.0 * u), g.text, k.ppp));
        }
        out
    }

    fn tab_panel(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        self.window(k, rect, St::default())
    }

    fn group(&self, k: &Kit, rect: Rect, legend: Option<Rect>) -> Vec<Shape> {
        let g = greys(k.look.dark);
        let mut out = inset(rect, &etched(g), k.ppp);
        if let Some(legend) = legend {
            // `legend { background: var(--surface) }` over the line.
            let r = prim::snap_rect(legend.expand2(vec2(2.0 * k.u(), 0.0)), k.ppp);
            out.push(fill(r, g.surface));
        }
        out
    }

    fn window(&self, k: &Kit, rect: Rect, _st: St) -> Vec<Shape> {
        let g = greys(k.look.dark);
        let mut out = vec![fill(prim::snap_rect(rect, k.ppp), g.surface)];
        out.extend(inset(rect, &window_rings(g), k.ppp));
        out
    }

    fn title_bar(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let (from, to) = match (st.inactive_window, k.look.dark) {
            (false, false) => (DIALOG_BLUE, DIALOG_BLUE_LIGHT),
            (false, true) => (DARK_BLUE, DARK_BLUE_LIGHT),
            (true, _) => (DIALOG_GRAY, DIALOG_GRAY_LIGHT),
        };
        vec![prim::gradient(
            prim::snap_rect(rect, k.ppp),
            Axis::Across,
            &[(0.0, from), (1.0, to)],
        )]
    }

    fn title_button(&self, k: &Kit, rect: Rect, glyph: Glyph, st: St) -> Vec<Shape> {
        let mut out = self.button(
            k,
            rect,
            St {
                focused: false,
                default: false,
                ..st
            },
        );
        let (rows, offset) = glyph_rows(glyph);
        let at = place(k, rect, rows, offset, vec2(16.0, 14.0));
        let at = shifted(at, st, k.u());
        match k.look.paint.icon {
            Some(ink) if !st.disabled => out.extend(sprite(at, rows, &[('#', ink)], k.ppp)),
            _ => out.extend(Self::ink(k, at, rows, st)),
        }
        out
    }

    fn scroll_track(&self, k: &Kit, rect: Rect, _vertical: bool, _st: St) -> Vec<Shape> {
        match k.dither {
            Some(texture) => vec![prim::tiled(rect, texture, vec2(2.0, 2.0), k.ppp)],
            None => vec![fill(rect, greys(k.look.dark).surface)],
        }
    }

    fn scroll_thumb(&self, k: &Kit, rect: Rect, _vertical: bool, _st: St) -> Vec<Shape> {
        // `background-color: var(--button-face)` under the raised rings: a
        // shade lighter than a button.
        let g = greys(k.look.dark);
        let mut out = vec![fill(prim::snap_rect(rect, k.ppp), g.face)];
        out.extend(inset(rect, &raised(g), k.ppp));
        out
    }

    fn scroll_arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        let g = greys(k.look.dark);
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![fill(r, g.surface)];
        if st.pressed && !st.disabled {
            // `button-down-active.svg`: a flat grey frame, the arrow one
            // pixel down and to the right.
            out.extend(prim::inset(
                r,
                &[(1, 1, g.shadow), (-1, -1, g.shadow)],
                k.ppp,
            ));
        } else {
            out.extend(inset(r, &window_rings(g), k.ppp));
        }
        out.extend(self.arrow(k, rect, dir, st));
        out
    }

    fn scroll_corner(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        vec![fill(prim::snap_rect(rect, k.ppp), greys(k.look.dark).face)]
    }

    fn menu(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        self.window(k, rect, St::default())
    }

    fn menu_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        if (st.hovered || st.on) && !st.disabled {
            vec![fill(prim::snap_rect(rect, k.ppp), k.colors().select)]
        } else {
            Vec::new()
        }
    }

    fn separator(&self, k: &Kit, rect: Rect, vertical: bool) -> Vec<Shape> {
        // An etched line: shadow, then highlight beside it.
        let g = greys(k.look.dark);
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        if vertical {
            let x = prim::snap(r.center().x - u, k.ppp);
            vec![
                fill(
                    Rect::from_min_max(pos2(x, r.top()), pos2(x + u, r.bottom())),
                    g.shadow,
                ),
                fill(
                    Rect::from_min_max(pos2(x + u, r.top()), pos2(x + 2.0 * u, r.bottom())),
                    g.highlight,
                ),
            ]
        } else {
            let y = prim::snap(r.center().y - u, k.ppp);
            vec![
                fill(
                    Rect::from_min_max(pos2(r.left(), y), pos2(r.right(), y + u)),
                    g.shadow,
                ),
                fill(
                    Rect::from_min_max(pos2(r.left(), y + u), pos2(r.right(), y + 2.0 * u)),
                    g.highlight,
                ),
            ]
        }
    }

    fn tooltip(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // Not in 98.css: what Windows 98 drew, pale yellow with a black edge -
        // in the dark reading too, as Windows kept its tips yellow whatever
        // the scheme, and as their black words need.
        let r = prim::snap_rect(rect, k.ppp);
        let (paper, edge) = (hex(0xffffe1), hex(0x000000));
        vec![
            fill(r, paper),
            prim::outline(r, egui::Rounding::ZERO, edge, k.ppp),
        ]
    }

    fn list(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        self.field(k, rect, St::default())
    }

    fn list_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // 98 lit nothing under the pointer: only the selection is blue.
        if st.on {
            vec![fill(prim::snap_rect(rect, k.ppp), k.colors().select)]
        } else {
            Vec::new()
        }
    }

    fn progress(&self, k: &Kit, rect: Rect, fraction: f32) -> Vec<Shape> {
        // `.progress-indicator.segmented`: `--border-sunken-inner`, 4 px of
        // padding, blue blocks 16 px wide with 2 px between.
        let g = greys(k.look.dark);
        let u = k.u();
        let mut out = vec![fill(prim::snap_rect(rect, k.ppp), g.surface)];
        out.extend(inset(rect, &[(-2, -2, g.face), (2, 2, g.shadow)], k.ppp));
        let inner = prim::inner(rect, 4.0, k.ppp);
        let end = inner.left() + inner.width() * fraction.clamp(0.0, 1.0);
        let blue = k.colors().select;
        let mut x = inner.left();
        while x < end {
            let right = (x + 16.0 * u).min(end);
            out.push(fill(
                Rect::from_min_max(pos2(x, inner.top()), pos2(right, inner.bottom())),
                blue,
            ));
            x += 18.0 * u;
        }
        out
    }

    fn status_bar(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        vec![fill(
            prim::snap_rect(rect, k.ppp),
            greys(k.look.dark).surface,
        )]
    }

    fn status_field(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `--border-status-field`.
        let g = greys(k.look.dark);
        inset(rect, &[(-1, -1, g.face), (1, 1, g.shadow)], k.ppp)
    }

    fn focus(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        prim::dotted(rect, greys(k.look.dark).text, k.ppp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_row_is_as_wide_as_the_others() {
        let all: Vec<&[&str]> = vec![&CHECKMARK, &RADIO, &RADIO_DOT];
        for rows in all {
            let w = rows[0].chars().count();
            assert!(rows.iter().all(|r| r.chars().count() == w), "{rows:?}");
        }
        for glyph in [
            Glyph::Minimize,
            Glyph::Maximize,
            Glyph::Restore,
            Glyph::Close,
            Glyph::Help,
            Glyph::Gear,
            Glyph::Plus,
            Glyph::Pin,
            Glyph::Pinned,
        ] {
            let (rows, _) = glyph_rows(glyph);
            let w = rows[0].chars().count();
            assert!(rows.iter().all(|r| r.chars().count() == w), "{glyph:?}");
        }
    }

    /// A 98 button at rest: its top-left pixel white, its bottom-right one
    /// the window frame, and the top-right one dark too - the first ring wins
    /// the corner.
    #[test]
    fn a_button_is_lit_from_the_top_left() {
        let k = Kit::bare(
            super::super::Look {
                skin: super::super::Skin::Win98,
                dark: false,
                ..Default::default()
            },
            1.0,
        );
        let shapes = Win98.button(
            &k,
            Rect::from_min_size(Pos2::ZERO, vec2(75.0, 23.0)),
            St::default(),
        );
        let colour_at = |x: f32, y: f32| {
            shapes
                .iter()
                .rev()
                .find_map(|s| match s {
                    Shape::Rect(r) if r.rect.contains(pos2(x + 0.5, y + 0.5)) => Some(r.fill),
                    _ => None,
                })
                .unwrap_or(Color32::TRANSPARENT)
        };
        assert_eq!(colour_at(0.0, 0.0), hex(0xffffff));
        assert_eq!(colour_at(74.0, 22.0), hex(0x0a0a0a));
        assert_eq!(colour_at(74.0, 0.0), hex(0x0a0a0a));
        assert_eq!(colour_at(1.0, 1.0), hex(0xdfdfdf));
        assert_eq!(colour_at(73.0, 21.0), hex(0x808080));
        assert_eq!(colour_at(10.0, 10.0), hex(0xc0c0c0));
    }
}
