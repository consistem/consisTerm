//! The classic Mac OS, after system.css (sakofchit/system.css, MIT).
//!
//! system.css draws in two colours, `--primary` white and `--secondary`
//! black, with one grey for an inactive title and one for disabled text. So
//! every painter here is built from [`Ink`], and the dark reading is the
//! same drawing with the two swapped - white lines on black - rather than a
//! second palette that could drift from the first.
//!
//! Most of the library's shapes are `border-image`s and SVGs that a browser
//! rasterises at fractional sizes (the button's 81-unit outline squeezed into
//! 5.5 px, the scroll arrows' 1.375 px cells). What is drawn here is what the
//! browser puts on the screen at 100%, read back pixel by pixel from a
//! headless render, with each anti-aliased half-pixel settled to the side the
//! shape reads as - so a corner is a staircase, never a smear.

use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, Pos2, Rect, Shape};

use super::prim::{self, fill, hex, inset, sprite};
use super::{Colors, Dir, Face, Fonts, Glyph, Kit, Metrics, St, Style};

pub struct System;

/// The two colours everything is drawn in, and the two greys.
#[derive(Clone, Copy)]
struct Ink {
    /// `--primary`: the paper.
    paper: Color32,
    /// `--secondary`: every line, every inverted highlight.
    ink: Color32,
    /// `--tertiary`: an inactive window's title.
    grey: Color32,
    /// `--disabled`.
    disabled: Color32,
}

fn ink(dark: bool) -> Ink {
    if dark {
        // The light reading turned over channel by channel, greys included,
        // so a disabled label stays as faint against black as it was
        // against white.
        Ink {
            paper: hex(0x000000),
            ink: hex(0xffffff),
            grey: hex(0x5a5a5a),
            disabled: hex(0x494847),
        }
    } else {
        Ink {
            paper: hex(0xffffff),
            ink: hex(0x000000),
            grey: hex(0xa5a5a5),
            disabled: hex(0xb6b7b8),
        }
    }
}

/// A rect with stepped corners: row `i` from the top (and, when `bottom`,
/// from the bottom) is brought in by `steps[i]` pixels at each end, and the
/// rows past the steps run the full width. Every rounded thing in system.css
/// comes out of the browser as one of these staircases at 100%, which a
/// tessellated rounding would blur.
fn stepped(rect: Rect, steps: &[u8], bottom: bool, colour: Color32, ppp: f32) -> Shape {
    let u = prim::unit(ppp);
    let r = prim::snap_rect(rect, ppp);
    let rows = (r.height() / u).round() as usize;
    let steps = &steps[..steps.len().min(if bottom { rows / 2 } else { rows })];
    let mut mesh = Mesh::default();
    for (i, &s) in steps.iter().enumerate() {
        let x0 = r.left() + f32::from(s) * u;
        let x1 = r.right() - f32::from(s) * u;
        if x1 <= x0 {
            continue;
        }
        let top = r.top() + i as f32 * u;
        mesh.add_colored_rect(Rect::from_min_max(pos2(x0, top), pos2(x1, top + u)), colour);
        if bottom {
            let y = r.bottom() - (i + 1) as f32 * u;
            mesh.add_colored_rect(Rect::from_min_max(pos2(x0, y), pos2(x1, y + u)), colour);
        }
    }
    let n = steps.len() as f32 * u;
    let foot = if bottom { r.bottom() - n } else { r.bottom() };
    let middle = Rect::from_min_max(pos2(r.left(), r.top() + n), pos2(r.right(), foot));
    if middle.is_positive() {
        mesh.add_colored_rect(middle, colour);
    }
    Shape::mesh(mesh)
}

/// `icon/button.svg` as `border-image: 30 stretch` at `border-width: 5.5px`
/// lays it down: a one-pixel outline whose corners step in 3, 1, 1.
const BUTTON_STEPS: [u8; 3] = [3, 1, 1];
/// The paper inside that outline.
const BUTTON_INSIDE: [u8; 2] = [2, 1];
/// `icon/button-default.svg` at `border-width: 0.5em`: the default button's
/// ring, three pixels thick, standing four outside the button.
const RING_STEPS: [u8; 5] = [5, 3, 2, 1, 1];
const RING_HOLE: [u8; 4] = [3, 2, 1, 1];

/// `icon/checkmark.svg`: not a tick but a cross, corner to corner.
const CHECKMARK: [&str; 12] = [
    "#..........#",
    ".#........#.",
    "..#......#..",
    "...#....#...",
    "....#..#....",
    ".....##.....",
    ".....##.....",
    "....#..#....",
    "...#....#...",
    "..#......#..",
    ".#........#.",
    "#..........#",
];

/// `icon/radio-border.svg`.
const RADIO: [&str; 12] = [
    "....####....",
    "..##....##..",
    ".#........#.",
    ".#........#.",
    "#..........#",
    "#..........#",
    "#..........#",
    "#..........#",
    ".#........#.",
    ".#........#.",
    "..##....##..",
    "....####....",
];

/// `icon/radio-border-focused.svg`, under the pointer or the keyboard: the
/// same circle two pixels thick.
const RADIO_FOCUSED: [&str; 12] = [
    "....####....",
    "..########..",
    ".####..####.",
    ".##......##.",
    "##........##",
    "##........##",
    "##........##",
    "##........##",
    ".##......##.",
    ".####..####.",
    "..########..",
    "....####....",
];

/// The whole disc, walls and inside: the paper laid under the circle, since
/// the SVG's inside is transparent over a white page.
const RADIO_DISC: [&str; 12] = [
    "....####....",
    "..########..",
    ".##########.",
    ".##########.",
    "############",
    "############",
    "############",
    "############",
    ".##########.",
    ".##########.",
    "..########..",
    "....####....",
];

/// `icon/radio-dot.svg`, three pixels in from the circle's corner
/// (`--radio-dot-top`, `--radio-dot-left`).
const RADIO_DOT: [&str; 6] = [".####.", "######", "######", "######", "######", ".####."];

/// `icon/select-button.svg`'s triangle, which sits five rows down and two
/// across in its 17-pixel square.
const SELECT_ARROW: [&str; 7] = [
    "#############",
    ".###########.",
    "..#########..",
    "...#######...",
    "....#####....",
    ".....###.....",
    "......#......",
];

/// `icon/scrollbar-up.svg`'s arrow, in its own cells: the SVG draws it on a
/// grid of 1.375 px squares, 13 by 11 of them, one cell in and two down in
/// a button 16 cells across and 17 along.
const SCROLL_ARROW: [&str; 11] = [
    "......#......",
    ".....#.#.....",
    "....#...#....",
    "...#.....#...",
    "..#.......#..",
    ".#.........#.",
    "####.....####",
    "...#.....#...",
    "...#.....#...",
    "...#.....#...",
    "...#######...",
];

/// `icon/scrollbar-up-active.svg`: the same arrow filled in while it is
/// held.
const SCROLL_ARROW_ACTIVE: [&str; 11] = [
    "......#......",
    ".....###.....",
    "....#####....",
    "...#######...",
    "..#########..",
    ".###########.",
    "#############",
    "...#######...",
    "...#######...",
    "...#######...",
    "...#######...",
];

/// One cell of the scroll arrows' grid, in CSS pixels.
const SCROLL_CELL: f32 = 1.0;
/// The scroll button's length along the bar, in those cells.
const SCROLL_CELLS: usize = 17;

/// The marks inside an 11 px title-bar box, nine pixels square: the close
/// box's starburst while it is held, the zoom box's small window, and the
/// app's own marks drawn in the same single-pixel hand.
fn small_glyph(glyph: Glyph, pressed: bool) -> &'static [&'static str] {
    match (glyph, pressed) {
        (Glyph::Close, true) => &[
            "....#....",
            ".#..#..#.",
            "..#...#..",
            ".........",
            "###...###",
            ".........",
            "..#...#..",
            ".#..#..#.",
            "....#....",
        ],
        (Glyph::Close, false) | (Glyph::Maximize, true) => &[],
        (Glyph::Maximize, false) => &[
            "######...",
            "#....#...",
            "#....#...",
            "#....#...",
            "#....#...",
            "######...",
            ".........",
            ".........",
            ".........",
        ],
        (Glyph::Restore, _) => &[
            "...######",
            "...#....#",
            "######..#",
            "#....#..#",
            "#....####",
            "#....#...",
            "######...",
            ".........",
            ".........",
        ],
        (Glyph::Minimize, _) => &[
            ".........",
            ".........",
            "#########",
            ".........",
            "#########",
            ".........",
            ".........",
            ".........",
            ".........",
        ],
        (Glyph::Help, _) => &[
            "..####...",
            ".#....#..",
            "......#..",
            ".....#...",
            "....#....",
            "....#....",
            ".........",
            "....#....",
            ".........",
        ],
        (Glyph::Gear, _) => &[
            "....#....",
            ".#.###.#.",
            "..#...#..",
            ".#.....#.",
            "##.....##",
            ".#.....#.",
            "..#...#..",
            ".#.###.#.",
            "....#....",
        ],
        (Glyph::Plus, _) => &[
            "....#....",
            "....#....",
            "....#....",
            "....#....",
            "#########",
            "....#....",
            "....#....",
            "....#....",
            "....#....",
        ],
        (Glyph::Pin, _) => &[
            "..#####..",
            "..#...#..",
            "..#...#..",
            ".#######.",
            "....#....",
            "....#....",
            "....#....",
            "....#....",
            ".........",
        ],
        (Glyph::Pinned, _) => &[
            "..#####..",
            "..#####..",
            "..#####..",
            ".#######.",
            "....#....",
            "....#....",
            "....#....",
            "....#....",
            ".........",
        ],
    }
}

#[cfg(test)]
fn box_glyph(glyph: Glyph, pressed: bool) -> &'static [&'static str] {
    match (glyph, pressed) {
        // `.close:active::before, ::after`: a plus and a rotated plus, the
        // close box's starburst. At rest the box is empty.
        (Glyph::Close, true) => &[
            "....................",
            "....................",
            ".........##.........",
            "....#....##....#....",
            "....##...##...##....",
            "....###..##..###....",
            ".....##..##..##.....",
            "....................",
            "....................",
            "..#####......#####..",
            "..#####......#####..",
            "....................",
            "....................",
            ".....##..##..##.....",
            "....###..##..###....",
            "....##...##...##....",
            "....#....##....#....",
            ".........##.........",
        ],
        // `.resize`: the right and bottom edges of a smaller window in the
        // box's corner - 60% of it, two pixels thick. Held, it empties.
        (Glyph::Maximize, false) => &[
            "....................",
            "....................",
            "..........##........",
            "..........##........",
            "..........##........",
            "..........##........",
            "..........##........",
            "..........##........",
            "..........##........",
            "..........##........",
            "..##########........",
            "..##########........",
        ],
        // Not in system.css, which has one zoom box for both ways: the
        // smaller window drawn whole, so restoring reads apart from zooming.
        (Glyph::Restore, false) => &[
            "....................",
            "....................",
            "..##########........",
            "..##########........",
            "..##......##........",
            "..##......##........",
            "..##......##........",
            "..##......##........",
            "..##......##........",
            "..##......##........",
            "..##########........",
            "..##########........",
        ],
        // System 7.5's window-shade box: two bars across the middle.
        (Glyph::Minimize, false) => &[
            "....................",
            "....................",
            "....................",
            "....................",
            "....................",
            "....................",
            "....................",
            "..################..",
            "..################..",
            "....................",
            "....................",
            "..################..",
            "..################..",
        ],
        // The app's own, in the same two-pixel hand.
        (Glyph::Help, _) => &[
            "....................",
            "....................",
            "....................",
            "........####........",
            ".......##..##.......",
            ".......##..##.......",
            "...........##.......",
            "..........##........",
            ".........##.........",
            ".........##.........",
            "....................",
            ".........##.........",
            ".........##.........",
        ],
        (Glyph::Gear, _) => &[
            "....................",
            "....................",
            "....................",
            "....................",
            ".........##.........",
            "......#..##..#......",
            ".......######.......",
            ".....###....###.....",
            ".....###....###.....",
            ".......######.......",
            "......#..##..#......",
            ".........##.........",
        ],
        (Glyph::Plus, _) => &[
            "....................",
            "....................",
            "....................",
            "....................",
            ".........##.........",
            ".........##.........",
            ".........##.........",
            "......########......",
            "......########......",
            ".........##.........",
            ".........##.........",
            ".........##.........",
        ],
        (Glyph::Pin, _) => &[
            "....................",
            "....................",
            "....................",
            "....................",
            "........####........",
            "........#..#........",
            "........#..#........",
            ".......######.......",
            ".........##.........",
            ".........##.........",
            ".........##.........",
        ],
        (Glyph::Pinned, _) => &[
            "....................",
            "....................",
            "....................",
            "....................",
            "........####........",
            "........####........",
            "........####........",
            ".......######.......",
            ".........##.........",
            ".........##.........",
            ".........##.........",
        ],
        _ => &[],
    }
}

/// Unit squares as one mesh: a pattern of hundreds of pixels is one shape
/// rather than hundreds.
fn squares(rects: impl Iterator<Item = Rect>, colour: Color32) -> Shape {
    let mut mesh = Mesh::default();
    for r in rects {
        mesh.add_colored_rect(r, colour);
    }
    Shape::mesh(mesh)
}

/// The scroll track's 50% pattern. system.css builds it from two 45-degree
/// gradients in a 4 px tile, offset by 2 px; at 100% the browser resolves
/// that to a checkerboard of two-pixel squares, anchored at the track's
/// corner, which is what is drawn.
fn checker(rect: Rect, colour: Color32, ppp: f32) -> Shape {
    let u = prim::unit(ppp);
    let r = prim::snap_rect(rect, ppp);
    let cols = (r.width() / (2.0 * u)).ceil() as i32;
    let rows = (r.height() / (2.0 * u)).ceil() as i32;
    let cells = (0..rows).flat_map(move |j| {
        (0..cols).filter(move |i| (i + j) % 2 == 0).map(move |i| {
            let min = pos2(r.left() + i as f32 * 2.0 * u, r.top() + j as f32 * 2.0 * u);
            Rect::from_min_max(min, (min + vec2(2.0 * u, 2.0 * u)).min(r.max))
        })
    });
    squares(cells, colour)
}

/// A scroll arrow's cells, turned to face `dir`, laid on the 1.375 px grid
/// from `origin`. Each cell's edges go to the nearest physical pixel, as a
/// browser drawing the SVG crisp would put them, so the cells come out one
/// or two pixels wide and never a blur.
fn scroll_cells(origin: Pos2, dir: Dir, rows: &[&str], ppp: f32) -> Vec<Rect> {
    let u = prim::unit(ppp);
    let edge = |base: f32, n: usize| prim::snap(base + n as f32 * SCROLL_CELL * u, ppp);
    let mut cells = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            if ch != '#' {
                continue;
            }
            // The SVG's own place for the cell, pointing up.
            let (x, y) = (c + 1, r + 2);
            let (gx, gy) = match dir {
                Dir::Up => (x, y),
                Dir::Down => (x, SCROLL_CELLS - 1 - y),
                Dir::Left => (y, x),
                Dir::Right => (SCROLL_CELLS - 1 - y, x),
            };
            cells.push(Rect::from_min_max(
                pos2(edge(origin.x, gx), edge(origin.y, gy)),
                pos2(edge(origin.x, gx + 1), edge(origin.y, gy + 1)),
            ));
        }
    }
    cells
}

/// A sprite turned to face `dir`, from rows drawn pointing down.
fn turned(rows: &[&str], dir: Dir) -> Vec<String> {
    let grid: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
    let h = grid.len();
    let w = grid.first().map_or(0, Vec::len);
    let at = |x: usize, y: usize| grid.get(y).and_then(|r| r.get(x)).copied().unwrap_or('.');
    match dir {
        Dir::Down => rows.iter().map(|r| (*r).to_owned()).collect(),
        Dir::Up => (0..h)
            .rev()
            .map(|y| (0..w).map(|x| at(x, y)).collect())
            .collect(),
        Dir::Right => (0..w).map(|x| (0..h).map(|y| at(x, y)).collect()).collect(),
        Dir::Left => (0..w)
            .map(|x| (0..h).rev().map(|y| at(x, y)).collect())
            .collect(),
    }
}

impl System {
    fn ink(k: &Kit) -> Ink {
        ink(k.look.dark)
    }

    /// The one-pixel frame most things wear.
    fn frame(rect: Rect, colour: Color32, ppp: f32) -> Vec<Shape> {
        inset(rect, &[(1, 1, colour), (-1, -1, colour)], ppp)
    }

    /// `box-shadow: 2px 2px`, the drop shadow of a menu: the box's own
    /// shape moved `n` pixels down and right, showing past its right and
    /// bottom edges.
    fn shadow(rect: Rect, n: f32, colour: Color32, ppp: f32) -> Vec<Shape> {
        let u = prim::unit(ppp);
        let r = prim::snap_rect(rect, ppp);
        let d = n * u;
        vec![
            fill(
                Rect::from_min_max(
                    pos2(r.right(), r.top() + d),
                    pos2(r.right() + d, r.bottom() + d),
                ),
                colour,
            ),
            fill(
                Rect::from_min_max(
                    pos2(r.left() + d, r.bottom()),
                    pos2(r.right(), r.bottom() + d),
                ),
                colour,
            ),
        ]
    }

    /// Where the stripes of a title bar run in `bar`: the 24 px band that
    /// `.title-bar` is tall inside its padding, two pixels in from each side
    /// (`padding: 0.2rem 0.1rem`), above the separator along its foot.
    /// Placed from where a box centred on the bar lands, one pixel above
    /// it - where the browser puts the band, the box's frame merging with
    /// the stripe above and the stripe below.
    fn band(k: &Kit, bar: Rect) -> Rect {
        let u = k.u();
        let r = prim::snap_rect(bar, k.ppp);
        let size = System.metrics().title_button.y * u;
        let boxed = prim::snap(bar.center().y - size / 2.0, k.ppp);
        // The lines run exactly as tall as the boxes, and up to their sides.
        let top = boxed.max(r.top());
        let bottom = (top + size).min(r.bottom() - u);
        // `.title-bar { padding: 0.2rem 0.1rem }` and the window's own edge:
        // paper before the first line and after the last.
        Rect::from_min_max(
            pos2(r.left() + 3.0 * u, top),
            pos2(r.right() - 3.0 * u, bottom),
        )
    }
}

impl Style for System {
    fn colors(&self, dark: bool) -> Colors {
        let i = ink(dark);
        Colors {
            face: i.paper,
            text: i.ink,
            text_disabled: i.disabled,
            text_disabled_shadow: None,
            field: i.paper,
            field_text: i.ink,
            select: i.ink,
            select_text: i.paper,
            title_text: i.ink,
            title_text_inactive: i.grey,
            title_shadow: None,
            link: i.ink,
            tooltip_text: i.ink,
            subtle: i.ink,
        }
    }

    fn metrics(&self) -> Metrics {
        // The sizes System 7 drew at rather than system.css's, which scales
        // every control up for a web page: at 100% the library's 35 px title
        // bar and 22 px scroll bars made a window twice the size the Mac's
        // were.
        Metrics {
            // Both faces are pixel fonts on a 16 px em, where one of their
            // pixels is one of the screen's; Geneva's letters are drawn small
            // within it, as Geneva 9's were.
            font_size: 16.0,
            title_font_size: 16.0,
            // 19 px, the System 7 title bar, with its line underneath.
            title_height: 19.0,
            // The close and zoom boxes: 11 px with a one-pixel frame.
            title_button: vec2(11.0, 11.0),
            title_button_gap: 6.0,
            close_gap: 6.0,
            window_border: 1.0,
            window_radius: 0.0,
            // The arrows' 17-cell drawing at a pixel a cell.
            scrollbar: 17.0,
            scroll_arrow: 17.0,
            checkbox: 15.0,
            radio: 12.0,
            button_min: vec2(58.0, 20.0),
            button_pad_x: 12.0,
            field_height: 20.0,
            tab_height: 20.0,
            select_arrow: 16.0,
            menu_item_height: 18.0,
            slider_thumb: vec2(10.0, 14.0),
            slider_track: 5.0,
            label_gap: 6.0,
        }
    }

    fn fonts(&self) -> Fonts {
        // ChiKareGo2 is the face system.css calls `Chicago_12`; Chicago has
        // no lighter weight, so it is the bold and the title too.
        let chicago = Face::Bundled(include_bytes!(
            "../../../assets/fonts/chikarego2/ChiKareGo2.ttf"
        ));
        // Body text in Geneva 9, as the Finder set it: Chicago at the size
        // of every label made the whole window half again too big.
        let geneva = Face::Bundled(include_bytes!(
            "../../../assets/fonts/finderskeepers/FindersKeepers.ttf"
        ));
        Fonts {
            body: Some(geneva),
            bold: Some(chicago),
            title: Some(chicago),
        }
    }

    fn pressed_shift(&self) -> f32 {
        // A held button inverts; nothing moves.
        0.0
    }

    fn title_centred(&self) -> bool {
        true
    }

    fn title_backing(&self, k: &Kit, text: Rect, st: St) -> Vec<Shape> {
        let _ = st;
        // `.title { padding: 0 0.5em; background: var(--primary) }`, its
        // line box as tall as the stripes.
        let u = k.u();
        let tall = System.metrics().title_button.y * u + 2.0 * u;
        let top = prim::snap(text.center().y - tall / 2.0, k.ppp);
        let r = Rect::from_min_max(
            pos2(prim::snap(text.left() - 8.0 * u, k.ppp), top),
            pos2(prim::snap(text.right() + 8.0 * u, k.ppp), top + tall),
        );
        vec![fill(r, Self::ink(k).paper)]
    }

    fn button_text(&self, k: &Kit, st: St) -> Color32 {
        let i = Self::ink(k);
        if st.disabled {
            i.disabled
        } else if st.pressed {
            // `.btn:active { color: var(--primary) }`.
            i.paper
        } else {
            i.ink
        }
    }

    fn button(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let i = Self::ink(k);
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let pressed = st.pressed && !st.disabled;
        let mut out = Vec::new();
        if st.default {
            // The ring stands outside the button, as the default button is
            // four pixels bigger all round in the CSS.
            let ring = r.expand(4.0 * u);
            out.push(stepped(ring, &RING_STEPS, true, i.ink, k.ppp));
            if pressed {
                // `.btn-default:active { border-radius: 12px }`: all of it
                // goes black.
                return out;
            }
            out.push(stepped(
                ring.shrink(3.0 * u),
                &RING_HOLE,
                true,
                i.paper,
                k.ppp,
            ));
        }
        out.push(stepped(r, &BUTTON_STEPS, true, i.ink, k.ppp));
        if !pressed {
            // `.btn:active { background: var(--secondary); border-radius:
            // 6px }` fills the outline's own shape; at rest it is paper.
            out.push(stepped(r.shrink(u), &BUTTON_INSIDE, true, i.paper, k.ppp));
        }
        out
    }

    fn field(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `input { border: 1.5px solid }`, which lands as one pixel. The
        // library inverts a focused field, but the text typed into it is not
        // the skin's to colour, so a focused field stays paper.
        let _ = st;
        let i = Self::ink(k);
        let mut out = vec![fill(prim::snap_rect(rect, k.ppp), i.paper)];
        out.extend(Self::frame(rect, i.ink, k.ppp));
        out
    }

    fn checkbox(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let i = Self::ink(k);
        let u = k.u();
        let line = if st.disabled { i.disabled } else { i.ink };
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![fill(r, i.paper)];
        out.extend(Self::frame(r, line, k.ppp));
        if (st.hovered || st.focused) && !st.disabled {
            // `:hover + label::before { outline: 1px solid }`, outside.
            out.extend(Self::frame(r.expand(u), i.ink, k.ppp));
        }
        if st.on {
            // `--checkmark-top` and `--checkmark-left`: 2 px in from the
            // box's outer corner, so the cross meets the far border.
            let at = pos2(r.left() + 2.0 * u, r.top() + 2.0 * u);
            out.extend(sprite(at, &CHECKMARK, &[('#', line)], k.ppp));
        }
        out
    }

    fn radio(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let i = Self::ink(k);
        let u = k.u();
        let line = if st.disabled { i.disabled } else { i.ink };
        let rows: &[&str] = if (st.hovered || st.focused) && !st.disabled {
            &RADIO_FOCUSED
        } else {
            &RADIO
        };
        let at = prim::centred(rect, rows, k.ppp);
        let mut out = sprite(at, &RADIO_DISC, &[('#', i.paper)], k.ppp);
        out.extend(sprite(at, rows, &[('#', line)], k.ppp));
        if st.on {
            let dot = pos2(at.x + 3.0 * u, at.y + 3.0 * u);
            out.extend(sprite(dot, &RADIO_DOT, &[('#', line)], k.ppp));
        }
        out
    }

    fn select(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let i = Self::ink(k);
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        // `box-shadow: 2px 2px 0 -1px`: the box shrunk by one and moved two,
        // which leaves one pixel showing past the right and the bottom.
        let mut out = Self::shadow(r.shrink(u).translate(vec2(u, u)), 1.0, i.ink, k.ppp);
        out.extend(self.field(k, r, st));
        out.extend(self.arrow(k, r, Dir::Down, st));
        out
    }

    fn select_text(&self, k: &Kit, st: St) -> Color32 {
        let i = Self::ink(k);
        if st.disabled {
            i.disabled
        } else {
            i.ink
        }
    }

    fn arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        let i = Self::ink(k);
        let colour = if st.disabled { i.disabled } else { i.ink };
        let rows = turned(&SELECT_ARROW, dir);
        let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let at = if dir == Dir::Down && r.height() >= 22.0 * u && r.width() >= 40.0 * u {
            // A whole select: `background-position: top 2px right 2px`
            // inside the border puts the 17 px square there, and the
            // triangle five rows down and two across in it.
            pos2(r.right() - 18.0 * u, r.top() + 8.0 * u)
        } else {
            prim::centred(rect, &rows, k.ppp)
        };
        sprite(at, &rows, &[('#', colour)], k.ppp)
    }

    fn slider_track(&self, k: &Kit, rect: Rect, _st: St) -> Vec<Shape> {
        // Not in system.css. A six-pixel groove in the library's hand: the
        // scroll track's checkerboard in a stepped outline.
        let i = Self::ink(k);
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let h = self.metrics().slider_track * u;
        let top = prim::snap(r.center().y - h / 2.0, k.ppp);
        let groove = Rect::from_min_max(pos2(r.left(), top), pos2(r.right(), top + h));
        vec![
            stepped(groove, &[1], true, i.ink, k.ppp),
            fill(groove.shrink(u), i.paper),
            checker(groove.shrink(2.0 * u), i.ink, k.ppp),
        ]
    }

    fn slider_thumb(&self, k: &Kit, centre: Pos2, st: St) -> Vec<Shape> {
        // Not in system.css: a small button standing on the groove, with a
        // grip line down its middle, inverted while held as a button is.
        let i = Self::ink(k);
        let u = k.u();
        let line = if st.disabled { i.disabled } else { i.ink };
        let size = self.metrics().slider_thumb * u;
        let r = prim::snap_rect(Rect::from_center_size(centre, size), k.ppp);
        let mut out = vec![stepped(r, &[1], true, line, k.ppp)];
        if !(st.pressed && !st.disabled) {
            out.push(fill(r.shrink(u), i.paper));
            let mid = prim::snap(r.center().x - u / 2.0, k.ppp);
            out.push(fill(
                Rect::from_min_max(
                    pos2(mid, r.top() + 3.0 * u),
                    pos2(mid + u, r.bottom() - 3.0 * u),
                ),
                line,
            ));
        }
        out
    }

    fn tab(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // Not in system.css: the tabs of System 7.5's control panels, an
        // outline with the button's top corners. The selected one stands
        // two pixels taller and runs one down over the panel's line, its
        // foot open; the others stand on that line.
        let i = Self::ink(k);
        let u = k.u();
        let mut r = prim::snap_rect(rect, k.ppp);
        if st.on {
            r = Rect::from_min_max(
                pos2(r.left(), r.top() - 2.0 * u),
                pos2(r.right(), r.bottom() + u),
            );
        }
        let inside = Rect::from_min_max(
            pos2(r.left() + u, r.top() + u),
            pos2(
                r.right() - u,
                if st.on { r.bottom() } else { r.bottom() - u },
            ),
        );
        vec![
            stepped(r, &BUTTON_STEPS, false, i.ink, k.ppp),
            stepped(inside, &BUTTON_INSIDE, false, i.paper, k.ppp),
        ]
    }

    fn tab_panel(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let i = Self::ink(k);
        let mut out = vec![fill(prim::snap_rect(rect, k.ppp), i.paper)];
        out.extend(Self::frame(rect, i.ink, k.ppp));
        out
    }

    fn group(&self, k: &Kit, rect: Rect, legend: Option<Rect>) -> Vec<Shape> {
        // Not in system.css: the one-pixel line everything else is framed
        // in, broken for the legend.
        let i = Self::ink(k);
        let mut out = Self::frame(rect, i.ink, k.ppp);
        if let Some(legend) = legend {
            let r = prim::snap_rect(legend.expand2(vec2(2.0 * k.u(), 0.0)), k.ppp);
            out.push(fill(r, i.paper));
        }
        out
    }

    fn window(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `.window { border: 0.1em solid }`, and the one-pixel drop shadow
        // every classic Mac window cast, just outside it.
        let _ = st;
        let i = Self::ink(k);
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = Self::shadow(r, 1.0, i.ink, k.ppp);
        out.push(fill(r, i.paper));
        out.extend(Self::frame(r, i.ink, k.ppp));
        out
    }

    fn title_bar(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let i = Self::ink(k);
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![fill(r, i.paper)];
        if !st.inactive_window {
            // `linear-gradient(var(--secondary) 50%, transparent 50%)` at
            // `background-size: 13.33%` of the bar: two pixels of ink, two
            // of paper, from the top of the band. `.inactive-title-bar`
            // drops them.
            let band = Self::band(k, r);
            let mut y = band.top();
            let mut lines = Vec::new();
            while y < band.bottom() {
                // One pixel of ink, one of paper: System 7's lines.
                lines.push(Rect::from_min_max(
                    pos2(band.left(), y),
                    pos2(band.right(), (y + u).min(band.bottom())),
                ));
                y += 2.0 * u;
            }
            out.push(squares(lines.into_iter(), i.ink));
        }
        // `.separator { border-top: 0.1rem solid }` under the bar.
        out.push(fill(
            Rect::from_min_max(pos2(r.left(), r.bottom() - u), r.max),
            i.ink,
        ));
        out
    }

    fn title_button(&self, k: &Kit, rect: Rect, glyph: Glyph, st: St) -> Vec<Shape> {
        // `.title-bar button`: a 2 px frame round paper, no bevel, nothing
        // on hover. An inactive window's boxes go grey rather than away, as
        // the app's own buttons have to stay where they can be found.
        let i = Self::ink(k);
        let line = if st.inactive_window || st.disabled {
            i.grey
        } else {
            i.ink
        };
        let r = prim::snap_rect(rect, k.ppp);
        let pressed = st.pressed && !st.disabled;
        // The app's own boxes invert while held; close and zoom do what the
        // CSS has them do.
        let own = !matches!(
            glyph,
            Glyph::Close | Glyph::Maximize | Glyph::Restore | Glyph::Minimize
        );
        // A colour of the user's draws the box's frame and its mark while the
        // window has the keyboard, on the paper every box is; an inactive
        // window's boxes keep the skin's own ink.
        let line = match k.button_colour(glyph) {
            Some(colour) if !st.inactive_window && !st.disabled => colour,
            _ => line,
        };
        let (inside, mark) = if pressed && own {
            (line, i.paper)
        } else if st.inactive_window || st.disabled {
            (i.paper, line)
        } else {
            (i.paper, k.glyph_ink(line))
        };
        let mut out = vec![fill(r, inside)];
        out.extend(inset(r, &[(1, 1, line), (-1, -1, line)], k.ppp));
        let rows = small_glyph(glyph, pressed);
        if !rows.is_empty() {
            let at = prim::centred(r, rows, k.ppp);
            out.extend(sprite(at, rows, &[('#', mark)], k.ppp));
        }
        out
    }

    fn scroll_track(&self, k: &Kit, rect: Rect, vertical: bool, _st: St) -> Vec<Shape> {
        // The checkerboard on paper, and `.window-pane::-webkit-scrollbar-
        // track { border-left: 4px solid }` between it and the page - along
        // the top of a horizontal bar, the same edge turned.
        let i = Self::ink(k);
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let edge = if vertical {
            Rect::from_min_max(r.min, pos2(r.left() + 4.0 * u, r.bottom()))
        } else {
            Rect::from_min_max(r.min, pos2(r.right(), r.top() + 4.0 * u))
        };
        vec![
            fill(r, i.paper),
            checker(r, i.ink, k.ppp),
            fill(edge, i.ink),
        ]
    }

    fn scroll_thumb(&self, k: &Kit, rect: Rect, vertical: bool, _st: St) -> Vec<Shape> {
        // `::-webkit-scrollbar-thumb`: paper in a 2 px border with
        // `border-right: none` - the window's own frame closes it. Turned,
        // the open side is the bottom.
        let i = Self::ink(k);
        let r = prim::snap_rect(rect, k.ppp);
        let layers: [prim::Inset; 2] = if vertical {
            [(2, 2, i.ink), (0, -2, i.ink)]
        } else {
            [(2, 2, i.ink), (-2, 0, i.ink)]
        };
        let mut out = vec![fill(r, i.paper)];
        out.extend(inset(r, &layers, k.ppp));
        out
    }

    fn scroll_arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        // `icon/scrollbar-*.svg`: a framed box, a two-pixel band on the side
        // that faces the track, and the arrow - hollow, or filled while held
        // (`*-active.svg`).
        let i = Self::ink(k);
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![fill(r, i.paper)];
        out.extend(Self::frame(r, i.ink, k.ppp));
        let band = match dir {
            Dir::Up => Rect::from_min_max(pos2(r.left(), r.bottom() - 2.0 * u), r.max),
            Dir::Down => Rect::from_min_max(r.min, pos2(r.right(), r.top() + 2.0 * u)),
            Dir::Left => Rect::from_min_max(pos2(r.right() - 2.0 * u, r.top()), r.max),
            Dir::Right => Rect::from_min_max(r.min, pos2(r.left() + 2.0 * u, r.bottom())),
        };
        out.push(fill(band, i.ink));
        let rows: &[&str] = if st.pressed && !st.disabled {
            &SCROLL_ARROW_ACTIVE
        } else {
            &SCROLL_ARROW
        };
        let colour = if st.disabled { i.disabled } else { i.ink };
        out.push(squares(
            scroll_cells(r.min, dir, rows, k.ppp).into_iter(),
            colour,
        ));
        out
    }

    fn scroll_corner(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `::-webkit-scrollbar { background-color: var(--primary) }`.
        vec![fill(prim::snap_rect(rect, k.ppp), Self::ink(k).paper)]
    }

    fn menu(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `ul[role="menu"] { border: 1px solid; box-shadow: 2px 2px }`.
        let i = Self::ink(k);
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = Self::shadow(r, 2.0, i.ink, k.ppp);
        out.push(fill(r, i.paper));
        out.extend(Self::frame(r, i.ink, k.ppp));
        out
    }

    fn menu_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `[role="menu-item"] > a:hover { background: var(--secondary) }`.
        if (st.hovered || st.on) && !st.disabled {
            vec![fill(prim::snap_rect(rect, k.ppp), Self::ink(k).ink)]
        } else {
            Vec::new()
        }
    }

    fn separator(&self, k: &Kit, rect: Rect, vertical: bool) -> Vec<Shape> {
        // `.divider::after { border-top: 1.5px dotted }`: every other pixel.
        let i = Self::ink(k);
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let dots: Vec<Rect> = if vertical {
            let x = prim::snap(r.center().x - u / 2.0, k.ppp);
            let n = (r.height() / u).round() as i32;
            (0..n)
                .step_by(2)
                .map(|j| Rect::from_min_size(pos2(x, r.top() + j as f32 * u), vec2(u, u)))
                .collect()
        } else {
            let y = prim::snap(r.center().y - u / 2.0, k.ppp);
            let n = (r.width() / u).round() as i32;
            (0..n)
                .step_by(2)
                .map(|j| Rect::from_min_size(pos2(r.left() + j as f32 * u, y), vec2(u, u)))
                .collect()
        };
        vec![squares(dots.into_iter(), i.ink)]
    }

    fn tooltip(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // Not in system.css: a pop-up, so the menu's frame and shadow.
        self.menu(k, rect)
    }

    fn list(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        self.field(k, rect, St::default())
    }

    fn list_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // The selection inverts, as a menu's does; the pointer lights
        // nothing.
        if st.on {
            vec![fill(prim::snap_rect(rect, k.ppp), Self::ink(k).ink)]
        } else {
            Vec::new()
        }
    }

    fn list_text(&self, k: &Kit, st: St) -> Color32 {
        let i = Self::ink(k);
        if st.on {
            i.paper
        } else {
            i.ink
        }
    }

    fn progress(&self, k: &Kit, rect: Rect, fraction: f32) -> Vec<Shape> {
        // Not in system.css: the Finder's copy bar, a frame filled with ink
        // from the left.
        let i = Self::ink(k);
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![fill(r, i.paper)];
        out.extend(Self::frame(r, i.ink, k.ppp));
        let inner = r.shrink(k.u());
        let end = prim::snap(
            inner.left() + inner.width() * fraction.clamp(0.0, 1.0),
            k.ppp,
        );
        if end > inner.left() {
            out.push(fill(
                Rect::from_min_max(inner.min, pos2(end, inner.bottom())),
                i.ink,
            ));
        }
        out
    }

    fn status_bar(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `.details-bar { border-width: 0.1rem 0 }`: paper under a line; the
        // window's frame is the lower one.
        let i = Self::ink(k);
        let r = prim::snap_rect(rect, k.ppp);
        vec![
            fill(r, i.paper),
            fill(
                Rect::from_min_max(r.min, pos2(r.right(), r.top() + k.u())),
                i.ink,
            ),
        ]
    }

    fn status_field(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // The details bar has no fields: its texts stand on the paper.
        let _ = (k, rect);
        Vec::new()
    }

    fn focus(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `:focus-visible + label::before { outline: 1px solid }`.
        Self::frame(rect, Self::ink(k).ink, k.ppp)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Look, Skin};
    use super::*;

    fn kit(dark: bool) -> Kit {
        Kit::bare(
            Look {
                skin: Skin::System,
                dark,
                ..Default::default()
            },
            1.0,
        )
    }

    /// The colour the last shape covering a pixel paints it.
    fn colour_at(shapes: &[Shape], x: f32, y: f32) -> Color32 {
        let p = pos2(x + 0.5, y + 0.5);
        shapes
            .iter()
            .rev()
            .find_map(|s| match s {
                Shape::Rect(r) if r.rect.contains(p) => Some(r.fill),
                // Every mesh here is axis-aligned squares, two triangles to
                // each, so a triangle's bounds are its square.
                Shape::Mesh(m) => m.indices.chunks(3).find_map(|t| {
                    let [a, b, c] = [t[0], t[1], t[2]].map(|i| m.vertices[i as usize].pos);
                    Rect::from_points(&[a, b, c])
                        .contains(p)
                        .then_some(m.vertices[t[0] as usize].color)
                }),
                _ => None,
            })
            .unwrap_or(Color32::TRANSPARENT)
    }

    #[test]
    fn every_icon_row_is_as_wide_as_the_others() {
        let all: Vec<&[&str]> = vec![
            &CHECKMARK,
            &RADIO,
            &RADIO_FOCUSED,
            &RADIO_DISC,
            &RADIO_DOT,
            &SELECT_ARROW,
            &SCROLL_ARROW,
            &SCROLL_ARROW_ACTIVE,
        ];
        for rows in all {
            let w = rows[0].chars().count();
            assert!(rows.iter().all(|r| r.chars().count() == w), "{rows:?}");
        }
    }

    #[test]
    fn no_title_box_mark_lands_on_the_box_frame() {
        let glyphs = [
            Glyph::Close,
            Glyph::Maximize,
            Glyph::Restore,
            Glyph::Minimize,
            Glyph::Help,
            Glyph::Gear,
            Glyph::Plus,
            Glyph::Pin,
            Glyph::Pinned,
        ];
        for glyph in glyphs {
            for pressed in [false, true] {
                for (y, row) in box_glyph(glyph, pressed).iter().enumerate() {
                    for (x, c) in row.chars().enumerate() {
                        if c == '#' {
                            assert!(
                                (2..18).contains(&x) && (2..18).contains(&y),
                                "{glyph:?} at {x},{y}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// The button's corner as the browser draws `button.svg` at 100%: the
    /// outline steps in three pixels on the top row, one on the next two,
    /// and the paper starts one pixel inside it.
    #[test]
    fn a_button_has_the_stepped_corner_of_its_border_image() {
        let k = kit(false);
        let shapes = System.button(
            &k,
            Rect::from_min_size(Pos2::ZERO, vec2(98.0, 29.0)),
            St::default(),
        );
        let black = hex(0x000000);
        let white = hex(0xffffff);
        assert_eq!(colour_at(&shapes, 2.0, 0.0), Color32::TRANSPARENT);
        assert_eq!(colour_at(&shapes, 3.0, 0.0), black);
        assert_eq!(colour_at(&shapes, 0.0, 1.0), Color32::TRANSPARENT);
        assert_eq!(colour_at(&shapes, 1.0, 1.0), black);
        assert_eq!(colour_at(&shapes, 2.0, 1.0), black);
        assert_eq!(colour_at(&shapes, 3.0, 1.0), white);
        assert_eq!(colour_at(&shapes, 0.0, 3.0), black);
        assert_eq!(colour_at(&shapes, 1.0, 3.0), white);
        assert_eq!(colour_at(&shapes, 97.0, 14.0), black);
        assert_eq!(colour_at(&shapes, 50.0, 28.0), black);
        assert_eq!(colour_at(&shapes, 50.0, 14.0), white);
    }

    #[test]
    fn a_held_button_inverts_and_so_does_its_label() {
        let k = kit(false);
        let held = St {
            pressed: true,
            ..St::default()
        };
        let shapes = System.button(&k, Rect::from_min_size(Pos2::ZERO, vec2(60.0, 28.0)), held);
        assert_eq!(colour_at(&shapes, 30.0, 14.0), hex(0x000000));
        assert_eq!(System.button_text(&k, held), hex(0xffffff));
        assert_eq!(System.button_text(&k, St::default()), hex(0x000000));
    }

    /// The dark reading is the light one with black and white swapped,
    /// pixel for pixel.
    #[test]
    fn the_dark_reading_is_the_light_one_turned_over() {
        let rect = Rect::from_min_size(pos2(4.0, 4.0), vec2(60.0, 30.0));
        let (light, dark) = (kit(false), kit(true));
        let flip = |c: Color32| Color32::from_rgb(255 - c.r(), 255 - c.g(), 255 - c.b());
        let default = St {
            default: true,
            ..St::default()
        };
        let pairs: Vec<(Vec<Shape>, Vec<Shape>)> = vec![
            (
                System.button(&light, rect, default),
                System.button(&dark, rect, default),
            ),
            (
                System.title_bar(&light, rect, St::default()),
                System.title_bar(&dark, rect, St::default()),
            ),
            (
                System.scroll_track(&light, rect, true, St::default()),
                System.scroll_track(&dark, rect, true, St::default()),
            ),
            (
                System.scroll_arrow(&light, rect, Dir::Up, St::default()),
                System.scroll_arrow(&dark, rect, Dir::Up, St::default()),
            ),
        ];
        for (l, d) in pairs {
            for y in 0..40 {
                for x in 0..70 {
                    let (a, b) = (
                        colour_at(&l, x as f32, y as f32),
                        colour_at(&d, x as f32, y as f32),
                    );
                    if a == Color32::TRANSPARENT {
                        assert_eq!(b, Color32::TRANSPARENT, "at {x},{y}");
                    } else {
                        assert_eq!(flip(a), b, "at {x},{y}");
                    }
                }
            }
        }
    }

    /// The track's pattern is half ink: a checkerboard of two-pixel squares
    /// anchored at the track's corner, behind the 4 px edge.
    #[test]
    fn the_scroll_track_is_a_two_pixel_checkerboard() {
        let k = kit(false);
        let r = Rect::from_min_size(Pos2::ZERO, vec2(22.0, 40.0));
        let shapes = System.scroll_track(&k, r, true, St::default());
        let black = hex(0x000000);
        let white = hex(0xffffff);
        assert_eq!(colour_at(&shapes, 1.0, 20.0), black, "the edge");
        assert_eq!(colour_at(&shapes, 4.0, 0.0), black);
        assert_eq!(colour_at(&shapes, 5.0, 1.0), black);
        assert_eq!(colour_at(&shapes, 6.0, 0.0), white);
        assert_eq!(colour_at(&shapes, 6.0, 2.0), black);
        assert_eq!(colour_at(&shapes, 4.0, 2.0), white);
    }

    /// The controls laid out as system.css's own showcase lays them, with
    /// their labels in the colours the app would give them - for setting
    /// beside a headless render of the library and comparing pixel by pixel.
    #[test]
    #[ignore = "writes pictures for a person to look at"]
    fn film_the_showcase() {
        use crate::ui::snapshot::{film, scenes, Keep, Look as Dressed, Scene, Stage};
        let out = crate::ui::snapshot::out_dir();
        for theme in ["System", "System Dark"] {
            let look = Dressed::builtin(theme).expect("a built-in System theme");
            for ppp in [1.0, 2.0] {
                let painted = look.clone();
                let scene = Scene {
                    name: "showcase",
                    stage: Stage::new(vec2(900.0, 520.0)),
                    cues: Vec::new(),
                    keep: Keep::Main,
                    draw: Box::new(move |ctx: &egui::Context| {
                        painted.prelude(ctx);
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none())
                            .show(ctx, |ui| {
                                if let Some(k) = Kit::of(ctx) {
                                    ui.painter().rect_filled(
                                        ui.max_rect(),
                                        0.0,
                                        ink(k.look.dark).paper,
                                    );
                                    showcase(ui, &k);
                                }
                            });
                    }),
                };
                let dir = out
                    .join(crate::ui::snapshot::slug(theme))
                    .join(crate::ui::snapshot::scale_dir(ppp));
                film(scene, ppp, |ctx| scenes::dress(ctx, &look), &dir).expect("filmed");
            }
        }
    }

    fn showcase(ui: &mut egui::Ui, k: &Kit) {
        let s = &System;
        let m = s.metrics();
        let p = ui.painter().clone();
        let u = k.u();
        let o = ui.max_rect().min;
        let font = egui::FontId::proportional(m.font_size);
        let title_font =
            egui::FontId::new(m.title_font_size, crate::ui::fonts::skin_title(ui.ctx()));
        let text = |at: Pos2, align: egui::Align2, t: &str, c: Color32| {
            p.text(at, align, t, font.clone(), c);
        };
        let at = |x: f32, y: f32| o + vec2(x, y) * u;
        let i = ink(k.look.dark);

        // Buttons: at rest, held, default, default held, disabled.
        let held = St {
            pressed: true,
            ..St::default()
        };
        let default = St {
            default: true,
            ..St::default()
        };
        let buttons = [
            ("Cancel", 10.0, 98.0, St::default()),
            ("Active", 120.0, 98.0, held),
            ("Find", 234.0, 82.0, default),
            (
                "Find",
                334.0,
                82.0,
                St {
                    pressed: true,
                    ..default
                },
            ),
            ("Disabled", 432.0, 111.0, St::default().disabled(true)),
        ];
        for (name, x, w, st) in buttons {
            let r = Rect::from_min_size(at(x, 14.0), vec2(w, m.button_min.y) * u);
            p.extend(s.button(k, r, st));
            text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                name,
                s.button_text(k, st),
            );
        }

        // A field, and a select.
        let f = Rect::from_min_size(at(10.0, 61.0), vec2(159.0, m.field_height) * u);
        p.extend(s.field(k, f, St::default()));
        text(
            pos2(f.left() + 6.0 * u, f.center().y),
            egui::Align2::LEFT_CENTER,
            "text field",
            i.ink,
        );
        let sel = Rect::from_min_size(at(352.0, 62.0), vec2(163.0, 22.0) * u);
        p.extend(s.select(k, sel, St::default()));
        text(
            pos2(sel.left() + 20.0 * u, sel.center().y),
            egui::Align2::LEFT_CENTER,
            "Select menu",
            s.select_text(k, St::default()),
        );

        // Radios and checkboxes.
        let mut x = 10.0;
        for (name, on, radio) in [
            ("Radio", false, true),
            ("Checked", true, true),
            ("Check", false, false),
            ("Checked", true, false),
        ] {
            let size = if radio { m.radio } else { m.checkbox };
            let r = Rect::from_min_size(at(x, 104.0), egui::Vec2::splat(size) * u);
            p.extend(if radio {
                s.radio(k, r, St::default().on(on))
            } else {
                s.checkbox(k, r, St::default().on(on))
            });
            text(
                pos2(r.right() + m.label_gap * u, r.center().y),
                egui::Align2::LEFT_CENTER,
                name,
                i.ink,
            );
            x += 90.0;
        }

        // A window: title bar, close and zoom boxes, a pane with its bar.
        let win = Rect::from_min_size(at(10.0, 131.0), vec2(362.0, 209.0) * u);
        p.extend(s.window(k, win, St::default()));
        let bar = Rect::from_min_size(
            win.min + vec2(u, u),
            vec2(win.width() - 2.0 * u, m.title_height * u),
        );
        p.extend(s.title_bar(k, bar, St::default()));
        let title = p.layout_no_wrap("System.css".into(), title_font.clone(), i.ink);
        let title_rect = Rect::from_center_size(bar.center() - vec2(0.0, 0.5 * u), title.size());
        p.extend(s.title_backing(k, title_rect, St::default()));
        p.galley(title_rect.min, title, i.ink);
        let size = m.title_button * u;
        let close = Rect::from_min_size(
            pos2(win.left() + 16.0 * u, bar.center().y - size.y / 2.0),
            size,
        );
        p.extend(s.title_button(k, close, Glyph::Close, St::default()));
        let zoom = Rect::from_min_size(
            pos2(
                win.right() - 16.0 * u - size.x,
                bar.center().y - size.y / 2.0,
            ),
            size,
        );
        p.extend(s.title_button(k, zoom, Glyph::Maximize, St::default()));
        let pane = Rect::from_min_max(pos2(win.left() + u, bar.bottom()), win.max - vec2(u, u));
        text(
            pane.min + vec2(32.0, 30.0) * u,
            egui::Align2::LEFT_CENTER,
            "Lorem ipsum dolor sit amet,",
            i.ink,
        );
        let sb = m.scrollbar * u;
        let arrow = m.scroll_arrow * u;
        let track = Rect::from_min_max(pos2(pane.right() - sb, pane.top()), pane.max);
        p.extend(s.scroll_track(k, track, true, St::default()));
        p.extend(s.scroll_arrow(
            k,
            Rect::from_min_size(track.min, vec2(sb, arrow)),
            Dir::Up,
            St::default(),
        ));
        p.extend(s.scroll_arrow(
            k,
            Rect::from_min_max(pos2(track.left(), track.bottom() - arrow), track.max),
            Dir::Down,
            held,
        ));
        p.extend(s.scroll_thumb(
            k,
            Rect::from_min_size(track.min + vec2(0.0, arrow), vec2(sb, 70.0 * u)),
            true,
            St::default(),
        ));

        // A menu bar title, a menu with a highlighted row and a divider.
        let file = Rect::from_min_size(at(384.0, 131.0), vec2(44.0, 28.0) * u);
        p.extend(s.menu_item(
            k,
            file,
            St {
                hovered: true,
                ..St::default()
            },
        ));
        text(file.center(), egui::Align2::CENTER_CENTER, "File", i.paper);
        let menu = Rect::from_min_size(at(384.0, 159.0), vec2(220.0, 107.0) * u);
        p.extend(s.menu(k, menu));
        for (n, name) in ["Action", "Another Action", "Something else", "Last"]
            .into_iter()
            .enumerate()
        {
            let row = Rect::from_min_size(
                menu.min + vec2(u, u + n as f32 * m.menu_item_height * u),
                vec2(menu.width() - 2.0 * u, m.menu_item_height * u),
            );
            let st = St {
                hovered: n == 1,
                ..St::default()
            };
            p.extend(s.menu_item(k, row, st));
            text(
                pos2(row.left() + 20.0 * u, row.center().y),
                egui::Align2::LEFT_CENTER,
                name,
                if n == 1 { i.paper } else { i.ink },
            );
            if n == 2 {
                p.extend(s.separator(
                    k,
                    Rect::from_min_size(pos2(row.left(), row.bottom() - u), vec2(row.width(), u)),
                    false,
                ));
            }
        }

        // An inactive window with a details bar, and the held boxes.
        let win = Rect::from_min_size(at(10.0, 354.0), vec2(362.0, 70.0) * u);
        p.extend(s.window(k, win, St::default()));
        let bar = Rect::from_min_size(
            win.min + vec2(u, u),
            vec2(win.width() - 2.0 * u, m.title_height * u),
        );
        let inactive = St {
            inactive_window: true,
            ..St::default()
        };
        p.extend(s.title_bar(k, bar, inactive));
        p.text(
            bar.center(),
            egui::Align2::CENTER_CENTER,
            "Inactive",
            title_font.clone(),
            i.grey,
        );
        let foot = Rect::from_min_max(pos2(win.left() + u, bar.bottom()), win.max - vec2(u, u));
        p.extend(s.status_bar(k, foot));
        text(
            pos2(foot.left() + 13.0 * u, foot.center().y),
            egui::Align2::LEFT_CENTER,
            "3 items",
            i.ink,
        );
        let win = Rect::from_min_size(at(384.0, 371.0), vec2(322.0, 37.0) * u);
        p.extend(s.window(k, win, St::default()));
        let bar = Rect::from_min_size(
            win.min + vec2(u, u),
            vec2(win.width() - 2.0 * u, m.title_height * u),
        );
        p.extend(s.title_bar(k, bar, St::default()));
        let close = Rect::from_min_size(
            pos2(win.left() + 16.0 * u, bar.center().y - size.y / 2.0),
            size,
        );
        p.extend(s.title_button(k, close, Glyph::Close, held));
        let zoom = Rect::from_min_size(
            pos2(
                win.right() - 16.0 * u - size.x,
                bar.center().y - size.y / 2.0,
            ),
            size,
        );
        p.extend(s.title_button(k, zoom, Glyph::Maximize, held));
        let mut x = win.left();
        for glyph in [
            Glyph::Minimize,
            Glyph::Restore,
            Glyph::Help,
            Glyph::Gear,
            Glyph::Plus,
            Glyph::Pin,
            Glyph::Pinned,
        ] {
            let b = Rect::from_min_size(pos2(x, win.bottom() + 12.0 * u), size);
            p.extend(s.title_button(k, b, glyph, St::default()));
            x += size.x + 6.0 * u;
        }
        let h = Rect::from_min_size(
            pos2(win.left(), win.bottom() + 44.0 * u),
            vec2(240.0 * u, sb),
        );
        p.extend(s.scroll_track(k, h, false, St::default()));
        p.extend(s.scroll_arrow(
            k,
            Rect::from_min_size(h.min, vec2(arrow, sb)),
            Dir::Left,
            held,
        ));
        p.extend(s.scroll_arrow(
            k,
            Rect::from_min_max(pos2(h.right() - arrow, h.top()), h.max),
            Dir::Right,
            St::default(),
        ));
        p.extend(s.scroll_thumb(
            k,
            Rect::from_min_size(h.min + vec2(arrow + 40.0 * u, 0.0), vec2(50.0 * u, sb)),
            false,
            St::default(),
        ));
    }

    #[test]
    fn a_scroll_arrow_cell_is_whole_pixels_at_every_scale() {
        for ppp in [1.0, 1.25, 1.5, 2.0] {
            for dir in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
                for r in scroll_cells(Pos2::ZERO, dir, &SCROLL_ARROW_ACTIVE, ppp) {
                    for v in [r.min.x, r.min.y, r.max.x, r.max.y] {
                        assert!(
                            (v * ppp - (v * ppp).round()).abs() < 1e-3,
                            "{ppp} {dir:?} {r:?}"
                        );
                    }
                    assert!(r.width() > 0.0 && r.height() > 0.0, "{ppp} {dir:?} {r:?}");
                }
            }
        }
    }
}
