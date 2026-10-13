//! The logo on the About page, its glow alive behind the ring.
//!
//! The icon is drawn with its glow baked in. Split by opacity, it comes apart
//! into the ring - nearly opaque - and the glow - the faint halo round it -
//! and the glow can then be drawn behind the ring on its own. It is meant to be
//! a detail, noticed only after looking a moment: the glow barely breathes, a
//! faint bloom carries it a little past where the icon stops, and a few dim
//! lights drift round it both ways, like traffic on a highway seen from far
//! off. The ring stays still: it is the thing being looked at.

use std::f32::consts::{PI, TAU};
use std::time::Duration;

use egui::epaint::{Mesh, Vertex};
use egui::{Color32, Context, Pos2, Rect, Sense, Shape, TextureHandle, TextureId, Ui, Vec2};

/// Opacity below which a pixel is all glow, and above which all ring. The
/// icon tool cuts the small icons round 140, what can be seen of the ring.
const GLOW_BELOW: f32 = 110.0;
const RING_ABOVE: f32 = 170.0;

/// One breath, in seconds: slow, and shallow enough to be felt rather than
/// seen.
const BREATH: f64 = 6.0;
/// One sway of the colour from cyan to magenta and back.
const SWAY: f64 = 11.0;

/// The bloom past the icon's own halo: each copy of the glow larger and
/// fainter than the last, a blur for the price of a few quads. Small steps,
/// so no copy's edge shows as a contour of its own.
const BLOOM: [(f32, f32); 4] = [(1.06, 0.13), (1.12, 0.10), (1.18, 0.07), (1.25, 0.04)];

/// The traffic: seconds for one lap (negative going the other way), seconds
/// for one light to come and go, and where each starts. Periods that share
/// no factor keep it from ever settling into a pattern.
const LIGHTS: [(f64, f64, f64); 4] = [
    (9.0, 7.0, 0.0),
    (13.0, 11.0, 2.1),
    (-11.0, 8.5, 4.0),
    (-17.0, 13.0, 5.3),
];
/// How bright a light is at its brightest, against the glow it rides on.
const LIGHT_PEAK: f32 = 0.42;
/// Half-width of a light along the ring, in radians.
const LIGHT_SPREAD: f32 = 0.38;
/// How far past the glow the lights spill, against the glow's own size.
const LIGHT_REACH: f32 = 1.08;
/// Slices of the fan the lights are drawn on: enough that a light glides
/// rather than stepping from slice to slice.
const SLICES: u32 = 96;

/// Draws the logo `side` points square, glowing, and asks for the next frame
/// while it is in view.
pub fn show(ui: &mut Ui, side: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let (ring, glow) = textures(ui.ctx());
    let minimized = ui.input(|i| i.viewport().minimized.unwrap_or(false));
    let time = if minimized { 0.0 } else { ui.input(|i| i.time) };
    let look = Look::at(time);
    // Light on a dark page only adds: blended over, every copy of the glow
    // also covered the page a little, and their stacked edges read as a dark
    // rim round the halo. On a light page adding saturates to nothing, and
    // covering is what shows.
    let additive = ui.visuals().dark_mode;
    let painter = ui.painter();
    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    for (scale, alpha) in BLOOM {
        let wide = Rect::from_center_size(rect.center(), rect.size() * scale * look.scale);
        painter.image(glow.id(), wide, uv, light(look.tint, alpha, additive));
    }
    let halo = Rect::from_center_size(rect.center(), rect.size() * look.scale);
    // Past full is drawn twice: a texture cannot be drawn brighter than it
    // is, only over itself.
    let mut left = look.intensity;
    while left > 0.0 {
        painter.image(glow.id(), halo, uv, light(look.tint, left, additive));
        left -= 1.0;
    }
    // The glow once more over itself - a texture cannot be drawn brighter
    // than it is, only twice - but only where a light is passing.
    painter.add(Shape::mesh(traffic(
        glow.id(),
        rect.center(),
        side * look.scale * LIGHT_REACH,
        look.tint,
        additive,
        time,
    )));
    painter.image(ring.id(), rect, uv, Color32::WHITE);
    if !minimized {
        // Only while it is drawn: the About page closed, nothing asks.
        ui.ctx().request_repaint_after(Duration::from_millis(33));
    }
}

/// How the glow looks at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Look {
    /// 1 is the glow as the icon has it.
    intensity: f32,
    /// Of the halo against the ring.
    scale: f32,
    tint: Color32,
}

impl Look {
    fn at(time: f64) -> Look {
        let breath = (0.5 - 0.5 * (time * std::f64::consts::TAU / BREATH).cos()) as f32;
        let sway = (0.5 + 0.5 * (time * std::f64::consts::TAU / SWAY).sin()) as f32;
        let lerp = |a: f32, b: f32| (a + (b - a) * sway).round() as u8;
        Look {
            intensity: 0.94 + 0.12 * breath,
            scale: 1.0 + 0.015 * breath,
            // Multiplied into the glow, so it can only take colour away: less
            // red leans it cyan, less green leans it magenta.
            tint: Color32::from_rgb(lerp(228.0, 255.0), lerp(255.0, 232.0), 255),
        }
    }
}

/// `tint` at `strength` of full, drawn over the page or added to it.
fn light(tint: Color32, strength: f32, additive: bool) -> Color32 {
    // `gamma_multiply` asserts its factor is within 0..=1.
    let color = tint.gamma_multiply(strength.clamp(0.0, 1.0));
    if additive {
        // Premultiplied with no alpha: egui's blend then adds the colour
        // and keeps all of what was under it.
        Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 0)
    } else {
        color
    }
}

/// How much the passing lights brighten the glow at `angle` round the ring.
fn lights_at(angle: f32, time: f64) -> f32 {
    LIGHTS
        .iter()
        .map(|&(lap, life, start)| {
            // Wrapped before narrowing: `time` counts from the app's start,
            // and days' worth of laps leave an f32 no precision to move a
            // frame's worth by, so the lights would stutter on a long uptime.
            let at = (start + time * std::f64::consts::TAU / lap).rem_euclid(std::f64::consts::TAU)
                as f32;
            // A light comes into view, passes, and is gone for as long again.
            let fade = (time * std::f64::consts::PI / life + start).sin().max(0.0) as f32;
            let off = (angle - at + PI).rem_euclid(TAU) - PI;
            LIGHT_PEAK * fade * fade * (-(off / LIGHT_SPREAD).powi(2)).exp()
        })
        .sum::<f32>()
        // Two crossing are one brighter flash, not a flare.
        .min(LIGHT_PEAK * 1.4)
}

/// The glow as a fan round `center`, `side` points square, dark but where a
/// light is passing. The brightness is in the vertex colours, so the whole of
/// the traffic is one mesh.
fn traffic(
    texture: TextureId,
    center: Pos2,
    side: f32,
    tint: Color32,
    additive: bool,
    time: f64,
) -> Mesh {
    let mut mesh = Mesh::with_texture(texture);
    // Out to the corners, so the fan covers the square; past its edge the
    // texture clamps to its last pixels, which are transparent.
    let reach = side * std::f32::consts::FRAC_1_SQRT_2;
    let vertex = |offset: Vec2, alpha: f32| Vertex {
        pos: center + offset,
        uv: Pos2::new(0.5, 0.5) + offset / side,
        color: light(tint, alpha, additive),
    };
    mesh.vertices.push(vertex(Vec2::ZERO, 0.0));
    for slice in 0..=SLICES {
        let angle = slice as f32 / SLICES as f32 * TAU;
        mesh.vertices
            .push(vertex(Vec2::angled(angle) * reach, lights_at(angle, time)));
    }
    for slice in 1..=SLICES {
        mesh.add_triangle(0, slice, slice + 1);
    }
    mesh
}

/// The ring and the glow, split once and kept in the context.
fn textures(ctx: &Context) -> (TextureHandle, TextureHandle) {
    let id = egui::Id::new("nit-about-logo");
    if let Some(found) = ctx.data(|d| d.get_temp::<(TextureHandle, TextureHandle)>(id)) {
        return found;
    }
    let image = image::load_from_memory(include_bytes!("../../assets/icon.ico"))
        .map(|image| image.into_rgba8())
        .unwrap_or_else(|_| image::RgbaImage::new(1, 1));
    let size = [image.width() as usize, image.height() as usize];
    let (ring, glow) = split(image.as_raw());
    let load = |name: &str, pixels: Vec<u8>| {
        ctx.load_texture(
            name,
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            egui::TextureOptions::LINEAR,
        )
    };
    let found = (load("about-ring", ring), load("about-glow", glow));
    ctx.data_mut(|d| d.insert_temp(id, found.clone()));
    found
}

/// RGBA pixels split into the ring and the glow: each pixel's opacity shared
/// between the two by how opaque it is, so the two drawn together, the glow
/// at its own strength, are the icon again.
fn split(rgba: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut ring = rgba.to_vec();
    let mut glow = rgba.to_vec();
    for (r, g) in ring
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(glow.as_chunks_mut::<4>().0)
    {
        let a = f32::from(r[3]);
        let t = ((a - GLOW_BELOW) / (RING_ABOVE - GLOW_BELOW)).clamp(0.0, 1.0);
        let share = t * t * (3.0 - 2.0 * t);
        r[3] = (a * share).round() as u8;
        g[3] = (a * (1.0 - share)).round() as u8;
    }
    (ring, glow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_and_the_glow_add_back_up_to_the_icon() {
        let pixels: Vec<u8> = (0..=255u8).flat_map(|a| [10, 200, 250, a]).collect();
        let (ring, glow) = split(&pixels);
        for (at, a) in (0..=255u8).enumerate() {
            let (r, g) = (ring[at * 4 + 3], glow[at * 4 + 3]);
            assert!(
                (i32::from(r) + i32::from(g) - i32::from(a)).abs() <= 1,
                "{a}"
            );
            if f32::from(a) <= GLOW_BELOW {
                assert_eq!(r, 0, "a faint pixel is all glow");
            }
            if f32::from(a) >= RING_ABOVE {
                assert_eq!(g, 0, "an opaque pixel is all ring");
            }
            // The colour is the icon's in both: only the opacity is shared.
            assert_eq!(&ring[at * 4..at * 4 + 3], &[10, 200, 250]);
            assert_eq!(&glow[at * 4..at * 4 + 3], &[10, 200, 250]);
        }
    }

    #[test]
    fn the_glow_breathes_between_dim_and_bright_and_back() {
        let dim = Look::at(0.0);
        let bright = Look::at(BREATH / 2.0);
        assert!(dim.intensity < 1.0 && bright.intensity > 1.0);
        assert!(bright.scale > dim.scale);
        assert_eq!(Look::at(BREATH * SWAY), Look::at(0.0), "it loops");
    }

    #[test]
    fn the_breath_stays_close_to_the_glow_the_icon_has() {
        for step in 0..100 {
            let look = Look::at(f64::from(step) * BREATH / 100.0);
            assert!((look.intensity - 1.0).abs() <= 0.1, "{look:?}");
            assert!(look.scale <= 1.02, "{look:?}");
        }
    }

    #[test]
    fn on_a_dark_page_the_glow_only_adds_light() {
        let added = light(Color32::from_rgb(200, 240, 255), 0.5, true);
        assert!(added.is_additive() && added.r() > 0, "{added:?}");
        let over = light(Color32::from_rgb(200, 240, 255), 0.5, false);
        assert!(over.a() > 0, "{over:?}");
        // Past full is clamped rather than tripping egui's assertion.
        assert_eq!(
            light(Color32::WHITE, 1.06, true),
            light(Color32::WHITE, 1.0, true)
        );
    }

    #[test]
    fn the_lights_still_glide_after_days_open() {
        let week = 7.0 * 24.0 * 3600.0;
        let frame = 1.0 / 30.0;
        let (now, next) = (lights_at(0.5, week), lights_at(0.5, week + frame));
        assert!(
            (now - next).abs() < 0.05,
            "a frame moves a light by a frame's worth"
        );
        let moved = (0..30).any(|n| {
            let t = week + f64::from(n) * frame;
            lights_at(0.5, t) != lights_at(0.5, t + frame)
        });
        assert!(moved, "and it does move");
    }

    #[test]
    fn the_icon_fades_to_nothing_at_its_edges() {
        // The bloom is drawn past the logo's box, where the page can clip it,
        // and the fan samples past the texture, where it clamps to the edge:
        // both are invisible only because the margin is.
        let image = image::load_from_memory(include_bytes!("../../assets/icon.ico"))
            .unwrap()
            .into_rgba8();
        let (w, h) = image.dimensions();
        let band = w / 50;
        let edge = image
            .enumerate_pixels()
            .filter(|(x, y, _)| (*x).min(*y).min(w - 1 - x).min(h - 1 - y) < band)
            .map(|(_, _, p)| p[3])
            .max();
        assert!(edge <= Some(4), "{edge:?}");
    }

    #[test]
    fn the_lights_are_faint_and_come_and_go() {
        let (mut brightest, mut darkest) = (0.0f32, f32::MAX);
        for step in 0..2000 {
            let time = f64::from(step) * 0.05;
            for slice in 0..SLICES {
                let light = lights_at(slice as f32 / SLICES as f32 * TAU, time);
                brightest = brightest.max(light);
                darkest = darkest.min(light);
            }
        }
        assert!(brightest > 0.1, "a light can be seen: {brightest}");
        assert!(
            brightest <= 0.6,
            "but never outshines the glow: {brightest}"
        );
        assert!(darkest < 0.01, "and somewhere there is no traffic");
    }

    #[test]
    fn the_traffic_is_one_fan_round_the_logo() {
        let mesh = traffic(
            TextureId::default(),
            Pos2::ZERO,
            100.0,
            Color32::WHITE,
            true,
            3.0,
        );
        assert_eq!(mesh.vertices.len(), SLICES as usize + 2);
        assert_eq!(mesh.indices.len(), SLICES as usize * 3);
        assert_eq!(
            mesh.vertices[0].color,
            Color32::TRANSPARENT,
            "dark at the hub"
        );
    }

    #[test]
    fn the_logo_draws_and_asks_for_frames_only_while_drawn() {
        let ctx = egui::Context::default();
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| show(ui, 168.0));
        });
        assert!(
            out.viewport_output
                .values()
                .any(|v| v.repaint_delay <= Duration::from_millis(33)),
            "a frame is asked for while it is on screen"
        );
        // At the top of a breath, where the glow is drawn past full.
        let breathing = egui::RawInput {
            time: Some(BREATH / 2.0),
            ..Default::default()
        };
        let _ = ctx.run(breathing, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| show(ui, 168.0));
        });
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |_| {});
        });
        assert!(
            out.viewport_output
                .values()
                .all(|v| v.repaint_delay > Duration::from_secs(1)),
            "and none once it is not"
        );
    }
}
