//! The few shapes every skin is built from, made to land on physical pixels.
//!
//! The CSS the skins reproduce is drawn by a browser that snaps every border
//! to the device's pixels. egui does not: a 1-point line placed half a point
//! inside an edge is crisp at 100% and a smear at 125%, where it covers one
//! pixel fully and the next by a quarter. So nothing here is a stroke. A line
//! is a filled rect whose edges are whole physical pixels, and a CSS pixel is
//! a whole number of physical pixels - one at 100% and 125%, two at 150% and
//! 200% - which is the choice a browser makes at those scales too.

use egui::epaint::{Mesh, RectShape, TessellationOptions, Tessellator};
use egui::{pos2, vec2, Color32, Pos2, Rect, Rounding, Shape, TextureId};

/// One CSS pixel, in points, at `ppp` physical pixels per point: a whole
/// number of physical pixels, and never none.
pub fn unit(ppp: f32) -> f32 {
    ppp.round().max(1.0) / ppp
}

/// `v` moved to the nearest physical pixel boundary.
pub fn snap(v: f32, ppp: f32) -> f32 {
    (v * ppp).round() / ppp
}

/// `rect` with every edge on a physical pixel boundary.
pub fn snap_rect(rect: Rect, ppp: f32) -> Rect {
    Rect::from_min_max(
        pos2(snap(rect.min.x, ppp), snap(rect.min.y, ppp)),
        pos2(snap(rect.max.x, ppp), snap(rect.max.y, ppp)),
    )
}

/// A flat rect, square-cornered.
pub fn fill(rect: Rect, colour: Color32) -> Shape {
    Shape::rect_filled(rect, Rounding::ZERO, colour)
}

/// One zero-blur inset `box-shadow` layer: offset in CSS pixels, and colour.
pub type Inset = (i8, i8, Color32);

/// A stack of zero-blur inset shadows, exactly as CSS paints it.
///
/// An inset shadow offset by (1, 1) shows as a band one pixel wide along the
/// top and down the left, and one offset by (-2, -2) as a band two wide along
/// the bottom and the right. Where two overlap - the corners - the one listed
/// first is on top, which is why the stack is painted back to front. Every
/// border 98.css draws is one of these stacks, and so is much of XP.css.
pub fn inset(rect: Rect, layers: &[Inset], ppp: f32) -> Vec<Shape> {
    let u = unit(ppp);
    let r = snap_rect(rect, ppp);
    let mut out = Vec::with_capacity(layers.len() * 2);
    for &(dx, dy, colour) in layers.iter().rev() {
        if colour.a() == 0 {
            continue;
        }
        let w = f32::from(dx.unsigned_abs()) * u;
        let h = f32::from(dy.unsigned_abs()) * u;
        if dy > 0 {
            out.push(fill(Rect::from_min_size(r.min, vec2(r.width(), h)), colour));
        } else if dy < 0 {
            out.push(fill(
                Rect::from_min_max(pos2(r.min.x, r.max.y - h), r.max),
                colour,
            ));
        }
        if dx > 0 {
            out.push(fill(
                Rect::from_min_size(r.min, vec2(w, r.height())),
                colour,
            ));
        } else if dx < 0 {
            out.push(fill(
                Rect::from_min_max(pos2(r.max.x - w, r.min.y), r.max),
                colour,
            ));
        }
    }
    out
}

/// `rect` brought in by `n` CSS pixels on every side.
pub fn inner(rect: Rect, n: f32, ppp: f32) -> Rect {
    snap_rect(rect, ppp).shrink(n * unit(ppp))
}

/// Pixel art: one character per CSS pixel, `.` or space for none, each other
/// character looked up in `ink`. Runs of one colour along a row are merged
/// into one rect, so a 16-pixel arrow is a dozen shapes rather than a hundred.
pub fn sprite(at: Pos2, rows: &[&str], ink: &[(char, Color32)], ppp: f32) -> Vec<Shape> {
    let u = unit(ppp);
    let at = pos2(snap(at.x, ppp), snap(at.y, ppp));
    let mut out = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        let chars: Vec<char> = row.chars().collect();
        let mut x = 0;
        while x < chars.len() {
            let c = chars[x];
            let mut end = x + 1;
            while end < chars.len() && chars[end] == c {
                end += 1;
            }
            if let Some(&(_, colour)) = ink.iter().find(|(k, _)| *k == c) {
                out.push(fill(
                    Rect::from_min_size(
                        pos2(at.x + x as f32 * u, at.y + y as f32 * u),
                        vec2((end - x) as f32 * u, u),
                    ),
                    colour,
                ));
            }
            x = end;
        }
    }
    out
}

/// The size of a sprite, in points.
pub fn sprite_size(rows: &[&str], ppp: f32) -> egui::Vec2 {
    let u = unit(ppp);
    let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    vec2(w as f32 * u, rows.len() as f32 * u)
}

/// Where a sprite of `rows` sits centred in `rect`, on the pixel grid.
pub fn centred(rect: Rect, rows: &[&str], ppp: f32) -> Pos2 {
    let size = sprite_size(rows, ppp);
    let at = rect.center() - size / 2.0;
    pos2(snap(at.x, ppp), snap(at.y, ppp))
}

/// Which way a gradient runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Top to bottom - CSS `180deg`, the default.
    Down,
    /// Left to right - CSS `90deg`.
    Across,
}

/// A CSS colour stop: where (0 to 1 along the axis) and what colour. Two
/// stops at the same place make a hard edge, as `#fff 45%, #eee 45%` does.
pub type Stop = (f32, Color32);

/// The gradient's colour at `t`, mixed in gamma space the way a browser mixes
/// sRGB stops, with premultiplied alpha.
pub fn colour_at(stops: &[Stop], t: f32) -> Color32 {
    let Some(&(first_at, first)) = stops.first() else {
        return Color32::TRANSPARENT;
    };
    if t <= first_at {
        return first;
    }
    for pair in stops.windows(2) {
        let (a_at, a) = pair[0];
        let (b_at, b) = pair[1];
        if t <= b_at {
            if b_at - a_at <= f32::EPSILON {
                return b;
            }
            return a.lerp_to_gamma(b, (t - a_at) / (b_at - a_at));
        }
    }
    stops.last().map_or(Color32::TRANSPARENT, |s| s.1)
}

/// A square-cornered gradient, exact at every stop: one band of two
/// triangles between each pair of stops, so a hard stop stays hard.
pub fn gradient(rect: Rect, axis: Axis, stops: &[Stop]) -> Shape {
    let mut mesh = Mesh::default();
    strips(&mut mesh, rect, rect, axis, stops);
    Shape::mesh(mesh)
}

/// Bands of `stops` laid along `axis` over `whole`, clipped to `part` - so a
/// piece of a gradient can be drawn and still meet the rest of it.
fn strips(mesh: &mut Mesh, whole: Rect, part: Rect, axis: Axis, stops: &[Stop]) {
    let (from, to) = match axis {
        Axis::Down => (whole.top(), whole.bottom()),
        Axis::Across => (whole.left(), whole.right()),
    };
    let (lo, hi) = match axis {
        Axis::Down => (part.top(), part.bottom()),
        Axis::Across => (part.left(), part.right()),
    };
    if hi <= lo || to <= from {
        return;
    }
    let along = |t: f32| from + t * (to - from);
    let mut cuts: Vec<f32> = vec![lo, hi];
    for &(at, _) in stops {
        let p = along(at);
        if p > lo && p < hi {
            cuts.push(p);
        }
    }
    cuts.sort_by(f32::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
    let t_of = |p: f32| (p - from) / (to - from);
    for pair in cuts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        // Just inside each end of the band, so a hard stop at `a` takes the
        // colour after it and one at `b` the colour before it.
        let ca = colour_at(stops, t_of(a) + 1e-5);
        let cb = colour_at(stops, t_of(b) - 1e-5);
        let base = mesh.vertices.len() as u32;
        match axis {
            Axis::Down => {
                mesh.colored_vertex(pos2(part.left(), a), ca);
                mesh.colored_vertex(pos2(part.right(), a), ca);
                mesh.colored_vertex(pos2(part.left(), b), cb);
                mesh.colored_vertex(pos2(part.right(), b), cb);
            }
            Axis::Across => {
                mesh.colored_vertex(pos2(a, part.top()), ca);
                mesh.colored_vertex(pos2(a, part.bottom()), ca);
                mesh.colored_vertex(pos2(b, part.top()), cb);
                mesh.colored_vertex(pos2(b, part.bottom()), cb);
            }
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base + 1, base + 3, base + 2);
    }
}

/// A gradient with rounded corners.
///
/// egui tessellates a rounded rect as a fan from its outline, whose colours
/// can only blend from edge to edge - a hard stop in the middle would be
/// smeared over the whole height. So the outline is tessellated for its
/// anti-aliased corners and recoloured by position, and the parts clear of
/// the corners are laid over it in exact bands.
pub fn rounded_gradient(
    rect: Rect,
    rounding: Rounding,
    axis: Axis,
    stops: &[Stop],
    ppp: f32,
) -> Shape {
    if rounding == Rounding::ZERO {
        return gradient(rect, axis, stops);
    }
    let mut mesh = Mesh::default();
    let mut tess = Tessellator::new(ppp, TessellationOptions::default(), [1, 1], vec![]);
    tess.tessellate_rect(
        &RectShape::filled(rect, rounding, Color32::WHITE),
        &mut mesh,
    );
    for v in &mut mesh.vertices {
        let t = match axis {
            Axis::Down => (v.pos.y - rect.top()) / rect.height().max(1e-3),
            Axis::Across => (v.pos.x - rect.left()) / rect.width().max(1e-3),
        };
        let coverage = f32::from(v.color.a()) / 255.0;
        v.color = colour_at(stops, t.clamp(0.0, 1.0)).gamma_multiply(coverage);
    }
    let r = rounding
        .nw
        .max(rounding.ne)
        .max(rounding.sw)
        .max(rounding.se);
    // The middle, full width, between the corners' rows...
    let middle = Rect::from_min_max(
        pos2(rect.left(), rect.top() + r),
        pos2(rect.right(), rect.bottom() - r),
    );
    // ...and the top and bottom bands between the corners' columns.
    let top = Rect::from_min_max(
        pos2(rect.left() + r, rect.top()),
        pos2(rect.right() - r, rect.top() + r),
    );
    let bottom = Rect::from_min_max(
        pos2(rect.left() + r, rect.bottom() - r),
        pos2(rect.right() - r, rect.bottom()),
    );
    for part in [middle, top, bottom] {
        if part.is_positive() {
            // Inset by the feathering, so the exact bands stay inside the
            // anti-aliased edge rather than drawing a hard one over it.
            let part = if part == middle {
                Rect::from_min_max(
                    pos2(part.left() + 0.5 / ppp, part.top()),
                    pos2(part.right() - 0.5 / ppp, part.bottom()),
                )
            } else if part == top {
                Rect::from_min_max(pos2(part.left(), part.top() + 0.5 / ppp), part.max)
            } else {
                Rect::from_min_max(part.min, pos2(part.right(), part.bottom() - 0.5 / ppp))
            };
            strips(&mut mesh, rect, part, axis, stops);
        }
    }
    Shape::mesh(mesh)
}

/// A rounded outline `n` CSS pixels wide, its outer edge on `rect`.
pub fn outline(rect: Rect, rounding: Rounding, colour: Color32, ppp: f32) -> Shape {
    let u = unit(ppp);
    let r = snap_rect(rect, ppp);
    Shape::rect_stroke(r.shrink(u / 2.0), rounding, egui::Stroke::new(u, colour))
}

/// A one-pixel dotted rectangle, the focus mark of every Windows control up
/// to 7: alternate pixels along each edge.
pub fn dotted(rect: Rect, colour: Color32, ppp: f32) -> Vec<Shape> {
    let u = unit(ppp);
    let r = snap_rect(rect, ppp);
    let mut out = Vec::new();
    let cols = (r.width() / u).round() as i32;
    let rows = (r.height() / u).round() as i32;
    for i in (0..cols).step_by(2) {
        let x = r.left() + i as f32 * u;
        out.push(fill(
            Rect::from_min_size(pos2(x, r.top()), vec2(u, u)),
            colour,
        ));
        out.push(fill(
            Rect::from_min_size(pos2(x, r.bottom() - u), vec2(u, u)),
            colour,
        ));
    }
    for j in (0..rows).step_by(2) {
        let y = r.top() + j as f32 * u;
        out.push(fill(
            Rect::from_min_size(pos2(r.left(), y), vec2(u, u)),
            colour,
        ));
        out.push(fill(
            Rect::from_min_size(pos2(r.right() - u, y), vec2(u, u)),
            colour,
        ));
    }
    out
}

/// A repeating texture over `rect`, one texel per CSS pixel, anchored at the
/// rect's corner - the dither of a Windows 98 scroll track.
pub fn tiled(rect: Rect, texture: TextureId, texels: egui::Vec2, ppp: f32) -> Shape {
    let u = unit(ppp);
    let r = snap_rect(rect, ppp);
    let uv = Rect::from_min_max(
        Pos2::ZERO,
        pos2(r.width() / u / texels.x, r.height() / u / texels.y),
    );
    Shape::image(texture, r, uv, Color32::WHITE)
}

/// A filled polygon - a slider's pointer, an arrow - from points in CSS
/// pixels relative to `at`.
pub fn polygon(at: Pos2, points: &[(f32, f32)], colour: Color32, ppp: f32) -> Shape {
    let u = unit(ppp);
    let at = pos2(snap(at.x, ppp), snap(at.y, ppp));
    Shape::convex_polygon(
        points
            .iter()
            .map(|&(x, y)| pos2(at.x + x * u, at.y + y * u))
            .collect(),
        colour,
        egui::Stroke::NONE,
    )
}

/// A bitmap drawn over `rect`, edges on the pixel grid, `tint` multiplying it
/// - white for the bitmap as it is, white at half alpha for half opacity.
pub fn bitmap(rect: Rect, texture: TextureId, tint: Color32, ppp: f32) -> Shape {
    let uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
    Shape::image(texture, snap_rect(rect, ppp), uv, tint)
}

/// A bitmap in three slices along `rect`: its two caps at their own width
/// and the middle tile stretched between them - which, for the one-pixel
/// tiles the Aqua bitmaps repeat, is the same thing.
pub fn three(
    rect: Rect,
    caps: (f32, f32),
    tiles: (TextureId, TextureId, TextureId),
    tint: Color32,
    vertical: bool,
    ppp: f32,
) -> Vec<Shape> {
    let u = unit(ppp);
    let r = snap_rect(rect, ppp);
    let (a, b) = (caps.0 * u, caps.1 * u);
    let (first, middle, last) = if vertical {
        (
            Rect::from_min_max(r.min, pos2(r.right(), r.top() + a)),
            Rect::from_min_max(pos2(r.left(), r.top() + a), pos2(r.right(), r.bottom() - b)),
            Rect::from_min_max(pos2(r.left(), r.bottom() - b), r.max),
        )
    } else {
        (
            Rect::from_min_max(r.min, pos2(r.left() + a, r.bottom())),
            Rect::from_min_max(pos2(r.left() + a, r.top()), pos2(r.right() - b, r.bottom())),
            Rect::from_min_max(pos2(r.right() - b, r.top()), r.max),
        )
    };
    let mut out = Vec::with_capacity(3);
    if middle.is_positive() {
        out.push(bitmap(middle, tiles.1, tint, ppp));
    }
    out.push(bitmap(first, tiles.0, tint, ppp));
    out.push(bitmap(last, tiles.2, tint, ppp));
    out
}

/// Paints `rect` one physical pixel at a time, `shade` giving the colour -
/// premultiplied - at a point in CSS pixels from the rect's corner.
///
/// For what no primitive draws: a glossy bead lit from off its centre, a
/// tab whose rows of colour run into rounded corners. A pixel whose corners
/// and centre all agree is that colour; one where they do not - an edge - is
/// the average of nine samples, which is the anti-aliasing. Runs of one
/// colour along a row are one rect.
pub fn raster(rect: Rect, ppp: f32, shade: impl Fn(f32, f32) -> Color32) -> Shape {
    let r = snap_rect(rect, ppp);
    if !r.is_finite() || r.width() * ppp > 16384.0 || r.height() * ppp > 16384.0 {
        return Shape::Noop;
    }
    let u = unit(ppp);
    let cols = (r.width() * ppp).round() as i32;
    let rows = (r.height() * ppp).round() as i32;
    // A physical pixel, in CSS pixels.
    let step = 1.0 / (u * ppp);
    let sample = |i: i32, j: i32| -> Color32 {
        let at = |dx: f32, dy: f32| shade((i as f32 + dx) * step, (j as f32 + dy) * step);
        let centre = at(0.5, 0.5);
        if [at(0.0, 0.0), at(1.0, 0.0), at(0.0, 1.0), at(1.0, 1.0)]
            .iter()
            .all(|c| *c == centre)
        {
            return centre;
        }
        let mut sum = [0u32; 4];
        for sy in [1.0 / 6.0, 0.5, 5.0 / 6.0] {
            for sx in [1.0 / 6.0, 0.5, 5.0 / 6.0] {
                for (total, v) in sum.iter_mut().zip(at(sx, sy).to_array()) {
                    *total += u32::from(v);
                }
            }
        }
        let [r, g, b, a] = sum.map(|v| ((v + 4) / 9) as u8);
        Color32::from_rgba_premultiplied(r, g, b, a)
    };
    let mut mesh = Mesh::default();
    let px = 1.0 / ppp;
    for j in 0..rows {
        let mut i = 0;
        while i < cols {
            let colour = sample(i, j);
            let mut end = i + 1;
            while end < cols && sample(end, j) == colour {
                end += 1;
            }
            if colour.a() > 0 {
                mesh.add_colored_rect(
                    Rect::from_min_max(
                        pos2(r.left() + i as f32 * px, r.top() + j as f32 * px),
                        pos2(r.left() + end as f32 * px, r.top() + (j + 1) as f32 * px),
                    ),
                    colour,
                );
            }
            i = end;
        }
    }
    Shape::mesh(mesh)
}

/// A box of `rect` with corner radii `[nw, ne, sw, se]` in CSS pixels: its
/// body the colour `body` gives for each CSS row, inside `rings` - one-pixel
/// edges from the outside in, as a border and a stack of `inset 0 0 0 1px`
/// shadows draw them.
///
/// The straight rows are a rect a run; only the corner squares are painted a
/// pixel at a time, which keeps a box the width of a window from costing a
/// raster of the whole of it every frame.
pub fn boxed(
    rect: Rect,
    radius: [f32; 4],
    body: &dyn Fn(f32) -> Color32,
    rings: &[Color32],
    ppp: f32,
) -> Vec<Shape> {
    let u = unit(ppp);
    let r = snap_rect(rect, ppp);
    if !r.is_finite() || r.width() * ppp > 16384.0 || r.height() * ppp > 16384.0 {
        return Vec::new();
    }
    let size = r.size() / u;
    let px = 1.0 / ppp;
    let rows = (r.height() * ppp).round() as i32;
    let n = rings.len();
    let [nw, ne, sw, se] = radius;
    let mut mesh = Mesh::default();
    let mut put = |x0: f32, x1: f32, top: f32, colour: Color32| {
        if x1 > x0 && colour.a() > 0 {
            mesh.add_colored_rect(
                Rect::from_min_max(pos2(x0, top), pos2(x1, top + px)),
                colour,
            );
        }
    };
    for j in 0..rows {
        let y = (j as f32 + 0.5) * px / u;
        let top = r.top() + j as f32 * px;
        let cut_l = if y < nw {
            nw
        } else if y > size.y - sw {
            sw
        } else {
            0.0
        };
        let cut_r = if y < ne {
            ne
        } else if y > size.y - se {
            se
        } else {
            0.0
        };
        // Which ring a whole row is, counting in from the nearer edge.
        let row_ring = y.min(size.y - y).floor().max(0.0) as usize;
        let middle = if row_ring < n {
            rings[row_ring]
        } else {
            body(y)
        };
        let inset = n as f32;
        for k in 0..n {
            let colour = rings[k.min(row_ring)];
            if (k as f32) >= cut_l {
                let x = r.left() + k as f32 * u;
                put(x, x + u, top, colour);
            }
            if (k as f32) >= cut_r {
                let x = r.right() - (k + 1) as f32 * u;
                put(x, x + u, top, colour);
            }
        }
        put(
            r.left() + inset.max(cut_l) * u,
            r.right() - inset.max(cut_r) * u,
            top,
            middle,
        );
    }
    let mut out = vec![Shape::mesh(mesh)];
    let shade = |x: f32, y: f32| -> Color32 {
        if !inside_rounded(x, y, size, radius) {
            return Color32::TRANSPARENT;
        }
        for (i, ring) in rings.iter().enumerate() {
            let d = (i + 1) as f32;
            let inner = radius.map(|v| (v - d).max(0.0));
            if !inside_rounded(x - d, y - d, size - egui::vec2(2.0 * d, 2.0 * d), inner) {
                return *ring;
            }
        }
        body(y)
    };
    let corners = [
        (r.min, nw),
        (pos2(r.right() - ne * u, r.top()), ne),
        (pos2(r.left(), r.bottom() - sw * u), sw),
        (pos2(r.right() - se * u, r.bottom() - se * u), se),
    ];
    for (at, radius) in corners {
        if radius <= 0.0 {
            continue;
        }
        let square = Rect::from_min_size(at, egui::vec2(radius, radius) * u);
        let offset = (square.min - r.min) / u;
        out.push(raster(square, ppp, |x, y| {
            shade(x + offset.x, y + offset.y)
        }));
    }
    out
}

/// `top` laid over `under`, both premultiplied: CSS's source-over.
pub fn over(top: Color32, under: Color32) -> Color32 {
    let [tr, tg, tb, ta] = top.to_array().map(u32::from);
    let [ur, ug, ub, ua] = under.to_array().map(u32::from);
    let keep = 255 - ta;
    let mix = |t: u32, u: u32| (t + (u * keep + 127) / 255).min(255) as u8;
    Color32::from_rgba_premultiplied(mix(tr, ur), mix(tg, ug), mix(tb, ub), mix(ta, ua))
}

/// How much of a point at (`x`, `y`) lies inside a rounded rect of `size`
/// with corner radius `radius`: 1 inside, 0 outside, by the signed distance
/// - a sample, which `raster`'s supersampling smooths at the edge.
pub fn inside_rounded(x: f32, y: f32, size: egui::Vec2, radius: [f32; 4]) -> bool {
    if x < 0.0 || y < 0.0 || x > size.x || y > size.y {
        return false;
    }
    // nw, ne, sw, se
    let corner = |cx: f32, cy: f32, r: f32| (x - cx).powi(2) + (y - cy).powi(2) <= r * r;
    let [nw, ne, sw, se] = radius;
    if x < nw && y < nw {
        return corner(nw, nw, nw);
    }
    if x > size.x - ne && y < ne {
        return corner(size.x - ne, ne, ne);
    }
    if x < sw && y > size.y - sw {
        return corner(sw, size.y - sw, sw);
    }
    if x > size.x - se && y > size.y - se {
        return corner(size.x - se, size.y - se, se);
    }
    true
}

/// A colour from `#rrggbb`, for the tables of a skin. Panics on a typo, which
/// a skin's own tests catch.
pub const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// The same with alpha, `rgba(r, g, b, a)` as CSS writes it.
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, (a.clamp(0.0, 1.0) * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rects(shapes: &[Shape]) -> Vec<(Rect, Color32)> {
        shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Rect(r) => Some((r.rect, r.fill)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_css_pixel_is_a_whole_number_of_physical_pixels_at_every_scale() {
        for ppp in [1.0, 1.25, 1.5, 1.75, 2.0, 2.5] {
            let px = unit(ppp) * ppp;
            assert!((px - px.round()).abs() < 1e-4, "{ppp}: {px}");
            assert!(px >= 1.0);
        }
    }

    /// The first shadow listed is on top where two meet, which is what makes
    /// a 98 button's top-right pixel dark rather than white.
    #[test]
    fn the_first_inset_shadow_listed_wins_the_corners() {
        let black = Color32::BLACK;
        let white = Color32::WHITE;
        let shapes = inset(
            Rect::from_min_size(Pos2::ZERO, vec2(10.0, 10.0)),
            &[(-1, -1, black), (1, 1, white)],
            1.0,
        );
        let painted = rects(&shapes);
        let last = painted.last().expect("something painted");
        assert_eq!(last.1, black, "the first layer is painted last");
        assert_eq!(painted.len(), 4);
    }

    #[test]
    fn a_sprite_merges_each_run_into_one_rect() {
        let shapes = sprite(
            Pos2::ZERO,
            &["##..#", ".###."],
            &[('#', Color32::BLACK)],
            1.0,
        );
        assert_eq!(shapes.len(), 3);
    }

    #[test]
    fn a_hard_stop_stays_hard() {
        let stops = [
            (0.0, Color32::WHITE),
            (0.5, Color32::WHITE),
            (0.5, Color32::BLACK),
            (1.0, Color32::BLACK),
        ];
        assert_eq!(colour_at(&stops, 0.49), Color32::WHITE);
        assert_eq!(colour_at(&stops, 0.51), Color32::BLACK);
    }
}
