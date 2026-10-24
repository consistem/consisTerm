//! The colour a tab can be marked with, and the swatches it is picked from.
//!
//! A mark rather than a fill: the shown tab is the terminal's own colour, so it
//! runs on into the terminal under it, and painting it red would break that
//! seam. So the colour is a band along the tab's edge, and a faint wash of it
//! over a tab that is not shown - enough to find "the red one" in a strip of
//! ten, which is what a name alone could not do.

use egui::{Color32, Rect, Response, Sense, Stroke, Ui, Vec2};

use crate::i18n::tr;
use crate::ui::tip::Tip;

/// The swatches offered, and what each is called in its tooltip.
///
/// Few, and far apart in hue: they are told apart at a glance across a strip,
/// not compared side by side. Mid-tone, so a band of one reads on a light theme
/// and on a dark one. A colour outside these - a hand-edited session file -
/// is drawn just the same; it simply has no swatch of its own.
pub const SWATCHES: [(&str, Color32); 8] = [
    ("Red", Color32::from_rgb(0xe0, 0x4f, 0x4f)),
    ("Orange", Color32::from_rgb(0xe8, 0x8a, 0x2e)),
    ("Yellow", Color32::from_rgb(0xd8, 0xb8, 0x2a)),
    ("Green", Color32::from_rgb(0x4c, 0xb0, 0x5a)),
    ("Teal", Color32::from_rgb(0x2a, 0xa8, 0xa8)),
    ("Blue", Color32::from_rgb(0x4a, 0x7e, 0xe0)),
    ("Purple", Color32::from_rgb(0x9a, 0x5c, 0xd8)),
    ("Pink", Color32::from_rgb(0xd8, 0x5c, 0xa8)),
];

/// How thick the band along a coloured tab is, in points.
pub const BAND: f32 = 3.0;

/// How strongly a tab that is not shown is washed with its colour. Low: the
/// name has to stay readable in the theme's own ink on top of it.
const WASH: f32 = 0.22;

/// A row of swatches, with "none" first. Returns the choice when one is
/// clicked: `Some(None)` is the colour being taken off.
pub fn picker(ui: &mut Ui, current: Option<Color32>) -> Option<Option<Color32>> {
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if swatch(ui, None, current.is_none())
            .tip(tr("No colour"))
            .clicked()
        {
            picked = Some(None);
        }
        for (name, colour) in SWATCHES {
            if swatch(ui, Some(colour), current == Some(colour))
                .tip(tr(name))
                .clicked()
            {
                picked = Some(Some(colour));
            }
        }
    });
    picked
}

/// One square to click. `None` is drawn as an empty square struck through,
/// which is what "no colour" looks like in every picker people have met.
fn swatch(ui: &mut Ui, colour: Option<Color32>, chosen: bool) -> Response {
    let side = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let visuals = ui.visuals();
    let inner = rect.shrink(3.0);
    let edge = visuals.widgets.noninteractive.fg_stroke.color;
    match colour {
        Some(colour) => {
            ui.painter().rect_filled(inner, 2.0, colour);
        }
        None => {
            ui.painter()
                .rect_stroke(inner, 2.0, Stroke::new(1.0_f32, edge));
            ui.painter().line_segment(
                [inner.left_bottom(), inner.right_top()],
                Stroke::new(1.0_f32, edge),
            );
        }
    }
    if chosen || response.hovered() {
        let ring = if chosen {
            visuals.strong_text_color()
        } else {
            edge
        };
        ui.painter()
            .rect_stroke(rect.shrink(0.5), 3.0, Stroke::new(1.5_f32, ring));
    }
    response
}

/// Marks a drawn tab with its colour: a wash over it unless it is the one
/// shown, and the band along the edge that faces away from the terminal - the
/// top of a tab in a row, the outer side of one in a column.
pub fn mark(ui: &Ui, rect: Rect, colour: Color32, selected: bool, vertical: bool) {
    // Under a skin the tab is the skin's art, with borders and bevels and
    // gloss of its own: a flat wash and a band laid over the whole of it
    // ran straight across them. The skin puts the mark inside its own tab.
    if let Some(kit) = crate::ui::skin::Kit::of(ui.ctx()) {
        let st = crate::ui::skin::St {
            on: selected,
            ..crate::ui::skin::St::default()
        };
        ui.painter()
            .extend(kit.style().tab_mark(&kit, rect, colour, st));
        return;
    }
    let painter = ui.painter();
    if !selected {
        painter.rect_filled(rect, 0.0, colour.gamma_multiply(WASH));
    }
    let band = if vertical {
        Rect::from_min_size(rect.min, Vec2::new(BAND, rect.height()))
    } else {
        Rect::from_min_size(rect.min, Vec2::new(rect.width(), BAND))
    };
    painter.rect_filled(band, 0.0, colour);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_two_swatches_are_the_same_colour() {
        for (i, (_, a)) in SWATCHES.iter().enumerate() {
            for (_, b) in &SWATCHES[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    /// A band has to stand off the shown tab whichever kind of theme it is on.
    #[test]
    fn every_swatch_reads_against_both_a_light_and_a_dark_tab() {
        for (name, colour) in SWATCHES {
            for behind in [Color32::from_rgb(0x1e, 0x1e, 0x1e), Color32::WHITE] {
                let contrast = crate::config::theme::contrast(colour, behind);
                assert!(contrast >= 1.5, "{name} on {behind:?}: {contrast}");
            }
        }
    }
}
