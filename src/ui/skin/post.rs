//! The stock egui widgets a skin cannot reach any other way.
//!
//! The app's own furniture asks the skin for its shapes. egui's stock widgets
//! cannot be asked - a `Button` paints its own fill, a `TextEdit` its own
//! frame, a menu its own box, and the dialogs, the colour picker and every
//! combo box's list are made of them. So under a skin their colours in
//! `Visuals` are set to *markers* (`marker`), and at the end of every frame
//! the shapes painted in a marker are found and swapped for what the skin
//! draws there: a marker button becomes a bevelled one, a marker field a
//! sunken one, a marker window a framed one.
//!
//! A marker is the skin's own colour at an alpha just short of opaque - 254
//! for one role, 253 for the next - which nothing else paints: a 98 button's
//! marker is 98 grey, so anything this misses still comes out the right
//! colour, only flat. The role is in the alpha because egui fades a disabled
//! widget's colours towards the page by a rule that halves the distance in
//! red, green and blue but leaves an alpha that high alone: the colour says
//! whether it was faded, the alpha which role it is, and no faded marker can
//! pass for another role's.

use std::collections::HashMap;
use std::sync::Arc;

use egui::epaint::ClippedShape;
use egui::layers::ShapeIdx;
use egui::{Color32, Context, Id, LayerId, Order, Rect, Shape};

use super::{Kit, Look, St, Style};

/// What a marker stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Role {
    /// A button's face - `weak_bg_fill` - in one of egui's four states.
    Button(State),
    /// A box - `bg_fill` - which is a checkbox's square, a slider's rail, and
    /// a scroll bar's handle.
    Box(State),
    /// `extreme_bg_color`: a text field, and a scroll bar's track.
    Field,
    /// `window_fill`: a window, a menu, a pop-up, a tooltip.
    Window,
    /// `selection.bg_fill`: a selected label, a selected row of a list.
    Selection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum State {
    Inactive,
    Hovered,
    Active,
    Open,
}

impl Role {
    fn code(self) -> u8 {
        match self {
            Role::Button(State::Inactive) => 1,
            Role::Button(State::Hovered) => 2,
            Role::Button(State::Active) => 3,
            Role::Button(State::Open) => 4,
            Role::Box(State::Inactive) => 5,
            Role::Box(State::Hovered) => 6,
            Role::Box(State::Active) => 7,
            Role::Box(State::Open) => 8,
            Role::Field => 9,
            Role::Window => 10,
            Role::Selection => 11,
        }
    }

    const ALL: [Role; 11] = [
        Role::Button(State::Inactive),
        Role::Button(State::Hovered),
        Role::Button(State::Active),
        Role::Button(State::Open),
        Role::Box(State::Inactive),
        Role::Box(State::Hovered),
        Role::Box(State::Active),
        Role::Box(State::Open),
        Role::Field,
        Role::Window,
        Role::Selection,
    ];
}

/// The marker for `role` over a skin whose colour there is `near`: that
/// colour at the role's own alpha.
pub(crate) fn marker(role: Role, near: Color32) -> Color32 {
    let [r, g, b, _] = near.to_array();
    let a = 255 - role.code();
    Color32::from_rgba_premultiplied(r.min(a), g.min(a), b.min(a), a)
}

/// Every marker this frame can contain, faded or not, and what it stands
/// for.
fn table(ctx: &Context) -> HashMap<Color32, (Role, bool)> {
    let visuals = ctx.style().visuals.clone();
    let fade = visuals.fade_out_to_color();
    let markers: Vec<(Role, Color32)> = Role::ALL
        .into_iter()
        .map(|role| {
            let colour = match role {
                Role::Button(s) => widget(&visuals, s).weak_bg_fill,
                Role::Box(s) => widget(&visuals, s).bg_fill,
                Role::Field => visuals.extreme_bg_color,
                Role::Window => visuals.window_fill,
                Role::Selection => visuals.selection.bg_fill,
            };
            (role, colour)
        })
        .filter(|(role, colour)| colour.a() == 255 - role.code())
        .collect();
    let mut out = HashMap::new();
    // The faded markers first and the markers themselves over them: a role
    // whose colour is the page's own fades to itself, and is then not
    // disabled after all.
    for &(role, colour) in &markers {
        out.insert(egui::ecolor::tint_color_towards(colour, fade), (role, true));
    }
    for &(role, colour) in &markers {
        out.insert(colour, (role, false));
    }
    out
}

fn widget(visuals: &egui::Visuals, state: State) -> &egui::style::WidgetVisuals {
    match state {
        State::Inactive => &visuals.widgets.inactive,
        State::Hovered => &visuals.widgets.hovered,
        State::Active => &visuals.widgets.active,
        State::Open => &visuals.widgets.open,
    }
}

fn installed_id() -> Id {
    Id::new("nit-skin-post-installed")
}

/// Registers the swap to run at the end of every frame of every window.
/// Once per context; calling it again does nothing.
pub fn install(ctx: &Context) {
    if ctx
        .data(|d| d.get_temp::<bool>(installed_id()))
        .unwrap_or(false)
    {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(installed_id(), true));
    ctx.on_end_frame("skin", Arc::new(rewrite));
}

/// Swaps every marker shape of the frame for the skin's.
fn rewrite(ctx: &Context) {
    let Some(kit) = Kit::of(ctx) else {
        return;
    };
    let table = table(ctx);
    if table.is_empty() {
        return;
    }
    // What a focused text field is outlined in.
    let focus = ctx.style().visuals.selection.stroke.color;
    let mut layers: Vec<LayerId> = ctx.memory(|m| m.layer_ids().collect());
    if !layers.contains(&LayerId::background()) {
        layers.push(LayerId::background());
    }
    // Read, worked out and written back in three steps rather than inside
    // one `graphics_mut`: a painter may load a bitmap of its skin, which takes
    // the context's lock that `graphics_mut` is holding.
    for layer in layers {
        let entries: Vec<ClippedShape> = ctx.graphics(|graphics| {
            graphics
                .get(layer)
                .map(|list| list.all_entries().cloned().collect())
                .unwrap_or_default()
        });
        let swapped = swaps(&kit, &table, focus, layer, &entries);
        if swapped.is_empty() {
            continue;
        }
        ctx.graphics_mut(|graphics| {
            if let Some(list) = graphics.get_mut(layer) {
                for (index, clip, shape) in swapped {
                    list.set(ShapeIdx(index), clip, shape);
                }
            }
        });
    }
}

/// What each marker shape in `entries` becomes, by index.
fn swaps(
    kit: &Kit,
    table: &HashMap<Color32, (Role, bool)>,
    focus: Color32,
    layer: LayerId,
    entries: &[ClippedShape],
) -> Vec<(usize, Rect, Shape)> {
    let style = kit.style();
    let mut out = Vec::new();
    let mut skip = None;
    // The row of a menu last lit, whose words are then the selection's ink:
    // white on 98's navy, where egui would keep them black.
    let mut lit: Option<Rect> = None;
    let on_light = kit.colors().select_text;
    // The controls swapped so far and the ink their words are in: a field's
    // and a button's own, not the page's - a user's light text for a dark
    // page cannot be read inside a skin's white field.
    let mut inks: Vec<(Rect, Color32)> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        if skip == Some(index) {
            continue;
        }
        if let Shape::Text(text) = &entry.shape {
            let at = text.visual_bounding_rect().center();
            let lit_row = lit.is_some_and(|row| row.contains(at));
            if !lit_row {
                if let Some(&(_, ink)) = inks.iter().rev().find(|(r, _)| r.contains(at)) {
                    let mut text = text.clone();
                    text.override_text_color = Some(ink);
                    out.push((index, entry.clip_rect, Shape::Text(text)));
                    continue;
                }
            }
        }
        if let (Some(row), Shape::Text(text)) = (lit, &entry.shape) {
            let words = text.visual_bounding_rect();
            if row.contains(words.center()) {
                let mut text = text.clone();
                text.override_text_color = Some(on_light);
                out.push((index, entry.clip_rect, Shape::Text(text)));
            }
            continue;
        }
        // A tooltip's words are the tooltip's ink, not the page's: a dark
        // reading writes light text, which on Aqua's or XP's pale yellow
        // help tag cannot be read.
        if let (Order::Tooltip, Shape::Text(text)) = (layer.order, &entry.shape) {
            let mut text = text.clone();
            text.override_text_color = Some(kit.colors().tooltip_text);
            out.push((index, entry.clip_rect, Shape::Text(text)));
            continue;
        }
        let Shape::Rect(rect) = &entry.shape else {
            continue;
        };
        let Some(&(role, disabled)) = table.get(&rect.fill) else {
            continue;
        };
        let r = rect.rect;
        let st = |s: State| St {
            hovered: matches!(s, State::Hovered),
            pressed: matches!(s, State::Active),
            on: matches!(s, State::Open),
            disabled,
            ..St::default()
        };
        let shapes = match role {
            Role::Button(s)
                if layer.order == Order::Foreground || layer.order == Order::Tooltip =>
            {
                // Inside a menu or a pop-up list a "button" is a row: its
                // highlight, not a bevelled slab.
                let row = style.menu_item(kit, r, st(s));
                if !row.is_empty() {
                    lit = Some(r);
                }
                row
            }
            Role::Button(s) => {
                inks.push((r, style.button_text(kit, st(s))));
                style.button(kit, r, st(s))
            }
            Role::Box(s) => {
                if r.width() <= 24.0 && (r.width() - r.height()).abs() < 1.0 {
                    style.checkbox(kit, r, st(s))
                } else {
                    let vertical = r.height() > r.width();
                    style.scroll_thumb(kit, r, vertical, st(s))
                }
            }
            Role::Field => {
                // A scroll bar is its track and then its handle, one after the
                // other; a field is a field.
                // egui grows a handle shorter than its least length about its
                // middle, which near either end puts part of it outside the
                // track: so overlapping is enough, and it is held inside.
                let handle = entries.get(index + 1).and_then(|next| match &next.shape {
                    Shape::Rect(h)
                        if matches!(table.get(&h.fill), Some((Role::Box(_), _)))
                            && r.intersects(h.rect) =>
                    {
                        Some((h.rect.intersect(r), table[&h.fill]))
                    }
                    _ => None,
                });
                match handle {
                    Some((handle, (Role::Box(s), disabled))) => {
                        skip = Some(index + 1);
                        out.push((index + 1, entries[index + 1].clip_rect, Shape::Noop));
                        scroll_bar(style, kit, r, handle, St { disabled, ..st(s) })
                    }
                    _ => {
                        inks.push((r, kit.colors().field_text));
                        let focused = rect.stroke.width > 0.0 && rect.stroke.color == focus;
                        style.field(
                            kit,
                            r,
                            St {
                                focused,
                                ..st(State::Inactive)
                            },
                        )
                    }
                }
            }
            Role::Window => match layer.order {
                Order::Tooltip => style.tooltip(kit, r),
                Order::Foreground => style.menu(kit, r),
                _ => style.window(kit, r, St::default()),
            },
            Role::Selection => {
                lit = Some(r);
                style.list_item(
                    kit,
                    r,
                    St {
                        on: true,
                        ..St::default()
                    },
                )
            }
        };
        out.push((index, entry.clip_rect, Shape::Vec(shapes)));
    }
    out
}

/// A scroll bar of egui's own: its track with the skin's arrows at the ends,
/// and the handle moved in from them.
///
/// egui runs the handle along the whole track and knows nothing of arrows.
/// Drawing them over the ends would hide the handle at either extreme, so
/// the handle is drawn where it would be on the track between the arrows -
/// the same fraction of the way along, the same fraction of its length.
fn scroll_bar(style: &dyn Style, kit: &Kit, track: Rect, handle: Rect, st: St) -> Vec<Shape> {
    let vertical = track.height() >= track.width();
    let arrow = style.metrics().scroll_arrow * kit.u();
    let mut out = Vec::new();
    let (lead, inner, trail, thumb) = if vertical && arrow > 0.0 && track.height() > 2.0 * arrow {
        let lead = Rect::from_min_size(track.min, egui::vec2(track.width(), arrow));
        let trail = Rect::from_min_max(egui::pos2(track.left(), track.bottom() - arrow), track.max);
        let inner = Rect::from_min_max(
            egui::pos2(track.left(), lead.bottom()),
            egui::pos2(track.right(), trail.top()),
        );
        let k = inner.height() / track.height();
        let top = inner.top() + (handle.top() - track.top()) * k;
        let thumb = Rect::from_min_max(
            egui::pos2(track.left(), top),
            egui::pos2(track.right(), top + handle.height() * k),
        );
        (Some(lead), inner, Some(trail), thumb)
    } else if !vertical && arrow > 0.0 && track.width() > 2.0 * arrow {
        let lead = Rect::from_min_size(track.min, egui::vec2(arrow, track.height()));
        let trail = Rect::from_min_max(egui::pos2(track.right() - arrow, track.top()), track.max);
        let inner = Rect::from_min_max(
            egui::pos2(lead.right(), track.top()),
            egui::pos2(trail.left(), track.bottom()),
        );
        let k = inner.width() / track.width();
        let left = inner.left() + (handle.left() - track.left()) * k;
        let thumb = Rect::from_min_max(
            egui::pos2(left, track.top()),
            egui::pos2(left + handle.width() * k, track.bottom()),
        );
        (Some(lead), inner, Some(trail), thumb)
    } else {
        (
            None,
            track,
            None,
            Rect::from_min_max(
                egui::pos2(track.left(), handle.top()),
                egui::pos2(track.right(), handle.bottom()),
            ),
        )
    };
    out.extend(style.scroll_track(kit, inner, vertical, St::default()));
    let (back, forward) = if vertical {
        (super::Dir::Up, super::Dir::Down)
    } else {
        (super::Dir::Left, super::Dir::Right)
    };
    if let Some(lead) = lead {
        out.extend(style.scroll_arrow(kit, lead, back, St::default()));
    }
    if let Some(trail) = trail {
        out.extend(style.scroll_arrow(kit, trail, forward, St::default()));
    }
    out.extend(style.scroll_thumb(kit, thumb, vertical, st));
    out
}

/// Sets the markers into `visuals`, for the skin `look` is wearing, with its
/// colours everywhere a marker is not needed.
pub(crate) fn mark(visuals: &mut egui::Visuals, look: Look) {
    let c = Kit::bare(look, 1.0).colors();
    let widgets = &mut visuals.widgets;
    for (state, w) in [
        (State::Inactive, &mut widgets.inactive),
        (State::Hovered, &mut widgets.hovered),
        (State::Active, &mut widgets.active),
        (State::Open, &mut widgets.open),
    ] {
        w.weak_bg_fill = marker(Role::Button(state), c.face);
        w.bg_fill = marker(Role::Box(state), c.face);
    }
    visuals.extreme_bg_color = marker(Role::Field, c.field);
    visuals.window_fill = marker(Role::Window, c.face);
    visuals.selection.bg_fill = marker(Role::Selection, c.select);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_role_has_an_alpha_of_its_own_that_fading_keeps() {
        let near = Color32::from_rgb(0xc0, 0xc0, 0xc0);
        let fade = Color32::from_rgb(0xff, 0xff, 0xff);
        let mut alphas = std::collections::HashSet::new();
        for role in Role::ALL {
            let m = marker(role, near);
            assert!(alphas.insert(m.a()), "{role:?} shares an alpha");
            assert_eq!(egui::ecolor::tint_color_towards(m, fade).a(), m.a());
        }
    }

    /// A track followed by the handle inside it is one scroll bar: the track
    /// becomes the skin's whole bar, arrows included, and the handle is
    /// folded into it rather than drawn again on top.
    #[test]
    fn an_egui_scroll_bar_gets_the_skins_arrows_and_one_thumb() {
        let look = Look {
            skin: super::super::Skin::Win98,
            dark: false,
            ..Default::default()
        };
        let kit = Kit::bare(look, 1.0);
        let c = look.skin.style().colors(false);
        let track = marker(Role::Field, c.field);
        let handle = marker(Role::Box(State::Inactive), c.face);
        let mut table = HashMap::new();
        table.insert(track, (Role::Field, false));
        table.insert(handle, (Role::Box(State::Inactive), false));
        let clip = Rect::EVERYTHING;
        let entries = vec![
            ClippedShape {
                clip_rect: clip,
                shape: Shape::rect_filled(
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(16.0, 100.0)),
                    0.0,
                    track,
                ),
            },
            ClippedShape {
                clip_rect: clip,
                shape: Shape::rect_filled(
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(16.0, 30.0)),
                    0.0,
                    handle,
                ),
            },
        ];
        let swapped = swaps(
            &kit,
            &table,
            Color32::WHITE,
            LayerId::background(),
            &entries,
        );
        let indices: Vec<usize> = swapped.iter().map(|(i, _, _)| *i).collect();
        assert!(indices.contains(&0) && indices.contains(&1), "{indices:?}");
        let handle_swap = swapped.iter().find(|(i, _, _)| *i == 1).expect("handle");
        assert!(
            matches!(handle_swap.2, Shape::Noop),
            "the handle was drawn twice"
        );
    }

    /// egui fades a disabled widget by a rule that keeps an alpha of 254 at
    /// 254, which is what lets a faded marker still be told apart.
    #[test]
    fn a_faded_marker_keeps_its_alpha() {
        let m = marker(Role::Button(State::Inactive), Color32::from_gray(0xc0));
        let faded = egui::ecolor::tint_color_towards(m, Color32::from_gray(0xc0));
        assert_eq!(faded.a(), 254);
    }
}
