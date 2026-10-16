//! What stands between the app and a skin's painters: a cache, and the
//! recolouring a skin's own tables cannot do.
//!
//! The painters reproduce their CSS a pixel row, sometimes a pixel, at a
//! time - an Aqua traffic light is a raster of every one of its pixels - and
//! the chrome asks for the same shapes every frame. Painted afresh each time,
//! Aqua's frame alone cost milliseconds a frame in a release build. So a
//! painter's shapes are kept, per context, by everything they depend on, and
//! a frame that asks again gets them moved to where they are wanted: a
//! window, a tab, a light is painted once for each size and state it takes.
//!
//! The shapes are painted at the origin, shifted only by the part of the
//! rect's corner that is a fraction of a physical pixel, so moving them by
//! the rest keeps every edge on the pixel grid the painter snapped it to.
//!
//! Two recolourings happen on the way into the cache, once rather than every
//! frame. XP's dark reading is its light one with the neutrals - the cream,
//! the white, the greys - taken down, their order kept so a bevel still
//! catches the light from the top, and the Luna blues left alone. And a
//! theme of the user's own that names an accent turns the skin's blues to it.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::{vec2, Color32, Pos2, Rect, Shape, Vec2};

use super::{Colors, Dir, Fonts, Glyph, Kit, Metrics, Skin, St, Style};

/// A skin's painters, behind the cache.
pub struct Tuned(&'static dyn Style);

static PLAIN: Tuned = Tuned(&super::Plain);
static WIN98: Tuned = Tuned(&super::win98::Win98);
static XP: Tuned = Tuned(&super::xp::Xp);
static AQUA: Tuned = Tuned(&super::aqua::Aqua);
static SYSTEM: Tuned = Tuned(&super::system::System);
static SNES: Tuned = Tuned(&super::snes::Snes);

pub fn of(skin: Skin) -> &'static dyn Style {
    match skin {
        Skin::None => &PLAIN,
        Skin::Win98 => &WIN98,
        Skin::Xp => &XP,
        Skin::Aqua => &AQUA,
        Skin::System => &SYSTEM,
        Skin::Snes => &SNES,
    }
}

/// How many painted controls a context keeps. A window has a few dozen on
/// screen; the rest are the sizes a resize went through, which are dropped
/// wholesale when the cache fills rather than tracked one by one.
const CAPACITY: usize = 512;

#[derive(Clone, Default)]
struct Cache(HashMap<u64, Arc<Vec<Shape>>>);

fn cache_id() -> egui::Id {
    egui::Id::new("nit-skin-shapes")
}

/// The physical-pixel part of `at`, in points: where shapes painted at the
/// origin are moved to.
fn whole(at: Pos2, ppp: f32) -> Vec2 {
    vec2((at.x * ppp).floor() / ppp, (at.y * ppp).floor() / ppp)
}

fn bits(r: Rect) -> [u32; 4] {
    [r.min.x, r.min.y, r.max.x, r.max.y].map(f32::to_bits)
}

impl Tuned {
    /// `paint` at `rect` from the cache, painting it the first time - once a
    /// kit has a context to keep it in; a bare kit paints every time.
    fn memo(
        &self,
        k: &Kit,
        tag: &'static str,
        rect: Rect,
        extra: impl Hash,
        paint: impl FnOnce(Rect) -> Vec<Shape>,
    ) -> Vec<Shape> {
        self.memo_then(k, tag, rect, extra, paint, |shapes| shapes)
    }

    /// [`Tuned::memo`], with `after` laid on once the skin's own recolouring
    /// is done: a colour the user gave the control itself, which the accent
    /// turning the skin's blues must not turn again.
    fn memo_then(
        &self,
        k: &Kit,
        tag: &'static str,
        rect: Rect,
        extra: impl Hash,
        paint: impl FnOnce(Rect) -> Vec<Shape>,
        after: impl FnOnce(Vec<Shape>) -> Vec<Shape>,
    ) -> Vec<Shape> {
        let Some(ctx) = k.ctx.as_ref() else {
            return after(toned_as(k, paint(rect), night_of(tag)));
        };
        if !rect.is_finite() {
            return Vec::new();
        }
        let shift = whole(rect.min, k.ppp);
        let local = rect.translate(-shift);
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (tag, bits(local), k.look, k.ppp.to_bits(), extra).hash(&mut h);
        let key = h.finish();
        let found = ctx.data(|d| {
            d.get_temp::<Cache>(cache_id())
                .and_then(|c| c.0.get(&key).cloned())
        });
        let shapes = match found {
            Some(shapes) => shapes,
            None => {
                let shapes = Arc::new(after(toned_as(k, paint(local), night_of(tag))));
                ctx.data_mut(|d| {
                    let cache = d.get_temp_mut_or_default::<Cache>(cache_id());
                    if cache.0.len() >= CAPACITY {
                        cache.0.clear();
                    }
                    cache.0.insert(key, shapes.clone());
                });
                shapes
            }
        };
        let mut out = (*shapes).clone();
        if shift != Vec2::ZERO {
            for shape in &mut out {
                shape.translate(shift);
            }
        }
        out
    }

    fn text(&self, k: &Kit, colour: Color32) -> Color32 {
        if night(k) && neutral(colour) && luma(colour) < 0.5 {
            k.colors().text
        } else {
            colour
        }
    }
}

/// What keeps Luna's own colours at night too: the blue title bar and its
/// tiles, white glyphs and all, and the pale yellow help tag, whose words
/// are written black.
fn on_title(tag: &str) -> bool {
    matches!(
        tag,
        "title_bar" | "title_button" | "title_backing" | "tooltip"
    )
}

/// The body of a window painted as `shapes`: their bounds less the frame.
fn rect_inside(shapes: &[Shape], border: f32) -> Rect {
    shapes
        .iter()
        .map(Shape::visual_bounding_rect)
        .fold(Rect::NOTHING, |a, b| a.union(b))
        .shrink(border)
}

/// How opaque a card is over the theme's chrome gradient - the theme's
/// "Window glass" - or `None` with no gradient, where a card is the skin's.
fn glass(k: &Kit) -> Option<f32> {
    let ctx = k.ctx.as_ref()?;
    crate::ui::shading::backdrop(ctx)?;
    Some(crate::ui::prefs::opacity(ctx).clamp(0.0, 1.0))
}

/// A card - a group box, a list - as glass over the theme's gradient: its
/// fill, the colour it is mostly painted in, at the glass's opacity, and its
/// lines left as they are. A card the skin leaves unfilled - 98's, the
/// classic Mac OS's, snes.css's - gets a veil of the page's colour at that
/// opacity under its lines, as the app's own cards are veils of the theme's
/// background.
fn glazed(k: &Kit, rect: Rect, mut shapes: Vec<Shape>) -> Vec<Shape> {
    let Some(opacity) = glass(k) else {
        return shapes;
    };
    let mut areas: HashMap<Color32, f32> = HashMap::new();
    for shape in &shapes {
        painted_area(shape, &mut areas);
    }
    let whole = rect.width().max(0.0) * rect.height().max(0.0);
    let body = areas
        .into_iter()
        .filter(|(c, _)| c.a() > 16)
        .max_by(|a, b| {
            a.1.total_cmp(&b.1)
                .then_with(|| a.0.to_array().cmp(&b.0.to_array()))
        })
        // A fill covers most of the card; a frame's lines never do.
        .filter(|&(_, area)| area > whole * 0.5)
        .map(|(c, _)| c);
    match body {
        Some(body) => {
            for shape in &mut shapes {
                recolour(shape, &mut |c| {
                    if c == body {
                        c.gamma_multiply(opacity)
                    } else {
                        c
                    }
                });
            }
        }
        None => {
            let veil = k.colors().face.gamma_multiply(opacity);
            shapes.insert(
                0,
                super::prim::fill(super::prim::snap_rect(rect, k.ppp), veil),
            );
        }
    }
    shapes
}

/// `k` with none of the user's colours: what a skin's own controls are
/// worked out against.
fn plain(k: &Kit) -> Kit {
    let mut plain = k.clone();
    plain.look.paint = super::Paint::default();
    plain
}

/// Whether `k` paints a dark reading made from the light one.
fn night(k: &Kit) -> bool {
    k.look.dark && k.look.skin == Skin::Xp
}

fn rgb(c: Color32) -> [f32; 3] {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    [r, g, b].map(|v| f32::from(v) / 255.0)
}

fn luma(c: Color32) -> f32 {
    let [r, g, b] = rgb(c);
    (r.max(g).max(b) + r.min(g).min(b)) / 2.0
}

/// A grey, or a colour so near one - XP's cream - that it is the page's.
fn neutral(c: Color32) -> bool {
    let [r, g, b] = rgb(c);
    r.max(g).max(b) - r.min(g).min(b) < 0.1
}

/// The dark reading of a light neutral: the range from mid-grey to white
/// pressed into the dark greys, in the same order.
///
/// By degrees: a colour is taken down as far as it is a neutral, so the
/// anti-aliased pixel where a white button meets its blue edge - half one,
/// half the other - goes half-way, rather than staying light and showing as
/// a white speck on the dark button.
pub(super) fn darken(c: Color32) -> Color32 {
    darken_from(c, 0.10)
}

/// [`darken`] with white taken to `floor` plus a little under a half:
/// a control is lifted off the page it sits on, as a light one was.
fn darken_from(c: Color32, floor: f32) -> Color32 {
    let l = luma(c);
    if l <= 0.55 {
        return c;
    }
    let [r, g, b] = rgb(c);
    let chroma = r.max(g).max(b) - r.min(g).min(b);
    let weight = ((0.40 - chroma) / 0.30).clamp(0.0, 1.0);
    if weight == 0.0 {
        return c;
    }
    let to = floor + (l - 0.55) * 0.45;
    let f = 1.0 - weight * (1.0 - to / l);
    let [r, g, b] = [r, g, b].map(|v| (v * f * 255.0).round() as u8);
    Color32::from_rgba_unmultiplied(r, g, b, c.a())
}

fn hue(c: Color32) -> Option<f32> {
    let [r, g, b] = rgb(c);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    if d < 0.15 {
        return None;
    }
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    Some(h * 60.0)
}

/// The skin's blues: the band of hue its selection colour is in.
fn blue(c: Color32, around: f32) -> bool {
    hue(c).is_some_and(|h| {
        let d = (h - around + 540.0).rem_euclid(360.0) - 180.0;
        d.abs() < 45.0
    })
}

/// `c` in the colour `to`, as light or as dark relative to `mean` as it was:
/// a bevel's highlight stays a highlight and its shadow a shadow, only now
/// of `to` - lighter than `to` is mixed toward white, darker toward black.
fn retint_colour(c: Color32, to: Color32, mean: f32) -> Color32 {
    let ratio = luma(c) / mean.max(1e-3);
    let [r, g, b] = rgb(to);
    let out = if ratio >= 1.0 {
        let t = ((ratio - 1.0) / (1.0 / mean.max(1e-3) - 1.0).max(1e-3)).clamp(0.0, 1.0);
        [r, g, b].map(|v| v + (1.0 - v) * t)
    } else {
        [r, g, b].map(|v| v * ratio)
    };
    let [r, g, b] = out.map(|v| (v * 255.0).round().clamp(0.0, 255.0) as u8);
    Color32::from_rgba_unmultiplied(r, g, b, c.a())
}

/// The mean lightness of the colours a control is painted in, by how often
/// each appears: what [`retint_colour`] measures each against.
fn mean_luma(colours: impl Iterator<Item = Color32>) -> f32 {
    let (sum, n) = colours
        .filter(|c| c.a() > 16)
        .fold((0.0, 0u32), |(s, n), c| (s + luma(c), n + 1));
    if n == 0 {
        0.5
    } else {
        sum / n as f32
    }
}

/// A bitmap's pixels in the colour `to`, its light and shade kept.
pub(super) fn retint_pixels(pixels: &mut [Color32], to: Color32, reference: Option<u8>) {
    let mean = reference.map_or_else(
        || mean_luma(pixels.iter().copied()),
        |r| f32::from(r) / 255.0,
    );
    for p in pixels {
        if p.a() > 0 {
            *p = retint_colour(*p, to, mean);
        }
    }
}

/// `shapes` - one control - recoloured to `to`; a bitmap among them is left
/// to the skin, which recolours its texture (see `Kit::image_as`).
fn retint(shapes: Vec<Shape>, to: Option<Color32>) -> Vec<Shape> {
    retint_but(shapes, to, None)
}

/// [`retint`], leaving `keep` - a mark already drawn in the user's ink - as
/// it is.
fn retint_but(mut shapes: Vec<Shape>, to: Option<Color32>, keep: Option<Color32>) -> Vec<Shape> {
    let Some(to) = to else {
        return shapes;
    };
    // Measured against the colour it is mostly painted in, by area - a
    // card's fill, a thumb's body - which becomes `to` itself; its lights and
    // shades become lighter and darker than `to` by as much. By area, not by
    // vertex: a rasterised box's flat middle is one quad and its curved edge
    // hundreds.
    let mut areas: HashMap<Color32, f32> = HashMap::new();
    for shape in &shapes {
        painted_area(shape, &mut areas);
    }
    let mean = areas
        .into_iter()
        .filter(|(c, _)| c.a() > 16)
        // A tie - a frame of two lines as long as each other - goes the
        // same way every frame, to the lighter: left to the map's order it
        // went one way one frame and the other the next, and the box flickered.
        .max_by(|a, b| {
            a.1.total_cmp(&b.1)
                .then_with(|| luma(a.0).total_cmp(&luma(b.0)))
                .then_with(|| a.0.to_array().cmp(&b.0.to_array()))
        })
        .map_or(0.5, |(c, _)| luma(c));
    for shape in &mut shapes {
        recolour(shape, &mut |c| {
            if c.a() == 0 || Some(c) == keep {
                c
            } else {
                retint_colour(c, to, mean)
            }
        });
    }
    shapes
}

/// How much of each colour `shape` paints, in square points.
fn painted_area(shape: &Shape, areas: &mut HashMap<Color32, f32>) {
    let rect_area = |r: Rect| r.width().max(0.0) * r.height().max(0.0);
    match shape {
        Shape::Vec(v) => v.iter().for_each(|s| painted_area(s, areas)),
        Shape::Rect(r) if r.fill_texture_id == egui::TextureId::default() => {
            *areas.entry(r.fill).or_default() += rect_area(r.rect);
        }
        Shape::Circle(c) => {
            *areas.entry(c.fill).or_default() += std::f32::consts::PI * c.radius * c.radius;
        }
        Shape::Mesh(m) if m.texture_id == egui::TextureId::default() => {
            for tri in m.indices.as_chunks::<3>().0 {
                let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| m.vertices[i as usize]);
                let area = ((b.pos - a.pos).x * (c.pos - a.pos).y
                    - (c.pos - a.pos).x * (b.pos - a.pos).y)
                    .abs()
                    / 2.0;
                *areas.entry(a.color).or_default() += area;
            }
        }
        _ => {}
    }
}

/// Every colour of `shapes` put through what the kit asks for.
fn toned(k: &Kit, shapes: Vec<Shape>) -> Vec<Shape> {
    toned_as(k, shapes, Night::Page)
}

/// How a painter's shapes are taken down for a dark reading made from the
/// light one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Night {
    /// Not at all: Luna's own colours at night too.
    Off,
    /// A page, a field, a frame: to the dark greys.
    Page,
    /// A control: to greys a step above the page, and its dark blue edges
    /// lifted - XP's navy button border is all but black on charcoal.
    Control,
}

fn night_of(tag: &str) -> Night {
    match tag {
        _ if on_title(tag) => Night::Off,
        "button" | "select" | "checkbox" | "radio" | "tab" | "scroll_thumb" | "scroll_arrow"
        | "slider_thumb" => Night::Control,
        _ => Night::Page,
    }
}

/// [`toned`], taken down at night as `how` says.
fn toned_as(k: &Kit, mut shapes: Vec<Shape>, how: Night) -> Vec<Shape> {
    let dark = if night(k) { how } else { Night::Off };
    // The skin's blues - the band of hue its selection is in - become the
    // accent itself, tone and all: the selection's blue is the accent, and a
    // gradient's lighter and darker blues are as much lighter and darker than
    // it, so 98's navy-to-blue title is the accent run to its light.
    let turn_by = k.look.paint.accent.and_then(|accent| {
        let select = k.look.skin.style().colors(k.look.dark).select;
        Some((hue(select)?, accent, luma(select)))
    });
    if dark == Night::Off && turn_by.is_none() {
        return shapes;
    }
    let mut seen: HashMap<Color32, Color32> = HashMap::new();
    let mut map = |c: Color32| -> Color32 {
        if c.a() == 0 {
            return c;
        }
        *seen.entry(c).or_insert_with(|| {
            let mut out = c;
            match dark {
                Night::Off => {}
                Night::Page => out = darken(out),
                Night::Control => {
                    out = darken_from(out, 0.20);
                    let [r, g, b] = rgb(out);
                    let l = luma(out);
                    if l < 0.32 && r.max(g).max(b) - r.min(g).min(b) > 0.2 {
                        let f = 0.48 / l.max(0.05);
                        let [r, g, b] = [r, g, b].map(|v| (v * f).min(1.0) * 255.0);
                        out = Color32::from_rgba_unmultiplied(r as u8, g as u8, b as u8, out.a());
                    }
                }
            }
            if let Some((own, accent, reference)) = turn_by {
                if blue(out, own) {
                    out = retint_colour(out, accent, reference);
                }
            }
            out
        })
    };
    for shape in &mut shapes {
        recolour(shape, &mut map);
    }
    shapes
}

fn recolour(shape: &mut Shape, map: &mut impl FnMut(Color32) -> Color32) {
    match shape {
        Shape::Vec(v) => v.iter_mut().for_each(|s| recolour(s, map)),
        Shape::Rect(r) if r.fill_texture_id != egui::TextureId::default() => {
            r.stroke.color = map(r.stroke.color);
        }
        Shape::Rect(r) => {
            r.fill = map(r.fill);
            r.stroke.color = map(r.stroke.color);
            // An image's tint multiplies the bitmap: white is the bitmap as
            // it is, and stays so.
        }
        Shape::Circle(c) => {
            c.fill = map(c.fill);
            c.stroke.color = map(c.stroke.color);
        }
        Shape::Ellipse(e) => {
            e.fill = map(e.fill);
            e.stroke.color = map(e.stroke.color);
        }
        Shape::LineSegment { stroke, .. } => {
            if let egui::epaint::ColorMode::Solid(c) = &mut stroke.color {
                *c = map(*c);
            }
        }
        Shape::Path(p) => {
            p.fill = map(p.fill);
            if let egui::epaint::ColorMode::Solid(c) = &mut p.stroke.color {
                *c = map(*c);
            }
        }
        // A textured mesh's colours multiply its bitmap, as an image's tint
        // does.
        Shape::Mesh(m) if m.texture_id == egui::TextureId::default() => {
            for v in &mut m.vertices {
                v.color = map(v.color);
            }
        }
        _ => {}
    }
}

impl Style for Tuned {
    fn colors(&self, dark: bool) -> Colors {
        self.0.colors(dark)
    }
    fn metrics(&self) -> Metrics {
        self.0.metrics()
    }
    fn fonts(&self) -> Fonts {
        self.0.fonts()
    }
    fn pressed_shift(&self) -> f32 {
        self.0.pressed_shift()
    }
    fn title_centred(&self) -> bool {
        self.0.title_centred()
    }
    fn window_top(&self) -> f32 {
        self.0.window_top()
    }
    fn close_width(&self) -> f32 {
        self.0.close_width()
    }
    fn title_backing(&self, k: &Kit, text: Rect, st: St) -> Vec<Shape> {
        self.memo(k, "title_backing", text, st, |r| {
            self.0.title_backing(k, r, st)
        })
    }
    // The words on a skin's own controls are in the skin's ink, worked out
    // as if the user had changed nothing: a button, a field, a tab keep the
    // look they had, and the user's page text need not read on them.
    fn button_text(&self, k: &Kit, st: St) -> Color32 {
        self.text(k, self.0.button_text(&plain(k), st))
    }
    fn select_text(&self, k: &Kit, st: St) -> Color32 {
        self.text(k, self.0.select_text(&plain(k), st))
    }
    fn tab_text(&self, k: &Kit, st: St) -> Color32 {
        // A selected tab in the user's colour wears the user's text for it.
        // The classic Mac OS's tab is never filled, so keeps its ink.
        match k.look.paint.select {
            Some(_) if st.on && k.look.skin != Skin::System => k.colors().select_text,
            _ => self.text(k, self.0.tab_text(&plain(k), st)),
        }
    }
    fn list_text(&self, k: &Kit, st: St) -> Color32 {
        match k.look.paint.select {
            Some(_) if st.on => k.colors().select_text,
            _ => self.text(k, self.0.list_text(&plain(k), st)),
        }
    }
    fn button(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        self.memo(k, "button", rect, st, |r| self.0.button(k, r, st))
    }
    fn field(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        self.memo(k, "field", rect, st, |r| self.0.field(k, r, st))
    }
    fn checkbox(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        self.memo(k, "checkbox", rect, st, |r| self.0.checkbox(k, r, st))
    }
    fn radio(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        self.memo(k, "radio", rect, st, |r| self.0.radio(k, r, st))
    }
    fn select(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        self.memo(k, "select", rect, st, |r| self.0.select(k, r, st))
    }
    fn arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        self.memo(k, "arrow", rect, (dir, st), |r| self.0.arrow(k, r, dir, st))
    }
    fn slider_track(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        self.memo(k, "slider_track", rect, st, |r| {
            self.0.slider_track(k, r, st)
        })
    }
    fn slider_thumb(&self, k: &Kit, centre: Pos2, st: St) -> Vec<Shape> {
        let at = Rect::from_min_size(centre, Vec2::ZERO);
        self.memo(k, "slider_thumb", at, st, |r| {
            self.0.slider_thumb(k, r.min, st)
        })
    }
    fn tab(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let to = k.look.paint.select.filter(|_| st.on);
        self.memo_then(
            k,
            "tab",
            rect,
            st,
            |r| self.0.tab(k, r, st),
            |v| retint(v, to),
        )
    }
    fn tab_panel(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        self.memo(k, "tab_panel", rect, (), |r| self.0.tab_panel(k, r))
    }
    fn group(&self, k: &Kit, rect: Rect, legend: Option<Rect>) -> Vec<Shape> {
        // Not kept: Aqua patches the sheet behind a legend, and the patch's
        // grain is where the screen's is, which moving it would break. A
        // group box is a handful of rects anyway.
        // On a face of the user's, a group box is that face, its frame and
        // its fill as much lighter or darker than it as the skin's are than
        // its own - XP's white card on cream is a lighter card on the user's.
        let face = k.look.paint.face.filter(|_| k.look.skin != Skin::Aqua);
        glazed(
            k,
            rect,
            retint(toned(k, self.0.group(k, rect, legend)), face),
        )
    }
    fn window(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        // The body inside the frame is the page, so the user's colour for
        // it; the frame round it stays the skin's. Aqua's body is its sheet,
        // which the face tints already.
        let face = k.look.paint.face.filter(|_| k.look.skin != Skin::Aqua);
        let border = (self.0.metrics().window_border * k.u()).round();
        self.memo_then(
            k,
            "window",
            rect,
            st,
            |r| self.0.window(k, r, st),
            |mut v| {
                if let Some(face) = face {
                    v.push(super::prim::fill(rect_inside(&v, border), face));
                }
                v
            },
        )
    }
    fn title_bar(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        self.memo(k, "title_bar", rect, st, |r| self.0.title_bar(k, r, st))
    }
    fn title_button(&self, k: &Kit, rect: Rect, glyph: Glyph, st: St) -> Vec<Shape> {
        let p = k.look.paint;
        // The classic Mac OS colours its boxes itself; everywhere else a
        // window keeps its buttons' colours with the keyboard elsewhere too.
        let to = k
            .button_colour(glyph)
            .filter(|_| k.look.skin != Skin::System);
        let paint = |r| self.0.title_button(k, r, glyph, st);
        self.memo_then(k, "title_button", rect, (glyph, st), paint, |shapes| {
            let mut shapes = retint_but(shapes, to, p.icon);
            // XP's close, minimize and maximize carry their white marks in
            // their bitmaps; the marks are the white in them.
            if let (Some(ink), Skin::Xp) = (p.icon, k.look.skin) {
                for shape in &mut shapes {
                    recolour(shape, &mut |c| {
                        if c.a() > 0 && neutral(c) && luma(c) > 0.85 {
                            ink.gamma_multiply(f32::from(c.a()) / 255.0)
                        } else {
                            c
                        }
                    });
                }
            }
            shapes
        })
    }
    fn scroll_track(&self, k: &Kit, rect: Rect, vertical: bool, st: St) -> Vec<Shape> {
        self.memo_then(
            k,
            "scroll_track",
            rect,
            (vertical, st),
            |r| self.0.scroll_track(k, r, vertical, st),
            |v| retint(v, k.look.paint.track),
        )
    }
    fn scroll_thumb(&self, k: &Kit, rect: Rect, vertical: bool, st: St) -> Vec<Shape> {
        self.memo_then(
            k,
            "scroll_thumb",
            rect,
            (vertical, st),
            |r| self.0.scroll_thumb(k, r, vertical, st),
            |v| retint(v, k.look.paint.handle),
        )
    }
    fn scroll_arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        self.memo(k, "scroll_arrow", rect, (dir, st), |r| {
            self.0.scroll_arrow(k, r, dir, st)
        })
    }
    fn scroll_corner(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        self.memo_then(
            k,
            "scroll_corner",
            rect,
            (),
            |r| self.0.scroll_corner(k, r),
            |v| retint(v, k.look.paint.track),
        )
    }
    fn menu(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        self.memo(k, "menu", rect, (), |r| self.0.menu(k, r))
    }
    fn menu_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        self.memo(k, "menu_item", rect, st, |r| self.0.menu_item(k, r, st))
    }
    fn separator(&self, k: &Kit, rect: Rect, vertical: bool) -> Vec<Shape> {
        self.memo(k, "separator", rect, vertical, |r| {
            self.0.separator(k, r, vertical)
        })
    }
    fn tooltip(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        self.memo(k, "tooltip", rect, (), |r| self.0.tooltip(k, r))
    }
    fn list(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // The sidebar's list, a card of its own: glazed as a card is.
        let pane = glass(k).map(f32::to_bits);
        self.memo_then(
            k,
            "list",
            rect,
            pane,
            |r| self.0.list(k, r),
            |v| glazed(k, rect, v),
        )
    }
    fn list_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        self.memo(k, "list_item", rect, st, |r| self.0.list_item(k, r, st))
    }
    fn progress(&self, k: &Kit, rect: Rect, fraction: f32) -> Vec<Shape> {
        // A bar that moves would fill the cache with every step of it.
        toned(k, self.0.progress(k, rect, fraction))
    }
    fn status_bar(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // The strip the page's words are written on: the page's colour.
        let face = k.look.paint.face.filter(|_| k.look.skin != Skin::Aqua);
        self.memo_then(
            k,
            "status_bar",
            rect,
            (),
            |r| self.0.status_bar(k, r),
            |v| retint(v, face),
        )
    }
    fn status_field(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        // The message is written in the page's ink, so its field is the
        // page's colour, sunk as the skin sinks it.
        let face = k.look.paint.face.filter(|_| k.look.skin != Skin::Aqua);
        self.memo_then(
            k,
            "status_field",
            rect,
            (),
            |r| self.0.status_field(k, r),
            |v| retint(v, face),
        )
    }
    fn focus(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        self.memo(k, "focus", rect, (), |r| self.0.focus(k, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn darkening_keeps_the_order_of_the_greys_and_leaves_the_blues() {
        let white = darken(Color32::WHITE);
        let cream = darken(Color32::from_rgb(0xec, 0xe9, 0xd8));
        let shade = darken(Color32::from_rgb(0xd8, 0xd0, 0xc4));
        assert!(luma(white) > luma(cream) && luma(cream) > luma(shade));
        assert!(luma(white) < 0.35);
        let blue = Color32::from_rgb(0x00, 0x53, 0xee);
        assert_eq!(darken(blue), blue);
    }

    #[test]
    fn an_accent_takes_the_skins_blue_tone_and_all_and_keeps_its_gradient() {
        let navy = Color32::from_rgb(0x00, 0x00, 0x80);
        let light = Color32::from_rgb(0x10, 0x84, 0xd0);
        let accent = Color32::from_rgb(0x2e, 0xa0, 0x43);
        // The selection's own blue becomes the accent exactly...
        assert_eq!(retint_colour(navy, accent, luma(navy)), accent);
        // ...and the gradient's lighter end a lighter shade of it.
        let end = retint_colour(light, accent, luma(navy));
        assert!(luma(end) > luma(accent));
        assert!(end.g() > end.r() && end.g() > end.b(), "{end:?}");
    }

    /// A painter's shapes come from the cache moved to the rect asked for,
    /// exactly where painting them there would have put them.
    #[test]
    fn a_cached_control_lands_where_a_fresh_one_would() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            for skin in [Skin::Xp, Skin::Aqua, Skin::Win98] {
                let look = super::super::Look {
                    skin,
                    ..Default::default()
                };
                super::super::set(ctx, look);
                let k = Kit::of(ctx).unwrap();
                let rect = Rect::from_min_size(pos2(37.0, 51.0), vec2(80.0, 24.0));
                let fresh = skin.style().button(&k, rect, St::default());
                let _ = k
                    .style()
                    .button(&k, rect.translate(vec2(100.0, 3.0)), St::default());
                let cached = k.style().button(&k, rect, St::default());
                let bounds = |v: &[Shape]| {
                    v.iter()
                        .map(Shape::visual_bounding_rect)
                        .fold(Rect::NOTHING, |a, b| a.union(b))
                };
                assert_eq!(bounds(&fresh), bounds(&cached), "{skin:?}");
            }
        });
    }
}
