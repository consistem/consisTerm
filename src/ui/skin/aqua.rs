//! Aqua, as Mac OS X drew it.
//!
//! After Will Meyers's `aqua-ui` (willmeyers/aqua-ui, MIT), value for value.
//! The window is its `.window.metal`: the brushed tile from QuickTime Player
//! on the 10.0 disc, repeated, inside the rim, sheen and shade of
//! `--aqua-metal-*`, with the title bar and the status bar the sheet showing
//! through, and the window controls its gems. Push buttons, pop-ups,
//! checkboxes, radios, scroll bars, sliders and progress bars are the bitmaps
//! it slices from the system's own `Extras.rsrc`, drawn as they are
//! (`assets/skins/aqua`); the fills its CSS spells out a pixel row at a time -
//! tabs, menu highlights, menus - are those rows.
//!
//! The dark reading, for "Tiger Graphite", is the same sheet taken down to
//! graphite, and the controls' bitmaps with it, so that the light text the
//! dark reading writes on them still reads.

use egui::{pos2, vec2, Color32, Pos2, Rect, Shape, TextureId};

use super::prim::{self, fill, hex, inside_rounded, over, raster, rgba};
use super::{Colors, Dir, Face, Fonts, Glyph, Kit, Metrics, St, Style};

pub struct Aqua;

/// One of the bitmaps, by its `aqua-ui` variable name.
macro_rules! png {
    ($name:literal) => {
        (
            $name,
            include_bytes!(concat!("../../../assets/skins/aqua/", $name, ".png"))
                as &'static [u8],
        )
    };
}

type Png = (&'static str, &'static [u8]);

/// A bitmap's texture, or `None` with no context to keep it in.
fn tex(k: &Kit, png: Png) -> Option<TextureId> {
    k.image(png.0, png.1)
}

/// A bitmap's texture in the colour the user gave its control, if any.
fn tex_to(k: &Kit, png: Png, to: Option<Color32>) -> Option<TextureId> {
    k.image_as(png.0, png.1, to, None)
}

/// [`tex_to`] for a slice of a control drawn in pieces, measured against
/// the lightness of the whole: `--aqua-scroll-thumb`'s blue, the track's grey.
fn slice_to(k: &Kit, png: Png, to: Option<Color32>, reference: u8) -> Option<TextureId> {
    k.image_as(png.0, png.1, to, Some(reference))
}

/// The 2x bitmap when the screen has the pixels for it.
fn sharp(k: &Kit, one: Png, two: Png) -> Option<TextureId> {
    if k.ppp >= 1.5 {
        tex(k, two)
    } else {
        tex(k, one)
    }
}

/// `rect` at `height` CSS pixels, centred on it: the bitmaps are drawn at
/// their own height whatever the control was given.
fn band(k: &Kit, rect: Rect, height: f32) -> Rect {
    let h = height * k.u();
    let top = prim::snap(rect.center().y - h / 2.0, k.ppp);
    Rect::from_min_max(pos2(rect.left(), top), pos2(rect.right(), top + h))
}

// ---------------------------------------------------------------------------
// The fills `aqua-ui` gives a pixel row at a time
// ---------------------------------------------------------------------------

/// `--aqua-btn-normal`: Push Button Fill, frame 0 - what a tab at rest is.
const BTN_NORMAL: [u32; 20] = [
    0x7c7c7c, 0xb4b4b4, 0xd2d2d2, 0xdcdcdc, 0xe1e1e1, 0xe5e5e5, 0xe3e3e3, 0xdbdbdb, 0xd6d6d6,
    0xd9d9d9, 0xdddddd, 0xe5e5e5, 0xeeeeee, 0xf7f7f7, 0xffffff, 0xffffff, 0xffffff, 0xffffff,
    0xffffff, 0xf8f8f8,
];

/// `--aqua-tab-selected`: Foreground Tab Caps, frame 0.
const TAB_SELECTED: [u32; 24] = [
    0x002d91, 0x9dbdde, 0xacc8e4, 0xafc9e4, 0x91b4db, 0x709dcd, 0x7aa4d2, 0x7ea8d3, 0x83abd8,
    0x88afda, 0x8cb3db, 0x90b6de, 0x94bbe1, 0x9bc0e7, 0xa0c4eb, 0xa3c8ef, 0xaacdf4, 0xb2d8ff,
    0xb5dbff, 0xbce3ff, 0xc1e7ff, 0xc6f0ff, 0xcffaff, 0xd3ffff,
];

/// `--aqua-tab-pressed`: frame 1, greys at alpha .49 laid over the tab.
const TAB_PRESSED: [u8; 24] = [
    36, 98, 101, 102, 95, 84, 88, 89, 91, 93, 94, 96, 98, 100, 102, 104, 107, 112, 114, 118, 120,
    124, 125, 125,
];

/// `--aqua-menu-selected`: Menu Title Selected Fill, the highlighted row.
const MENU_SELECTED: [u32; 22] = [
    0x285cb7, 0x346cbe, 0x3973c1, 0x336abd, 0x285cb7, 0x336abd, 0x3871c0, 0x336abd, 0x285cb7,
    0x336abd, 0x3973c1, 0x336abd, 0x285cb7, 0x336abd, 0x3973c1, 0x336abd, 0x285cb7, 0x336abd,
    0x3973c1, 0x336abd, 0x2557b4, 0x1642ab,
];

/// `--aqua-pin-menu`: the four rows a menu's pinstripes repeat.
const PIN_MENU: [u32; 4] = [0xfafafa, 0xe9e9e9, 0xfafafa, 0xffffff];

/// The colour of a row table at CSS row `y`, the last row standing for every
/// one past the table's end - CSS's px stops do the same.
fn row_of(table: &[u32], y: f32) -> Color32 {
    let i = (y.max(0.0) as usize).min(table.len() - 1);
    hex(table[i])
}

// ---------------------------------------------------------------------------
// Boxes with rounded corners, filled a row at a time
// ---------------------------------------------------------------------------

/// A box with rounded corners and, with `border`, a one-pixel edge: see
/// [`prim::boxed`].
fn boxed(
    k: &Kit,
    rect: Rect,
    radius: [f32; 4],
    body: &dyn Fn(f32) -> Color32,
    border: Option<Color32>,
) -> Vec<Shape> {
    prim::boxed(rect, radius, body, border.as_slice(), k.ppp)
}

/// A ring `width` CSS pixels wide just outside `rect`: the focus glow, CSS's
/// `outline: 3px solid` in the light accent at .55.
fn ring(k: &Kit, rect: Rect, radius: f32, width: f32) -> Shape {
    let u = k.u();
    let outer = rect.expand(width * u);
    let size = outer.size() / u;
    let colour = rgba(109, 179, 255, 0.55);
    let r_out = radius + width;
    raster(outer, k.ppp, move |x, y| {
        let inside_outer = inside_rounded(x, y, size, [r_out; 4]);
        let inside_inner = inside_rounded(
            x - width,
            y - width,
            size - vec2(2.0 * width, 2.0 * width),
            [radius; 4],
        );
        if inside_outer && !inside_inner {
            colour
        } else {
            Color32::TRANSPARENT
        }
    })
}

// ---------------------------------------------------------------------------
// The window: `.window.metal`
// ---------------------------------------------------------------------------

/// `--aqua-metal-*`: the sheet's edges, and what the tile is multiplied by.
#[derive(Clone, Copy)]
struct Metal {
    /// White for the tile as it is.
    tint: Color32,
    rim: Color32,
    sheen: Color32,
    shade: Color32,
    light: Color32,
}

/// `--aqua-metal-base`, the tile's mean.
const METAL_BASE: Color32 = hex(0xd2d2d2);

/// The dark reading's metal: the same sheet taken down to a graphite that
/// light text reads on.
const DARK_METAL: f32 = 0.42;

fn metal(k: &Kit) -> Metal {
    let dark = k.look.dark;
    // A face of the user's own tints the sheet: the tile is multiplied by
    // how far the face is from the tile's mean.
    let tint = match k.look.paint.face {
        Some(face) => {
            let [r, g, b, _] = face.to_array();
            let f = |v: u8| ((f32::from(v) / f32::from(METAL_BASE.r())).min(1.0) * 255.0) as u8;
            Color32::from_rgb(f(r), f(g), f(b))
        }
        None if dark => Color32::from_gray((255.0 * DARK_METAL) as u8),
        None => Color32::WHITE,
    };
    if dark {
        Metal {
            tint,
            rim: hex(0x161616),
            sheen: hex(0x7a7a7a),
            shade: hex(0x2a2a2a),
            light: hex(0x5c5c5c),
        }
    } else {
        Metal {
            tint,
            rim: hex(0x555555),
            sheen: hex(0xe8e8e8),
            shade: hex(0x7d7d7d),
            light: hex(0xbbbbbb),
        }
    }
}

/// How the window's sheet is lit at its top and its foot.
const LIGHT: (f32, f32) = (1.04, 0.92);

/// How the main window's sheet is lit at `y`: what a patch laid over it -
/// a group's legend - has to match.
fn window_light(k: &Kit, y: f32) -> f32 {
    let height = k
        .ctx
        .as_ref()
        .map_or(600.0, |ctx| ctx.screen_rect().height())
        .max(1.0);
    LIGHT.0 + (LIGHT.1 - LIGHT.0) * (y / height).clamp(0.0, 1.0)
}

/// `c` scaled by `f`, alpha kept.
fn scaled(c: Color32, f: f32) -> Color32 {
    let [r, g, b, a] = c.to_array();
    let s = |v: u8| (f32::from(v) * f).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(s(r), s(g), s(b), a)
}

/// The brushed tile repeated over `rect`, one texel a CSS pixel and anchored
/// where the screen's corner is, so two pieces of sheet meet without a seam;
/// multiplied by `tint` and lit from `light.0` at the top to `light.1` at the
/// foot, as Panther's sheet is brighter where the title is; its corners
/// rounded by `radius`, `[nw, ne, sw, se]`, anti-aliased.
///
/// One tessellated rounded rect whose texture coordinates are set from where
/// each vertex is: a whole window of metal is a few dozen vertices, not a
/// rect for every row of it.
fn sheet(k: &Kit, rect: Rect, radius: [f32; 4], tint: Color32, light: (f32, f32)) -> Shape {
    let u = k.u();
    let r = prim::snap_rect(rect, k.ppp);
    let [nw, ne, sw, se] = radius.map(|v| v * u);
    let rounding = egui::Rounding { nw, ne, sw, se };
    let texture = k.texture("aqua-metal", egui::TextureOptions::NEAREST_REPEAT, || {
        let image = image::load_from_memory(png!("aqua-metal-tile").1)
            .ok()?
            .into_rgba8();
        Some(egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ))
    });
    let mut mesh = egui::epaint::Mesh::default();
    let mut tess = egui::epaint::Tessellator::new(
        k.ppp,
        egui::epaint::TessellationOptions::default(),
        [1, 1],
        vec![],
    );
    tess.tessellate_rect(
        &egui::epaint::RectShape::filled(r, rounding, Color32::WHITE),
        &mut mesh,
    );
    let tile = 128.0 * u;
    let height = r.height().max(1e-3);
    // With no texture to multiply, the tile's mean stands for it.
    let base = if texture.is_some() {
        tint
    } else {
        scaled(METAL_BASE, f32::from(tint.r()) / 255.0)
    };
    for v in &mut mesh.vertices {
        let t = ((v.pos.y - r.top()) / height).clamp(0.0, 1.0);
        let lit = scaled(base, light.0 + (light.1 - light.0) * t);
        // The tessellator's alpha is the edge's coverage.
        v.color = lit.gamma_multiply(f32::from(v.color.a()) / 255.0);
        if texture.is_some() {
            v.uv = pos2(v.pos.x / tile, v.pos.y / tile);
        }
    }
    if let Some(texture) = texture {
        mesh.texture_id = texture;
    }
    Shape::mesh(mesh)
}

/// A one-pixel line just inside the top or the foot of `r`, clear of its
/// corners of `radius` CSS pixels: an inset `box-shadow` of `0 1px 0`.
fn edge_line(k: &Kit, r: Rect, radius: f32, top: bool, colour: Color32) -> Shape {
    let u = k.u();
    let r = prim::snap_rect(r, k.ppp);
    let y = if top {
        r.top() + u
    } else {
        r.bottom() - 2.0 * u
    };
    let inset = (radius * 0.6).max(1.0) * u;
    fill(
        Rect::from_min_max(pos2(r.left() + inset, y), pos2(r.right() - inset, y + u)),
        colour,
    )
}

/// The gem a window control is, from `aqua-ui`: at rest, under the pointer
/// with its mark, held; and the grey one of an inactive window, with its 2x.
/// `None` for the app's own controls, which have no gem of their own.
fn gem(glyph: Glyph, st: St) -> Option<(Png, Option<Png>)> {
    let off = st.inactive_window || st.disabled;
    let frame = if st.pressed {
        3
    } else if st.hovered {
        1
    } else {
        0
    };
    Some(match (glyph, off, frame) {
        (Glyph::Close, true, _) => (png!("gem-close-off-0"), Some(png!("gem-close-off-0@2x"))),
        (Glyph::Close, _, 0) => (png!("gem-close-0"), None),
        (Glyph::Close, _, 1) => (png!("gem-close-1"), None),
        (Glyph::Close, _, _) => (png!("gem-close-3"), None),
        (Glyph::Minimize, true, _) => (png!("gem-min-off-0"), Some(png!("gem-min-off-0@2x"))),
        (Glyph::Minimize, _, 0) => (png!("gem-min-0"), None),
        (Glyph::Minimize, _, 1) => (png!("gem-min-1"), None),
        (Glyph::Minimize, _, _) => (png!("gem-min-3"), None),
        (Glyph::Maximize | Glyph::Restore, true, _) => {
            (png!("gem-zoom-off-0"), Some(png!("gem-zoom-off-0@2x")))
        }
        (Glyph::Maximize | Glyph::Restore, _, 0) => (png!("gem-zoom-0"), None),
        (Glyph::Maximize | Glyph::Restore, _, 1) => (png!("gem-zoom-1"), None),
        (Glyph::Maximize | Glyph::Restore, _, _) => (png!("gem-zoom-3"), None),
        _ => return None,
    })
}

/// The mark on one of the app's own gems: the grey gem, marked always.
fn mark(glyph: Glyph) -> &'static [&'static str] {
    match glyph {
        Glyph::Help => &[".###.", "#...#", "...#.", "..#..", ".....", "..#.."],
        Glyph::Gear => &[".#.#.", "#####", "##.##", "#####", ".#.#."],
        Glyph::Pin => &[".###.", ".#.#.", "#####", "..#..", "..#.."],
        Glyph::Pinned => &[".###.", ".###.", "#####", "..#..", "..#.."],
        _ => &["..#..", "..#..", "#####", "..#..", "..#.."],
    }
}

/// One window control: 15 px of gem centred on `rect`.
fn light(k: &Kit, rect: Rect, glyph: Glyph, st: St) -> Vec<Shape> {
    let u = k.u();
    let place = Rect::from_center_size(rect.center(), vec2(15.0, 15.0) * u);
    let found = gem(glyph, st);
    let own = found.is_none();
    let (one, two) = found.unwrap_or((png!("gem-close-off-0"), Some(png!("gem-close-off-0@2x"))));
    let inactive = st.inactive_window || st.disabled;
    // The theme's colour for the control: a window control's lit gem, or
    // the app's own grey one, recoloured; an inactive window's are grey.
    let to = k.button_colour(glyph).filter(|_| !inactive);
    let texture = match (two, to) {
        (Some(_), Some(to)) => tex_to(k, one, Some(to)),
        (Some(two), None) => sharp(k, one, two),
        (None, to) => tex_to(k, one, to),
    };
    // `.inactive .title-bar-controls button { opacity: .55 }`.
    let alpha = if (st.inactive_window || st.disabled) && !own {
        0.55
    } else {
        1.0
    };
    let mut out = match texture {
        Some(texture) => vec![prim::bitmap(
            place,
            texture,
            Color32::WHITE.gamma_multiply(alpha),
            k.ppp,
        )],
        None => vec![Shape::circle_filled(rect.center(), 6.5 * u, hex(0xa3a6ab))],
    };
    if own {
        let rows = mark(glyph);
        let ink = match k.look.paint.icon {
            Some(ink) => ink,
            None if st.pressed => rgba(10, 14, 20, 0.9),
            None => rgba(30, 34, 40, 0.75),
        };
        out.extend(prim::sprite(
            prim::centred(place, rows, k.ppp),
            rows,
            &[('#', ink)],
            k.ppp,
        ));
    }
    out
}

/// A push button's three bitmaps across `strip` - 14 px caps and the fill
/// between - clipped as `button { border-radius: 10px }` clips them: the
/// caps are cut from a square canvas, and only the clip makes them a pill.
/// Each cap is a tessellated half-rounded rect with the bitmap mapped over
/// it, so its curve is anti-aliased.
fn pill(
    k: &Kit,
    strip: Rect,
    tiles: (TextureId, TextureId, TextureId),
    tint: Color32,
) -> Vec<Shape> {
    let u = k.u();
    let r = prim::snap_rect(strip, k.ppp);
    let cap = (14.0 * u).min(r.width() / 2.0);
    let radius = (10.0 * u).min(r.height() / 2.0);
    let piece = |rect: Rect, texture: TextureId, rounding: egui::Rounding| -> Shape {
        let mut mesh = egui::epaint::Mesh::default();
        let mut tess = egui::epaint::Tessellator::new(
            k.ppp,
            egui::epaint::TessellationOptions::default(),
            [1, 1],
            vec![],
        );
        tess.tessellate_rect(
            &egui::epaint::RectShape::filled(rect, rounding, Color32::WHITE),
            &mut mesh,
        );
        let size = rect.size().max(vec2(1e-3, 1e-3));
        for v in &mut mesh.vertices {
            v.uv = ((v.pos - rect.min) / size).to_pos2();
            v.color = tint.gamma_multiply(f32::from(v.color.a()) / 255.0);
        }
        mesh.texture_id = texture;
        Shape::mesh(mesh)
    };
    let left = Rect::from_min_max(r.min, pos2(r.left() + cap, r.bottom()));
    let right = Rect::from_min_max(pos2(r.right() - cap, r.top()), r.max);
    let middle = Rect::from_min_max(left.right_top(), right.left_bottom());
    let mut out = Vec::with_capacity(3);
    if middle.is_positive() {
        out.push(prim::bitmap(middle, tiles.1, tint, k.ppp));
    }
    out.push(piece(
        left,
        tiles.0,
        egui::Rounding {
            nw: radius,
            sw: radius,
            ..Default::default()
        },
    ));
    out.push(piece(
        right,
        tiles.2,
        egui::Rounding {
            ne: radius,
            se: radius,
            ..Default::default()
        },
    ));
    out
}

/// What the controls' bitmaps are multiplied by: as they are on the light
/// sheet, graphite on the dark one - left white there, they would put light
/// labels on a white button.
fn controls_tint(k: &Kit, st: St) -> Color32 {
    let base = if k.look.dark {
        Color32::from_gray(98)
    } else {
        Color32::WHITE
    };
    if st.disabled {
        base.gamma_multiply(0.55)
    } else {
        base
    }
}

// ---------------------------------------------------------------------------
// Fallbacks for a kit with no context, which has no bitmaps
// ---------------------------------------------------------------------------

fn plain_button(k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
    let rows: &[u32] = if st.default || st.pressed {
        &TAB_SELECTED
    } else {
        &BTN_NORMAL
    };
    boxed(
        k,
        rect,
        [10.0; 4],
        &|y| row_of(rows, y),
        Some(hex(0x7f7f7f)),
    )
}

// ---------------------------------------------------------------------------
// The skin
// ---------------------------------------------------------------------------

impl Style for Aqua {
    fn colors(&self, dark: bool) -> Colors {
        if dark {
            return Colors {
                // The dark sheet's mean.
                face: hex(0x585858),
                text: hex(0xf0f0f0),
                text_disabled: hex(0xa4a4a4),
                text_disabled_shadow: None,
                field: hex(0x1f2024),
                field_text: hex(0xe6e8eb),
                select: hex(0x4d83d2),
                select_text: hex(0xffffff),
                title_text: hex(0xf2f2f2),
                title_text_inactive: hex(0xa8a8a8),
                title_shadow: Some(rgba(0, 0, 0, 0.5)),
                link: hex(0x8ab8ff),
                tooltip_text: hex(0x000000),
                subtle: hex(0xc8c8c8),
            };
        }
        Colors {
            // `--aqua-metal-base`: a page on the sheet is the sheet.
            face: METAL_BASE,
            text: hex(0x000000),
            // `--aqua-text-disabled`.
            text_disabled: hex(0x8d8d8d),
            text_disabled_shadow: None,
            field: hex(0xffffff),
            field_text: hex(0x000000),
            // `--aqua-selection`.
            select: hex(0x4d83d2),
            select_text: hex(0xffffff),
            // `.window.metal .title-bar-text`, with `.title-bar-text`'s
            // emboss under it.
            title_text: hex(0x1e1e1e),
            title_text_inactive: hex(0x7b7b7b),
            title_shadow: Some(rgba(255, 255, 255, 0.7)),
            link: hex(0x1d5ec7),
            tooltip_text: hex(0x000000),
            subtle: hex(0x404040),
        }
    }

    fn metrics(&self) -> Metrics {
        Metrics {
            // `--aqua-font-size`.
            font_size: 13.0,
            title_font_size: 13.0,
            // `.title-bar { height: 26px }`.
            title_height: 26.0,
            // `.title-bar-controls button`, `gap: 5px`: as wide as the gem
            // drawn in it, and as tall as the bar less 3 px above and
            // below, so the bar keeps to its 26 px with tabs in it too.
            title_button: vec2(15.0, 20.0),
            title_button_gap: 5.0,
            close_gap: 5.0,
            // The ring round the window, inside it here: a window's edge is
            // the screen's.
            window_border: 1.0,
            // `.window { border-radius: 8px 8px 6px 6px }`.
            window_radius: 8.0,
            // `--sb-size`, `--sb-arrow`.
            scrollbar: 15.0,
            scroll_arrow: 17.0,
            // The checkbox and radio bitmaps.
            checkbox: 15.0,
            radio: 14.0,
            // `min-width: 68px; height: 20px`, with 14 px caps.
            button_min: vec2(68.0, 20.0),
            button_pad_x: 14.0,
            // 3 px of padding round a 13 px line and the 1 px border.
            field_height: 22.0,
            // `.tabs > button { height: 24px }`.
            tab_height: 24.0,
            // `popup-capR`, the arrows' end of a pop-up.
            select_arrow: 21.0,
            // `.menu > li { padding: 1px 22px; line-height: 17px }`.
            menu_item_height: 19.0,
            // `sl-south`, and `--sl-track`.
            slider_thumb: vec2(15.0, 19.0),
            slider_track: 7.0,
            label_gap: 4.0,
        }
    }

    fn fonts(&self) -> Fonts {
        // `--aqua-font`, as far as a machine without a Mac has it: Lucida
        // Grande's own sibling ships with Windows.
        const LUCIDA: &[&str] = &["Lucida Grande", "Lucida Sans Unicode", "Segoe UI", "Tahoma"];
        Fonts {
            body: Some(Face::System(LUCIDA)),
            bold: Some(Face::System(LUCIDA)),
            title: None,
        }
    }

    fn title_centred(&self) -> bool {
        true
    }

    fn button(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let strip = band(k, rect, 20.0);
        let pressed = st.pressed && !st.disabled;
        let tint = controls_tint(k, st);
        // The blue of a default button is the blue of the scroll thumb: a
        // colour of the user's for the one is theirs for the other, the
        // three slices measured against the gel's own mid-tone so they stay
        // one piece. The white ones keep their white.
        let blue = |png: Png| {
            let gel = png.0.ends_with("-d") || png.0.ends_with("-3");
            match k.look.paint.handle {
                Some(handle) if gel => slice_to(k, png, Some(handle), 150),
                _ => tex(k, png),
            }
        };
        let layer = |caps: (Png, Png, Png)| -> Option<Vec<Shape>> {
            Some(pill(
                k,
                strip,
                (blue(caps.0)?, blue(caps.1)?, blue(caps.2)?),
                tint,
            ))
        };
        let rest = (png!("pb-capL-0"), png!("pb-fill-0"), png!("pb-capR-0"));
        let default = (png!("pb-capL-d"), png!("pb-fill-d"), png!("pb-capR-d"));
        let held = (png!("pb-capL-2"), png!("pb-fill-2"), png!("pb-capR-2"));
        let held_default = (png!("pb-capL-3"), png!("pb-fill-3"), png!("pb-capR-3"));
        // `button:active` lays frame 2 over frame 0; a default button, frame
        // 2 over frame 3.
        let layers = match (st.default, pressed) {
            (false, false) => vec![rest],
            (true, false) => vec![default],
            (false, true) => vec![rest, held],
            (true, true) => vec![held_default, held],
        };
        // `--aqua-btn-shadow`, as far as it shows round a 20 px pill: a
        // soft dark band under it, less of one while it is held.
        let u = k.u();
        let shade = if pressed { 0.18 } else { 0.30 };
        let mut out = vec![
            Shape::rect_filled(
                strip.translate(vec2(0.0, u)),
                egui::Rounding::same(10.0 * u),
                rgba(0, 0, 0, shade),
            ),
            Shape::rect_filled(
                strip
                    .translate(vec2(0.0, 2.0 * u))
                    .shrink2(vec2(2.0 * u, 0.0)),
                egui::Rounding::same(10.0 * u),
                rgba(0, 0, 0, shade * 0.4),
            ),
        ];
        for caps in layers {
            match layer(caps) {
                Some(shapes) => out.extend(shapes),
                None => return plain_button(k, strip, st),
            }
        }
        if st.focused {
            out.push(ring(k, strip.expand(k.u()), 10.0, 3.0));
        }
        out
    }

    fn button_text(&self, k: &Kit, st: St) -> Color32 {
        match (k.look.dark, st.disabled) {
            (false, false) => hex(0x000000),
            (false, true) => hex(0x8d8d8d),
            (true, false) => hex(0xf0f0f0),
            (true, true) => hex(0x9a9a9a),
        }
    }

    fn field(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let c = k.colors();
        let inside = if st.disabled {
            if k.look.dark {
                hex(0x2a2b30)
            } else {
                hex(0xf0f0f0)
            }
        } else {
            c.field
        };
        let edge = if st.focused {
            hex(0x4d83d2)
        } else {
            hex(0xa9a9a9)
        };
        // `border-top-color: #949494`, and `inset 0 1px 2px rgba(0,0,0,.16)`
        // as the two rows under it.
        let body = move |y: f32| -> Color32 {
            if !st.focused && y < 1.0 {
                hex(0x949494)
            } else if y < 2.0 {
                over(rgba(0, 0, 0, 0.16), inside)
            } else if y < 3.0 {
                over(rgba(0, 0, 0, 0.07), inside)
            } else {
                inside
            }
        };
        let mut out = boxed(k, rect, [2.0; 4], &body, Some(edge));
        if st.focused {
            out.push(ring(k, rect, 2.0, 3.0));
        }
        out
    }

    fn checkbox(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let pressed = st.pressed && !st.disabled;
        let (one, two) = match (st.on, pressed) {
            (false, false) => (png!("cbox-0"), png!("cbox-0@2x")),
            (true, false) => (png!("cbox-1"), png!("cbox-1@2x")),
            (false, true) => (png!("cbox-3"), png!("cbox-3@2x")),
            (true, true) => (png!("cbox-4"), png!("cbox-4@2x")),
        };
        let place = Rect::from_center_size(rect.center(), vec2(15.0, 16.0) * k.u());
        let Some(texture) = sharp(k, one, two) else {
            return boxed(
                k,
                rect,
                [3.0; 4],
                &|y| row_of(&BTN_NORMAL, y * 1.4),
                Some(hex(0x7f7f7f)),
            );
        };
        let mut out = vec![prim::bitmap(
            place,
            texture,
            controls_tint(k, St::default()),
            k.ppp,
        )];
        if st.disabled {
            // `cbox-6` / `cbox-7`: the disabled veil over the box.
            let veil = if st.on {
                sharp(k, png!("cbox-7"), png!("cbox-7@2x"))
            } else {
                sharp(k, png!("cbox-6"), png!("cbox-6@2x"))
            };
            if let Some(veil) = veil {
                out.push(prim::bitmap(
                    place,
                    veil,
                    controls_tint(k, St::default()),
                    k.ppp,
                ));
            }
        }
        if st.focused {
            out.push(ring(k, rect, 3.0, 3.0));
        }
        out
    }

    fn radio(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let pressed = st.pressed && !st.disabled;
        let (one, two) = match (st.on, pressed) {
            (false, false) => (png!("radio-0"), png!("radio-0@2x")),
            (true, false) => (png!("radio-1"), png!("radio-1@2x")),
            (false, true) => (png!("radio-3"), png!("radio-3@2x")),
            (true, true) => (png!("radio-4"), png!("radio-4@2x")),
        };
        let place = Rect::from_center_size(rect.center(), vec2(14.0, 15.0) * k.u());
        let Some(texture) = sharp(k, one, two) else {
            return vec![
                Shape::circle_filled(rect.center(), rect.width() / 2.0, hex(0xe5e5e5)),
                Shape::circle_stroke(
                    rect.center(),
                    rect.width() / 2.0,
                    egui::Stroke::new(1.0_f32, hex(0x7f7f7f)),
                ),
            ];
        };
        let mut out = vec![prim::bitmap(
            place,
            texture,
            controls_tint(k, St::default()),
            k.ppp,
        )];
        if st.disabled {
            let veil = if st.on {
                sharp(k, png!("radio-7"), png!("radio-7@2x"))
            } else {
                sharp(k, png!("radio-6"), png!("radio-6@2x"))
            };
            if let Some(veil) = veil {
                out.push(prim::bitmap(
                    place,
                    veil,
                    controls_tint(k, St::default()),
                    k.ppp,
                ));
            }
        }
        if st.focused {
            out.push(ring(k, rect, rect.width() / 2.0 / k.u(), 3.0));
        }
        out
    }

    fn select(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // The pop-up button: a 6 px cap, the fill, and the 21 px end with the
        // up-and-down arrows, all 20 px tall.
        let strip = band(k, rect, 20.0);
        let tiles = if k.ppp >= 1.5 {
            (
                tex(k, png!("popup-capL@2x")),
                tex(k, png!("popup-fill@2x")),
                tex(k, png!("popup-capR@2x")),
            )
        } else {
            (
                tex(k, png!("popup-capL")),
                tex(k, png!("popup-fill")),
                tex(k, png!("popup-capR")),
            )
        };
        let (Some(l), Some(m), Some(r)) = tiles else {
            return plain_button(k, strip, st);
        };
        let mut out = prim::three(
            strip,
            (6.0, 21.0),
            (l, m, r),
            controls_tint(k, st),
            false,
            k.ppp,
        );
        if st.focused {
            out.push(ring(k, strip, 4.0, 3.0));
        }
        out
    }

    fn select_text(&self, k: &Kit, st: St) -> Color32 {
        self.button_text(k, st)
    }

    fn arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        // The tree view's disclosure triangle: `border-left: 7px solid
        // #1a1a1a` between two 4.5 px slopes.
        let ink = if st.disabled {
            hex(0x8d8d8d)
        } else {
            hex(0x1a1a1a)
        };
        let c = rect.center();
        let u = k.u();
        let (a, b, tip) = match dir {
            Dir::Right => (
                c + vec2(-3.5, -4.5) * u,
                c + vec2(-3.5, 4.5) * u,
                c + vec2(3.5, 0.0) * u,
            ),
            Dir::Left => (
                c + vec2(3.5, -4.5) * u,
                c + vec2(3.5, 4.5) * u,
                c + vec2(-3.5, 0.0) * u,
            ),
            Dir::Down => (
                c + vec2(-4.5, -3.5) * u,
                c + vec2(4.5, -3.5) * u,
                c + vec2(0.0, 3.5) * u,
            ),
            Dir::Up => (
                c + vec2(-4.5, 3.5) * u,
                c + vec2(4.5, 3.5) * u,
                c + vec2(0.0, -3.5) * u,
            ),
        };
        vec![Shape::convex_polygon(
            vec![a, tip, b],
            ink,
            egui::Stroke::NONE,
        )]
    }

    fn slider_track(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let strip = band(k, rect, 7.0);
        match (
            tex(k, png!("sl-capL")),
            tex(k, png!("sl-fill")),
            tex(k, png!("sl-capR")),
        ) {
            (Some(l), Some(m), Some(r)) => prim::three(
                strip,
                (4.0, 4.0),
                (l, m, r),
                controls_tint(k, st),
                false,
                k.ppp,
            ),
            _ => boxed(k, strip, [3.5; 4], &|_| hex(0xa8a8a8), Some(hex(0x535353))),
        }
    }

    fn slider_thumb(&self, k: &Kit, centre: Pos2, st: St) -> Vec<Shape> {
        let (one, two) = if st.disabled {
            (png!("sl-south-2"), png!("sl-south-2@2x"))
        } else if st.pressed {
            (png!("sl-south-1"), png!("sl-south-1@2x"))
        } else {
            (png!("sl-south-0"), png!("sl-south-0@2x"))
        };
        let place = Rect::from_center_size(centre, vec2(15.0, 19.0) * k.u());
        match sharp(k, one, two) {
            Some(texture) => vec![prim::bitmap(place, texture, Color32::WHITE, k.ppp)],
            None => vec![Shape::circle_filled(centre, 7.0 * k.u(), hex(0x88b4e1))],
        }
    }

    fn tab_text(&self, k: &Kit, st: St) -> Color32 {
        // `[aria-selected="true"] { color: #fff }`; on the dark sheet the
        // tabs are graphite, and every label is light.
        if st.on || k.look.dark {
            hex(0xffffff)
        } else {
            hex(0x000000)
        }
    }

    fn tab(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `.tabs > button`: 7 px top corners, the push button's rows at rest,
        // the selected tab's blue rows, an inset 1 px edge; pressed, frame 1
        // laid over at .49.
        let edge = if st.on { hex(0x002d91) } else { hex(0x8a8a8a) };
        let selected = st.on;
        let pressed = st.pressed;
        let night: f32 = if k.look.dark { 0.42 } else { 1.0 };
        let body = move |y: f32| -> Color32 {
            let base = scaled(
                if selected {
                    row_of(&TAB_SELECTED, y)
                } else {
                    row_of(&BTN_NORMAL, y)
                },
                if selected { night.max(0.7) } else { night },
            );
            if pressed {
                let g = TAB_PRESSED[(y.max(0.0) as usize).min(TAB_PRESSED.len() - 1)];
                over(rgba(g, g, g, 0.49), base)
            } else {
                base
            }
        };
        boxed(k, rect, [7.0, 7.0, 0.0, 0.0], &body, Some(edge))
    }

    fn tab_panel(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `.tab-panel`: a 1 px #9a9a9a edge, 5 px corners.
        let face = k.colors().face;
        boxed(k, rect, [5.0; 4], &move |_| face, Some(hex(0x9a9a9a)))
    }

    fn group(&self, k: &Kit, rect: Rect, legend: Option<Rect>) -> Vec<Shape> {
        // `fieldset`: #b6b6b6, 5 px corners, a ten-percent grey wash - a
        // wash, so the sheet shows through it.
        let (wash, edge) = if k.look.dark {
            (rgba(255, 255, 255, 0.06), hex(0x3a3a3a))
        } else {
            (rgba(129, 129, 129, 0.10), hex(0xb6b6b6))
        };
        let mut out = boxed(k, rect, [5.0; 4], &move |_| wash, Some(edge));
        if let Some(legend) = legend {
            // `legend { padding: 0 5px }` over the edge: a patch of the
            // sheet, lit as the window is at that height.
            let r = prim::snap_rect(legend.expand2(vec2(5.0 * k.u(), 0.0)), k.ppp);
            let lit = window_light(k, r.center().y);
            out.push(sheet(k, r, [0.0; 4], metal(k).tint, (lit, lit)));
        }
        out
    }

    fn window(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // Lit from the title down, as Panther's sheet was.
        // `.window.metal`: 8 px top corners and 6 px foot ones, the tile on
        // `--aqua-metal-base`, `inset 0 1px 0` sheen and `inset 0 -1px 0`
        // shade, and the 1 px rim - inside the edge here, the window's edge
        // being the screen's. An inactive window's sheet is the same sheet.
        let _ = st;
        let m = metal(k);
        let r = prim::snap_rect(rect, k.ppp);
        let u = k.u();
        let radius = [8.0, 8.0, 6.0, 6.0];
        vec![
            sheet(k, r, radius, m.tint, LIGHT),
            edge_line(k, r.shrink(u), 7.0, true, m.sheen),
            edge_line(k, r.shrink(u), 5.0, false, m.shade),
            Shape::rect_stroke(
                r.shrink(u / 2.0),
                egui::Rounding {
                    nw: 8.0 * u,
                    ne: 8.0 * u,
                    sw: 6.0 * u,
                    se: 6.0 * u,
                },
                egui::Stroke::new(u, m.rim),
            ),
        ]
    }

    fn title_bar(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // `.window.metal .title-bar { background: none }`: the sheet shows
        // through.
        let _ = (k, rect, st);
        Vec::new()
    }

    fn title_button(&self, k: &Kit, rect: Rect, glyph: Glyph, st: St) -> Vec<Shape> {
        light(k, rect, glyph, st)
    }

    fn scroll_track(&self, k: &Kit, rect: Rect, vertical: bool, st: St) -> Vec<Shape> {
        let inactive = st.inactive_window;
        let tile = match (vertical, inactive) {
            (true, false) => png!("sb-track-v-0"),
            (true, true) => png!("sb-track-v-1"),
            (false, false) => png!("sb-track-h-0"),
            (false, true) => png!("sb-track-h-1"),
        };
        let Some(track) = slice_to(k, tile, k.look.paint.track, 225) else {
            return vec![fill(rect, hex(0xe4e4e4))];
        };
        let mut out = vec![prim::bitmap(
            rect,
            track,
            controls_tint(k, St::default()),
            k.ppp,
        )];
        // The cups where the track meets the arrows: 6 px at each end.
        let u = k.u();
        let cups = if vertical {
            (
                slice_to(k, png!("sb-cup-up"), k.look.paint.track, 225),
                slice_to(k, png!("sb-cup-down"), k.look.paint.track, 225),
                Rect::from_min_size(rect.min, vec2(rect.width(), 6.0 * u)),
                Rect::from_min_max(pos2(rect.left(), rect.bottom() - 6.0 * u), rect.max),
            )
        } else {
            (
                slice_to(k, png!("sb-cup-left"), k.look.paint.track, 225),
                slice_to(k, png!("sb-cup-right"), k.look.paint.track, 225),
                Rect::from_min_size(rect.min, vec2(6.0 * u, rect.height())),
                Rect::from_min_max(pos2(rect.right() - 6.0 * u, rect.top()), rect.max),
            )
        };
        if let (Some(near), Some(far)) = (cups.0, cups.1) {
            let tint = controls_tint(k, St::default());
            out.push(prim::bitmap(cups.2, near, tint, k.ppp));
            out.push(prim::bitmap(cups.3, far, tint, k.ppp));
        }
        out
    }

    fn scroll_thumb(&self, k: &Kit, rect: Rect, vertical: bool, st: St) -> Vec<Shape> {
        // Two 10 px caps and the fill between them.
        let handle = k.look.paint.handle;
        let tiles = if vertical {
            (
                slice_to(k, png!("sb-thumb-v-a"), handle, 150),
                slice_to(k, png!("sb-thumb-v-fill"), handle, 150),
                slice_to(k, png!("sb-thumb-v-b"), handle, 150),
            )
        } else {
            (
                slice_to(k, png!("sb-thumb-h-a"), handle, 150),
                slice_to(k, png!("sb-thumb-h-fill"), handle, 150),
                slice_to(k, png!("sb-thumb-h-b"), handle, 150),
            )
        };
        let (Some(a), Some(m), Some(b)) = tiles else {
            return boxed(k, rect, [7.0; 4], &|_| hex(0x5093e3), Some(hex(0x0039b3)));
        };
        let length = if vertical {
            rect.height()
        } else {
            rect.width()
        };
        let cap = (10.0 * k.u()).min(length / 2.0) / k.u();
        prim::three(
            rect,
            (cap, cap),
            (a, m, b),
            controls_tint(k, st),
            vertical,
            k.ppp,
        )
    }

    fn scroll_arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        let frame = if st.inactive_window {
            2
        } else if st.pressed {
            1
        } else {
            0
        };
        let png = match (dir, frame) {
            (Dir::Up, 0) => png!("sb-up-0"),
            (Dir::Up, 1) => png!("sb-up-1"),
            (Dir::Up, _) => png!("sb-up-2"),
            (Dir::Down, 0) => png!("sb-down-0"),
            (Dir::Down, 1) => png!("sb-down-1"),
            (Dir::Down, _) => png!("sb-down-2"),
            (Dir::Left, 0) => png!("sb-left-0"),
            (Dir::Left, 1) => png!("sb-left-1"),
            (Dir::Left, _) => png!("sb-left-2"),
            (Dir::Right, 0) => png!("sb-right-0"),
            (Dir::Right, 1) => png!("sb-right-1"),
            (Dir::Right, _) => png!("sb-right-2"),
        };
        match tex(k, png) {
            Some(texture) => vec![prim::bitmap(rect, texture, controls_tint(k, st), k.ppp)],
            None => {
                let mut out = vec![fill(rect, hex(0xe4e4e4))];
                out.extend(self.arrow(k, rect, dir, st));
                out
            }
        }
    }

    fn scroll_corner(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        match slice_to(k, png!("sb-track-v-0"), k.look.paint.track, 225) {
            Some(track) => vec![prim::bitmap(
                rect,
                track,
                controls_tint(k, St::default()),
                k.ppp,
            )],
            None => vec![fill(rect, hex(0xe4e4e4))],
        }
    }

    fn menu(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `.menu`: a 1 px #8a8a8a edge, 5 px bottom corners, the menu's
        // pinstripes.
        if k.look.dark {
            let face = hex(0x34363b);
            return boxed(
                k,
                rect,
                [0.0, 0.0, 5.0, 5.0],
                &move |_| face,
                Some(hex(0x5c5f65)),
            );
        }
        boxed(
            k,
            rect,
            [0.0, 0.0, 5.0, 5.0],
            &|y| row_of(&PIN_MENU, y.rem_euclid(4.0)),
            Some(hex(0x8a8a8a)),
        )
    }

    fn menu_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        if (st.hovered || st.on) && !st.disabled {
            // `--aqua-menu-selected`, its 22 rows from the top of the row.
            boxed(k, rect, [0.0; 4], &|y| row_of(&MENU_SELECTED, y), None)
        } else {
            Vec::new()
        }
    }

    fn separator(&self, k: &Kit, rect: Rect, vertical: bool) -> Vec<Shape> {
        // `.menu > li.separator`: 1 px of #c8c8c8 over 1 px of white.
        let u = k.u();
        let r = prim::snap_rect(rect, k.ppp);
        let (dark, light) = if k.look.dark {
            (hex(0x1c1d20), hex(0x4a4d53))
        } else {
            (hex(0xc8c8c8), hex(0xffffff))
        };
        if vertical {
            let x = prim::snap(r.center().x - u, k.ppp);
            vec![
                fill(
                    Rect::from_min_max(pos2(x, r.top()), pos2(x + u, r.bottom())),
                    dark,
                ),
                fill(
                    Rect::from_min_max(pos2(x + u, r.top()), pos2(x + 2.0 * u, r.bottom())),
                    light,
                ),
            ]
        } else {
            let y = prim::snap(r.center().y - u, k.ppp);
            vec![
                fill(
                    Rect::from_min_max(pos2(r.left(), y), pos2(r.right(), y + u)),
                    dark,
                ),
                fill(
                    Rect::from_min_max(pos2(r.left(), y + u), pos2(r.right(), y + 2.0 * u)),
                    light,
                ),
            ]
        }
    }

    fn tooltip(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // Aqua's help tags: pale yellow, a thin grey edge.
        boxed(k, rect, [0.0; 4], &|_| hex(0xffffc7), Some(hex(0x8a8a8a)))
    }

    fn list(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `.tree-view`: the text field's box, white.
        self.field(k, rect, St::default())
    }

    fn list_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        if st.on {
            // `tbody tr[aria-selected="true"]`.
            vec![fill(prim::snap_rect(rect, k.ppp), k.colors().select)]
        } else if st.hovered {
            // Aqua lit no row under the pointer; a faint wash says which one
            // a click would take, and leaves its text as it was.
            let wash = if k.look.dark {
                rgba(255, 255, 255, 0.10)
            } else {
                rgba(0, 0, 0, 0.07)
            };
            vec![fill(prim::snap_rect(rect, k.ppp), wash)]
        } else {
            Vec::new()
        }
    }

    fn progress(&self, k: &Kit, rect: Rect, fraction: f32) -> Vec<Shape> {
        // The 20 px groove in three pieces, and the indicator's first frame
        // repeated along the done part.
        let strip = band(k, rect, 20.0);
        let mut out = match (
            tex(k, png!("prog-capL")),
            tex(k, png!("prog-fill")),
            tex(k, png!("prog-capR")),
        ) {
            (Some(l), Some(m), Some(r)) => prim::three(
                strip,
                (4.0, 4.0),
                (l, m, r),
                controls_tint(k, St::default()),
                false,
                k.ppp,
            ),
            _ => boxed(k, strip, [4.0; 4], &|_| hex(0xe3e3e3), Some(hex(0xa3a3a3))),
        };
        let done = Rect::from_min_size(
            strip.min,
            vec2(strip.width() * fraction.clamp(0.0, 1.0), strip.height()),
        );
        if done.width() > 0.0 {
            let indicator = k.texture(
                "aqua-prog-ind",
                egui::TextureOptions::NEAREST_REPEAT,
                || {
                    let image = image::load_from_memory(png!("prog-ind").1)
                        .ok()?
                        .into_rgba8();
                    // The first of its sixteen frames, 16 by 20.
                    let frame = image::imageops::crop_imm(&image, 0, 0, 16, 20).to_image();
                    Some(egui::ColorImage::from_rgba_unmultiplied(
                        [16, 20],
                        frame.as_raw(),
                    ))
                },
            );
            match indicator {
                Some(texture) => out.push(prim::tiled(done, texture, vec2(16.0, 20.0), k.ppp)),
                None => out.push(fill(done, hex(0x5c99dc))),
            }
        }
        out
    }

    fn status_bar(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // `.metal .status-bar`: no fill of its own, and `inset 0 1px 0`
        // shade over `inset 0 2px 0` light along its top.
        let m = metal(k);
        let r = prim::snap_rect(rect, k.ppp);
        let u = k.u();
        vec![
            fill(
                Rect::from_min_max(r.min, pos2(r.right(), r.top() + u)),
                m.shade,
            ),
            fill(
                Rect::from_min_max(
                    pos2(r.left(), r.top() + u),
                    pos2(r.right(), r.top() + 2.0 * u),
                ),
                m.light,
            ),
        ]
    }

    fn status_field(&self, _k: &Kit, _rect: Rect) -> Vec<Shape> {
        // Aqua's status line was the bare surface.
        Vec::new()
    }

    fn focus(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        vec![ring(k, rect, 3.0, 3.0)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::skin::{Look, Skin};

    fn kit(dark: bool) -> Kit {
        Kit::bare(
            Look {
                skin: Skin::Aqua,
                dark,
                ..Default::default()
            },
            1.0,
        )
    }

    /// The colour a painted shape leaves at a point, the last one on top.
    fn colour_at(shapes: &[Shape], at: Pos2) -> Color32 {
        let mut found = Color32::TRANSPARENT;
        for shape in shapes {
            if let Shape::Mesh(mesh) = shape {
                for tri in mesh.indices.chunks(3) {
                    let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| mesh.vertices[i as usize]);
                    let r = Rect::from_points(&[a.pos, b.pos, c.pos]);
                    if r.contains(at) {
                        found = a.color;
                    }
                }
            }
        }
        found
    }

    /// Runs `f` with a kit that has a context, and so the bitmaps.
    fn with_kit(dark: bool, f: impl Fn(&Kit)) {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            crate::ui::skin::set(
                ctx,
                Look {
                    skin: Skin::Aqua,
                    dark,
                    ..Default::default()
                },
            );
            f(&Kit::of(ctx).unwrap());
        });
    }

    fn texture_of(shapes: &[Shape]) -> Option<egui::TextureId> {
        shapes.iter().find_map(|s| match s {
            Shape::Rect(r) => Some(r.fill_texture_id),
            Shape::Mesh(m) => Some(m.texture_id),
            _ => None,
        })
    }

    /// Under the pointer a window control is another of its gems - the one
    /// with its mark - and held, a third.
    #[test]
    fn a_window_control_swaps_its_gem_under_the_pointer_and_held() {
        with_kit(false, |k| {
            let rect = Rect::from_min_size(Pos2::ZERO, vec2(13.0, 13.0));
            let rest = texture_of(&light(k, rect, Glyph::Close, St::default()));
            let hover = St {
                hovered: true,
                ..St::default()
            };
            let held = St {
                pressed: true,
                ..hover
            };
            let over = texture_of(&light(k, rect, Glyph::Close, hover));
            let down = texture_of(&light(k, rect, Glyph::Close, held));
            assert!(rest.is_some());
            assert_ne!(rest, over);
            assert_ne!(over, down);
        });
    }

    /// The window is the brushed tile, repeated, as one small mesh - not a
    /// rect for each of its rows, which is what made the skin slow.
    #[test]
    fn the_window_is_one_small_textured_sheet() {
        with_kit(true, |k| {
            let rect = Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0));
            let shapes = Aqua.window(k, rect, St::default());
            let Some(Shape::Mesh(sheet)) = shapes.first() else {
                panic!("no sheet");
            };
            assert_ne!(sheet.texture_id, egui::TextureId::default());
            assert!(sheet.vertices.len() < 400, "{}", sheet.vertices.len());
            // The dark sheet is the tile taken down, not left white.
            let middle = sheet.vertices[sheet.vertices.len() / 2].color;
            assert!(middle.r() < 140, "{middle:?}");
        });
    }

    /// A rounded box leaves its corner pixel clear and fills its middle.
    #[test]
    fn a_rounded_box_is_clear_at_its_corner() {
        let k = kit(false);
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(40.0, 24.0));
        let shapes = boxed(&k, rect, [7.0, 7.0, 0.0, 0.0], &|_| hex(0x123456), None);
        assert_eq!(colour_at(&shapes, pos2(0.5, 0.5)), Color32::TRANSPARENT);
        assert_eq!(colour_at(&shapes, pos2(20.5, 12.5)), hex(0x123456));
    }

    /// The selected tab's rows are `--aqua-tab-selected`'s, edge and all.
    #[test]
    fn a_selected_tab_is_the_system_blue_row_by_row() {
        let k = kit(false);
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(80.0, 24.0));
        let shapes = Aqua.tab(&k, rect, St::default().on(true));
        assert_eq!(colour_at(&shapes, pos2(40.5, 0.5)), hex(0x002d91));
        assert_eq!(colour_at(&shapes, pos2(40.5, 10.5)), hex(0x8cb3db));
    }
}
