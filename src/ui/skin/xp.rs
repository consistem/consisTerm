//! Windows XP's Luna, after XP.css (botoxparty/XP.css, MIT).
//!
//! XP.css is 98.css underneath (`gui/*.scss`) with a theme laid over it
//! (`themes/XP/*.scss`); what is reproduced here is the cascade's result,
//! and the selector or variable each value comes from is named beside it.
//!
//! Two kinds of drawing. Every icon - the title-bar tiles, the scroll
//! arrows, the drop-down button, the slider's pointer, the check and the
//! radio dot - is pixel art in the library, and is drawn from `bitmaps`,
//! transcribed from the SVGs. Everything else is a CSS box: rounded
//! corners, a one-pixel border, a gradient, a stack of inset shadows. Those
//! go through `raster`, which paints them the way a browser does - one
//! device pixel at a time, the corners anti-aliased by how much of each
//! pixel the curve covers - because egui's rounded rects can neither take a
//! hard gradient stop nor clip an inset shadow to a curve, and an XP button
//! is both.
//!
//! The dark reading is not XP's - XP had none. It is Luna at night: the
//! painters draw the light reading, and on the way to the screen its
//! neutrals - the cream, the white, the greys - are taken down in the order
//! they were in, so a button is still lit from the top, while the blues of
//! the title bar and the orange of a hovered control stay as they were (see
//! `tuned`).

mod bitmaps;

use egui::epaint::Mesh;
use egui::{pos2, vec2, Color32, Pos2, Rect, Shape};

use super::prim::{self, colour_at, hex, rgba, Inset, Stop};
use super::{Colors, Dir, Face, Fonts, Glyph, Kit, Metrics, St, Style};
use bitmaps::Bitmap;

pub struct Xp;

/// `--surface`: a dialog, a window's body.
const SURFACE: Color32 = hex(0xece9d8);
/// `--dialog-blue` as the XP theme sets it: selected text, a focused select.
const DIALOG_BLUE: Color32 = hex(0x2267cb);
/// `button { border: 1px solid #003c74 }`.
const BUTTON_BORDER: Color32 = hex(0x003c74);
/// `--radio-border`: a checkbox's and a radio's edge.
const CHECK_BORDER: Color32 = hex(0x1d5281);
/// `--radio-border-disabled`.
const CHECK_BORDER_DISABLED: Color32 = hex(0xcac8bb);
/// `select`, `ul.tree-view` and `input[type=text]`'s border.
const FIELD_BORDER: Color32 = hex(0x7f9db9);
const WHITE: Color32 = hex(0xffffff);

/// `button`'s background: white, fading to the face in the last seventh.
const BUTTON: [Stop; 3] = [
    (0.0, hex(0xffffff)),
    (0.86, hex(0xecebe5)),
    (1.0, hex(0xd8d0c4)),
];
/// `button:not(:disabled):active`: the same turned over, dark at the top.
const BUTTON_PRESSED: [Stop; 4] = [
    (0.0, hex(0xcdcac3)),
    (0.08, hex(0xe3e3db)),
    (0.94, hex(0xe5e5de)),
    (1.0, hex(0xf2f2f1)),
];
/// `button:not(:disabled):hover`: the orange glow inside the border.
const BUTTON_HOVER: [Inset; 4] = [
    (-1, 1, hex(0xfff0cf)),
    (1, 2, hex(0xfdd889)),
    (-2, 2, hex(0xfbc761)),
    (2, -2, hex(0xe5a01a)),
];
/// `button:focus`: the blue one. XP drew the default button the same way.
const BUTTON_FOCUS: [Inset; 5] = [
    (-1, 1, hex(0xcee7ff)),
    (1, 2, hex(0x98b8ea)),
    (-2, 2, hex(0xbcd4f6)),
    (1, -1, hex(0x89ade4)),
    (2, -2, hex(0x89ade4)),
];
/// `--radio-bg`, at 135deg.
const CHECK: [Stop; 2] = [(0.0, hex(0xdcdcd7)), (1.0, hex(0xffffff))];
/// `--radio-bg-active`.
const CHECK_PRESSED: [Stop; 2] = [(0.0, hex(0xb0b0a7)), (1.0, hex(0xe3e1d2))];
/// `--radio-inner-shadow-hover`.
const CHECK_HOVER: [Inset; 2] = [(-2, -2, hex(0xf8b636)), (2, 2, hex(0xfedf9c))];

/// `.window`'s three rings: blue, lit from the top left.
const WINDOW_RINGS: [Inset; 6] = [
    (-1, -1, hex(0x00138c)),
    (1, 1, hex(0x0831d9)),
    (-2, -2, hex(0x001ea0)),
    (2, 2, hex(0x166aee)),
    (-3, -3, hex(0x003bda)),
    (3, 3, hex(0x0855dd)),
];
/// `.title-bar`'s background.
const TITLE: [Stop; 8] = [
    (0.0, hex(0x0997ff)),
    (0.08, hex(0x0053ee)),
    (0.40, hex(0x0050ee)),
    (0.88, hex(0x0066ff)),
    (0.93, hex(0x0066ff)),
    (0.95, hex(0x005bff)),
    (0.96, hex(0x003dd7)),
    (1.0, hex(0x003dd7)),
];
/// What Luna washed an inactive title bar toward. XP.css has no inactive
/// window at all.
const INACTIVE_WASH: Color32 = hex(0xa6b8e8);

/// `menu[role=tablist] button`.
const TAB: [Stop; 4] = [
    (0.0, hex(0xffffff)),
    (0.26, hex(0xfafaf9)),
    (0.95, hex(0xf0f0ea)),
    (1.0, hex(0xecebe5)),
];
/// `--tab-bg`, the panel under the tabs.
const TAB_PANEL: [Stop; 2] = [(0.0, hex(0xfcfcfe)), (1.0, hex(0xf4f3ee))];
/// The orange line along a selected or hovered tab's top, `#e68b2c` over
/// `inset 0 2px #ffc73c`.
const TAB_LINE: Color32 = hex(0xe68b2c);
const TAB_GLOW: Color32 = hex(0xffc73c);

/// `progress`'s `--determinate-track`, top to bottom.
const PROGRESS: [Stop; 8] = [
    (0.0, hex(0xacedad)),
    (0.14, hex(0x7be47d)),
    (0.28, hex(0x4cda50)),
    (0.42, hex(0x2ed330)),
    (0.57, hex(0x42d845)),
    (0.71, hex(0x76e275)),
    (0.85, hex(0x8fe791)),
    (1.0, hex(0xffffff)),
];

/// `::-webkit-scrollbar-thumb`.
const THUMB: Color32 = hex(0xc8d6fb);
const THUMB_SHADOW: [Inset; 2] = [(-3, 0, hex(0xbad1fc)), (1, 1, hex(0xb7caf5))];

/// What Windows XP itself drew where XP.css has nothing: menus are white in
/// the grey edge of the Luna theme.
const MENU_EDGE: Color32 = hex(0xaca899);

/// The app's own title-bar marks, in the hand of XP's: three pixels thick
/// where the minimize bar is, white on a blue tile.
fn own_glyph(glyph: Glyph) -> &'static [&'static str] {
    match glyph {
        Glyph::Plus => &[
            "...###...",
            "...###...",
            "...###...",
            "#########",
            "#########",
            "#########",
            "...###...",
            "...###...",
            "...###...",
        ],
        Glyph::Gear => &[
            "....###....",
            ".##.###.##.",
            ".#########.",
            "..##...##..",
            "###.....###",
            "###.....###",
            "###.....###",
            "..##...##..",
            ".#########.",
            ".##.###.##.",
            "....###....",
        ],
        Glyph::Pin => &[
            "..#####..",
            "..#...#..",
            "..#...#..",
            "..#...#..",
            ".##...##.",
            "#########",
            "....#....",
            "....#....",
            "....#....",
            "....#....",
        ],
        _ => &[
            "..#####..",
            "..#####..",
            "..#####..",
            "..#####..",
            ".#######.",
            "#########",
            "....#....",
            "....#....",
            "....#....",
            "....#....",
        ],
    }
}

/// The chevrons of `icon/scroll-arrow-*.svg`, alone, for an arrow drawn on
/// something other than a scroll button.
fn chevron(dir: Dir) -> &'static [&'static str] {
    match dir {
        Dir::Up => &[
            "....#....",
            "...###...",
            "..#####..",
            ".###.###.",
            "###...###",
            ".#.....#.",
        ],
        Dir::Down => &[
            ".#.....#.",
            "###...###",
            ".###.###.",
            "..#####..",
            "...###...",
            "....#....",
        ],
        Dir::Left => &[
            "....#.", "...###", "..###.", ".###..", "###...", ".###..", "..###.", "...###",
            "....#.",
        ],
        Dir::Right => &[
            ".#....", "###...", ".###..", "..###.", "...###", "..###.", ".###..", "###...",
            ".#....",
        ],
    }
}
/// The chevrons' ink.
const CHEVRON: Color32 = hex(0x4d6185);

// ---------------------------------------------------------------------------
// Painting a CSS box.

/// How the inside of a box is painted, over its padding box as CSS lays a
/// background out.
#[derive(Clone, Copy)]
enum Fill<'a> {
    Flat(Color32),
    /// `linear-gradient(180deg, ...)`.
    Down(&'a [Stop]),
    /// `linear-gradient(135deg, ...)`, top left to bottom right.
    Diagonal(&'a [Stop]),
}

/// One CSS box: what `raster` paints.
#[derive(Clone, Copy)]
struct Boxed<'a> {
    /// `border-radius`, CSS pixels: top left, top right, bottom right,
    /// bottom left.
    radius: [f32; 4],
    /// A one-pixel border on each side - top, right, bottom, left - or none.
    border: [Option<Color32>; 4],
    fill: Fill<'a>,
    /// Zero-blur inset shadows over the padding box, the first listed on top.
    shadows: &'a [Inset],
}

impl<'a> Boxed<'a> {
    fn new(radius: f32, border: Option<Color32>, fill: Fill<'a>) -> Self {
        Boxed {
            radius: [radius; 4],
            border: [border; 4],
            fill,
            shadows: &[],
        }
    }

    fn shadows(mut self, shadows: &'a [Inset]) -> Self {
        self.shadows = shadows;
        self
    }
}

/// Whether `(x, y)` is inside a `w` by `h` box with corners of `r`.
fn inside(x: f32, y: f32, w: f32, h: f32, r: [f32; 4]) -> bool {
    if x < 0.0 || y < 0.0 || x > w || y > h {
        return false;
    }
    let round = |cx: f32, cy: f32, rr: f32| (x - cx).powi(2) + (y - cy).powi(2) <= rr * rr;
    if x < r[0] && y < r[0] {
        return round(r[0], r[0], r[0]);
    }
    if x > w - r[1] && y < r[1] {
        return round(w - r[1], r[1], r[1]);
    }
    if x > w - r[2] && y > h - r[2] {
        return round(w - r[2], h - r[2], r[2]);
    }
    if x < r[3] && y > h - r[3] {
        return round(r[3], h - r[3], r[3]);
    }
    true
}

/// How much of the pixel whose top-left corner is `(x, y)` the box covers:
/// sixteen samples where a corner's curve passes, whole elsewhere.
fn coverage(x: f32, y: f32, w: f32, h: f32, r: [f32; 4]) -> f32 {
    if x + 1.0 <= 0.0 || y + 1.0 <= 0.0 || x >= w || y >= h {
        return 0.0;
    }
    let near = |left: bool, top: bool, rr: f32| {
        rr > 0.0
            && (if left { x < rr } else { x + 1.0 > w - rr })
            && (if top { y < rr } else { y + 1.0 > h - rr })
    };
    let edge = x < 0.0 || y < 0.0 || x + 1.0 > w || y + 1.0 > h;
    let curved = near(true, true, r[0])
        || near(false, true, r[1])
        || near(false, false, r[2])
        || near(true, false, r[3]);
    if !edge && !curved {
        return 1.0;
    }
    let mut n = 0;
    for i in 0..4 {
        for j in 0..4 {
            let sx = x + (i as f32 + 0.5) / 4.0;
            let sy = y + (j as f32 + 0.5) / 4.0;
            if inside(sx, sy, w, h, r) {
                n += 1;
            }
        }
    }
    n as f32 / 16.0
}

fn add(a: Color32, b: Color32) -> Color32 {
    Color32::from_rgba_premultiplied(
        a.r().saturating_add(b.r()),
        a.g().saturating_add(b.g()),
        a.b().saturating_add(b.b()),
        a.a().saturating_add(b.a()),
    )
}

/// A colour from `0xAARRGGBB`.
fn argb(c: u32) -> Color32 {
    Color32::from_rgba_unmultiplied((c >> 16) as u8, (c >> 8) as u8, c as u8, (c >> 24) as u8)
}

fn quad(mesh: &mut Mesh, r: Rect, c: Color32) {
    let base = mesh.vertices.len() as u32;
    mesh.colored_vertex(r.left_top(), c);
    mesh.colored_vertex(r.right_top(), c);
    mesh.colored_vertex(r.left_bottom(), c);
    mesh.colored_vertex(r.right_bottom(), c);
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 1, base + 3, base + 2);
}

/// One row of a raster: runs of one colour, in device pixels.
type Runs = Vec<(f32, f32, Color32)>;

/// Paints `b` over `rect` one device pixel at a time, as a browser
/// rasterises it: each pixel's colour is the border's or the background's
/// (or the shadow over it) where its centre falls, weighted by how much of
/// the pixel the rounded outline covers. Runs of one colour along a row are
/// one quad, and a row like the one above it stretches that row's quads, so
/// a tall flat window is a handful of quads rather than one per row.
fn raster(k: &Kit, rect: Rect, b: &Boxed) -> Shape {
    let ppp = k.ppp;
    let n = ppp.round().max(1.0);
    let (x0, y0) = ((rect.min.x * ppp).round(), (rect.min.y * ppp).round());
    let (w, h) = (
        (rect.max.x * ppp).round() - x0,
        (rect.max.y * ppp).round() - y0,
    );
    if w <= 0.0 || h <= 0.0 {
        return Shape::Noop;
    }
    let r = b.radius.map(|v| (v * n).min(w / 2.0).min(h / 2.0));
    let bw = b.border.map(|c| if c.is_some() { n } else { 0.0 });
    let (pl, pt) = (bw[3], bw[0]);
    let (pw, ph) = (w - bw[1] - bw[3], h - bw[0] - bw[2]);
    // CSS's inner radius: the outer one less the border beside it.
    let ir = [
        (r[0] - bw[0].max(bw[3])).max(0.0),
        (r[1] - bw[0].max(bw[1])).max(0.0),
        (r[2] - bw[2].max(bw[1])).max(0.0),
        (r[3] - bw[2].max(bw[3])).max(0.0),
    ];
    let bordered = bw.iter().any(|&v| v > 0.0);
    let (cw, ch) = (pw / n, ph / n);
    let fill_at = |x: f32, y: f32| -> Color32 {
        match b.fill {
            Fill::Flat(c) => c,
            Fill::Down(stops) => colour_at(stops, (y / ch.max(1e-3)).clamp(0.0, 1.0)),
            Fill::Diagonal(stops) => {
                // The 135deg gradient line runs corner to corner, as long as
                // the box's projection onto it.
                let s = std::f32::consts::FRAC_1_SQRT_2;
                let len = (cw + ch) * s;
                let t = 0.5 + ((x - cw / 2.0) + (y - ch / 2.0)) * s / len.max(1e-3);
                colour_at(stops, t.clamp(0.0, 1.0))
            }
        }
    };
    let pixel = |i: f32, j: f32| -> Color32 {
        let outer = coverage(i, j, w, h, r);
        if outer <= 0.0 {
            return Color32::TRANSPARENT;
        }
        let within = if bordered {
            coverage(i - pl, j - pt, pw, ph, ir)
        } else {
            outer
        };
        let mut c = Color32::TRANSPARENT;
        if within > 0.0 {
            let (x, y) = (i + 0.5 - pl, j + 0.5 - pt);
            let shadow = b.shadows.iter().find(|&&(dx, dy, _)| {
                !inside(x - f32::from(dx) * n, y - f32::from(dy) * n, pw, ph, ir)
            });
            let inner = match shadow {
                Some(&(_, _, s)) => s,
                None => fill_at(x / n, y / n),
            };
            c = inner.gamma_multiply(within);
        }
        if outer > within {
            // Which side's border: the band the centre is in, the nearest
            // where two meet.
            let (cx, cy) = (i + 0.5, j + 0.5);
            let d = [cy, w - cx, h - cy, cx];
            let side = (0..4)
                .filter(|&s| bw[s] > 0.0)
                .min_by(|&a, &z| (d[a] / bw[a]).total_cmp(&(d[z] / bw[z])));
            if let Some(ink) = side.and_then(|s| b.border[s]) {
                c = add(c, ink.gamma_multiply(outer - within));
            }
        }
        c
    };
    let max_shadow = b
        .shadows
        .iter()
        .map(|&(dx, dy, _)| dx.unsigned_abs().max(dy.unsigned_abs()))
        .max()
        .unwrap_or(0);
    let rmax = r.iter().fold(0.0_f32, |a, &v| a.max(v));
    // Past this far in from either side a row is one colour - unless the
    // gradient runs across it.
    let edge = (rmax + (f32::from(max_shadow) + 1.0) * n + 1.0).ceil();
    let diagonal = matches!(b.fill, Fill::Diagonal(_));
    let mut mesh = Mesh::default();
    let flush = |mesh: &mut Mesh, runs: &Runs, from: f32, to: f32| {
        for &(a, z, c) in runs {
            if c.a() > 0 {
                let min = pos2((x0 + a) / ppp, (y0 + from) / ppp);
                let max = pos2((x0 + z) / ppp, (y0 + to) / ppp);
                quad(mesh, Rect::from_min_max(min, max), c);
            }
        }
    };
    let mut held: Runs = Vec::new();
    let mut held_from = 0.0;
    let mut j = 0.0;
    while j < h {
        let mut runs: Runs = Vec::new();
        let mut push = |a: f32, z: f32, c: Color32| match runs.last_mut() {
            Some(last) if last.2 == c && last.1 == a => last.1 = z,
            _ => runs.push((a, z, c)),
        };
        let mut i = 0.0;
        while i < w {
            if !diagonal && i == edge && w - edge > edge {
                push(i, w - edge, pixel((w / 2.0).floor(), j));
                i = w - edge;
                continue;
            }
            push(i, i + 1.0, pixel(i, j));
            i += 1.0;
        }
        if runs != held {
            flush(&mut mesh, &held, held_from, j);
            held = runs;
            held_from = j;
        }
        j += 1.0;
    }
    flush(&mut mesh, &held, held_from, h);
    Shape::mesh(mesh)
}

/// An icon, one CSS pixel to an entry, its top left at `at`.
fn bitmap(k: &Kit, at: Pos2, b: &Bitmap, alpha: f32) -> Shape {
    let u = k.u();
    let at = pos2(prim::snap(at.x, k.ppp), prim::snap(at.y, k.ppp));
    let mut mesh = Mesh::default();
    for y in 0..b.h {
        let row = &b.px[y * b.w..(y + 1) * b.w];
        let mut x = 0;
        while x < b.w {
            let mut end = x + 1;
            while end < b.w && row[end] == row[x] {
                end += 1;
            }
            if row[x] != 0 {
                let r = Rect::from_min_size(
                    pos2(at.x + x as f32 * u, at.y + y as f32 * u),
                    vec2((end - x) as f32 * u, u),
                );
                quad(&mut mesh, r, argb(row[x]).gamma_multiply(alpha));
            }
            x = end;
        }
    }
    Shape::mesh(mesh)
}

/// An icon centred in `rect`, on the pixel grid.
fn centred(k: &Kit, rect: Rect, b: &Bitmap, alpha: f32) -> Shape {
    let size = vec2(b.w as f32, b.h as f32) * k.u();
    bitmap(k, rect.center() - size / 2.0, b, alpha)
}

/// A one-pixel strip of an icon stretched over `rect` - the scroll track's
/// background, `repeat`ed along the bar.
fn stretched(k: &Kit, rect: Rect, b: &Bitmap, vertical: bool) -> Shape {
    let u = k.u();
    let r = prim::snap_rect(rect, k.ppp);
    let mut mesh = Mesh::default();
    let count = if vertical { b.w } else { b.h };
    for i in 0..count {
        let last = i + 1 == count;
        let part = if vertical {
            let a = r.left() + i as f32 * u;
            let z = if last {
                r.right()
            } else {
                (a + u).min(r.right())
            };
            Rect::from_min_max(pos2(a, r.top()), pos2(z, r.bottom()))
        } else {
            let a = r.top() + i as f32 * u;
            let z = if last {
                r.bottom()
            } else {
                (a + u).min(r.bottom())
            };
            Rect::from_min_max(pos2(r.left(), a), pos2(r.right(), z))
        };
        if part.is_positive() {
            quad(&mut mesh, part, argb(b.px[i]));
        }
    }
    Shape::mesh(mesh)
}

/// `rect` grown by whole CSS pixels: top, right, bottom, left.
fn grow(k: &Kit, rect: Rect, t: f32, r: f32, b: f32, l: f32) -> Rect {
    let u = k.u();
    let s = prim::snap_rect(rect, k.ppp);
    Rect::from_min_max(
        pos2(s.left() - l * u, s.top() - t * u),
        pos2(s.right() + r * u, s.bottom() + b * u),
    )
}

impl Style for Xp {
    fn colors(&self, dark: bool) -> Colors {
        if dark {
            let night = super::tuned::darken;
            return Colors {
                face: night(SURFACE),
                text: hex(0xe8e8e8),
                text_disabled: hex(0x8c8c8c),
                text_disabled_shadow: None,
                field: night(WHITE),
                field_text: hex(0xf0f0f0),
                select: DIALOG_BLUE,
                select_text: WHITE,
                title_text: WHITE,
                title_text_inactive: hex(0xd8e4f8),
                title_shadow: Some(hex(0x0f1089)),
                link: hex(0x7fb2ff),
                tooltip_text: hex(0x000000),
                subtle: hex(0xb4b4b4),
            };
        }
        Colors {
            face: SURFACE,
            // `body { color: #222222 }`, which labels inherit.
            text: hex(0x222222),
            // `:disabled { color: var(--button-shadow); text-shadow: 1px
            // 1px 0 var(--button-highlight) }` - 98's emboss, kept by XP.css.
            text_disabled: hex(0x808080),
            text_disabled_shadow: Some(WHITE),
            field: WHITE,
            field_text: hex(0x000000),
            select: DIALOG_BLUE,
            select_text: WHITE,
            title_text: WHITE,
            title_text_inactive: hex(0xd8e4f8),
            // `.title-bar { text-shadow: 1px 1px #0f1089 }`.
            title_shadow: Some(hex(0x0f1089)),
            // `--link-blue`.
            link: hex(0x0000ff),
            tooltip_text: hex(0x000000),
            subtle: hex(0x5a5a5a),
        }
    }

    fn metrics(&self) -> Metrics {
        Metrics {
            font_size: 11.0,
            title_font_size: 13.0,
            // `.title-bar { height: 28px }` starts at the window's top edge,
            // over its rings; laid out inside the 3 px of them it is the 25
            // left, and `title_bar` paints up over the rings.
            title_height: 25.0,
            title_button: vec2(21.0, 21.0),
            // `.title-bar-controls button { margin-left: 2px }`.
            title_button_gap: 2.0,
            close_gap: 2.0,
            window_border: 3.0,
            window_radius: 8.0,
            // `::-webkit-scrollbar { width: 17px }`, and 17 px buttons.
            scrollbar: 17.0,
            scroll_arrow: 17.0,
            // `--checkbox-width` 13 px plus the 1 px `--radio-border` round
            // it, which XP.css adds outside: 15 on the page.
            checkbox: 15.0,
            radio: 15.0,
            button_min: vec2(75.0, 23.0),
            button_pad_x: 12.0,
            // `input[type=text], select { height: 23px }`.
            field_height: 23.0,
            tab_height: 22.0,
            // The 15 px drop-down image, 2 px in from the 1 px border.
            select_arrow: 18.0,
            menu_item_height: 20.0,
            slider_thumb: vec2(11.0, 21.0),
            slider_track: 2.0,
            label_gap: 6.0,
        }
    }

    fn fonts(&self) -> Fonts {
        Fonts {
            // `--sans-serif: "Pixelated MS Sans Serif"`, the same face 98.css
            // ships: every control, label and field is set in it.
            body: Some(Face::Bundled(include_bytes!(
                "../../../assets/fonts/ms-sans-serif/MS Sans Serif.ttf"
            ))),
            bold: Some(Face::Bundled(include_bytes!(
                "../../../assets/fonts/ms-sans-serif-bold/MS Sans Serif Bold.ttf"
            ))),
            // `.title-bar { font-family: "Trebuchet MS" }`, bold from
            // `.title-bar-text`. Windows' own, so looked for, not shipped.
            title: Some(Face::System(&["Trebuchet MS", "Tahoma", "Verdana"])),
        }
    }

    fn button_text(&self, k: &Kit, st: St) -> Color32 {
        // `button` keeps the browser's ButtonText, black, not the body's
        // #222.
        if st.disabled {
            k.colors().text_disabled
        } else {
            hex(0x000000)
        }
    }

    fn button(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let live = !st.disabled;
        let fill = if live && st.pressed {
            &BUTTON_PRESSED[..]
        } else {
            &BUTTON[..]
        };
        // `:not(:disabled):hover` outranks `:focus`, and both outrank
        // `:active`'s `box-shadow: none`.
        let shadows: &[Inset] = if live && st.hovered {
            &BUTTON_HOVER
        } else if st.focused || (st.default && live) {
            &BUTTON_FOCUS
        } else {
            &[]
        };
        let b = Boxed::new(3.0, Some(BUTTON_BORDER), Fill::Down(fill)).shadows(shadows);
        let mut out = vec![raster(k, rect, &b)];
        if st.focused {
            // 98.css's `outline: 1px dotted #000; outline-offset: -4px`,
            // which the XP theme never takes away.
            out.extend(prim::dotted(
                prim::inner(rect, 4.0, k.ppp),
                hex(0x000000),
                k.ppp,
            ));
        }
        out
    }

    fn field(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `input[type=text] { border: solid #7f9db9 1px }`: flat, no bevel,
        // and nothing changes on focus or when disabled.
        let _ = st;
        vec![raster(
            k,
            rect,
            &Boxed::new(0.0, Some(FIELD_BORDER), Fill::Flat(WHITE)),
        )]
    }

    fn checkbox(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let live = !st.disabled;
        let r = prim::snap_rect(rect, k.ppp);
        let b = if live {
            let fill = if st.pressed { &CHECK_PRESSED } else { &CHECK };
            let hover: &[Inset] = if st.hovered && !st.pressed {
                &CHECK_HOVER
            } else {
                &[]
            };
            Boxed::new(0.0, Some(CHECK_BORDER), Fill::Diagonal(fill)).shadows(hover)
        } else {
            Boxed::new(0.0, Some(CHECK_BORDER_DISABLED), Fill::Flat(WHITE))
        };
        let mut out = vec![raster(k, r, &b)];
        if st.on {
            // `::after { left: -17px }` against 98.css's `top: 1px`: the
            // 11 px check lands one pixel in and two down.
            let mark = if live {
                &bitmaps::CHECKMARK
            } else {
                &bitmaps::CHECKMARK_DISABLED
            };
            out.push(bitmap(k, r.min + vec2(1.0, 2.0) * k.u(), mark, 1.0));
        }
        out
    }

    fn radio(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let live = !st.disabled;
        let r = prim::snap_rect(rect, k.ppp);
        let round = r.width().min(r.height()) / k.u() / 2.0;
        let b = if live {
            let fill = if st.pressed { &CHECK_PRESSED } else { &CHECK };
            let hover: &[Inset] = if st.hovered && !st.pressed {
                &CHECK_HOVER
            } else {
                &[]
            };
            Boxed::new(round, Some(CHECK_BORDER), Fill::Diagonal(fill)).shadows(hover)
        } else {
            Boxed::new(round, Some(CHECK_BORDER_DISABLED), Fill::Flat(WHITE))
        };
        let mut out = vec![raster(k, r, &b)];
        if st.on {
            let dot = if live {
                &bitmaps::RADIO_DOT
            } else {
                &bitmaps::RADIO_DOT_DISABLED
            };
            out.push(centred(k, r, dot, 1.0));
        }
        out
    }

    fn select(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let live = !st.disabled;
        let mut out = Vec::new();
        if st.focused && live {
            // 98.css's `select:focus` fills it with `--dialog-blue`; XP's
            // keeps two pixels of white inside the border round it.
            const RING: [Inset; 2] = [(2, 2, WHITE), (-2, -2, WHITE)];
            let b = Boxed::new(0.0, Some(FIELD_BORDER), Fill::Flat(DIALOG_BLUE)).shadows(&RING);
            out.push(raster(k, r, &b));
        } else {
            out.extend(self.field(k, r, st));
        }
        // `background-position: top 2px right 2px` inside the border.
        let image = if !live {
            &bitmaps::DROPDOWN
        } else if st.pressed {
            &bitmaps::DROPDOWN_ACTIVE
        } else if st.hovered {
            &bitmaps::DROPDOWN_HOVER
        } else {
            &bitmaps::DROPDOWN
        };
        let at = pos2(r.right() - (3.0 + image.w as f32) * u, r.top() + 3.0 * u);
        out.push(bitmap(k, at, image, if live { 1.0 } else { 0.5 }));
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
        let rows = chevron(dir);
        let ink = if st.disabled { MENU_EDGE } else { CHEVRON };
        prim::sprite(prim::centred(rect, rows, k.ppp), rows, &[('#', ink)], k.ppp)
    }

    fn slider_track(&self, k: &Kit, rect: Rect, _st: St) -> Vec<Shape> {
        // `::-webkit-slider-runnable-track`: 2 px of #ecebe4 with a #f3f2ea
        // right and bottom border, ringed by box-shadows - #9d9c99 above and
        // to the left, white below and to the right.
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let top = prim::snap(r.center().y - u, k.ppp);
        let track = Rect::from_min_max(pos2(r.left(), top), pos2(r.right(), top + 2.0 * u));
        let grey = hex(0x9d9c99);
        let span = |x0: f32, y0: f32, x1: f32, y1: f32, c| {
            prim::fill(Rect::from_min_max(pos2(x0, y0), pos2(x1, y1)), c)
        };
        let (l, t, rr, b) = (track.left(), track.top(), track.right(), track.bottom());
        vec![
            span(l - u, t - u, rr + u, t, grey),
            span(l - u, t, l, b, grey),
            span(rr, t, rr + u, b, WHITE),
            span(l - u, b, rr + u, b + u, WHITE),
            prim::fill(track, hex(0xf3f2ea)),
            span(l, t, rr - u, t + u, hex(0xecebe4)),
        ]
    }

    fn slider_thumb(&self, k: &Kit, centre: Pos2, st: St) -> Vec<Shape> {
        // `icon/indicator-horizontal.svg`, `translateY(-8px)` from the
        // track's top: it hangs a pixel and a half below the middle.
        let u = k.u();
        let top = prim::snap(centre.y - u, k.ppp) - 8.0 * u;
        let left = prim::snap(centre.x - 5.5 * u, k.ppp);
        let alpha = if st.disabled { 0.5 } else { 1.0 };
        vec![bitmap(
            k,
            pos2(left, top),
            &bitmaps::INDICATOR_HORIZONTAL,
            alpha,
        )]
    }

    fn tab(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let mut out = Vec::new();
        if st.on {
            // `[aria-selected=true]`: flat #fcfcfe, #919b9c sides, no bottom
            // border, the orange top; 2 px taller and down over the panel's
            // edge, as 98.css raises it.
            let r = grow(k, rect, 2.0, 0.0, 1.0, 0.0);
            let side = Some(hex(0x919b9c));
            let b = Boxed {
                radius: [3.0, 3.0, 0.0, 0.0],
                border: [Some(TAB_LINE), side, None, side],
                fill: Fill::Flat(hex(0xfcfcfe)),
                shadows: &[(0, 2, TAB_GLOW)],
            };
            out.push(raster(k, r, &b));
        } else {
            let lit = st.hovered && !st.disabled;
            let edge = Some(hex(0x91a7b4));
            let b = Boxed {
                radius: [3.0, 3.0, 0.0, 0.0],
                border: [if lit { Some(TAB_LINE) } else { edge }, edge, edge, edge],
                fill: Fill::Down(&TAB),
                shadows: if lit { &[(0, 2, TAB_GLOW)] } else { &[] },
            };
            out.push(raster(k, rect, &b));
        }
        if st.focused {
            out.extend(prim::dotted(
                prim::inner(rect, 4.0, k.ppp),
                hex(0x222222),
                k.ppp,
            ));
        }
        out
    }

    fn tab_panel(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `[role=tabpanel]`: the pale gradient in a #919b9c border, an inner
        // ring of #fcfcfe, and `1px 2px 2px rgba(208, 206, 191, .75)` - a
        // soft shadow to the right and below, drawn as the two rows of it a
        // browser's blur leaves visible.
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        const RING: [Inset; 2] = [(1, 1, hex(0xfcfcfe)), (-1, -1, hex(0xfcfcfe))];
        let mut out = Vec::new();
        for (i, a) in [(1.0, 0.45), (2.0, 0.2)] {
            let shade = rgba(208, 206, 191, 0.75 * a);
            let below = Rect::from_min_max(
                pos2(r.left() + (i + 1.0) * u, r.bottom() + (i - 1.0) * u),
                pos2(r.right() + i * u, r.bottom() + i * u),
            );
            let beside = Rect::from_min_max(
                pos2(r.right() + (i - 1.0) * u, r.top() + (i + 1.0) * u),
                pos2(r.right() + i * u, r.bottom() + (i - 1.0) * u),
            );
            out.push(prim::fill(below, shade));
            out.push(prim::fill(beside, shade));
        }
        let b = Boxed::new(0.0, Some(hex(0x919b9c)), Fill::Down(&TAB_PANEL)).shadows(&RING);
        out.push(raster(k, r, &b));
        out
    }

    fn group(&self, k: &Kit, rect: Rect, legend: Option<Rect>) -> Vec<Shape> {
        // `fieldset`: white, in a 4 px-rounded #d0d0bf line. The legend is
        // `color: #0046d5` - text, which the caller sets - on no background
        // of its own: the line simply stops either side of it.
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![raster(
            k,
            r,
            &Boxed::new(4.0, Some(hex(0xd0d0bf)), Fill::Flat(WHITE)),
        )];
        if let Some(legend) = legend {
            let u = k.u();
            let gap = prim::snap_rect(legend.expand2(vec2(2.0 * u, 0.0)), k.ppp);
            let cut = Rect::from_min_max(pos2(gap.left(), r.top()), pos2(gap.right(), r.top() + u));
            out.push(prim::fill(cut, SURFACE));
        }
        out
    }

    fn window(&self, k: &Kit, rect: Rect, _st: St) -> Vec<Shape> {
        // `.window`: the three blue rings over the surface, the top corners
        // rounded 8 px.
        let b = Boxed {
            radius: [8.0, 8.0, 0.0, 0.0],
            border: [None; 4],
            fill: Fill::Flat(SURFACE),
            shadows: &WINDOW_RINGS,
        };
        vec![raster(k, rect, &b)]
    }

    fn title_bar(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `.title-bar` begins at the window's own edge, over its rings:
        // painted up and out by the window's border to stand where the CSS
        // puts it. Its own border is #0831d9 on top and at the left and
        // #001ea0 at the right; corners 8 px left, 7 px right.
        let m = self.metrics().window_border;
        let r = grow(k, rect, m, m, 0.0, m);
        let wash = |c: Color32| {
            if st.inactive_window {
                c.lerp_to_gamma(INACTIVE_WASH, 0.74)
            } else {
                c
            }
        };
        let stops = TITLE.map(|(at, c)| (at, wash(c)));
        let (lit, dark) = (Some(wash(hex(0x0831d9))), Some(wash(hex(0x001ea0))));
        let bar = Boxed {
            radius: [8.0, 7.0, 0.0, 0.0],
            border: [lit, dark, None, lit],
            fill: Fill::Down(&stops),
            shadows: &[],
        };
        vec![raster(k, r, &bar)]
    }

    fn title_button(&self, k: &Kit, rect: Rect, glyph: Glyph, st: St) -> Vec<Shape> {
        use bitmaps::*;
        let live = !st.disabled;
        let state = if live && st.pressed {
            2
        } else if live && st.hovered {
            1
        } else {
            0
        };
        let own = matches!(
            glyph,
            Glyph::Gear | Glyph::Plus | Glyph::Pin | Glyph::Pinned
        );
        let tiles: [&Bitmap; 3] = match glyph {
            Glyph::Minimize => [&MINIMIZE, &MINIMIZE_HOVER, &MINIMIZE_ACTIVE],
            Glyph::Maximize => [&MAXIMIZE, &MAXIMIZE_HOVER, &MAXIMIZE_ACTIVE],
            Glyph::Restore => [&RESTORE, &RESTORE_HOVER, &RESTORE_ACTIVE],
            Glyph::Help => [&HELP, &HELP_HOVER, &HELP_ACTIVE],
            Glyph::Close => [&CLOSE, &CLOSE_HOVER, &CLOSE_ACTIVE],
            _ => [&BLANK, &BLANK_HOVER, &BLANK_ACTIVE],
        };
        // XP.css has no unfocused window; Luna's tiles faded with its bar.
        let alpha = if st.inactive_window || !live {
            0.6
        } else {
            1.0
        };
        let mut out = vec![centred(k, rect, tiles[state], alpha)];
        if own {
            // The pressed tiles draw their marks in #78a2d8, sunk into the
            // darker blue.
            let ink = if state == 2 {
                hex(0x78a2d8)
            } else {
                k.glyph_ink(WHITE)
            }
            .gamma_multiply(alpha);
            let rows = own_glyph(glyph);
            out.extend(prim::sprite(
                prim::centred(rect, rows, k.ppp),
                rows,
                &[('#', ink)],
                k.ppp,
            ));
        }
        out
    }

    fn scroll_track(&self, k: &Kit, rect: Rect, vertical: bool, _st: St) -> Vec<Shape> {
        // `icon/scroll-background.svg`: one row of 17 shades, repeated down.
        let strip = if vertical {
            &bitmaps::SCROLL_BACKGROUND
        } else {
            &bitmaps::SCROLL_BACKGROUND_HORIZONTAL
        };
        vec![stretched(k, rect, strip, vertical)]
    }

    fn scroll_thumb(&self, k: &Kit, rect: Rect, vertical: bool, _st: St) -> Vec<Shape> {
        // `::-webkit-scrollbar-thumb`: #c8d6fb in a 1 px white border,
        // rounded 2 px, `inset -3px 0 #bad1fc, inset 1px 1px #b7caf5`, and
        // the grip centred on it.
        let b = Boxed::new(2.0, Some(WHITE), Fill::Flat(THUMB)).shadows(&THUMB_SHADOW);
        let grip = if vertical {
            &bitmaps::SCROLL_THUMB
        } else {
            &bitmaps::SCROLL_THUMB_HORIZONTAL
        };
        let mut out = vec![raster(k, rect, &b)];
        let r = prim::snap_rect(rect, k.ppp);
        if r.width() > (grip.w as f32 + 4.0) * k.u() && r.height() > (grip.h as f32 + 4.0) * k.u() {
            out.push(centred(k, r, grip, 1.0));
        }
        out
    }

    fn scroll_arrow(&self, k: &Kit, rect: Rect, dir: Dir, _st: St) -> Vec<Shape> {
        // `icon/scroll-arrow-*.svg`, 17 by 17, over the track they sit on.
        let image = match dir {
            Dir::Up => &bitmaps::SCROLL_ARROW_UP,
            Dir::Down => &bitmaps::SCROLL_ARROW_DOWN,
            Dir::Left => &bitmaps::SCROLL_ARROW_LEFT,
            Dir::Right => &bitmaps::SCROLL_ARROW_RIGHT,
        };
        let vertical = matches!(dir, Dir::Up | Dir::Down);
        let mut out = self.scroll_track(k, rect, vertical, St::default());
        out.push(centred(k, rect, image, 1.0));
        out
    }

    fn scroll_corner(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `::-webkit-scrollbar-corner { background: var(--button-face) }`.
        vec![prim::fill(prim::snap_rect(rect, k.ppp), hex(0xdfdfdf))]
    }

    fn menu(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // Not in XP.css: Luna's menu, white in a grey line.
        vec![raster(
            k,
            rect,
            &Boxed::new(0.0, Some(MENU_EDGE), Fill::Flat(WHITE)),
        )]
    }

    fn menu_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        if (st.hovered || st.on) && !st.disabled {
            vec![prim::fill(prim::snap_rect(rect, k.ppp), DIALOG_BLUE)]
        } else {
            Vec::new()
        }
    }

    fn separator(&self, k: &Kit, rect: Rect, vertical: bool) -> Vec<Shape> {
        // Luna's menu separator: one grey line.
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let line = if vertical {
            let x = prim::snap(r.center().x, k.ppp);
            Rect::from_min_max(pos2(x, r.top()), pos2(x + u, r.bottom()))
        } else {
            let y = prim::snap(r.center().y, k.ppp);
            Rect::from_min_max(pos2(r.left(), y), pos2(r.right(), y + u))
        };
        vec![prim::fill(line, MENU_EDGE)]
    }

    fn tooltip(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // Not in XP.css: XP's own, pale yellow in black.
        vec![raster(
            k,
            rect,
            &Boxed::new(0.0, Some(hex(0x000000)), Fill::Flat(hex(0xffffe1))),
        )]
    }

    fn list(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `ul.tree-view`: white, `border: 1px solid #7f9db9`.
        self.field(k, rect, St::default())
    }

    fn list_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `.tree-view a:focus` fills with `--dialog-blue`; nothing lights
        // under the pointer.
        if st.on {
            vec![prim::fill(prim::snap_rect(rect, k.ppp), DIALOG_BLUE)]
        } else {
            Vec::new()
        }
    }

    fn progress(&self, k: &Kit, rect: Rect, fraction: f32) -> Vec<Shape> {
        // `progress`: white, `1px solid #686868`, rounded 4 px, with a 1 px
        // blurred inner shadow of the same grey - the ring of pale grey a
        // browser's blur leaves inside the border. `padding: 1px 2px 1px 0`.
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        const BLUR: [Inset; 2] = [(1, 1, hex(0xdcdcdc)), (-1, -1, hex(0xdcdcdc))];
        let track = Boxed::new(4.0, Some(hex(0x686868)), Fill::Flat(WHITE)).shadows(&BLUR);
        let mut out = vec![raster(k, r, &track)];
        let inner = Rect::from_min_max(
            pos2(r.left() + u, r.top() + 2.0 * u),
            pos2(r.right() - 3.0 * u, r.bottom() - 2.0 * u),
        );
        let done = prim::snap(inner.width() * fraction.clamp(0.0, 1.0), k.ppp);
        if done >= u {
            // `::-webkit-progress-value`: the green gradient rounded 2 px,
            // cut by white stripes 2 px wide every 10 - the blocks.
            let value = Rect::from_min_size(inner.min, vec2(done, inner.height()));
            out.push(raster(
                k,
                value,
                &Boxed::new(2.0, None, Fill::Down(&PROGRESS)),
            ));
            let mut x = value.left();
            while x < value.right() {
                let z = (x + 2.0 * u).min(value.right());
                out.push(prim::fill(
                    Rect::from_min_max(pos2(x, value.top()), pos2(z, value.bottom())),
                    WHITE,
                ));
                x += 10.0 * u;
            }
        }
        out
    }

    fn status_bar(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `.status-bar { box-shadow: inset 0 1px 2px var(--button-shadow) }`:
        // a grey line along the top, blurred into the next two rows.
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let mut out = vec![prim::fill(r, SURFACE)];
        for (i, a) in [(0.0, 0.75), (1.0, 0.35), (2.0, 0.1)] {
            let row = Rect::from_min_size(pos2(r.left(), r.top() + i * u), vec2(r.width(), u));
            out.push(prim::fill(row, rgba(128, 128, 128, a)));
        }
        out
    }

    fn status_field(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `.status-bar-field`: no box, only a pale groove between fields -
        // translucent white at the left, translucent #d0cebf at the right.
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        vec![
            prim::fill(
                Rect::from_min_max(r.min, pos2(r.left() + u, r.bottom())),
                rgba(255, 255, 255, 0.75),
            ),
            prim::fill(
                Rect::from_min_max(pos2(r.right() - u, r.top()), r.max),
                rgba(208, 206, 191, 0.75),
            ),
        ]
    }

    fn focus(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        prim::dotted(rect, hex(0x000000), k.ppp)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Look, Skin};
    use super::*;

    fn kit(ppp: f32) -> Kit {
        Kit::bare(
            Look {
                skin: Skin::Xp,
                dark: false,
                ..Default::default()
            },
            ppp,
        )
    }

    /// The colour a point ends up, painting `shapes` over `under` - the
    /// quads of meshes and flat rects alike.
    fn pixel(shapes: &[Shape], x: f32, y: f32, under: Color32) -> Color32 {
        let p = pos2(x, y);
        let mut c = under;
        let mut over = |q: Color32| {
            let keep = 1.0 - f32::from(q.a()) / 255.0;
            let mix = |a: u8, b: u8| (f32::from(a) * keep).round() as u8 + b;
            c = Color32::from_rgb(mix(c.r(), q.r()), mix(c.g(), q.g()), mix(c.b(), q.b()));
        };
        for s in shapes {
            match s {
                Shape::Rect(r) if r.rect.contains(p) => over(r.fill),
                Shape::Mesh(m) => {
                    for q in m.vertices.chunks(4) {
                        if Rect::from_min_max(q[0].pos, q[3].pos).contains(p) {
                            over(q[0].color);
                        }
                    }
                }
                _ => {}
            }
        }
        c
    }

    #[test]
    fn every_icon_is_as_many_pixels_as_its_size_says() {
        use bitmaps::*;
        for b in [
            &MINIMIZE,
            &MAXIMIZE_HOVER,
            &RESTORE_ACTIVE,
            &HELP,
            &CLOSE,
            &BLANK,
            &BLANK_HOVER,
            &BLANK_ACTIVE,
            &SCROLL_ARROW_UP,
            &DROPDOWN,
            &INDICATOR_HORIZONTAL,
            &CHECKMARK,
            &RADIO_DOT,
            &SCROLL_THUMB,
            &SCROLL_BACKGROUND,
            &SCROLL_BACKGROUND_HORIZONTAL,
        ] {
            assert_eq!(b.px.len(), b.w * b.h);
        }
        for glyph in [Glyph::Gear, Glyph::Plus, Glyph::Pin, Glyph::Pinned] {
            let rows = own_glyph(glyph);
            assert!(rows.iter().all(|r| r.len() == rows[0].len()), "{glyph:?}");
        }
        for dir in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
            let rows = chevron(dir);
            assert!(rows.iter().all(|r| r.len() == rows[0].len()), "{dir:?}");
        }
    }

    /// `border: 1px solid #003c74` along the straight edges, white just
    /// inside the top, and the corner pixel left to the page by the 3 px
    /// radius - at 100% and at 200%.
    #[test]
    fn a_button_is_edged_in_xp_blue_and_rounded_off_at_the_corners() {
        for ppp in [1.0, 2.0] {
            let k = kit(ppp);
            let half = 0.5 / ppp;
            let shapes = Xp.button(
                &k,
                Rect::from_min_size(Pos2::ZERO, vec2(75.0, 23.0)),
                St::default(),
            );
            let at = |x: f32, y: f32| pixel(&shapes, x + half, y + half, SURFACE);
            assert_eq!(at(37.0, 0.0), BUTTON_BORDER, "{ppp}");
            assert_eq!(at(0.0, 11.0), BUTTON_BORDER, "{ppp}");
            assert_eq!(at(75.0 - 1.0 / ppp, 11.0), BUTTON_BORDER, "{ppp}");
            assert!(at(37.0, 1.0).r() >= 0xfd, "{ppp}");
            assert_eq!(at(0.0, 0.0), SURFACE, "{ppp}");
        }
    }

    /// The pressed button is darker at its top than its foot; the resting
    /// one the other way round.
    #[test]
    fn a_pressed_button_turns_its_gradient_over() {
        let k = kit(1.0);
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(75.0, 23.0));
        let rest = Xp.button(&k, rect, St::default());
        let held = Xp.button(
            &k,
            rect,
            St {
                pressed: true,
                ..St::default()
            },
        );
        let (top, bottom) = (1.5, 21.5);
        assert!(pixel(&rest, 37.5, top, SURFACE).r() > pixel(&rest, 37.5, bottom, SURFACE).r());
        assert!(pixel(&held, 37.5, top, SURFACE).r() < pixel(&held, 37.5, bottom, SURFACE).r());
    }

    #[test]
    fn the_window_wears_three_blue_rings_and_the_title_bar_covers_their_top() {
        let k = kit(1.0);
        let win = Rect::from_min_size(Pos2::ZERO, vec2(300.0, 100.0));
        let shapes = Xp.window(&k, win, St::default());
        let at = |x: f32, y: f32| pixel(&shapes, x + 0.5, y + 0.5, Color32::BLACK);
        assert_eq!(at(0.0, 50.0), hex(0x0831d9));
        assert_eq!(at(2.0, 50.0), hex(0x0855dd));
        assert_eq!(at(299.0, 50.0), hex(0x00138c));
        assert_eq!(at(150.0, 50.0), SURFACE);
        let bar = Xp.title_bar(
            &k,
            Rect::from_min_size(pos2(3.0, 3.0), vec2(294.0, 25.0)),
            St::default(),
        );
        assert_eq!(pixel(&bar, 150.5, 0.5, Color32::BLACK), hex(0x0831d9));
        assert_eq!(pixel(&bar, 150.5, 27.5, Color32::BLACK), hex(0x003dd7));
    }

    #[test]
    fn a_checkbox_is_ringed_in_blue_and_lit_toward_its_bottom_right() {
        let k = kit(1.0);
        let shapes = Xp.checkbox(
            &k,
            Rect::from_min_size(Pos2::ZERO, vec2(15.0, 15.0)),
            St::default(),
        );
        assert_eq!(pixel(&shapes, 7.5, 0.5, SURFACE), CHECK_BORDER);
        let dim = pixel(&shapes, 1.5, 1.5, SURFACE);
        let lit = pixel(&shapes, 13.5, 13.5, SURFACE);
        assert!(dim.r() < lit.r());
    }
}
