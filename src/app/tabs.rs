//! Tabs and the panes inside them: opening, closing, splitting, choosing.
//!
//! A tab holds one session, or two when it is split. [`At`] is how the rest of
//! the app names one of them without caring which; everything that acts on "the
//! terminal" takes one.
//!
//! [`At`]: super::At

use super::*;
use crate::ui::tip::Tip;

/// A tab name being edited.
pub(super) struct Renaming {
    /// The tab, by the uid of its own session.
    ///
    /// Uids rather than indices, here and for the names: tabs move under an
    /// open dialog - reordered, docked into one another, split apart - and a
    /// pane's number changes when one before it closes. A name typed for one
    /// session must land on that session or nowhere.
    pub(super) tab: u64,
    /// One name per pane, in pane order, each with the uid of its session.
    /// Several fields because a split tab's entry names whichever session has
    /// the keyboard; the `1:`, `2:` in front of them are the panes themselves
    /// and not part of any name.
    pub(super) names: Vec<(u64, String)>,
    /// The colour as picked in the dialog, put on the tab only when it is
    /// confirmed - so Cancel leaves the tab as it was.
    pub(super) colour: Option<egui::Color32>,
    /// Cleared after the field has been given focus once. Without this the
    /// terminal claims the keyboard back - it grabs focus whenever nothing else
    /// holds it - and the new name would be typed into IRIS.
    pub(super) focus: bool,
}

/// A tab being dragged out of the strip, to be dropped beside the panes of
/// another.
///
/// By uid rather than index: the strip reorders tabs under a drag, and the
/// tab being dropped onto is the one the user was looking at when they took
/// hold of this one, wherever it has moved since.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TabDrag {
    /// The tab being dragged.
    pub(super) uid: u64,
    /// The tab that was shown when the drag began, which is what it is
    /// dropped into.
    pub(super) onto: u64,
    /// The pointer has left the strip for the terminal, so the tab it will
    /// be dropped into is the one shown, with the places it can go marked.
    pub(super) over_terminal: bool,
    /// The tab in the strip whose middle the pointer is over, which a drop
    /// there joins - so a tab can be put into another without that one
    /// having to be shown first.
    pub(super) into_tab: Option<u64>,
    /// The tab whose middle the pointer has been held over, and since when -
    /// see [`TAB_SPRING`].
    pub(super) held: Option<(u64, f64)>,
    /// A tab held over has opened, and is the one shown - and the one the
    /// drop goes into - for the rest of the drag.
    pub(super) sprung: bool,
}

impl App {
    /// Connects a new tab to whatever `new_tab_profile` currently names.
    pub fn open_new_tab(&mut self) {
        let profile = self.new_tab_profile.clone();
        self.open_tab(profile);
    }

    pub fn open_tab(&mut self, profile: Profile) {
        let (cols, rows) = self.initial_size(&profile);
        self.tabs
            .push(Tab::new(profile, &self.settings, cols, rows));
        self.active = self.tabs.len() - 1;
    }

    /// Opens the easter egg, or goes back to the board that is already open.
    ///
    /// One at a time: typing `/snake` again is somebody coming back to the
    /// game rather than asking for a second one, and the score they were
    /// playing for is on the board they left.
    pub(super) fn open_snake_tab(&mut self) {
        if let Some(index) = self.tabs.iter().position(Tab::is_game) {
            self.active = index;
            return;
        }
        let game = Snake::new(Some(config::snake_score_path()));
        self.tabs.push(Tab::snake(game));
        self.active = self.tabs.len() - 1;
    }

    /// Closes a tab and everything in it - every session, when it is split.
    pub fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        // Ask IRIS to halt so it releases locks; Drop kills anything that
        // ignores the request.
        for session in self.tabs[index].sessions() {
            if let Some(session) = session.session.as_ref() {
                session.request_halt();
            }
        }
        let closed = self.tabs.remove(index);
        if !closed.is_game() {
            self.remember_closed(saved_tab(&closed));
        }
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len().saturating_sub(1);
        }
    }

    /// Closes one pane, and nothing else.
    ///
    /// On a tab that is not split there is only the one session, so this is
    /// [`App::close_tab`]. On a split tab it closes the session the user
    /// pointed at and leaves the others where they are - which is what the
    /// right-click menu over a pane has to mean, since the other panes belong
    /// to whatever is running in them.
    pub(super) fn close_pane(&mut self, at: At) {
        let Some(tab) = self.tabs.get_mut(at.tab) else {
            return;
        };
        if tab.split.is_none() {
            self.close_tab(at.tab);
            return;
        }
        // Ask IRIS to halt so it releases its locks; dropping the session kills
        // anything that ignores the request.
        let Some(going) = tab.pane(at.pane) else {
            return;
        };
        if let Some(session) = going.session.as_ref() {
            session.request_halt();
        }
        let closed = saved_pane(going);
        tab.detach_pane(at.pane);
        self.remember_closed(closed);
    }

    fn remember_closed(&mut self, tab: SavedTab) {
        if self.closed_tabs.len() == CLOSED_TABS_KEPT {
            self.closed_tabs.remove(0);
        }
        self.closed_tabs.push(tab);
    }

    /// Opens the most recently closed tab again, in front: same profile, same
    /// name and split, back in the namespace it was in, or - for a shell that
    /// reports one - the folder. Each press goes one further back.
    pub(super) fn reopen_closed_tab(&mut self) {
        let Some(entry) = self.closed_tabs.pop() else {
            self.set_status(tr("No closed tab to reopen."));
            return;
        };
        self.restore_tab(entry);
        self.active = self.tabs.len() - 1;
    }

    /// The session the user is working in: the focused pane of the active tab.
    pub(super) fn active_tab(&self) -> Option<&Tab> {
        self.pane(self.focused_at())
    }

    /// Where that session lives.
    pub(super) fn focused_at(&self) -> At {
        At {
            tab: self.active,
            pane: self
                .tabs
                .get(self.active)
                .map_or(Pane::FIRST, |tab| tab.focus),
        }
    }

    /// The session at `at`, while both the tab and the pane are still there.
    pub(super) fn pane(&self, at: At) -> Option<&Tab> {
        self.tabs.get(at.tab)?.pane(at.pane)
    }

    pub(super) fn pane_mut(&mut self, at: At) -> Option<&mut Tab> {
        self.tabs.get_mut(at.tab)?.pane_mut(at.pane)
    }

    /// Every session open, in every tab. What "is anything still connected"
    /// has to count, since a split tab holds several.
    pub(super) fn sessions(&self) -> impl Iterator<Item = &Tab> {
        self.tabs.iter().flat_map(|tab| tab.sessions())
    }

    /// The view of the pane the keyboard is in, which is where a find bar
    /// belongs: a search is made in one transcript, not in all of them.
    pub(super) fn focused_view(&self) -> Option<&terminal_view::ViewState> {
        let tab = self.tabs.get(self.active)?;
        Some(&tab.pane(tab.focus)?.view)
    }

    pub(super) fn focused_view_mut(&mut self) -> Option<&mut terminal_view::ViewState> {
        let tab = self.tabs.get_mut(self.active)?;
        let focus = tab.focus;
        Some(&mut tab.pane_mut(focus)?.view)
    }

    /// The tab strip, in no more than `width` of the row it is drawn in.
    ///
    /// Only ever called from a left-to-right row, and that matters:
    /// `allocate_ui_at_rect` gives the child the *parent's* layout, and what it
    /// then advances the row by is the child's `min_rect`. In a right-to-left
    /// parent that rect starts at the right-hand edge, so a narrow strip of
    /// tabs would measure as ending where the row ends and leave nothing after
    /// it - which is precisely how the title bar lost its drag area.
    pub(super) fn tab_strip_bounded(&mut self, ui: &mut egui::Ui, width: f32) {
        let height = ui.available_height().max(ui.spacing().interact_size.y);
        let rect = egui::Rect::from_min_size(ui.cursor().min, egui::Vec2::new(width, height));
        // The bar has already held back its own free strip to drag by - see
        // `FREE_STRIP` in `crate::ui::chrome` - and `width` is what is left,
        // across the theme's left and right spaces alike. Holding back more
        // here left a band of nothing between the tabs and the buttons.
        ui.allocate_ui_at_rect(rect, |ui| self.tab_strip_in(ui, 0.0, Strip::TitleBar));
    }

    /// The tab strip as a row of its own, the tabs sharing its width.
    pub(super) fn tab_strip(&mut self, ui: &mut egui::Ui) {
        self.tab_strip_in(ui, 0.0, Strip::Row);
    }

    /// The tabs as a column down a title bar on the side of the window, one
    /// under another at the column's full width, the way Vivaldi and Opera
    /// show theirs there.
    pub(super) fn tab_column(&mut self, ui: &mut egui::Ui) {
        self.tab_strip_in(ui, 0.0, Strip::Column);
    }

    /// The tabs share the row out between them, the way GNOME's Files and
    /// Text Editor do - all of it but `reserve`, which in the title bar is
    /// the space the window is dragged by and must not be taken.
    ///
    /// A tab dragged is reordered or tiled, never the window moved, title bar
    /// or not. In a column the tabs are reordered up and down, and scrolled
    /// the same way.
    fn tab_strip_in(&mut self, ui: &mut egui::Ui, reserve: f32, strip: Strip) {
        let vertical = strip == Strip::Column;
        let mut to_close = None;
        let mut to_rename = None;
        let mut to_colour = None;
        let mut to_split = None;
        let mut to_unsplit = None;
        let mut to_activate = None;
        let mut to_reopen = false;
        let can_reopen = !self.closed_tabs.is_empty();
        let with_namespace = self.settings.show_namespace_in_tab;
        let close_side = self.settings.tab_close_side;
        let theme = self.theme();
        // The shown tab is the terminal's own colour, so it runs on into the
        // terminal under it, as elementary's does; a theme that names its own
        // selected tab still gets that.
        let selected_colours = (
            Some(theme.tab_selected.unwrap_or(theme.background)),
            theme.tab_selected_text,
        );
        let close_colour = crate::ui::chrome::close_tab_colour(&theme.window_buttons);
        let divider = theme
            .ui_border
            .unwrap_or(theme.ui_foreground)
            .gamma_multiply(0.18);
        // Shrunk to the tabs rather than filling the row: in the title bar the
        // space left over is what the window is dragged by, and a scroll area
        // that claimed the whole width would take all of it.
        // The strip's own bar follows the same setting as the terminal's, so
        // "show scrollbars" means every scrollbar in the app rather than every
        // scrollbar except this one. Off still scrolls - the wheel and a drag
        // both work - it simply draws nothing along the bottom of the tabs.
        let visibility = if self.settings.show_scrollbars {
            egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded
        } else {
            egui::scroll_area::ScrollBarVisibility::AlwaysHidden
        };

        // The wheel over the strip, before the scroll area reads the input.
        //
        // egui turns a shifted wheel into horizontal scrolling for us, which
        // left the plain wheel - the one that is actually under the finger -
        // doing nothing at all over a row of tabs. The two are swapped back
        // here: the plain wheel scrolls the strip, and shift walks the
        // selection from tab to tab.
        //
        // Rewritten rather than handled afterwards so the scroll lands on the
        // same frame it was rolled on, and consumed when it changes tabs so
        // the strip does not also slide sideways under the pointer.
        let mut step = 0.0_f32;
        // A column scrolls the way the wheel turns already.
        if !vertical && ui.rect_contains_pointer(ui.available_rect_before_wrap()) {
            ui.input_mut(|i| {
                if i.modifiers.shift {
                    // Already swapped by egui, so the shifted wheel arrives on
                    // x. `raw` rather than the smoothed delta: smoothing
                    // spreads one notch over several frames, which would walk
                    // the selection several tabs from one flick.
                    step = i.raw_scroll_delta.x;
                    i.raw_scroll_delta = egui::Vec2::ZERO;
                    i.smooth_scroll_delta = egui::Vec2::ZERO;
                } else {
                    i.raw_scroll_delta = egui::vec2(i.raw_scroll_delta.y, 0.0);
                    i.smooth_scroll_delta = egui::vec2(i.smooth_scroll_delta.y, 0.0);
                }
            });
        }
        if step != 0.0 && !self.tabs.is_empty() {
            // Up is back, the way it is in a list. Wrapped at both ends: the
            // wheel has no stop, and a selection that silently refuses to move
            // reads as the gesture not working.
            let last = self.tabs.len() - 1;
            to_activate = Some(if step > 0.0 {
                self.active.checked_sub(1).unwrap_or(last)
            } else if self.active >= last {
                0
            } else {
                self.active + 1
            });
        }

        // Where each tab was drawn this frame, and which one is being dragged:
        // what a drag is measured against once every tab has been laid out.
        let mut spans: Vec<(f32, f32)> = Vec::with_capacity(self.tabs.len());
        let mut rects: Vec<egui::Rect> = Vec::with_capacity(self.tabs.len());
        let mut into_tab = None;
        let mut dragging = None;
        let mut to_move = None;
        let mut drag_began = None;
        let mut over_terminal = false;
        let gap = 0.0;
        let count = self.tabs.len().max(1) as f32;
        let room = ui.available_width() - reserve;
        // Fixed, each tab sizes itself to its name between the two bounds -
        // the space the tabs took before they shared the row - and is drawn
        // exactly as a shared one is.
        let shared = if vertical {
            Some(ui.available_width())
        } else {
            (self.settings.tab_width == crate::config::TabWidth::Shared)
                .then(|| ((room - gap * (count - 1.0)) / count).max(TAB_MIN_WIDTH))
        };
        // A column's tabs are each one tab tall, and so are a skin's in a row
        // of their own: the row sizes itself to what is in it, so a tab told
        // to take the row's height came out as tall as its text, and the
        // skin's taller tab - the shown one standing two pixels higher - cut
        // its own label off at the top. In the title bar the tab fits the
        // bar instead.
        let skinned_row = strip == Strip::Row && crate::ui::skin::Kit::of(ui.ctx()).is_some();
        let tall = (vertical || skinned_row).then(|| tab_height(ui));
        let scroll = if vertical {
            egui::ScrollArea::vertical().auto_shrink([false, true])
        } else {
            egui::ScrollArea::horizontal().auto_shrink([true, false])
        };

        scroll
            .scroll_bar_visibility(visibility)
            .show(ui, |ui| {
            let lay_out = |ui: &mut egui::Ui, add: &mut dyn FnMut(&mut egui::Ui)| {
                if vertical {
                    ui.vertical(|ui| add(ui));
                } else {
                    tab_row(ui, |ui| add(ui));
                }
            };
            lay_out(ui, &mut |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(gap, 0.0);
                for index in 0..self.tabs.len() {
                    let selected = index == self.active;
                    let split = self.tabs[index].split.is_some();
                    let can_split = self.tabs[index].can_split();
                    let colour = self.tabs[index].colour;
                    let mut label = self.tabs[index].strip_label(with_namespace);
                    if self.tabs[index].focused().ended {
                        label.push_str(" (ended)");
                    } else if self.tabs[index].focused().sql {
                        // On the label rather than in the name, so renaming
                        // the tab neither loses it nor bakes it in.
                        label.push_str(" · SQL");
                    }
                    let (response, close) =
                        tab_pill(
                        ui,
                        self.tabs[index].uid,
                        selected,
                        label,
                        selected_colours,
                        close_colour,
                        [shared, tall],
                        close_side,
                    );
                    if let Some(colour) = colour {
                        crate::ui::tab_colour::mark(
                            ui,
                            response.rect,
                            colour,
                            selected,
                            vertical,
                        );
                    }
                    // A line between two tabs neither of which is shown: the
                    // shown one is set off by its colour already.
                    let next_selected = index + 1 == self.active;
                    if index + 1 < self.tabs.len() && !selected && !next_selected {
                        let r = response.rect;
                        if vertical {
                            ui.painter().hline(
                                r.left() + r.width() * 0.1..=r.right() - r.width() * 0.1,
                                r.bottom() - 0.5,
                                egui::Stroke::new(1.0_f32, divider),
                            );
                        } else {
                        ui.painter().line_segment(
                            [
                                egui::pos2(r.right() - 0.5, r.top() + r.height() * 0.25),
                                egui::pos2(r.right() - 0.5, r.bottom() - r.height() * 0.25),
                            ],
                            egui::Stroke::new(1.0_f32, divider),
                        );
                        }
                    }
                    let response = response.tip(tr(
                        "Double-click to rename or colour, drag to reorder. Drop it on the middle of another tab, or on the terminal, to tile the two together.",
                    ));
                    if response.clicked() {
                        to_activate = Some(index);
                    }
                    // The tab being moved is the one shown, the way it is
                    // everywhere else tabs are dragged: what is under the strip
                    // should be the session the user has hold of.
                    // Never the window being moved, even in the title bar: a
                    // tab taken hold of is being reordered or tiled, and the
                    // bar's empty part - which `reserve` keeps - is what moves
                    // the window.
                    if response.drag_started() {
                        to_activate = Some(index);
                        drag_began = Some(index);
                    }
                    if response.dragged() {
                        dragging = Some(index);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    }
                    if response.double_clicked() {
                        to_rename = Some(index);
                    }
                    // What every other tabbed program does, and the reason the
                    // small cross is not the only way out: closing several
                    // tabs in a row means aiming at a cross the width of a
                    // character each time, and the middle button closes
                    // whatever is under the pointer.
                    if response.middle_clicked() {
                        to_close = Some(index);
                    }
                    response.context_menu(|ui| {
                        if ui.button(tr("Rename...")).clicked() {
                            to_rename = Some(index);
                            ui.close_menu();
                        }
                        // Here as well as in the rename dialog: marking a tab
                        // is one click, and should not need a dialog.
                        ui.menu_button(tr("Colour"), |ui| {
                            if let Some(picked) = crate::ui::tab_colour::picker(ui, colour) {
                                to_colour = Some((index, picked));
                                ui.close_menu();
                            }
                        });
                        ui.separator();
                        // Splits the pane the keyboard is in, which is the one
                        // the entry is named after.
                        if can_split {
                            if ui
                                .button(tr("Split to right"))
                                .tip(tr(
                                    "Opens another session in this tab, beside this one. Click into a pane to type in it.",
                                ))
                                .clicked()
                            {
                                to_split = Some((index, Toward::Right));
                                ui.close_menu();
                            }
                            if ui.button(tr("Split to bottom")).clicked() {
                                to_split = Some((index, Toward::Down));
                                ui.close_menu();
                            }
                        }
                        if split
                            && ui
                                .button(tr("Remove split"))
                                .tip(tr(
                                    "Gives every session in this tab a tab of its own. Nothing is closed.",
                                ))
                                .clicked()
                        {
                            to_unsplit = Some(index);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(tr("Close"))
                            .tip(if split {
                                tr("Closes every session in this tab.")
                            } else {
                                tr("Closes this session.")
                            })
                            .clicked()
                        {
                            to_close = Some(index);
                            ui.close_menu();
                        }
                        // Here as well as on the keyboard, so the gesture can
                        // be found by anyone who does not know the chord.
                        if ui
                            .add_enabled(
                                can_reopen,
                                egui::Button::new(tr("Reopen closed tab"))
                                    .shortcut_text("Ctrl+Shift+T"),
                            )
                            .clicked()
                        {
                            to_reopen = true;
                            ui.close_menu();
                        }
                    });
                    if close.is_some_and(|close| close.tip(tr("Close this tab.")).clicked()) {
                        to_close = Some(index);
                    }
                    rects.push(response.rect);
                    spans.push(if vertical {
                        (response.rect.top(), response.rect.bottom())
                    } else {
                        (response.rect.left(), response.rect.right())
                    });
                }

                let pointer = ui.ctx().pointer_interact_pos();
                if let (Some(from), Some(pointer)) = (dragging, pointer) {
                    // Along the strip, whichever way it runs.
                    let (along, near, far) = if vertical {
                        (pointer.y, ui.clip_rect().top(), ui.clip_rect().bottom())
                    } else {
                        (pointer.x, ui.clip_rect().left(), ui.clip_rect().right())
                    };
                    // Off the strip altogether, the tab is on its way to the
                    // terminal to be docked, and is neither reordered nor
                    // scrolled along the strip after it.
                    let across = if vertical { pointer.x } else { pointer.y };
                    let band = if vertical {
                        ui.clip_rect().x_range()
                    } else {
                        ui.clip_rect().y_range()
                    };
                    over_terminal = !band.expand(TAB_DOCK_SLACK).contains(across);
                    // Over the middle of another tab, the drop joins the two
                    // rather than reordering; the tab is only moved past one
                    // once the pointer is through its middle half.
                    into_tab = (!over_terminal)
                        .then(|| dock_onto(&spans, from, along))
                        .flatten();
                    to_move = if over_terminal || into_tab.is_some() {
                        None
                    } else {
                        drop_slot(&spans, from, along).map(|to| (from, to))
                    };
                    if let Some(target) = into_tab {
                        let fits = self.tabs[target].pane_count() + self.tabs[from].pane_count()
                            <= tab::MAX_PANES
                            && !self.tabs[target].is_game()
                            && !self.tabs[from].is_game();
                        let colour = if fits {
                            ui.visuals().selection.bg_fill
                        } else {
                            ui.visuals().error_fg_color
                        };
                        let rect = rects[target].shrink(1.0);
                        ui.painter()
                            .rect_filled(rect, 3.0, colour.gamma_multiply(0.3));
                        ui.painter()
                            .rect_stroke(rect, 3.0, egui::Stroke::new(2.0_f32, colour));
                    }
                    // Held against either edge of the strip, the drag scrolls
                    // it: the tabs beyond the edge are where the user is
                    // trying to put this one, and they cannot be reached by
                    // a pointer that has nowhere further to go.
                    let edge = 24.0;
                    let push = if over_terminal {
                        0.0
                    } else if along < near + edge {
                        1.0
                    } else if along > far - edge {
                        -1.0
                    } else {
                        0.0
                    };
                    if push != 0.0 {
                        let delta = push * 8.0;
                        ui.scroll_with_delta(if vertical {
                            egui::vec2(0.0, delta)
                        } else {
                            egui::vec2(delta, 0.0)
                        });
                        // The pointer holding still is still asking for the
                        // strip to move, and an idle loop asks for no frames.
                        ui.ctx().request_repaint();
                    }
                }
            });
        });

        if let Some(index) = drag_began {
            // What was shown before this drag made the dragged tab the one
            // shown, which is what it would be dropped into.
            let onto = self.tabs.get(self.active).map_or(0, |tab| tab.uid);
            self.tab_drag = Some(TabDrag {
                uid: self.tabs[index].uid,
                onto,
                over_terminal: false,
                into_tab: None,
                held: None,
                sprung: false,
            });
        }
        // Let go over the middle of another tab: the dragged one joins it,
        // beside the pane that has the keyboard there, and that tab is shown.
        if dragging.is_none() && ui.input(|i| i.pointer.any_released()) {
            if let Some((uid, into)) = self
                .tab_drag
                .and_then(|drag| drag.into_tab.map(|into| (drag.uid, into)))
            {
                self.tab_drag = None;
                if let Some(index) = self.tabs.iter().position(|tab| tab.uid == into) {
                    let onto = At {
                        tab: index,
                        pane: self.tabs[index].focus,
                    };
                    self.dock_tab(uid, onto, Toward::Right);
                    // Docking has taken a tab out of the strip, so nothing
                    // else collected by index this frame still means what it
                    // did.
                    return;
                }
            }
        }
        if dragging.is_some() {
            if let Some(drag) = self.tab_drag.as_mut() {
                drag.over_terminal = over_terminal;
                let target = into_tab.map(|target| self.tabs[target].uid);
                drag.into_tab = target;
                // Held over the middle of a tab, it springs open - the way a
                // folder does under a dragged file - so the drop can be aimed
                // at its panes rather than only at its name.
                let now = ui.input(|i| i.time);
                match (target, drag.held) {
                    (Some(target), Some((held, since))) if target == held => {
                        let waited = now - since;
                        if waited >= TAB_SPRING {
                            drag.onto = target;
                            drag.sprung = true;
                        } else {
                            ui.ctx().request_repaint_after(Duration::from_secs_f64(
                                TAB_SPRING - waited,
                            ));
                        }
                    }
                    (Some(target), _) => {
                        drag.held = Some((target, now));
                        ui.ctx()
                            .request_repaint_after(Duration::from_secs_f64(TAB_SPRING));
                    }
                    (None, _) => drag.held = None,
                }
            }
        }
        if let Some((from, to)) = to_move {
            self.move_tab(from, to);
            // The tab ends where the pointer is, and it was active already
            // from the moment it was taken hold of.
            to_activate = Some(to);
        }
        if let Some(drag) = self.tab_drag.filter(|_| dragging.is_some()) {
            // Over the terminal, or once a tab has sprung open, the tab shown
            // is the one the drop goes into, so it is aimed at what is on
            // screen; otherwise it is the dragged one, as for a reorder.
            // Looked up by uid after any reorder above has moved the tabs.
            let onto = self.tabs.iter().position(|tab| tab.uid == drag.onto);
            let moving = self.tabs.iter().position(|tab| tab.uid == drag.uid);
            if let (Some(onto), Some(moving)) = (onto, moving) {
                to_activate = Some(if (drag.over_terminal || drag.sprung) && onto != moving {
                    onto
                } else {
                    moving
                });
            }
        }
        if let Some(index) = to_activate {
            self.activate_tab(index);
        }
        if let Some((index, side)) = to_split {
            if let Some(focus) = self.tabs.get(index).map(|tab| tab.focus) {
                self.split_pane(
                    At {
                        tab: index,
                        pane: focus,
                    },
                    side,
                );
            }
        }
        if let Some(index) = to_unsplit {
            self.unsplit_tab(index);
        }
        if let Some((index, colour)) = to_colour {
            if let Some(tab) = self.tabs.get_mut(index) {
                tab.colour = colour;
            }
        }
        if let Some(index) = to_rename {
            // Seeded with the names on screen, so renaming is an edit rather
            // than starting from nothing.
            let tab = &self.tabs[index];
            self.renaming = Some(Renaming {
                tab: tab.uid,
                names: tab
                    .sessions()
                    .map(|s| (s.uid, s.title(with_namespace)))
                    .collect(),
                colour: tab.colour,
                focus: true,
            });
        }
        // Applied after the loop: closing a tab shifts every index after it.
        if let Some(index) = to_close {
            self.close_tab(index);
        }
        if to_reopen {
            self.reopen_closed_tab();
        }
    }

    /// Moves the tab at `from` to `to`, shifting the ones between along.
    ///
    /// Everything that remembers a tab by its index has to move with it, which
    /// is the active tab. A rename in progress remembers it by uid, and needs
    /// nothing.
    pub(super) fn move_tab(&mut self, from: usize, to: usize) {
        if from == to || from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        self.active = moved_index(self.active, from, to);
    }

    /// Makes a tab the active one, which is the tab drawn and the one every
    /// other feature works on. Which of its panes is current is the tab's own
    /// business - see [`Tab::focus`].
    pub(super) fn activate_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    /// Opens another session against one side of a pane, of the same kind
    /// as the session in that pane.
    ///
    /// The new session takes the keyboard, the way a newly opened tab does. A
    /// tab already holding [`tab::MAX_PANES`] sessions is left alone, and
    /// says so.
    pub(super) fn split_pane(&mut self, at: At, side: Toward) {
        // The pane's own profile, not the one Ctrl+T would open: splitting a
        // CMD pane is asking for a second CMD beside it, and it used to come up
        // as an IRIS session because that was the new-tab default.
        let Some(profile) = self.pane(at).map(|pane| pane.profile.clone()) else {
            return;
        };
        self.open_pane(at, side, profile);
    }

    /// [`App::split_pane`], with the shell called `shell` in the new pane
    /// instead of the pane's own kind of session. A shell gone from the list
    /// since the menu was drawn splits nothing, and says so.
    pub(super) fn split_pane_with_shell(&mut self, at: At, side: Toward, shell: &str) {
        let Some(found) = crate::plugins::shells::available()
            .into_iter()
            .find(|s| s.name == shell)
        else {
            self.set_status(tr1("{} is no longer available.", shell));
            return;
        };
        self.open_pane(at, side, Profile::for_shell(&found));
    }

    fn open_pane(&mut self, at: At, side: Toward, profile: Profile) {
        // A tab holding the easter egg holds no session and cannot hold one:
        // splitting it would put a live IRIS session in a pane nothing draws.
        // Asked before the session is started, so a refusal starts nothing.
        let Some(tab) = self.tabs.get(at.tab) else {
            return;
        };
        if tab.is_game() || tab.pane(at.pane).is_none() {
            return;
        }
        if !tab.can_split() {
            self.set_status(tr1(
                "A tab holds at most {} sessions.",
                &tab::MAX_PANES.to_string(),
            ));
            return;
        }
        let (cols, rows) = self.initial_size(&profile);
        let opened = Tab::new(profile, &self.settings, cols, rows);
        if let Ok(pane) = self.tabs[at.tab].add_pane(at.pane, side, opened) {
            self.tabs[at.tab].focus = pane;
            self.active = at.tab;
        }
    }

    /// Writes the transcripts to `to` from now on, and offers to take the
    /// ones already written along.
    ///
    /// Every session that is logging starts a new file in the new folder
    /// straight away, before anything is moved: a file still open for writing
    /// cannot be moved on Windows, and the transcript of a session that kept
    /// writing to the old folder would be the one file left behind.
    pub(super) fn change_log_dir(&mut self, ctx: &Context, to: std::path::PathBuf) {
        let from = self.settings.log_dir.clone();
        if from == to {
            return;
        }
        self.settings.log_dir = to.clone();
        self.handle_request(ctx, UiRequest::SettingsChanged);
        let settings = &self.settings;
        for tab in &mut self.tabs {
            tab.for_each_session_mut(|session| session.reopen_log(settings));
        }
        let plan = crate::features::log_move::Plan::new(&from, &to);
        if plan.is_empty() {
            self.set_status(tr1("Logs now go to {}", &to.display().to_string()));
        } else {
            self.panels.log_move = Some(crate::ui::log_move_view::LogMove::Asking(plan));
        }
    }

    /// Moves one pane of a split tab against a side of another of its
    /// panes. The keyboard goes with it: it is the pane just taken hold of.
    pub(super) fn move_pane(&mut self, at: At, onto: Pane, side: Toward) {
        let Some(tab) = self.tabs.get_mut(at.tab) else {
            return;
        };
        let Some(split) = tab.split.as_mut() else {
            return;
        };
        if split.tree.relocate(at.pane, onto, side) {
            split.zoomed = false;
            tab.focus = at.pane;
        }
    }

    /// Moves the keyboard to the pane next to the focused one, the way
    /// `toward` says, in the active tab. Nothing there, nothing moves.
    pub(super) fn focus_pane(&mut self, toward: Toward) {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return;
        };
        if let Some(pane) = tab.neighbour(toward) {
            tab.focus = pane;
        }
    }

    /// Trades the focused pane's place with the one next to it, the way
    /// `toward` says. The keyboard stays with the session, which is now
    /// where the other one was - the pane has been moved, not the focus.
    pub(super) fn swap_pane(&mut self, toward: Toward) {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return;
        };
        let focus = tab.focus;
        let Some(other) = tab.neighbour(toward) else {
            return;
        };
        if let Some(split) = tab.split.as_mut() {
            split.tree.swap(focus, other);
        }
    }

    /// Gives one pane of a split tab a tab of its own, just after this one,
    /// and shows it there. Nothing is closed.
    pub(super) fn detach_to_tab(&mut self, at: At) {
        let Some(gone) = self
            .tabs
            .get_mut(at.tab)
            .and_then(|tab| tab.detach_pane(at.pane))
        else {
            return;
        };
        self.tabs.insert(at.tab + 1, gone);
        self.active = at.tab + 1;
    }

    /// Takes a split tab back to one session, and gives each of the others a
    /// tab of its own, in pane order, just after it.
    ///
    /// Promoted rather than closed: they are live IRIS sessions, and "remove
    /// split" is a sentence about the layout. Closing the tab is what closes
    /// them all. The keyboard stays with whichever session had it.
    pub(super) fn unsplit_tab(&mut self, index: usize) {
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        let had_focus = tab.focus;
        let others = tab.detach_all();
        let count = others.len();
        for (i, other) in others.into_iter().enumerate() {
            self.tabs.insert(index + 1 + i, other);
        }
        if had_focus.0 <= count {
            self.active = index + had_focus.0;
        }
    }

    /// Moves the tab with `uid` into another, against the `side` of the pane
    /// at `onto` - a tab dropped onto the terminal.
    pub(super) fn dock_tab(&mut self, uid: u64, onto: At, side: Toward) {
        let Some(from) = self.tabs.iter().position(|tab| tab.uid == uid) else {
            return;
        };
        let Some(host) = self.tabs.get(onto.tab) else {
            return;
        };
        if from == onto.tab || host.pane(onto.pane).is_none() {
            return;
        }
        let total = host.pane_count() + self.tabs[from].pane_count();
        if total > tab::MAX_PANES || host.is_game() || self.tabs[from].is_game() {
            self.set_status(tr1(
                "A tab holds at most {} sessions.",
                &tab::MAX_PANES.to_string(),
            ));
            return;
        }
        let moving = self.tabs.remove(from);
        let host = if from < onto.tab {
            onto.tab - 1
        } else {
            onto.tab
        };
        if let Err(moving) = self.tabs[host].adopt(onto.pane, side, moving) {
            // Checked above, so this is not reached; but a tab is a live
            // session, and one put back is better than one dropped.
            self.tabs.insert(from.min(self.tabs.len()), *moving);
            return;
        }
        self.active = host;
    }

    /// What [`App::restore_session`] needs to open these tabs again.
    ///
    /// The easter egg is left out: it has no far side to reconnect to, and
    /// `/snake` is how anybody who wants it back gets it.
    pub(super) fn session_snapshot(&self) -> SavedSession {
        let kept: Vec<bool> = self.tabs.iter().map(|tab| !tab.is_game()).collect();
        let tabs = self
            .tabs
            .iter()
            .filter(|tab| !tab.is_game())
            .map(saved_tab)
            .collect();
        SavedSession {
            active: kept_index(self.active, &kept),
            tabs,
        }
    }

    /// Opens again the tabs [`App::session_snapshot`] wrote down, in the same
    /// order, with the same names and splits, and the same one in front.
    ///
    /// Each one connects the way a newly opened tab would, autologon included:
    /// the profile is all it keeps, and the password was never in it.
    pub(super) fn restore_session(&mut self, saved: SavedSession) {
        for entry in saved.tabs {
            self.restore_tab(entry);
        }
        self.active = saved.active.min(self.tabs.len().saturating_sub(1));
    }

    /// Opens one saved tab at the end of the strip.
    fn restore_tab(&mut self, entry: SavedTab) {
        let (panes, tree, focus) = saved_layout(&entry);
        self.open_tab(entry.profile);
        let index = self.tabs.len() - 1;
        let tab = &mut self.tabs[index];
        tab.custom_title = entry.title;
        tab.colour = entry
            .colour
            .as_deref()
            .and_then(crate::config::theme::parse_hex);
        tab.replay(&entry.screen);
        tab.resume_namespace = entry.namespace;
        let Some(tree) = tree else {
            return;
        };
        let mut others = Vec::with_capacity(panes.len());
        for pane in panes {
            let (cols, rows) = self.initial_size(&pane.profile);
            let mut opened = Tab::new(pane.profile, &self.settings, cols, rows);
            opened.custom_title = pane.title;
            opened.replay(&pane.screen);
            opened.resume_namespace = pane.namespace;
            others.push(opened);
        }
        let tab = &mut self.tabs[index];
        tab.split = Some(Split {
            tree,
            others,
            zoomed: false,
        });
        tab.focus = focus;
    }

    /// Names one tab.
    ///
    /// A window rather than an editable label in the strip: the terminal claims
    /// the keyboard whenever nothing else holds it, and a field that has to win
    /// that fight every frame is a worse trade than one dialog.
    pub(super) fn rename_tab_dialog(&mut self, ctx: &Context) {
        let Some(mut renaming) = self.renaming.take() else {
            return;
        };
        // Gone, or no longer a tab of its own - docked into another, or its
        // session moved out from under it: the dialog has nothing to name.
        let Some(index) = self.tabs.iter().position(|tab| tab.uid == renaming.tab) else {
            return;
        };

        let mut open = true;
        let mut commit = false;
        let mut cancel = false;

        crate::ui::dialog::show(ctx, "nit-rename-tab", tr("Rename tab"), &mut open, |ui| {
            {
                // A split tab is several sessions under one entry, so it is
                // renamed several names at a time. The `1:`, `2:` are the
                // panes themselves and cannot be edited away.
                let split = renaming.names.len() > 1;
                for (i, (_, name)) in renaming.names.iter_mut().enumerate() {
                    let mut response = None;
                    ui.horizontal(|ui| {
                        if split {
                            ui.label(format!("{}:", i + 1));
                        }
                        response =
                            Some(ui.add(egui::TextEdit::singleline(name).desired_width(220.0)));
                    });
                    let response = response.expect("the field is always added");
                    if i == 0 && renaming.focus {
                        response.request_focus();
                        renaming.focus = false;
                    }
                    if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        commit = true;
                    }
                }

                // The one button there used to be for this is what an empty
                // field already does, and two ways to say the same thing in a
                // dialog this small only asks the user which one is real.
                ui.weak(tr("Empty goes back to default."));
                ui.add_space(4.0);
                ui.label(tr("Colour"));
                if let Some(picked) = crate::ui::tab_colour::picker(ui, renaming.colour) {
                    renaming.colour = picked;
                }
                use crate::ui::dialog::{actions, button, Role};
                actions(ui, |ui| {
                    if button(ui, tr("Rename"), Role::Suggested).clicked() {
                        commit = true;
                    }
                    if button(ui, tr("Cancel"), Role::Plain).clicked() {
                        cancel = true;
                    }
                });
            }
        });

        if commit {
            let named = |text: &str| {
                let name = text.trim().to_string();
                (!name.is_empty()).then_some(name)
            };
            self.tabs[index].colour = renaming.colour;
            // Each to its own session, wherever it is now - a pane closed
            // while the dialog was open takes its name with it.
            for (uid, name) in &renaming.names {
                for tab in &mut self.tabs {
                    tab.for_each_session_mut(|session| {
                        if session.uid == *uid {
                            session.custom_title = named(name);
                        }
                    });
                }
            }
        } else if !(cancel || !open) {
            // Still open, so the draft survives to the next frame.
            self.renaming = Some(renaming);
        }
    }
}

/// The row the tabs are laid out in: the full height it is given, aligned to
/// its top.
///
/// Not `ui.horizontal`, which starts a row a button high and centres what is
/// taller in it. A tab is taller than a button, so it was pushed 2 px down from
/// the top of the title bar, with that strip of bar showing above it and its
/// own bottom cut off.
fn tab_row<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let size = egui::vec2(ui.available_width(), ui.available_height());
    ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Min), add)
        .inner
}

/// Which way a strip of tabs runs, and what it is part of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Strip {
    /// A row of its own, under the title bar.
    Row,
    /// The title bar itself, which the tabs then also move the window by.
    TitleBar,
    /// A column down a title bar on the side of the window.
    Column,
}

/// The narrowest and widest a tab is drawn. Between the two it fits its name;
/// past the widest the name is cut short with an ellipsis, so one long name
/// cannot push every other tab off the strip.
const TAB_MIN_WIDTH: f32 = 120.0;
const TAB_MAX_WIDTH: f32 = 220.0;

/// The air between a tab's close button and its name. The name is held off
/// both ends by this much, not only the button's, so it stays centred whichever
/// side the button is on; without it a name long enough to be cut short ran
/// right up against the cross.
const TAB_CLOSE_GAP: f32 = 6.0;

/// How tall a tab is: a little taller than a button, so the strip reads as
/// tabs rather than as one more row of buttons. The title bar sizes its row by
/// this when the tabs are in it, or the buttons ahead of them would be centred
/// on a row the tabs then make taller.
pub(super) fn tab_height(ui: &egui::Ui) -> f32 {
    match crate::ui::skin::Kit::of(ui.ctx()) {
        // A skin's tab, grown with the title bar scale as a widget is.
        Some(kit) => {
            let scale = (ui.spacing().interact_size.y / 18.0).max(1.0);
            (kit.metrics().tab_height * kit.u() * scale).round()
        }
        None => ui.spacing().interact_size.y + 4.0,
    }
}

/// One tab, drawn the way elementary's Files and Terminal draw theirs: the
/// whole height of its bar, square, the one shown filled with the terminal's
/// colour so it runs on into the terminal, the rest flat until hovered. The
/// close button is inside it, at whichever end `close_at` says - shown on the
/// tab being looked at and on the one under the pointer - and the name is
/// centred. `width` is
/// the tab's share of a row it fills; `None` sizes it to its name.
///
/// The close button is registered after the tab, so it is the one a click on
/// it reaches; the tab itself takes clicks, drags and the context menu.
/// Where the shown tab was drawn this frame, and in what.
pub(super) fn shown_tab_id() -> egui::Id {
    egui::Id::new("nit-shown-tab")
}

/// Paints over the boundary between the shown tab and the terminal it runs on
/// into, a pixel each side, in the tab's colour.
///
/// At a title bar scale the boundary falls between two pixels, and egui
/// feathers the edge of every fill it draws: the tab and the terminal each
/// covered only part of that row, and the bar's own colour showed through
/// between them as a line. Painted from the terminal's side, after both, on
/// their own layer, and only the width of the tab.
pub(super) fn seal_shown_tab(
    ctx: &egui::Context,
    layer: egui::LayerId,
    terminal: egui::Rect,
    bar: crate::config::BarPosition,
    background: egui::Color32,
) {
    let Some((tab, fill)) = ctx.data(|d| d.get_temp::<(egui::Rect, egui::Color32)>(shown_tab_id()))
    else {
        return;
    };
    let near = |a: f32, b: f32| (a - b).abs() <= 2.0;
    let Some((own, theirs)) = seam_bands(tab, terminal, bar, ctx.pixels_per_point(), near) else {
        return;
    };
    let painter = ctx.layer_painter(layer);
    painter.rect_filled(own, 0.0, background);
    painter.rect_filled(theirs, 0.0, fill);
}

/// The two bands that close the seam between the tab and the terminal: the
/// pixel the terminal's edge falls inside, in the terminal's colour, and from
/// there to the tab, in the tab's. Split at the pixel boundary so neither
/// reaches into the other: one band in the tab's colour over the whole seam
/// put a pixel of the tab on the terminal.
fn seam_bands(
    tab: egui::Rect,
    terminal: egui::Rect,
    bar: crate::config::BarPosition,
    pixels_per_point: f32,
    near: impl Fn(f32, f32) -> bool,
) -> Option<(egui::Rect, egui::Rect)> {
    use crate::config::BarPosition;
    let down = |v: f32| (v * pixels_per_point).floor() / pixels_per_point;
    let up = |v: f32| (v * pixels_per_point).ceil() / pixels_per_point;
    let px = 1.0 / pixels_per_point;
    let across = |ys: std::ops::RangeInclusive<f32>| egui::Rect::from_x_y_ranges(tab.x_range(), ys);
    let along = |xs: std::ops::RangeInclusive<f32>| egui::Rect::from_x_y_ranges(xs, tab.y_range());
    Some(match bar {
        BarPosition::Top if near(tab.bottom(), terminal.top()) => {
            let e = terminal.top();
            (
                across(down(e)..=up(e)),
                across(tab.bottom().min(down(e)) - px..=down(e)),
            )
        }
        BarPosition::Bottom if near(tab.top(), terminal.bottom()) => {
            let e = terminal.bottom();
            (
                across(down(e)..=up(e)),
                across(up(e)..=tab.top().max(up(e)) + px),
            )
        }
        BarPosition::Left if near(tab.right(), terminal.left()) => {
            let e = terminal.left();
            (
                along(down(e)..=up(e)),
                along(tab.right().min(down(e)) - px..=down(e)),
            )
        }
        BarPosition::Right if near(tab.left(), terminal.right()) => {
            let e = terminal.right();
            (
                along(down(e)..=up(e)),
                along(up(e)..=tab.left().max(up(e)) + px),
            )
        }
        _ => return None,
    })
}

/// Below this contrast ratio a tab's cross is drawn in the tab's ink instead
/// of the theme's colour for it. Low on purpose: it catches a colour that is
/// gone, not one that is merely soft.
const MIN_CROSS_CONTRAST: f32 = 1.5;

#[allow(clippy::too_many_arguments)] // one tab's look, every part of which the strip decides
fn tab_pill(
    ui: &mut egui::Ui,
    uid: u64,
    selected: bool,
    text: String,
    colours: (Option<egui::Color32>, Option<egui::Color32>),
    close_colour: Option<egui::Color32>,
    [width, height]: [Option<f32>; 2],
    close_at: crate::config::TabCloseSide,
) -> (egui::Response, Option<egui::Response>) {
    let base = tab_height(ui);
    // A row's full height, so the tab runs from top to bottom of its bar; a
    // column's tabs are each one tab tall.
    // Under a skin the row is the title bar's, which the tab fits rather
    // than stretches; without one a tab is at least a tab tall.
    let skinned = crate::ui::skin::Kit::of(ui.ctx()).is_some();
    let height = height.unwrap_or_else(|| {
        if skinned {
            ui.available_height()
        } else {
            ui.available_height().max(base)
        }
    });
    let close_side = base - 8.0;
    let pad = 8.0;
    let font = egui::TextStyle::Button.resolve(ui.style());
    let outer = width.unwrap_or(TAB_MAX_WIDTH);
    let inset = pad + close_side + TAB_CLOSE_GAP;
    let room = outer - 2.0 * inset;
    let mut job = egui::text::LayoutJob::simple_singleline(text, font, egui::Color32::PLACEHOLDER);
    job.wrap = egui::text::TextWrapping {
        max_width: room,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = ui.fonts(|f| f.layout_job(job));
    let width = width
        .unwrap_or_else(|| (galley.size().x + 2.0 * inset).clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH));
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let response = ui.interact(
        rect,
        egui::Id::new(("nit-tab", uid)),
        egui::Sense::click_and_drag(),
    );
    let close_x = match close_at {
        crate::config::TabCloseSide::Left => rect.left() + pad + close_side / 2.0,
        crate::config::TabCloseSide::Right => rect.right() - pad - close_side / 2.0,
    };
    // On whole pixels, its side an even number of them: the cross is drawn
    // about the square's centre, and a square starting part-way into a pixel
    // put the cross a pixel off the middle of the square it is hovered in.
    let close_rect = pixel_square(
        ui.ctx().pixels_per_point(),
        egui::pos2(close_x, rect.center().y),
        close_side,
    );
    let shows_close = selected || ui.rect_contains_pointer(rect);
    let close = shows_close.then(|| {
        ui.interact(
            close_rect,
            egui::Id::new(("nit-tab-close", uid)),
            egui::Sense::click(),
        )
    });

    if let Some(kit) = crate::ui::skin::Kit::of(ui.ctx()) {
        if ui.is_rect_visible(rect) {
            skinned_tab(
                ui,
                &kit,
                rect,
                &response,
                selected,
                galley,
                close.as_ref(),
                close_rect,
            );
        }
        return (response, close);
    }
    if ui.is_rect_visible(rect) {
        let visuals = ui.visuals();
        if selected {
            let fill = colours.0.unwrap_or(visuals.extreme_bg_color);
            ui.painter().rect_filled(rect, 0.0, fill);
            // For the terminal to seal the edge they share - see
            // `seal_shown_tab`.
            ui.ctx()
                .data_mut(|d| d.insert_temp(shown_tab_id(), (rect, fill)));
        } else if response.hovered() || response.dragged() {
            ui.painter().rect_filled(
                rect,
                0.0,
                visuals.widgets.hovered.weak_bg_fill.gamma_multiply(0.5),
            );
        }
        let ink = match colours.1 {
            Some(ink) if selected => ink,
            _ if selected => visuals.strong_text_color(),
            _ => visuals.text_color().gamma_multiply(0.7),
        };
        let at = rect.center() - galley.size() / 2.0;
        ui.painter().galley(at, galley, ink);

        if let Some(close) = close.as_ref() {
            if close.hovered() {
                ui.painter()
                    .rect_filled(close_rect, 2.0, visuals.widgets.hovered.bg_fill);
            }
            let arm = close_side * 0.22;
            let c = close_rect.center();
            // The theme's colour for it when there is one - the Themes page
            // offers it - and otherwise the tab's own ink. Also the ink when
            // the colour all but vanishes into what the tab is filled with
            // just now: one colour has to serve the shown tab and a hovered
            // one, and on a theme whose two differ widely it cannot suit both.
            let behind = if selected {
                colours.0.unwrap_or(visuals.extreme_bg_color)
            } else {
                visuals.panel_fill
            };
            let colour = close_colour
                .filter(|&c| crate::config::theme::contrast(c, behind) >= MIN_CROSS_CONTRAST)
                .unwrap_or(ink);
            let stroke = egui::Stroke::new(1.4_f32, colour);
            ui.painter().line_segment(
                [c + egui::vec2(-arm, -arm), c + egui::vec2(arm, arm)],
                stroke,
            );
            ui.painter().line_segment(
                [c + egui::vec2(-arm, arm), c + egui::vec2(arm, -arm)],
                stroke,
            );
        }
    }
    (response, close)
}

/// One tab as the skin draws a tab: its frame, the name in the skin's ink,
/// and the cross as a small pixel mark, on a button of its own while the
/// pointer is over it.
#[allow(clippy::too_many_arguments)] // the parts of one tab `tab_pill` has already worked out
fn skinned_tab(
    ui: &egui::Ui,
    kit: &crate::ui::skin::Kit,
    rect: egui::Rect,
    response: &egui::Response,
    selected: bool,
    galley: std::sync::Arc<egui::Galley>,
    close: Option<&egui::Response>,
    close_rect: egui::Rect,
) {
    use crate::ui::skin::{prim, St};
    let style = kit.style();
    let st = St {
        on: selected,
        hovered: response.hovered(),
        ..St::default()
    };
    let painter = ui.painter();
    painter.extend(style.tab(kit, rect, st));
    // The shown tab stands two pixels taller, and its name with it.
    let lift = if selected { 2.0 * kit.u() } else { 0.0 };
    let at = rect.center() - galley.size() / 2.0 - egui::vec2(0.0, lift);
    // Over the page's text colour the galley was laid out in: the tab's own
    // ink, as the tab is the skin's - a user's page text need not read on it.
    painter.galley_with_override_text_color(
        egui::pos2(at.x.round(), at.y.round()),
        galley,
        style.tab_text(kit, st),
    );
    if let Some(close) = close {
        let square = pixel_square(
            kit.ppp,
            close_rect.center() - egui::vec2(0.0, lift),
            close_rect.width().min(16.0 * kit.u()),
        );
        if close.hovered() {
            painter.extend(style.button(
                kit,
                square,
                St {
                    hovered: true,
                    pressed: close.is_pointer_button_down_on(),
                    ..St::default()
                },
            ));
        }
        const CROSS: [&str; 6] = ["##..##", ".####.", "..##..", "..##..", ".####.", "##..##"];
        let at = prim::centred(square, &CROSS, kit.ppp);
        let ink = style.tab_text(kit, st);
        painter.extend(prim::sprite(at, &CROSS, &[('#', ink)], kit.ppp));
    }
}

/// A square of about `side` points centred on `centre`, its edges on whole
/// pixels and its side an even number of them - so the middle of it falls on
/// a pixel boundary, where a symmetric mark drawn about it stays symmetric.
fn pixel_square(ppp: f32, centre: egui::Pos2, side: f32) -> egui::Rect {
    let px = ((side * ppp / 2.0).round() * 2.0).max(2.0);
    let middle = (centre.to_vec2() * ppp).round();
    let min = (middle - egui::Vec2::splat(px / 2.0)) / ppp;
    egui::Rect::from_min_size(min.to_pos2(), egui::Vec2::splat(px / ppp))
}

/// Where the tab being dragged belongs, given the left and right edge of every
/// tab as drawn and where the pointer is - or `None` while it is still over its
/// own place.
///
/// A tab moves once the pointer passes the *middle* of a neighbour, not its
/// edge. Tabs differ in width, and swapping at the edge would put a wide
/// neighbour straight back under the pointer, which would swap them back on
/// the next frame and flicker for as long as the pointer stayed there. Past
/// the middle, the neighbour ends up on the far side of the pointer either way.
///
/// A pointer that has passed several middles in one frame - a fast flick -
/// moves the tab all the way, not one place per frame.
fn drop_slot(spans: &[(f32, f32)], from: usize, x: f32) -> Option<usize> {
    let middle = |i: usize| (spans[i].0 + spans[i].1) / 2.0;
    if from >= spans.len() {
        return None;
    }
    (from + 1..spans.len())
        .rev()
        .find(|&i| x > middle(i))
        .or_else(|| (0..from).find(|&i| x < middle(i)))
}

/// The tab a dragged one would join if let go at `x`: one whose middle half
/// the pointer is over, other than the dragged tab itself.
///
/// The middle half rather than the whole tab, so the quarters at either end
/// are still the way past it - [`drop_slot`] moves a tab once the pointer is
/// through a neighbour's middle, which is then the far quarter.
fn dock_onto(spans: &[(f32, f32)], from: usize, x: f32) -> Option<usize> {
    spans.iter().enumerate().position(|(i, &(start, end))| {
        let quarter = (end - start) / 4.0;
        i != from && x > start + quarter && x < end - quarter
    })
}

/// Where the tab at `index` is after the one at `from` has moved to `to`.
fn moved_index(index: usize, from: usize, to: usize) -> usize {
    if index == from {
        to
    } else if from < to && (from + 1..=to).contains(&index) {
        index - 1
    } else if to < from && (to..from).contains(&index) {
        index + 1
    } else {
        index
    }
}

/// How many closed tabs Ctrl+Alt+T can go back through. Each one holds up to
/// [`SAVED_SCREEN_LINES`] of text, so the list is not left to grow all day.
const CLOSED_TABS_KEPT: usize = 20;

/// How far past the strip's own edge a dragged tab may stray and still be
/// reordering, in points. A hand dragging along a row wanders up and down; it
/// should take a deliberate move toward the terminal to start docking.
const TAB_DOCK_SLACK: f32 = 12.0;

/// How long, in seconds, a dragged tab is held over the middle of another
/// before that one is shown. Long enough not to flick through every tab the
/// pointer crosses on its way along the strip.
const TAB_SPRING: f64 = 1.5;

/// A tab as [`SavedTab`] keeps it, split included.
fn saved_tab(tab: &Tab) -> SavedTab {
    let split = tab.split.as_ref();
    SavedTab {
        colour: tab.colour.map(crate::config::theme::to_hex),
        panes: split
            .map(|split| {
                split
                    .others
                    .iter()
                    .map(|other| SavedPane {
                        profile: saved_profile(other),
                        namespace: saved_namespace(other),
                        title: other.custom_title.clone(),
                        screen: saved_screen(other),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        layout: split.map(|split| split.tree.encode()),
        focus: if split.is_some() { tab.focus.0 } else { 0 },
        ..saved_pane(tab)
    }
}

/// The other panes a saved tab holds, where they go, and which one had the
/// keyboard - or no layout at all for a tab that was not split.
///
/// Whatever the file says, what comes back is something the tab can hold: a
/// layout that does not name each pane exactly once is replaced by the panes
/// in a row, and panes past [`tab::MAX_PANES`] are left closed. A file from
/// before a tab could hold more than two is read through its one `split`.
fn saved_layout(entry: &SavedTab) -> (Vec<SavedPane>, Option<Node>, Pane) {
    let (mut panes, layout, focus) = match (&entry.split, entry.panes.is_empty()) {
        (Some(split), true) => {
            let pane = SavedPane {
                profile: split.profile.clone(),
                title: split.title.clone(),
                namespace: split.namespace.clone(),
                screen: split.screen.clone(),
            };
            let dir = match split.dir {
                SavedDir::Right => SplitDir::Right,
                SavedDir::Bottom => SplitDir::Bottom,
            };
            let ratio = if split.ratio.is_finite() {
                split.ratio.clamp(0.0, 1.0)
            } else {
                0.5
            };
            let tree = Node::Split {
                dir,
                ratio,
                first: Box::new(Node::Leaf(Pane(0))),
                second: Box::new(Node::Leaf(Pane(1))),
            };
            (vec![pane], Some(tree), usize::from(entry.second_focused))
        }
        _ => (
            entry.panes.clone(),
            entry.layout.as_deref().and_then(Node::parse),
            entry.focus,
        ),
    };
    if panes.is_empty() {
        return (panes, None, Pane::FIRST);
    }
    panes.truncate(tab::MAX_PANES - 1);
    let count = panes.len() + 1;
    let tree = layout.filter(|tree| tree.covers(count)).unwrap_or_else(|| {
        let mut row = Node::Leaf(Pane(0));
        for i in 1..count {
            row.dock(Pane(i - 1), Toward::Right, Node::Leaf(Pane(i)));
        }
        row
    });
    let focus = if focus < count {
        Pane(focus)
    } else {
        Pane::FIRST
    };
    (panes, Some(tree), focus)
}

/// One pane on its own, as a tab of its own: what closing one pane of a split
/// leaves to reopen.
fn saved_pane(tab: &Tab) -> SavedTab {
    SavedTab {
        profile: saved_profile(tab),
        namespace: saved_namespace(tab),
        title: tab.custom_title.clone(),
        screen: saved_screen(tab),
        ..SavedTab::default()
    }
}

/// How many lines of a pane's output are kept for the next run. Enough to see
/// what was being done in it; the transcript log is where the whole of it is.
const SAVED_SCREEN_LINES: usize = 2_000;

/// A pane's profile, with a shell pointed at the folder it was last in.
///
/// Only a folder that is still there: a shell asked to start somewhere that
/// has gone does not start at all, and where it opened is a better answer.
fn saved_profile(tab: &Tab) -> Profile {
    let mut profile = tab.profile.clone();
    if let (Some(shell), Some(cwd)) = (profile.shell.as_mut(), tab.cwd.as_ref()) {
        let cwd = std::path::PathBuf::from(cwd);
        if cwd.is_dir() {
            shell.cwd = Some(cwd);
        }
    }
    profile
}

/// The namespace to take a pane back to. Only an IRIS one has any, and a pane
/// that never reached a prompt has nothing worth going back to.
fn saved_namespace(tab: &Tab) -> Option<String> {
    tab.namespace.clone().filter(|_| !tab.profile.is_shell())
}

/// The last [`SAVED_SCREEN_LINES`] lines a pane showed, as plain text.
fn saved_screen(tab: &Tab) -> String {
    let text = analyze::transcript(&tab.grid, analyze::Scope::All);
    let lines: Vec<&str> = text.lines().collect();
    let from = lines.len().saturating_sub(SAVED_SCREEN_LINES);
    lines[from..].join("\n")
}

/// Where the tab at `index` ends up once only the tabs marked in `kept` are
/// left: the nearest kept tab before it when it was not kept, or the first
/// one when there is none before it.
fn kept_index(index: usize, kept: &[bool]) -> usize {
    let before = kept.iter().take(index).filter(|&&k| k).count();
    let survives = kept.get(index).copied().unwrap_or(false);
    if survives {
        before
    } else {
        before.saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An app with a tab per name and no session behind any of them.
    fn app_of(names: &[&str]) -> App {
        let ctx = Context::default();
        let mut app = App::for_snapshot(&ctx, Settings::default(), Vec::new());
        for name in names {
            app.add_snapshot_tab(name, "USER>");
        }
        app
    }

    fn strip(app: &App) -> Vec<Vec<String>> {
        app.tabs
            .iter()
            .map(|tab| tab.sessions().map(|s| s.profile.name.clone()).collect())
            .collect()
    }

    /// Puts tab `from` into tab `onto` against the side of its first pane, the
    /// way a drop does.
    fn dock(app: &mut App, from: usize, onto: usize, side: Toward) {
        let uid = app.tabs[from].uid;
        let onto = At {
            tab: onto,
            pane: Pane::FIRST,
        };
        app.dock_tab(uid, onto, side);
    }

    /// The close square sits on whole pixels with its middle between two, at
    /// every display scale, wherever the tab happens to start.
    #[test]
    fn the_close_square_is_whole_pixels_with_an_even_side() {
        for ppp in [1.0, 1.25, 1.5, 2.0] {
            for x in [10.0, 10.3, 10.5, 10.8] {
                let r = pixel_square(ppp, egui::pos2(x, 7.37), 13.0);
                for v in [r.min.x, r.min.y, r.width()] {
                    let px = v * ppp;
                    assert!((px - px.round()).abs() < 1e-3, "{ppp} {x}: {r:?}");
                }
                assert_eq!((r.width() * ppp).round() as i32 % 2, 0);
                assert!((r.center().x - x).abs() <= 1.0 / ppp);
            }
        }
    }

    #[test]
    fn the_middle_of_another_tab_joins_it_and_its_ends_reorder() {
        let spans = [(0.0, 100.0), (100.0, 200.0), (200.0, 300.0)];
        assert_eq!(dock_onto(&spans, 0, 150.0), Some(1));
        assert_eq!(dock_onto(&spans, 0, 260.0), Some(2));
        assert_eq!(dock_onto(&spans, 0, 50.0), None, "not onto itself");
        // The far quarter of a neighbour is past its middle: a reorder.
        assert_eq!(dock_onto(&spans, 0, 180.0), None);
        assert_eq!(drop_slot(&spans, 0, 180.0), Some(1));
        assert_eq!(dock_onto(&spans, 0, 110.0), None);
    }

    /// A tab dropped beside another leaves the strip and turns up as a pane
    /// of the one it was dropped on, which is then the tab shown - wherever in
    /// the strip either of them was.
    #[test]
    fn a_tab_dropped_beside_another_becomes_a_pane_of_it() {
        let mut app = app_of(&["a", "b", "c"]);
        dock(&mut app, 2, 0, Toward::Right);
        assert_eq!(strip(&app), [vec!["a", "c"], vec!["b"]]);
        assert_eq!(app.active, 0);
        dock(&mut app, 0, 1, Toward::Left);
        assert_eq!(strip(&app), [vec!["b", "a", "c"]]);
        assert_eq!(app.active, 0);
        let tree = &app.tabs[0].split.as_ref().unwrap().tree;
        assert_eq!(tree.panes(), [Pane(1), Pane(2), Pane(0)]);
    }

    /// A pane dragged by its grip lands where it was dropped, keeps its
    /// session, and takes the keyboard.
    #[test]
    fn a_pane_moved_by_its_grip_lands_where_it_was_dropped() {
        let mut app = app_of(&["a", "b", "c"]);
        dock(&mut app, 1, 0, Toward::Right);
        dock(&mut app, 1, 0, Toward::Right);
        let at = At {
            tab: 0,
            pane: Pane(0),
        };
        app.move_pane(at, Pane(2), Toward::Down);
        let tab = &app.tabs[0];
        let tree = &tab.split.as_ref().unwrap().tree;
        assert_eq!(tree.neighbour(Pane(2), Toward::Down), Some(Pane(0)));
        assert_eq!(tab.focused().profile.name, "a");
        assert_eq!(strip(&app), [vec!["a", "b", "c"]], "nothing renumbered");
    }

    /// The grip, driven by the pointer as a user drives it: pressed, dragged
    /// across to the far side of the other pane, and let go there - the
    /// release landing over that pane, not over the grip's own.
    #[test]
    fn a_pane_dragged_by_its_grip_and_let_go_over_another_moves() {
        use egui::{Event, PointerButton, RawInput};
        let ctx = Context::default();
        let mut app = App::for_snapshot(&ctx, Settings::default(), Vec::new());
        app.add_snapshot_tab("a", "USER>");
        app.add_snapshot_tab("b", "USER>");
        dock(&mut app, 1, 0, Toward::Right);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 500.0));
        let run = |app: &mut App, events: Vec<Event>| {
            let input = RawInput {
                screen_rect: Some(screen),
                events,
                ..RawInput::default()
            };
            let _ = ctx.run(input, |ctx| app.frame(ctx));
        };
        run(&mut app, Vec::new());
        run(&mut app, Vec::new());
        let rects = app.pane_rects.clone();
        assert_eq!(rects.len(), 2, "{rects:?}");
        let grip = egui::pos2(rects[0].center().x, rects[0].top() + 7.0);
        let target = egui::pos2(rects[1].right() - 10.0, rects[1].center().y);
        let press = |pressed, pos| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        run(&mut app, vec![Event::PointerMoved(grip)]);
        run(&mut app, vec![press(true, grip)]);
        run(
            &mut app,
            vec![Event::PointerMoved(grip + egui::vec2(20.0, 20.0))],
        );
        run(&mut app, vec![Event::PointerMoved(target)]);
        run(&mut app, vec![Event::PointerMoved(target)]);
        run(&mut app, vec![press(false, target)]);
        run(&mut app, Vec::new());
        let tree = &app.tabs[0].split.as_ref().unwrap().tree;
        assert_eq!(tree.panes(), [Pane(1), Pane(0)], "{tree:?}");
        assert!(app.pane_grab.is_none());
    }

    #[test]
    fn a_tab_cannot_be_dropped_into_itself() {
        let mut app = app_of(&["a", "b"]);
        dock(&mut app, 1, 1, Toward::Right);
        assert_eq!(strip(&app), [vec!["a"], vec!["b"]]);
    }

    /// Past the limit the drop is refused, and both tabs are left whole.
    #[test]
    fn a_drop_that_would_overfill_a_tab_is_refused_and_loses_nothing() {
        let names: Vec<String> = (0..=tab::MAX_PANES).map(|i| i.to_string()).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut app = app_of(&names);
        // Fill the first tab to the limit, then try one more.
        for _ in 1..tab::MAX_PANES {
            dock(&mut app, 1, 0, Toward::Right);
        }
        assert_eq!(app.tabs[0].pane_count(), tab::MAX_PANES);
        assert_eq!(app.tabs.len(), 2);
        dock(&mut app, 1, 0, Toward::Right);
        assert_eq!(app.tabs[0].pane_count(), tab::MAX_PANES);
        assert_eq!(app.tabs.len(), 2, "the tab dropped is still there");
    }

    /// Removing the split gives each session a tab of its own just after the
    /// tab, in pane order, and the tab shown is the one the keyboard was in.
    #[test]
    fn removing_a_split_gives_every_pane_a_tab_after_it() {
        let mut app = app_of(&["a", "b", "c", "z"]);
        dock(&mut app, 1, 0, Toward::Right);
        dock(&mut app, 1, 0, Toward::Down);
        app.tabs[0].focus = Pane(2);
        app.unsplit_tab(0);
        assert_eq!(strip(&app), [vec!["a"], vec!["b"], vec!["c"], vec!["z"]]);
        assert_eq!(app.active, 2);
    }

    #[test]
    fn a_detached_pane_gets_the_next_tab_and_is_shown_there() {
        let mut app = app_of(&["a", "b", "z"]);
        dock(&mut app, 1, 0, Toward::Right);
        app.detach_to_tab(At {
            tab: 0,
            pane: Pane::FIRST,
        });
        assert_eq!(strip(&app), [vec!["b"], vec!["a"], vec!["z"]]);
        assert_eq!(app.active, 1);
    }

    /// The keyboard moves to the pane on screen in the direction asked, and
    /// a swap moves the pane while the keyboard stays with its session.
    #[test]
    fn the_keyboard_walks_the_panes_and_a_swap_carries_the_pane_with_it() {
        let mut app = app_of(&["a", "b"]);
        dock(&mut app, 1, 0, Toward::Right);
        app.tabs[0].focus = Pane::FIRST;
        app.focus_pane(Toward::Right);
        assert_eq!(app.tabs[0].focus, Pane(1));
        app.focus_pane(Toward::Right);
        assert_eq!(app.tabs[0].focus, Pane(1), "nothing further right");
        app.swap_pane(Toward::Left);
        assert_eq!(app.tabs[0].focused().profile.name, "b");
        let tree = &app.tabs[0].split.as_ref().unwrap().tree;
        assert_eq!(tree.panes(), [Pane(1), Pane(0)], "b is now on the left");
    }

    fn pane_named(name: &str) -> SavedPane {
        SavedPane {
            profile: Profile {
                name: name.into(),
                ..Profile::default()
            },
            ..SavedPane::default()
        }
    }

    /// A file written by a build whose tabs held two sessions at most comes
    /// back as the same two, the same way round.
    #[test]
    fn a_split_saved_by_an_older_build_comes_back_as_it_was() {
        let entry = SavedTab {
            split: Some(crate::config::session::SavedSplit {
                dir: SavedDir::Bottom,
                ratio: 0.3,
                profile: pane_named("b").profile,
                ..Default::default()
            }),
            second_focused: true,
            ..SavedTab::default()
        };
        let (panes, tree, focus) = saved_layout(&entry);
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].profile.name, "b");
        assert_eq!(tree.map(|t| t.encode()).as_deref(), Some("B0.3(0,1)"));
        assert_eq!(focus, Pane(1));
    }

    /// A layout that does not fit the panes the file lists is not trusted:
    /// the panes come back in a row instead, and a focus past them on the
    /// first.
    #[test]
    fn a_saved_layout_that_does_not_fit_its_panes_falls_back_to_a_row() {
        let entry = SavedTab {
            panes: vec![pane_named("b"), pane_named("c")],
            layout: Some("R0.5(0,1)".into()),
            focus: 7,
            ..SavedTab::default()
        };
        let (panes, tree, focus) = saved_layout(&entry);
        assert_eq!(panes.len(), 2);
        let tree = tree.expect("a layout");
        assert!(tree.covers(3));
        assert_eq!(tree.panes(), [Pane(0), Pane(1), Pane(2)]);
        assert_eq!(focus, Pane::FIRST);
    }

    #[test]
    fn a_saved_tab_with_more_panes_than_a_tab_may_hold_is_cut_down() {
        let entry = SavedTab {
            panes: (0..20).map(|i| pane_named(&i.to_string())).collect(),
            ..SavedTab::default()
        };
        let (panes, tree, _) = saved_layout(&entry);
        assert_eq!(panes.len() + 1, tab::MAX_PANES);
        assert!(tree.expect("a layout").covers(tab::MAX_PANES));
    }

    /// What is written down for a split tab reads back into the same layout.
    #[test]
    fn a_split_tab_saves_a_layout_it_can_be_restored_from() {
        let mut app = app_of(&["a", "b", "c"]);
        dock(&mut app, 1, 0, Toward::Down);
        dock(&mut app, 1, 0, Toward::Left);
        app.tabs[0].colour = Some(egui::Color32::from_rgb(0x4c, 0xb0, 0x5a));
        let saved = saved_tab(&app.tabs[0]);
        assert_eq!(saved.colour.as_deref(), Some("#4cb05a"));
        assert!(saved.split.is_none(), "the old form is never written");
        let (panes, tree, focus) = saved_layout(&saved);
        assert_eq!(panes.len(), 2);
        assert_eq!(
            tree.as_ref(),
            Some(&app.tabs[0].split.as_ref().unwrap().tree)
        );
        assert_eq!(focus, app.tabs[0].focus);
    }

    /// The tab's colour stops at the pixel the terminal's edge is in, and the
    /// terminal's at the tab's: a band of the tab's colour reaching a pixel
    /// into the terminal was the lip on Windows 98 Dark's focused tab.
    #[test]
    fn the_seam_bands_never_cross_into_the_other_side() {
        use crate::config::BarPosition;
        use egui::{pos2, Rect};
        for ppp in [1.0_f32, 1.25, 1.5, 2.0] {
            for bar in BarPosition::ALL {
                // A seam half a pixel off the lattice at 125%.
                let e = 100.4;
                let (terminal, tab) = match bar {
                    BarPosition::Top => (
                        Rect::from_min_max(pos2(0.0, e), pos2(400.0, 300.0)),
                        Rect::from_min_max(pos2(10.0, 80.0), pos2(90.0, e)),
                    ),
                    BarPosition::Bottom => (
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, e)),
                        Rect::from_min_max(pos2(10.0, e), pos2(90.0, 120.0)),
                    ),
                    BarPosition::Left => (
                        Rect::from_min_max(pos2(e, 0.0), pos2(400.0, 300.0)),
                        Rect::from_min_max(pos2(60.0, 10.0), pos2(e, 40.0)),
                    ),
                    BarPosition::Right => (
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(e, 300.0)),
                        Rect::from_min_max(pos2(e, 10.0), pos2(140.0, 40.0)),
                    ),
                };
                let near = |a: f32, b: f32| (a - b).abs() <= 2.0;
                let (own, theirs) = seam_bands(tab, terminal, bar, ppp, near).unwrap();
                let lattice = |v: f32| ((v * ppp).round() - v * ppp).abs() < 1e-3;
                // The tab's band starts on a pixel boundary and outside the
                // terminal's pixels; the terminal's covers its edge's pixel.
                let (own_lo, own_hi, their_lo, their_hi) = match bar {
                    BarPosition::Top | BarPosition::Bottom => {
                        (own.top(), own.bottom(), theirs.top(), theirs.bottom())
                    }
                    _ => (own.left(), own.right(), theirs.left(), theirs.right()),
                };
                assert!(own_lo <= e && e <= own_hi, "{bar:?} at {ppp}");
                let meets = match bar {
                    BarPosition::Top | BarPosition::Left => their_hi == own_lo,
                    _ => their_lo == own_hi,
                };
                assert!(meets, "{bar:?} at {ppp}: {own:?} {theirs:?}");
                assert!(lattice(own_lo) && lattice(own_hi), "{bar:?} at {ppp}");
            }
        }
    }

    #[test]
    fn the_active_tab_keeps_its_place_when_a_game_before_it_is_left_out() {
        assert_eq!(kept_index(2, &[true, false, true]), 1);
        assert_eq!(kept_index(0, &[true, false, true]), 0);
    }

    #[test]
    fn an_active_game_that_is_left_out_hands_the_front_to_the_tab_before_it() {
        assert_eq!(kept_index(1, &[true, false, true]), 0);
        assert_eq!(kept_index(0, &[false, true]), 0);
        assert_eq!(kept_index(0, &[false]), 0);
    }

    /// Three tabs of different widths, side by side: 0..50, 50..250, 250..300.
    const SPANS: [(f32, f32); 3] = [(0.0, 50.0), (50.0, 250.0), (250.0, 300.0)];

    #[test]
    fn a_tab_stays_put_until_the_pointer_passes_a_neighbours_middle() {
        assert_eq!(drop_slot(&SPANS, 0, 140.0), None, "short of 150");
        assert_eq!(drop_slot(&SPANS, 0, 160.0), Some(1));
        assert_eq!(
            drop_slot(&SPANS, 2, 160.0),
            None,
            "short of 150, from the right"
        );
        assert_eq!(drop_slot(&SPANS, 2, 140.0), Some(1));
    }

    /// Swapped past the middle, the wide neighbour lands on the far side of
    /// the pointer - so the next frame, with the pointer where it was, leaves
    /// the tabs alone rather than swapping them back.
    #[test]
    fn a_swap_with_a_wider_neighbour_does_not_swap_straight_back() {
        let x = 160.0;
        assert_eq!(drop_slot(&SPANS, 0, x), Some(1));
        // After the swap: the wide tab first, the dragged one after it.
        let swapped = [(0.0, 200.0), (200.0, 250.0), (250.0, 300.0)];
        assert_eq!(drop_slot(&swapped, 1, x), None);
    }

    #[test]
    fn a_fast_drag_moves_the_tab_past_every_middle_it_crossed() {
        assert_eq!(drop_slot(&SPANS, 0, 290.0), Some(2));
        assert_eq!(drop_slot(&SPANS, 2, 10.0), Some(0));
    }

    #[test]
    fn the_pointer_over_the_dragged_tab_itself_moves_nothing() {
        assert_eq!(drop_slot(&SPANS, 1, 60.0), None);
        assert_eq!(drop_slot(&SPANS, 1, 240.0), None);
    }

    /// Every index is carried along: the moved tab goes to its new place, the
    /// ones it passed shift by one towards where it came from, and the rest
    /// stay where they were.
    #[test]
    fn moving_a_tab_shifts_only_the_tabs_it_passed() {
        // 0 1 2 3 4, with 1 moved to 3: 0 2 3 1 4.
        let after: Vec<usize> = (0..5).map(|i| moved_index(i, 1, 3)).collect();
        assert_eq!(after, vec![0, 3, 1, 2, 4]);
        // And back the other way, 3 to 1: 0 3 1 2 4.
        let after: Vec<usize> = (0..5).map(|i| moved_index(i, 3, 1)).collect();
        assert_eq!(after, vec![0, 2, 3, 1, 4]);
    }

    /// The same remapping, checked against what `Vec::remove` and `insert`
    /// actually do to the tabs, for every pair of places.
    #[test]
    fn the_remapping_agrees_with_moving_the_tabs_themselves() {
        for from in 0..5 {
            for to in 0..5 {
                let mut tabs: Vec<usize> = (0..5).collect();
                let tab = tabs.remove(from);
                tabs.insert(to, tab);
                for original in 0..5 {
                    let now = moved_index(original, from, to);
                    assert_eq!(tabs[now], original, "{original}, moving {from} to {to}");
                }
            }
        }
    }

    #[test]
    fn a_tab_in_the_title_bar_runs_from_the_top_of_the_bar_to_its_bottom() {
        let ctx = egui::Context::default();
        let mut drawn = None;
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 400.0));
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::TopBottomPanel::top("bar")
                    .frame(egui::Frame::none())
                    .show_separator_line(false)
                    .show(ctx, |ui| {
                        let height = tab_height(ui);
                        let size = egui::vec2(ui.available_width(), height);
                        let layout = egui::Layout::left_to_right(egui::Align::Center);
                        ui.allocate_ui_with_layout(size, layout, |ui| {
                            egui::ScrollArea::horizontal()
                                .auto_shrink([true, false])
                                .show(ui, |ui| {
                                    tab_row(ui, |ui| {
                                        let (tab, _) = tab_pill(
                                            ui,
                                            1,
                                            true,
                                            "tab".into(),
                                            (None, None),
                                            None,
                                            [Some(200.0), None],
                                            crate::config::TabCloseSide::Left,
                                        );
                                        drawn = Some((tab.rect, height));
                                    });
                                });
                        });
                    });
            },
        );
        let (tab, height) = drawn.unwrap();
        assert_eq!(tab.top(), 0.0);
        assert_eq!(tab.height(), height);
    }
}
