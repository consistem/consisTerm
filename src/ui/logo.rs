//! The logo on the About page, its glow breathing behind the ring.
//!
//! The icon is drawn with its glow baked in. Split by opacity, it comes apart
//! into the ring - nearly opaque - and the glow - the faint halo round it -
//! and the glow can then be drawn behind the ring brighter and dimmer, a
//! little wider and narrower, its colour leaning towards the cyan and then the
//! magenta of the ring itself. The ring stays still: it is the thing being
//! looked at.

use std::time::Duration;

use egui::{Color32, Context, Rect, Sense, TextureHandle, Ui, Vec2};

/// Opacity below which a pixel is all glow, and above which all ring. The
/// icon tool cuts the small icons round 140, what can be seen of the ring.
const GLOW_BELOW: f32 = 110.0;
const RING_ABOVE: f32 = 170.0;

/// One breath, in seconds: slow enough to be calm, quick enough to be seen.
const BREATH: f64 = 4.0;
/// One sway of the colour from cyan to magenta and back.
const SWAY: f64 = 11.0;

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
    let painter = ui.painter();
    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let halo = Rect::from_center_size(rect.center(), rect.size() * look.scale);
    // Twice past full: a texture cannot be drawn brighter than it is, only
    // over itself.
    let mut left = look.intensity;
    while left > 0.0 {
        let alpha = left.min(1.0);
        painter.image(glow.id(), halo, uv, look.tint.gamma_multiply(alpha));
        left -= 1.0;
    }
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
            intensity: 0.55 + 0.95 * breath,
            scale: 1.0 + 0.06 * breath,
            // Multiplied into the glow, so it can only take colour away: less
            // red leans it cyan, less green leans it magenta.
            tint: Color32::from_rgb(lerp(190.0, 255.0), lerp(255.0, 200.0), 255),
        }
    }
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
