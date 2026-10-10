//! The app's dialogs, drawn the way GNOME and macOS draw an alert: no title
//! bar, the title centred and bold inside a rounded card, the buttons along
//! the bottom on the right with the one that answers the question last and in
//! the accent colour, and the window behind veiled until it is answered.
//!
//! One shape for all of them, so a dialog reads as the app asking something
//! rather than as one of egui's debug windows - which is what the default
//! frame, with its title bar and its close cross, looks like.

use egui::{Align, Color32, Context, Key, Layout, Margin, Response, RichText, Rounding, Ui};

/// What a button does, which decides how it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Plain,
    /// The answer the dialog is there to get: filled with the accent colour.
    Suggested,
    /// An answer that deletes or closes something: filled red, as GNOME's
    /// destructive action and macOS's are.
    Destructive,
}

/// GNOME's destructive red.
const DESTRUCTIVE: Color32 = Color32::from_rgb(0xc0, 0x1c, 0x28);

/// Shows a dialog over a veil, centred, until `open` is cleared - by a button
/// in `contents`, or by Esc.
pub fn show<R>(
    ctx: &Context,
    id: &str,
    title: &str,
    open: &mut bool,
    contents: impl FnOnce(&mut Ui) -> R,
) {
    veil(ctx, id);
    let frame = egui::Frame::window(&ctx.style())
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::symmetric(22.0, 18.0));
    let shown = egui::Window::new(title)
        .id(egui::Id::new(id))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .frame(frame)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        // After the anchor, which would turn it off: the card's background
        // then senses a drag, which moves the whole window below - the card
        // itself stays anchored, centred, whatever it is dragged by.
        .movable(true)
        .show(ctx, |ui| {
            ui.ctx()
                .data_mut(|d| d.insert_temp(buttons_id(), Vec::<egui::Rect>::new()));
            ui.set_min_width(320.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(title).strong().size(16.0));
            });
            ui.add_space(4.0);
            contents(ui);
        });
    // The veil and the window are both areas in the same layer order, and a
    // click on an area brings it to the front of that order. Clicking the veil
    // therefore buried the dialog underneath it: still painted, but no longer
    // reachable by the pointer, so no button would answer and the dialog could
    // not be dismissed at all. Lifting the window every frame keeps it above
    // its own veil whatever was clicked.
    if let Some(shown) = shown {
        ctx.move_to_top(shown.response.layer_id);
        // The card's free space moves the window, as a GNOME dialog's does:
        // the veil covers the title bar, and a window whose dialog's buttons
        // were off screen could not otherwise be moved to reach them. Fields
        // and buttons are drawn over the background, so they keep their
        // clicks.
        card_drags_window(ctx, &shown.response);
    }
    // With the title bar gone there is no cross to close it by, and Esc is
    // what both desktops answer a dialog with instead.
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        *open = false;
    }
}

/// Dims the whole window and swallows clicks, so nothing behind a dialog can
/// be reached while it waits for an answer.
fn veil(ctx: &Context, id: &str) {
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new((id, "veil")))
        .order(egui::Order::Middle)
        .fixed_pos(screen.min)
        .interactable(true)
        .show(ctx, |ui| {
            // Allocated before it is painted, because an area's painter is
            // clipped to what the area has claimed - and the click-and-drag
            // sense is the half that makes it modal rather than decorative.
            let behind = ui.allocate_response(screen.size(), egui::Sense::click_and_drag());
            // Nothing behind a dialog answers the pointer, but the window
            // itself still moves by it.
            drags_window(ctx, &behind);
            ui.painter()
                .rect_filled(screen, 0.0, Color32::from_black_alpha(110));
        });
}

/// Where this frame's dialog buttons are, so a press on one is told apart
/// from a press on the card behind it.
fn buttons_id() -> egui::Id {
    egui::Id::new("nit-dialog-buttons")
}

/// Moves the window by a drag that began on the card itself, not on one of
/// its buttons. egui gives a drag that starts on a button to the background
/// behind it, which would move the window out from under a slow click and
/// swallow its release; so where the press landed is noted when it lands,
/// and a press on a button never moves anything. Fields and labels take their
/// own drags - to select text - and never reach the card at all.
fn card_drags_window(ctx: &Context, card: &Response) {
    let bare = card.id.with("bare-press");
    if ctx.input(|i| i.pointer.primary_pressed()) {
        let at = ctx.input(|i| i.pointer.press_origin());
        let buttons: Vec<egui::Rect> = ctx.data(|d| d.get_temp(buttons_id())).unwrap_or_default();
        let on_button = at.is_some_and(|at| buttons.iter().any(|r| r.contains(at)));
        ctx.data_mut(|d| d.insert_temp(bare, !on_button));
    }
    if ctx.data(|d| d.get_temp::<bool>(bare)).unwrap_or(false) {
        drags_window(ctx, card);
    }
}

fn drags_window(ctx: &Context, response: &Response) {
    if response.drag_started_by(egui::PointerButton::Primary) {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
}

/// The row of buttons along the bottom, against the right edge.
///
/// Laid out right to left, so `buttons` adds them from the right: the answer
/// first, then Cancel, then anything that belongs further off to the left.
pub fn actions(ui: &mut Ui, buttons: impl FnOnce(&mut Ui)) {
    ui.add_space(6.0);
    // A row of its own height. `with_layout` would hand the buttons all the
    // height the window offers, and a window offers the whole screen: the
    // card grew to fill it, the buttons floating in the middle of the space.
    let size = egui::vec2(ui.available_width(), 32.0);
    ui.allocate_ui_with_layout(size, Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        buttons(ui);
    });
}

/// A dialog button: rounded, roomy, and filled for the roles that are filled.
pub fn button(ui: &mut Ui, text: &str, role: Role) -> Response {
    let fill = match role {
        Role::Plain => None,
        Role::Suggested => Some(ui.visuals().selection.bg_fill),
        Role::Destructive => Some(DESTRUCTIVE),
    };
    let mut label = RichText::new(text);
    if fill.is_some() {
        label = label.color(Color32::WHITE).strong();
    }
    let mut button = egui::Button::new(label)
        .rounding(Rounding::same(8.0))
        .min_size(egui::vec2(88.0, 30.0));
    if let Some(fill) = fill {
        button = button.fill(fill);
    }
    let response = ui.add(button);
    let rect = response.rect;
    ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_default::<Vec<egui::Rect>>(buttons_id())
            .push(rect)
    });
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2, ViewportCommand};

    /// Whether pressing at `at` and dragging away asks the window to move,
    /// with a dialog whose one button is reported back through `button`.
    fn drags_from(at: impl Fn(Rect, Rect) -> Pos2) -> bool {
        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 600.0));
        let mut button = Rect::NOTHING;
        let frame = |events: Vec<Event>, button: &mut Rect| {
            let input = RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            ctx.run(input, |ctx| {
                let mut open = true;
                show(ctx, "test-dialog", "Title", &mut open, |ui| {
                    ui.label("Some words");
                    *button = self::button(ui, "Send", Role::Suggested).rect;
                });
            })
        };
        // Two frames to lay the card out and remember where it is.
        frame(vec![], &mut button);
        frame(vec![], &mut button);
        let card = ctx
            .memory(|m| m.area_rect(egui::Id::new("test-dialog")))
            .unwrap();
        let from = at(card, button);
        let press = |pressed| Event::PointerButton {
            pos: from,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let mut moved = false;
        for events in [
            vec![Event::PointerMoved(from), press(true)],
            vec![Event::PointerMoved(from + Vec2::new(30.0, 0.0))],
            vec![Event::PointerMoved(from + Vec2::new(60.0, 0.0))],
        ] {
            let out = frame(events, &mut button);
            moved |= out.viewport_output.values().any(|v| {
                v.commands
                    .iter()
                    .any(|c| matches!(c, ViewportCommand::StartDrag))
            });
        }
        moved
    }

    #[test]
    fn the_free_space_of_a_dialog_moves_the_window() {
        assert!(drags_from(|card, _| card.left_top() + Vec2::new(6.0, 6.0)));
    }

    #[test]
    fn the_veil_round_a_dialog_moves_the_window() {
        assert!(drags_from(|_, _| Pos2::new(10.0, 10.0)));
    }

    /// The background takes drags now; a button on it still takes its click.
    #[test]
    fn a_dialog_button_still_answers_a_click() {
        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 600.0));
        let mut button = Rect::NOTHING;
        let mut clicked = false;
        let frame = |events: Vec<Event>, button: &mut Rect, clicked: &mut bool| {
            let input = RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                let mut open = true;
                show(ctx, "test-dialog", "Title", &mut open, |ui| {
                    let response = self::button(ui, "Send", Role::Suggested);
                    *button = response.rect;
                    *clicked |= response.clicked();
                });
            });
        };
        frame(vec![], &mut button, &mut clicked);
        frame(vec![], &mut button, &mut clicked);
        let at = button.center();
        let press = |pressed| Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        frame(
            vec![Event::PointerMoved(at), press(true)],
            &mut button,
            &mut clicked,
        );
        frame(vec![press(false)], &mut button, &mut clicked);
        assert!(clicked);
    }

    #[test]
    fn a_dialog_button_is_pressed_not_dragged() {
        assert!(!drags_from(|_, button| button.center()));
    }
}
