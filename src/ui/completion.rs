//! The autocomplete popup, drawn under the cursor.
//!
//! What it offers and when is decided in [`crate::features::autocomplete`];
//! this only draws it. It takes no input of its own, deliberately: a widget
//! that answered a click would take the keyboard from the terminal, the
//! terminal losing focus closes the popup, and the click would land on nothing.
//! Its keys are the terminal's - see [`crate::ui::input::CompletionKey`].

use egui::{Align2, Color32, Context, Rect, RichText};

use crate::config::Theme;
use crate::features::autocomplete::{Loading, Popup, MAX_SHOWN};
use crate::i18n::tr;

/// The narrowest the popup is drawn, in columns of the terminal.
const MIN_COLUMNS: f32 = 24.0;

/// How wide a hint may make the popup before it wraps, in columns.
const HINT_COLUMNS: f32 = 56.0;

/// Draws `popup` just below `caret`, the cursor's cell on screen.
///
/// In the terminal's own colours and font size, so it reads as part of the
/// line it is completing rather than as a dialog that has opened over it.
pub fn show(ctx: &Context, popup: &Popup, caret: Rect, theme: &Theme, tab_uid: u64) {
    // Under the word being typed rather than under the cursor, so the
    // suggestions line up with the letters they continue - sigil and all,
    // since every suggestion in one popup shares the sigil that was typed.
    let sigil = popup
        .items
        .first()
        .map_or(0, |c| c.category.sigil().chars().count());
    let typed = (popup.token.text.chars().count() + sigil) as f32;
    let left = caret.left() - typed * caret.width();
    let id = egui::Id::new(("nit-completion", tab_uid));
    // Below the line, unless it would not fit there - then above it, if there
    // is more room above. Left to itself egui pushes an area that runs off the
    // bottom back up onto the screen, which put the list over the very line
    // being completed. The height is the one drawn last frame, or a guess from
    // the row count on the frame it first opens.
    let row = caret.height() * 1.1;
    let rows = popup.items.len() + usize::from(popup.hint.is_some());
    let height = ctx
        .memory(|m| m.area_rect(id))
        .map_or(rows as f32 * row + 12.0, |r| r.height());
    let screen = ctx.screen_rect();
    let room_below = screen.bottom() - caret.bottom();
    let room_above = caret.top() - screen.top();
    let below = height <= room_below || room_below >= room_above;
    let (pivot, top, room) = if below {
        (Align2::LEFT_TOP, caret.bottom(), room_below)
    } else {
        (Align2::LEFT_BOTTOM, caret.top(), room_above)
    };
    // A subscript's values can run to dozens, and a list that tall was cut
    // off by the edge of the window with no way to see the rest. So the list
    // scrolls: no taller than the room on its side - less the frame and the
    // hint - and never more than `MAX_SHOWN` rows, past which a list stops
    // being something read at a glance.
    let hint_height = if popup.hint.is_some() { row } else { 0.0 };
    let list_height = (room - 16.0 - hint_height)
        .min(MAX_SHOWN as f32 * row)
        .max(row);
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .interactable(false)
        .pivot(pivot)
        .fixed_pos(egui::pos2(left, top))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(theme.background)
                .stroke(egui::Stroke::new(1.0_f32, theme.selection))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let size = caret.height() * 0.75;
                    // Not as narrow as the suggestions: a subscript's values
                    // can be "1" and "2", and a popup that narrow wrapped the
                    // hint over it a word to a line. A few dozen columns, or
                    // the hint's own width if that is less, within the room
                    // left to the right of the word.
                    let hint_width = popup.hint.as_ref().map_or(0.0, |hint| {
                        ui.fonts(|f| {
                            f.layout_no_wrap(
                                hint.clone(),
                                egui::FontId::proportional(size * 0.85),
                                Color32::WHITE,
                            )
                            .size()
                            .x
                        })
                    });
                    let wanted = (caret.width() * MIN_COLUMNS)
                        .max(hint_width.min(caret.width() * HINT_COLUMNS) + 8.0);
                    ui.set_min_width(wanted.min(screen.right() - left - 16.0).max(0.0));
                    if let Some(hint) = &popup.hint {
                        egui::Frame::none()
                            .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(hint)
                                        .size(size * 0.85)
                                        .italics()
                                        .color(theme.foreground.gamma_multiply(0.75)),
                                );
                            });
                    }
                    egui::ScrollArea::vertical()
                        .id_source(("nit-completion-list", tab_uid))
                        .max_height(list_height)
                        // As tall as asked, not as tall as the area says there
                        // is room for. An area is laid out as if it grew down
                        // from its corner, so one opened upwards from the last
                        // line offered only the line or two below that corner,
                        // and the list never grew past it. Still shrinks to a
                        // short list: `auto_shrink` sees to that.
                        .min_scrolled_height(list_height)
                        .auto_shrink([true, true])
                        .show(ui, |ui| {
                            for (index, candidate) in popup.items.iter().enumerate() {
                                let selected = index == popup.selected;
                                let fill = if selected {
                                    theme.selection
                                } else {
                                    Color32::TRANSPARENT
                                };
                                let response = egui::Frame::none()
                                    .fill(fill)
                                    .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(candidate.display())
                                                    .monospace()
                                                    .size(size)
                                                    .color(theme.syntax_color(kind_of(candidate)))
                                                    .strong(),
                                            );
                                            ui.label(
                                                RichText::new(candidate.label())
                                                    .size(size * 0.85)
                                                    .color(theme.foreground.gamma_multiply(0.6)),
                                            );
                                        });
                                    })
                                    .response;
                                // The popup takes no input, so the wheel never
                                // reaches it: the list scrolls by following
                                // the selection the arrow keys move.
                                if selected {
                                    response.scroll_to_me(None);
                                }
                            }
                        });
                    // Under the list: how many there are, and whether
                    // anything is still on its way.
                    if popup.status.is_some() || popup.loading != Loading::No {
                        egui::Frame::none()
                            .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let faint = theme.foreground.gamma_multiply(0.6);
                                    let text = match (&popup.status, popup.loading) {
                                        (Some(status), _) => status.clone(),
                                        (None, _) => tr("Loading").to_string(),
                                    };
                                    ui.label(RichText::new(text).size(size * 0.8).color(faint));
                                    if popup.loading != Loading::No {
                                        loading_dots(ui, size * 0.8, faint);
                                    }
                                });
                            });
                    }
                });
        });
}

/// How long each of the three dots stays the last one lit.
const DOT_STEP: f64 = 0.3;

/// Three dots lit one after another and then all put out - the row a
/// terminal has always shown for "still working" - drawn at text size and
/// asking for a frame only when the next one comes on.
fn loading_dots(ui: &mut egui::Ui, size: f32, colour: Color32) {
    let time = ui.input(|i| i.time);
    let lit = loading_phase(time);
    let r = (size * 0.13).max(1.5);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size * 1.6, size), egui::Sense::hover());
    for i in 0..3 {
        let centre = egui::pos2(
            rect.left() + r + i as f32 * (rect.width() - 2.0 * r) / 2.0,
            rect.center().y + size * 0.2,
        );
        let on = i < lit;
        let fill = if on {
            colour
        } else {
            colour.gamma_multiply(0.2)
        };
        ui.painter().circle_filled(centre, r, fill);
    }
    let next = DOT_STEP - time % DOT_STEP;
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f64(next));
}

/// How many of the three dots are lit at `time`: 1, 2, 3, then none.
fn loading_phase(time: f64) -> usize {
    ((time / DOT_STEP).floor() as usize) % 4
}

/// The colour a suggestion is drawn in: the colour it will have on the line
/// once accepted, which says what it is before the label beside it is read.
fn kind_of(candidate: &crate::features::autocomplete::Candidate) -> crate::term::syntax::Kind {
    use crate::features::autocomplete::Category;
    use crate::term::syntax::Kind;
    match candidate.category {
        Category::Command | Category::SqlKeyword => Kind::Command,
        Category::Function | Category::SqlFunction => Kind::Function,
        Category::SystemVariable => Kind::SystemVariable,
        Category::SystemClass | Category::Class | Category::Table => Kind::ObjectClass,
        Category::Global => Kind::Global,
        Category::Routine => Kind::Routine,
        Category::Entry => Kind::Extrinsic,
        Category::Subscript => Kind::Number,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dots_come_on_one_at_a_time_and_then_all_go_out() {
        let phases: Vec<usize> = (0..8)
            .map(|n| loading_phase(n as f64 * DOT_STEP + 0.01))
            .collect();
        assert_eq!(phases, [0, 1, 2, 3, 0, 1, 2, 3]);
    }
}
