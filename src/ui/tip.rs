//! Tooltips that keep a readable width.
//!
//! egui lays a tooltip out inside the size its area had the last time it was
//! shown, and lets it shrink to its text but never grow back past that. So a
//! tooltip once squeezed - shown against the edge of a small window, or
//! sharing an area id with one that held a single word - wrapped every word
//! onto a line of its own from then on. A floor under the width, worked out
//! from the text itself, gives the area room to widen again on the next frame.

use egui::{Label, Response, TextStyle, TextWrapMode, Ui, WidgetText};

/// The narrowest a tooltip with a sentence in it is drawn. A shorter text is
/// as wide as itself; a longer one wraps at egui's `tooltip_width`.
const MIN_WIDTH: f32 = 280.0;

/// `on_hover_text` and `on_disabled_hover_text`, held to a readable width.
pub trait Tip {
    fn tip(self, text: impl Into<WidgetText>) -> Self;
    fn disabled_tip(self, text: impl Into<WidgetText>) -> Self;
}

impl Tip for Response {
    fn tip(self, text: impl Into<WidgetText>) -> Self {
        let text = text.into();
        self.on_hover_ui(|ui| label(ui, text))
    }

    fn disabled_tip(self, text: impl Into<WidgetText>) -> Self {
        let text = text.into();
        self.on_disabled_hover_ui(|ui| label(ui, text))
    }
}

/// The text of a tooltip, at least as wide as [`min_width`] allows.
pub fn label(ui: &mut Ui, text: WidgetText) {
    ui.set_min_width(min_width(ui, &text));
    ui.add(Label::new(text));
}

/// How wide `text` asks to be laid out: its own width on one line, up to
/// `MIN_WIDTH`, and never more than the screen leaves.
pub fn min_width(ui: &Ui, text: &WidgetText) -> f32 {
    let natural = text
        .clone()
        .into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextStyle::Body,
        )
        .size()
        .x;
    let screen = ui.ctx().screen_rect().width() - 16.0;
    natural.min(MIN_WIDTH).min(screen.max(0.0)).ceil()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Pos2, RawInput, Rect, Sense, Vec2};

    /// The tooltip's width after hovering a button whose tooltip first held
    /// one word and then a sentence.
    fn shown_width(use_tip: bool) -> f32 {
        let ctx = Context::default();
        let text = "New session on CONSISTEM (Ctrl+T). Right-click to connect somewhere else.";
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 420.0));
        let mut width = 0.0;
        for frame in 0..12 {
            let input = RawInput {
                screen_rect: Some(screen),
                time: Some(frame as f64 * 0.5),
                events: vec![egui::Event::PointerMoved(Pos2::new(40.0, 400.0))],
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    // The first frames squeeze the tooltip: the text it shows
                    // then is a single word, as a reused area id's was.
                    let rect = Rect::from_min_size(Pos2::new(30.0, 390.0), Vec2::splat(20.0));
                    let response = ui.interact(rect, ui.id().with("button"), Sense::click());
                    let shown = if frame < 4 { "1" } else { text };
                    if use_tip {
                        response.tip(shown);
                    } else {
                        response.on_hover_text(shown);
                    }
                });
            });
            let id = ctx.memory(|m| {
                m.layer_ids()
                    .filter(|l| l.order == egui::Order::Tooltip)
                    .collect::<Vec<_>>()
            });
            for layer in id {
                if let Some(rect) = ctx.memory(|m| m.area_rect(layer.id)) {
                    width = rect.width();
                }
            }
        }
        width
    }

    #[test]
    fn a_tooltip_squeezed_once_widens_again_for_a_sentence() {
        assert!(shown_width(true) >= MIN_WIDTH, "{}", shown_width(true));
    }

    #[test]
    fn egui_alone_leaves_a_squeezed_tooltip_narrow() {
        // What the floor is for. If this starts failing, egui has fixed it
        // and `Tip` can go.
        assert!(shown_width(false) < 100.0, "{}", shown_width(false));
    }
}
