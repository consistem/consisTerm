//! The furniture of a preferences window in the style of macOS System Settings
//! and GNOME Settings: a sidebar of categories with a search field on top, and
//! pages of rounded cards holding rows - a title, perhaps a grey line under it,
//! and the control pushed to the right edge.
//!
//! Drawn here rather than borrowed from egui, which has a checkbox but no
//! switch, a combo box but no segmented control, and no notion of a row whose
//! label and control sit at opposite ends. The point of the layout is that a
//! long list of settings reads as a column of short sentences, each with its
//! answer beside it, grouped into a few cards a page.
//!
//! Every colour is derived from the current `Visuals`, so a page looks like
//! the theme it is drawn in; the accent is the theme's selection colour. The
//! one fixed set is the category tiles in the sidebar, which carry meaning by
//! colour the way System Settings' do and are legible on every background
//! because the mark on them is white.
//!
//! The pieces, from the outside in:
//!
//! - [`sidebar`]: the search field and the category list.
//! - [`page_header`], [`page_header_with_back`], [`column()`],
//!   [`scroll_column`]: a page's title (with the way back, on a sub-page) and
//!   its centred column.
//! - [`section`], [`card`], [`footer`]: a bold heading, the boxed list under
//!   it, and a grey note after it.
//! - [`Card::row`], [`Card::toggle`], [`Card::nav`], [`Card::buttons`],
//!   [`Card::custom`]: what goes in a card. [`row`] is the same row outside one.
//! - [`toggle`], [`segmented`], [`popup`], [`slider`], [`button`]: the
//!   controls a row carries on its right.
//! - [`fold`], [`matches()`]: the search's notion of "the same text".

use crate::ui::tip::Tip;
use std::hash::Hash;

use egui::{
    pos2, vec2, Align, Color32, FontId, Id, Layout, Rect, Response, Rounding, Sense, Shape, Stroke,
    TextStyle, Ui, Visuals, WidgetText,
};

use crate::ui::icons::{self, Symbol};
use crate::ui::shading::{darken, lighten};
use crate::ui::skin::{Kit, St};

/// Between a row's edge and its text, and between its control and its edge.
const PAD_X: f32 = 12.0;
/// Above and below a row's text.
const PAD_Y: f32 = 7.0;
/// A row is never shorter than this, so a card of one-line rows reads as an
/// even list rather than as text of whatever height it happened to wrap to.
const MIN_ROW: f32 = 38.0;
/// Between a row's text and its control.
const GAP: f32 = 16.0;
/// A card's corners.
const CARD_RADIUS: f32 = 10.0;
/// A sidebar entry's height, and the corners of its selection pill.
const SIDEBAR_ROW: f32 = 30.0;
const PILL_RADIUS: f32 = 7.0;
/// The coloured tile in front of a sidebar entry.
const TILE: f32 = 20.0;

/// Draws what follows in `ui` `factor` times larger: the text, and the room
/// the widgets take, so a row of tabs and buttons grows as a whole rather than
/// its labels outgrowing the controls they are on. A no-op at 1.
pub fn scale_style(ui: &mut egui::Ui, factor: f32) {
    if !factor.is_finite() || (factor - 1.0).abs() < f32::EPSILON {
        return;
    }
    let style = ui.style_mut();
    for font in style.text_styles.values_mut() {
        font.size *= factor;
    }
    let spacing = &mut style.spacing;
    spacing.interact_size *= factor;
    spacing.button_padding *= factor;
    spacing.item_spacing *= factor;
    spacing.icon_width *= factor;
    spacing.icon_width_inner *= factor;
}

/// How opaque the cards and the sidebar are over a theme's backdrop when
/// nothing has said otherwise.
pub const DEFAULT_OPACITY: f32 = 0.6;

fn opacity_id() -> Id {
    Id::new("nit-sheet-opacity")
}

/// Records how opaque the preferences furniture is over a theme's backdrop,
/// for everything drawn after it. Kept in the context, like the backdrop
/// itself, because the windows that draw it are handed nothing of the settings.
pub fn set_opacity(ctx: &egui::Context, opacity: f32) {
    ctx.data_mut(|d| d.insert_temp(opacity_id(), opacity.clamp(0.0, 1.0)));
}

/// The opacity [`set_opacity`] last recorded.
pub fn opacity(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(opacity_id()))
        .unwrap_or(DEFAULT_OPACITY)
}

/// The colours the furniture is drawn in, all taken from the theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// Behind the category list.
    pub sidebar: Color32,
    /// A card, a step off the page so it reads as a box without a heavy line.
    pub card: Color32,
    /// A card's outline: faint, since the fill already says where it is.
    pub card_stroke: Color32,
    /// The hairline between two rows of a card.
    pub separator: Color32,
    /// The search field, and the track of a segmented control.
    pub field: Color32,
    /// A switch that is off.
    pub track_off: Color32,
    /// Under the pointer.
    pub hover: Color32,
    pub text: Color32,
    /// A row's second line, and a card's footer.
    pub subtle: Color32,
    /// Section headings and page titles.
    pub heading: Color32,
    /// The selection pill, a switch that is on, a selected segment.
    pub accent: Color32,
    /// Text and marks drawn on the accent.
    pub on_accent: Color32,
    /// The accent as text on the page - a link, the way back: the theme's
    /// link colour, which `Theme::visuals` works out against what is really
    /// behind the page, a gradient included.
    pub accent_text: Color32,
}

impl Palette {
    /// The palette for `ui`, as opaque over a backdrop as Settings says.
    pub fn for_ui(ui: &Ui) -> Self {
        if let Some(kit) = Kit::of(ui.ctx()) {
            return Self::skinned(&kit);
        }
        Self::with_opacity(ui.visuals(), opacity(ui.ctx()))
    }

    /// Under a skin: no veils, no hairlines and no washes - the skin draws
    /// boxes and fields of its own - and the skin's own ink everywhere.
    fn skinned(kit: &Kit) -> Self {
        let c = kit.colors();
        Palette {
            sidebar: Color32::TRANSPARENT,
            card: Color32::TRANSPARENT,
            card_stroke: Color32::TRANSPARENT,
            separator: Color32::TRANSPARENT,
            field: c.field,
            track_off: c.subtle,
            hover: Color32::TRANSPARENT,
            text: c.text,
            subtle: c.subtle,
            heading: c.text,
            accent: c.select,
            on_accent: c.select_text,
            accent_text: c.link,
        }
    }

    pub fn from_visuals(visuals: &Visuals) -> Self {
        Self::with_opacity(visuals, DEFAULT_OPACITY)
    }

    fn with_opacity(visuals: &Visuals, opacity: f32) -> Self {
        let text = visuals.text_color();
        let base = visuals.window_fill.to_opaque();
        let dark = luminance(base) < 0.5;
        let accent = visuals.selection.bg_fill.to_opaque();
        // Chosen against the accent itself rather than taken from the theme:
        // the selection colour is picked to sit behind terminal text, and the
        // theme says nothing about what reads on top of it at pill size.
        let on_accent = if luminance(accent) < 0.62 {
            Color32::WHITE
        } else {
            Color32::from_rgb(0x1d, 0x1d, 0x1f)
        };
        // Translucent text colour rather than mixed shades, so the same
        // hairlines and fields work on a card, on the sidebar and on a
        // gradient alike without being worked out for each.
        let ink = |alpha: f32| text.gamma_multiply(alpha);
        let accent_text = visuals.hyperlink_color;
        let shared = Palette {
            sidebar: Color32::TRANSPARENT,
            card: Color32::TRANSPARENT,
            card_stroke: ink(0.10),
            separator: ink(0.12),
            field: ink(0.09),
            track_off: ink(0.24),
            hover: ink(0.07),
            text,
            subtle: ink(0.62),
            heading: visuals.strong_text_color(),
            accent,
            on_accent,
            accent_text,
        };
        // A theme with a painted backdrop leaves the panels transparent (see
        // `Theme::visuals`). Opaque cards there would cover the backdrop with
        // flat grey boxes, and a window that is nothing but cards would lose
        // the theme entirely. So under a backdrop the cards and the sidebar
        // are veils of the theme's own background - frosted glass in the
        // theme's colour rather than a milky white film over it.
        if visuals.panel_fill.a() == 0 {
            let opacity = opacity.clamp(0.0, 1.0);
            let lifted = if dark {
                lighten(base, 0.07)
            } else {
                lighten(base, 0.45)
            };
            return Palette {
                card: lifted.gamma_multiply(opacity),
                sidebar: darken(base, if dark { 0.8 } else { 0.94 }).gamma_multiply(opacity * 0.8),
                ..shared
            };
        }
        // On a light theme the cards are near white on a grey page, as on a
        // Mac; on a dark one lighter is still "in front", only by much less, or
        // the cards turn grey and the text on them loses its contrast.
        Palette {
            card: if dark {
                lighten(base, 0.06)
            } else {
                lighten(base, 0.7)
            },
            sidebar: if dark {
                darken(base, 0.82)
            } else {
                darken(base, 0.95)
            },
            ..shared
        }
    }
}

/// Perceived brightness, 0 to 1.
fn luminance(color: Color32) -> f32 {
    (0.299 * f32::from(color.r()) + 0.587 * f32::from(color.g()) + 0.114 * f32::from(color.b()))
        / 255.0
}

/// The font a row's second line and a footer are set in: a step below the
/// body, not egui's `Small`, which is small enough to be a caption.
fn subtitle_font(ui: &Ui) -> FontId {
    let body = TextStyle::Body.resolve(ui.style());
    FontId::new((body.size * 0.88).round(), body.family)
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

/// `text` as the search compares it: lower case, and with the accents off.
///
/// Accent-blind because the interface is also in Portuguese, and nobody types
/// "configuração" with its cedilla into a search box - nor should "senha"
/// fail to find "Senha" because one was capitalized.
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

/// Whether every word of `query` is found somewhere in `haystacks`.
///
/// Word by word, and each word anywhere, so "tray close" finds "Close to the
/// tray" and a word can be matched by a keyword while the next is matched by
/// the title. An empty query matches everything.
pub fn matches(query: &str, haystacks: &[&str]) -> bool {
    let folded: Vec<String> = haystacks.iter().map(|h| fold(h)).collect();
    fold(query)
        .split_whitespace()
        .all(|word| folded.iter().any(|h| h.contains(word)))
}

// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------

/// One entry of the category list.
#[derive(Clone, Copy, Debug)]
pub struct SidebarItem<'a> {
    pub label: &'a str,
    pub symbol: Symbol,
    /// The tile behind the symbol.
    pub tile: Color32,
}

/// What the sidebar was asked for.
pub struct SidebarOutput {
    /// The entry clicked this frame.
    pub clicked: Option<usize>,
    /// The search field, for a caller that wants to give it the keyboard.
    pub search: Response,
}

/// The search field and the category list, filling `ui`.
///
/// `selected` is `None` while a search is showing its results: no one page is
/// the one on screen then, and a pill on the last one would say otherwise.
/// `search_id` is the field's id, so the caller can put the keyboard in it -
/// Ctrl+F does.
pub fn sidebar(
    ui: &mut Ui,
    search_id: Id,
    query: &mut String,
    hint: &str,
    items: &[SidebarItem<'_>],
    selected: Option<usize>,
) -> SidebarOutput {
    let palette = Palette::for_ui(ui);
    let outer = ui.max_rect();
    let kit = Kit::of(ui.ctx());
    if kit.is_none() {
        ui.painter()
            .rect_filled(outer, Rounding::same(CARD_RADIUS), palette.sidebar);
    }

    let inner = outer.shrink2(vec2(8.0, 10.0));
    let mut ui = ui.child_ui_with_id_source(inner, Layout::top_down(Align::Min), "sidebar", None);
    let search = search_field(&mut ui, search_id, query, hint, &palette);
    ui.add_space(10.0);

    // Under a skin the categories are a list box: a sunken field with the
    // entries inside its border.
    if let Some(kit) = &kit {
        let list = Rect::from_min_max(ui.cursor().min, ui.max_rect().max);
        ui.painter().extend(kit.style().list(kit, list));
        let border = 2.0 * kit.u() + 1.0;
        ui = ui.child_ui_with_id_source(
            list.shrink(border),
            Layout::top_down(Align::Min),
            "sidebar-list",
            None,
        );
    }

    let mut clicked = None;
    egui::ScrollArea::vertical()
        .id_source(search_id.with("list"))
        .auto_shrink([false, false])
        .show(&mut ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            // Evenly spaced, no clusters: with eleven entries, gaps that set
            // off groups of one read as the list being laid out wrong.
            for (index, item) in items.iter().enumerate() {
                if sidebar_entry(ui, item, selected == Some(index), &palette).clicked() {
                    clicked = Some(index);
                }
            }
        });
    SidebarOutput { clicked, search }
}

/// The rounded search box: a magnifier, the text, and a clear button once
/// there is something to clear.
fn search_field(
    ui: &mut Ui,
    id: Id,
    query: &mut String,
    hint: &str,
    palette: &Palette,
) -> Response {
    let kit = Kit::of(ui.ctx());
    let height = match &kit {
        Some(kit) => kit.px(kit.metrics().field_height),
        None => ui.spacing().interact_size.y + 8.0,
    };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let focused = ui.memory(|m| m.has_focus(id));
    if let Some(kit) = &kit {
        let st = St {
            focused,
            ..St::default()
        };
        ui.painter().extend(kit.style().field(kit, rect, st));
    } else {
        ui.painter()
            .rect_filled(rect, Rounding::same(height / 2.0), palette.field);
        if focused {
            ui.painter().rect_stroke(
                rect.shrink(0.5),
                Rounding::same(height / 2.0),
                Stroke::new(1.5_f32, palette.accent),
            );
        }
    }
    let glyph = Rect::from_center_size(
        pos2(rect.left() + height / 2.0 + 2.0, rect.center().y),
        vec2(14.0, 14.0),
    );
    icons::symbol(ui.painter(), glyph, Symbol::Magnifier, palette.subtle);

    let clear_room = if query.is_empty() { 0.0 } else { height };
    let text_rect = Rect::from_min_max(
        pos2(glyph.right() + 6.0, rect.top()),
        pos2(rect.right() - 8.0 - clear_room, rect.bottom()),
    );
    let mut child = ui.child_ui_with_id_source(
        text_rect,
        Layout::left_to_right(Align::Center),
        id.with("text"),
        None,
    );
    // Typed into the skin's field, so in the field's ink, not the page's.
    if let Some(kit) = &kit {
        child.visuals_mut().override_text_color = Some(kit.colors().field_text);
    }
    let response = child.add(
        egui::TextEdit::singleline(query)
            .id(id)
            .frame(false)
            .hint_text(hint)
            .desired_width(text_rect.width()),
    );

    if !query.is_empty() {
        let clear = Rect::from_center_size(
            pos2(rect.right() - height / 2.0, rect.center().y),
            vec2(height - 8.0, height - 8.0),
        );
        let hit = ui.interact(clear, id.with("clear"), Sense::click());
        let fill = if hit.hovered() {
            palette.track_off
        } else {
            palette.field
        };
        ui.painter()
            .circle_filled(clear.center(), clear.width() / 2.0, fill);
        icons::draw(
            ui.painter(),
            clear,
            icons::Glyph::SmallCross,
            palette.subtle,
            Color32::TRANSPARENT,
        );
        if hit.clicked() {
            query.clear();
            response.request_focus();
        }
    }
    response
}

fn sidebar_entry(
    ui: &mut Ui,
    item: &SidebarItem<'_>,
    selected: bool,
    palette: &Palette,
) -> Response {
    if let Some(kit) = Kit::of(ui.ctx()) {
        return skinned_sidebar_entry(ui, item, selected, &kit);
    }
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), SIDEBAR_ROW), Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, Rounding::same(PILL_RADIUS), palette.accent);
    } else if response.hovered() {
        painter.rect_filled(rect, Rounding::same(PILL_RADIUS), palette.hover);
    }
    if response.has_focus() {
        painter.rect_stroke(
            rect.shrink(1.0),
            Rounding::same(PILL_RADIUS),
            Stroke::new(1.5_f32, palette.accent.lerp_to_gamma(palette.text, 0.4)),
        );
    }
    let tile = Rect::from_center_size(
        pos2(rect.left() + 6.0 + TILE / 2.0, rect.center().y),
        vec2(TILE, TILE),
    );
    painter.rect_filled(tile, Rounding::same(5.0), item.tile);
    icons::symbol(painter, tile.shrink(3.0), item.symbol, Color32::WHITE);

    let colour = if selected {
        palette.on_accent
    } else {
        palette.text
    };
    let font = TextStyle::Body.resolve(ui.style());
    let galley = painter.layout_no_wrap(item.label.to_owned(), font, colour);
    let pos = pos2(tile.right() + 9.0, rect.center().y - galley.size().y / 2.0);
    painter.with_clip_rect(rect).galley(pos, galley, colour);
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            selected,
            item.label,
        )
    });
    response
}

/// A category as a row of a list box: the symbol in the row's ink rather than
/// on a coloured tile - the tiles are System Settings', and no list of the
/// skins' era had them - and the selection in the skin's own colours.
fn skinned_sidebar_entry(
    ui: &mut Ui,
    item: &SidebarItem<'_>,
    selected: bool,
    kit: &Kit,
) -> Response {
    let height = kit.px(kit.metrics().menu_item_height + 2.0);
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let style = kit.style();
    let st = St {
        on: selected,
        hovered: response.hovered(),
        focused: response.has_focus(),
        ..St::default()
    };
    let painter = ui.painter();
    painter.extend(style.list_item(kit, rect, st));
    if st.focused {
        painter.extend(style.focus(kit, rect));
    }
    let ink = style.list_text(kit, st);
    let glyph = Rect::from_center_size(
        pos2(rect.left() + 4.0 + 8.0, rect.center().y),
        vec2(14.0, 14.0),
    );
    icons::symbol(painter, glyph, item.symbol, ink);
    let font = TextStyle::Body.resolve(ui.style());
    let galley = painter.layout_no_wrap(item.label.to_owned(), font, ink);
    let pos = pos2(
        glyph.right() + 6.0,
        (rect.center().y - galley.size().y / 2.0).round(),
    );
    painter.with_clip_rect(rect).galley(pos, galley, ink);
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            selected,
            item.label,
        )
    });
    response
}

// ---------------------------------------------------------------------------
// Pages
// ---------------------------------------------------------------------------

/// A page's title, large and bold, at the top of the content side.
pub fn page_header(ui: &mut Ui, title: &str) {
    page_header_with_back(ui, title, None);
}

/// A sub-page's title, with a "<" link back to the page it was opened from
/// above it. Returns true when the link was clicked.
pub fn page_header_with_back(ui: &mut Ui, title: &str, back: Option<&str>) -> bool {
    let palette = Palette::for_ui(ui);
    let mut clicked = false;
    ui.add_space(4.0);
    if let Some(back) = back {
        let font = TextStyle::Body.resolve(ui.style());
        let galley = ui
            .painter()
            .layout_no_wrap(back.to_owned(), font, palette.accent_text);
        let chevron = 12.0;
        let size = vec2(
            chevron + 4.0 + galley.size().x,
            galley.size().y.max(chevron),
        );
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let colour = if response.hovered() {
            palette.text
        } else {
            palette.subtle
        };
        let glyph = Rect::from_center_size(
            pos2(rect.left() + chevron / 2.0, rect.center().y),
            vec2(chevron, chevron),
        );
        // The right chevron turned round: one mark, so the two read as the
        // way in and the way back out.
        let painter = ui.painter();
        painter.line_segment(
            [
                pos2(glyph.right() - 3.0, glyph.top() + 2.0),
                pos2(glyph.left() + 3.0, glyph.center().y),
            ],
            Stroke::new(1.6_f32, colour),
        );
        painter.line_segment(
            [
                pos2(glyph.left() + 3.0, glyph.center().y),
                pos2(glyph.right() - 3.0, glyph.bottom() - 2.0),
            ],
            Stroke::new(1.6_f32, colour),
        );
        painter.galley(
            pos2(glyph.right() + 4.0, rect.center().y - galley.size().y / 2.0),
            galley,
            colour,
        );
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        clicked = response.clicked();
        ui.add_space(2.0);
    }
    let size = TextStyle::Body.resolve(ui.style()).size * 1.5;
    let mut text = egui::RichText::new(title)
        .size(size.round())
        .strong()
        .color(palette.heading);
    if Kit::of(ui.ctx()).is_some() {
        // A real bold face rather than egui's brighter `strong`.
        text = text.family(crate::ui::fonts::skin_bold(ui.ctx()));
    }
    ui.label(text);
    ui.add_space(6.0);
    clicked
}

/// The rest of `ui` as a vertical scroll, with a [`column()`] inside it - the
/// body of an ordinary page. Each `id_source` keeps its own scroll offset, so
/// coming back to a page finds it where it was left.
pub fn scroll_column<R>(
    ui: &mut Ui,
    id_source: impl Hash,
    max_width: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    egui::ScrollArea::vertical()
        .id_source(id_source)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            column(ui, max_width, |ui| {
                let inner = add(ui);
                ui.add_space(16.0);
                inner
            })
        })
        .inner
}

/// A column no wider than `max_width`, centred in what `ui` has.
///
/// GNOME's "clamp": a settings row stretched across a maximized window puts
/// its switch a screen's width from the words it answers.
pub fn column<R>(ui: &mut Ui, max_width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let available = ui.available_width();
    let width = available.min(max_width);
    let left = ui.cursor().min.x + ((available - width) / 2.0).floor();
    let top = ui.cursor().min.y;
    let rect = Rect::from_min_size(pos2(left, top), vec2(width, 0.0));
    let mut child = ui.child_ui_with_id_source(rect, Layout::top_down(Align::Min), "column", None);
    child.set_width(width);
    let inner = add(&mut child);
    let used = child.min_rect();
    // Claimed across the whole width, so a scroll area around it keeps its
    // own width rather than shrinking to the column.
    ui.allocate_rect(
        Rect::from_min_max(
            pos2(ui.max_rect().left(), top),
            pos2(ui.max_rect().right(), used.bottom()),
        ),
        Sense::hover(),
    );
    inner
}

/// A bold heading over a card.
pub fn section(ui: &mut Ui, title: &str) {
    section_heading(ui, title, Sense::hover());
}

/// A heading that is also a link - a search result's page name, which takes
/// you to the page.
pub fn section_link(ui: &mut Ui, title: &str) -> Response {
    section_heading(ui, title, Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn section_heading(ui: &mut Ui, title: &str, sense: Sense) -> Response {
    let palette = Palette::for_ui(ui);
    ui.add_space(14.0);
    let skinned = Kit::of(ui.ctx()).is_some();
    let mut font = TextStyle::Body.resolve(ui.style());
    if skinned {
        font.family = crate::ui::fonts::skin_bold(ui.ctx());
    }
    let galley = ui.painter().layout(
        title.to_owned(),
        font,
        palette.heading,
        ui.available_width() - 2.0 * PAD_X,
    );
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y), sense);
    let pos = pos2(rect.left() + 4.0, rect.top());
    // Drawn twice, a hair apart: egui has no bold weight to ask for - its
    // `strong` only changes the colour - and a heading that is merely brighter
    // than the rows under it does not read as a heading.
    let painter = ui.painter();
    painter.galley(pos, galley.clone(), palette.heading);
    // A skin has a bold face of its own; drawing it twice would smear it.
    if !skinned {
        painter.galley(pos + vec2(0.45, 0.0), galley.clone(), palette.heading);
    }
    if sense.click && response.hovered() {
        let y = rect.top() + galley.size().y;
        painter.hline(
            pos.x..=pos.x + galley.size().x,
            y,
            Stroke::new(1.0_f32, palette.heading),
        );
    }
    ui.add_space(6.0);
    response
}

/// A grey note under a card.
pub fn footer(ui: &mut Ui, text: &str) {
    let palette = Palette::for_ui(ui);
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        ui.add_space(PAD_X);
        ui.add(
            egui::Label::new(
                egui::RichText::new(text)
                    .font(subtitle_font(ui))
                    .color(palette.subtle),
            )
            .wrap(),
        );
    });
}

thread_local! {
    /// Set by [`titled_card`] for the [`card`] it is about to draw, which then
    /// leaves the framing to it.
    static IN_GROUP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A [`section`] heading and the [`card`] under it, `enabled` or greyed out.
///
/// One call rather than two because under a skin they are one thing: a group
/// box whose legend is the heading, sitting in a gap in its frame, the way
/// every dialog of the period grouped its settings. Without a skin it draws
/// exactly the two it replaces, the heading left out of the greying as
/// before.
pub fn titled_card<R>(
    ui: &mut Ui,
    heading: Option<&str>,
    enabled: bool,
    id_source: impl Hash,
    add: impl FnOnce(&mut Card<'_>) -> R,
) -> R {
    let Some(kit) = Kit::of(ui.ctx()) else {
        match heading {
            Some(heading) => section(ui, heading),
            None => ui.add_space(10.0),
        }
        return ui
            .add_enabled_ui(enabled, |ui| card(ui, id_source, add))
            .inner;
    };
    let c = kit.colors();
    let font = TextStyle::Body.resolve(ui.style());
    let legend = heading.map(|heading| {
        let ink = if enabled { c.text } else { c.text_disabled };
        ui.painter()
            .layout_no_wrap(heading.to_owned(), font.clone(), ink)
    });
    let legend_height = legend.as_ref().map_or(0.0, |g| g.size().y);
    ui.add_space(8.0 + legend_height / 2.0);
    let background = ui.painter().add(Shape::Noop);
    let top = ui.cursor().min.y;
    IN_GROUP.with(|g| g.set(true));
    let shown = ui.add_enabled_ui(enabled, |ui| {
        egui::Frame::none()
            .inner_margin(egui::Margin {
                left: 4.0,
                right: 4.0,
                top: legend_height / 2.0 + 4.0,
                bottom: 4.0,
            })
            .show(ui, |ui| card(ui, id_source, add))
            .inner
    });
    let rect = Rect::from_min_max(
        pos2(shown.response.rect.left(), top),
        shown.response.rect.max,
    );
    let legend_rect = legend
        .as_ref()
        .map(|g| Rect::from_min_size(pos2(rect.left() + 8.0, top - g.size().y / 2.0), g.size()));
    let mut shapes = kit.style().group(&kit, rect, legend_rect);
    if let (Some(galley), Some(at)) = (legend, legend_rect) {
        shapes.push(Shape::galley(at.min, galley, c.text));
    }
    ui.painter().set(background, Shape::Vec(shapes));
    shown.inner
}

/// A boxed list: a rounded card that rows are added to, with a hairline
/// between each two of them.
///
/// `id_source` keeps the rows of one card apart from another's, so two cards
/// that both have a "Name" row do not share its remembered layout.
pub fn card<R>(ui: &mut Ui, id_source: impl Hash, add: impl FnOnce(&mut Card<'_>) -> R) -> R {
    let palette = Palette::for_ui(ui);
    let framed = IN_GROUP.with(|g| g.replace(false));
    // Reserved before the contents and filled in after, when the card's
    // height is known: the background has to be under the rows, and only the
    // rows know how tall they came out.
    let background = ui.painter().add(Shape::Noop);
    let shown = ui.push_id(id_source, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.set_width(ui.available_width());
        let mut card = Card { ui, rows: 0 };
        add(&mut card)
    });
    let rect = shown.response.rect;
    // Under a skin a card is the inside of a group box: framed by
    // `titled_card` when it has a heading, a plain group box when not.
    if let Some(kit) = Kit::of(ui.ctx()) {
        if !framed {
            ui.painter()
                .set(background, Shape::Vec(kit.style().group(&kit, rect, None)));
        }
    } else {
        ui.painter().set(
            background,
            Shape::Vec(vec![
                Shape::rect_filled(rect, Rounding::same(CARD_RADIUS), palette.card),
                Shape::rect_stroke(
                    rect.shrink(0.5),
                    Rounding::same(CARD_RADIUS),
                    Stroke::new(1.0_f32, palette.card_stroke),
                ),
            ]),
        );
    }
    shown.inner
}

/// The inside of a [`card`]. Each method adds one row, with a separator above
/// it unless it is the first.
pub struct Card<'u> {
    ui: &'u mut Ui,
    rows: usize,
}

impl Card<'_> {
    /// The hairline above every row but the first, inset on the left the way
    /// a grouped list's is, so the card's edge stays unbroken.
    fn next_row(&mut self) {
        if self.rows > 0 {
            let palette = Palette::for_ui(self.ui);
            let y = self.ui.cursor().min.y;
            let rect = self.ui.max_rect();
            self.ui.painter().hline(
                rect.left() + PAD_X..=rect.right(),
                y,
                Stroke::new(1.0_f32, palette.separator),
            );
        }
        self.rows += 1;
    }

    /// A title on the left and `control` on the right. See [`row`].
    pub fn row<R>(&mut self, spec: Row<'_>, control: impl FnOnce(&mut Ui) -> R) -> RowResponse<R> {
        self.next_row();
        row(self.ui, spec, control)
    }

    /// A row with a [`toggle`] on its right. Returns true when it was flipped.
    ///
    /// Clicking the row's words flips it too, as on GNOME - a switch is a
    /// small target at the far end of a long sentence - unless the row is a
    /// [`Row::link()`], whose words lead somewhere instead.
    pub fn toggle(&mut self, spec: Row<'_>, value: &mut bool) -> RowResponse<bool> {
        let activates = !spec.link;
        let spec = Row { link: true, ..spec };
        let mut shown = self.row(spec, |ui| toggle(ui, value).changed());
        if activates && shown.label.clicked() {
            *value = !*value;
            shown.inner = true;
        }
        shown
    }

    /// A row that leads somewhere: the whole row is the target, with a
    /// chevron on the right saying so.
    pub fn nav(&mut self, spec: Row<'_>) -> Response {
        self.next_row();
        let spec = Row { link: true, ..spec };
        let palette = Palette::for_ui(self.ui);
        let shown = row(self.ui, spec, |ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
            icons::symbol(ui.painter(), rect, Symbol::ChevronRight, palette.subtle);
        });
        let whole = self
            .ui
            .interact(shown.rect, shown.label.id.with("nav"), Sense::click());
        if whole.hovered() || shown.label.hovered() {
            self.ui
                .ctx()
                .set_cursor_icon(egui::CursorIcon::PointingHand);
            // Over the words rather than under them, since the row has been
            // drawn by now - which is why it is the faint hover ink and not
            // anything stronger.
            self.ui.painter().rect_filled(
                shown.rect.shrink(2.0),
                Rounding::same(CARD_RADIUS - 2.0),
                palette.hover,
            );
        }
        whole.union(shown.label)
    }

    /// A row of buttons along the right edge, like the "Add Controls..." at
    /// the foot of a card. Laid out right to left: the first one added is the
    /// rightmost.
    pub fn buttons<R>(&mut self, add: impl FnOnce(&mut Ui) -> R) -> R {
        self.row(Row::new(""), add).inner
    }

    /// Anything at all, full width with a row's padding round it.
    pub fn custom<R>(&mut self, add: impl FnOnce(&mut Ui) -> R) -> R {
        self.next_row();
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(PAD_X, PAD_Y + 2.0))
            .show(self.ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 6.0;
                add(ui)
            })
            .inner
    }

    /// The card's own `Ui`, for scrolling to a row or reading the style.
    pub fn ui(&mut self) -> &mut Ui {
        self.ui
    }
}

/// What a row says about itself.
#[derive(Clone, Copy, Debug, Default)]
pub struct Row<'a> {
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    /// Shown when the pointer rests on the row's words.
    pub hint: Option<&'a str>,
    /// How strongly the row is washed in the accent, 0 to 1 - the brief flash
    /// on a row a search result has just led to.
    pub highlight: f32,
    /// The row's words are a link: they react to the pointer and their click
    /// is reported in [`RowResponse::label`].
    pub link: bool,
}

impl<'a> Row<'a> {
    pub fn new(title: &'a str) -> Self {
        Row {
            title,
            ..Row::default()
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<Option<&'a str>>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    pub fn hint(mut self, hint: impl Into<Option<&'a str>>) -> Self {
        self.hint = hint.into();
        self
    }

    pub fn highlight(mut self, strength: f32) -> Self {
        self.highlight = strength;
        self
    }

    pub fn link(mut self, link: bool) -> Self {
        self.link = link;
        self
    }
}

/// What a row came to.
pub struct RowResponse<R> {
    /// What the control returned.
    pub inner: R,
    /// The row's words.
    pub label: Response,
    /// The whole row.
    pub rect: Rect,
}

/// One row: `spec`'s title and subtitle on the left, `control` on the right.
///
/// The control is drawn in a right-to-left layout, centred on the row: the
/// first widget it adds is the rightmost, so a value and the button beside it
/// are added button first.
///
/// The words wrap to what the control leaves them, and the control is only
/// measured once it is drawn - so the width it took is remembered and used on
/// the next frame. A row whose control changes width settles a frame later,
/// which is why a change asks for that frame.
pub fn row<R>(ui: &mut Ui, spec: Row<'_>, control: impl FnOnce(&mut Ui) -> R) -> RowResponse<R> {
    let palette = Palette::for_ui(ui);
    let id = ui.next_auto_id().with(("prefs-row", spec.title));
    let width = ui.available_width();
    let left = ui.cursor().min.x;
    let top = ui.cursor().min.y;
    let background = ui.painter().add(Shape::Noop);

    let remembered = ui.data(|d| d.get_temp::<f32>(id));
    let body = TextStyle::Body.resolve(ui.style());
    // A control too wide to stand beside its title - seven choices in a wide
    // pixel face - goes on a line of its own under it, rather than over it.
    let title_line = ui
        .painter()
        .layout_no_wrap(spec.title.to_owned(), body.clone(), palette.text)
        .size()
        .x;
    let stacked = !spec.title.is_empty()
        && remembered.is_some_and(|w| w + title_line + GAP + 2.0 * PAD_X > width);
    let control_width = if stacked {
        0.0
    } else {
        remembered.unwrap_or(width * 0.4).clamp(0.0, width * 0.7)
    };
    let text_room = (width - 2.0 * PAD_X - control_width - GAP).max(width * 0.3);
    let title = ui
        .painter()
        .layout(spec.title.to_owned(), body, palette.text, text_room);
    let subtitle = spec.subtitle.filter(|s| !s.is_empty()).map(|s| {
        ui.painter()
            .layout(s.to_owned(), subtitle_font(ui), palette.subtle, text_room)
    });
    let text_width = title
        .size()
        .x
        .max(subtitle.as_ref().map_or(0.0, |g| g.size().x));
    let text_height = title.size().y + subtitle.as_ref().map_or(0.0, |g| g.size().y + 1.0);
    let line = ui.spacing().interact_size.y + PAD_Y;
    let height = if stacked {
        text_height + 2.0 * PAD_Y + line
    } else {
        (text_height + 2.0 * PAD_Y).max(MIN_ROW)
    };

    let control_left = if spec.title.is_empty() || stacked {
        left + PAD_X
    } else {
        left + PAD_X + text_width + GAP
    };
    let control_rect = if stacked {
        Rect::from_min_max(
            pos2(control_left, top + height - PAD_Y - line),
            pos2(left + width - PAD_X, top + height - PAD_Y),
        )
    } else {
        Rect::from_min_max(
            pos2(control_left, top),
            pos2((left + width - PAD_X).max(control_left), top + height),
        )
    };
    let mut child = ui.child_ui_with_id_source(
        control_rect,
        Layout::right_to_left(Align::Center),
        id.with("control"),
        None,
    );
    let inner = control(&mut child);
    let used = child.min_rect();
    let measured = if used.width() > 0.5 {
        used.width()
    } else {
        0.0
    };
    if remembered.is_none_or(|w| (w - measured).abs() > 0.5) {
        ui.data_mut(|d| d.insert_temp(id, measured));
        ui.ctx().request_repaint();
    }

    let rect = Rect::from_min_size(pos2(left, top), vec2(width, height));
    ui.allocate_rect(rect, Sense::hover());
    let label_rect = if stacked {
        Rect::from_min_max(rect.min, pos2(rect.right(), control_rect.top()))
    } else {
        Rect::from_min_max(rect.min, pos2(control_left - GAP / 2.0, rect.bottom()))
    };
    let sense = if spec.link {
        Sense::click()
    } else {
        Sense::hover()
    };
    let mut label = ui.interact(label_rect, id.with("label"), sense);
    if spec.link && label.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if let Some(hint) = spec.hint {
        label = label.tip(hint);
    }

    let painter = ui.painter();
    let wash = if spec.highlight > 0.0 {
        palette
            .accent
            .gamma_multiply(0.45 * spec.highlight.min(1.0))
    } else if spec.link && label.hovered() {
        palette.hover
    } else {
        Color32::TRANSPARENT
    };
    if wash != Color32::TRANSPARENT {
        painter.set(
            background,
            Shape::rect_filled(rect.shrink(2.0), Rounding::same(CARD_RADIUS - 2.0), wash),
        );
    }
    let mut y = if stacked {
        top + PAD_Y
    } else {
        rect.center().y - text_height / 2.0
    };
    let x = left + PAD_X;
    let title_height = title.size().y;
    painter.galley(pos2(x, y), title, palette.text);
    y += title_height + 1.0;
    if let Some(subtitle) = subtitle {
        painter.galley(pos2(x, y), subtitle, palette.subtle);
    }
    if label.has_focus() {
        painter.rect_stroke(
            label_rect.shrink(2.0),
            Rounding::same(4.0),
            Stroke::new(1.5_f32, palette.accent),
        );
    }
    RowResponse { inner, label, rect }
}

// ---------------------------------------------------------------------------
// Controls
// ---------------------------------------------------------------------------

/// A switch: a pill-shaped track with a round knob that slides across it, the
/// track in the accent when it is on.
///
/// Focusable like any button, and Space or Enter flips it once it has the
/// keyboard. The response is marked changed on the frame it is flipped.
pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    if let Some(kit) = Kit::of(ui.ctx()) {
        return checkbox(ui, on, &kit);
    }
    let height = (ui.spacing().interact_size.y + 2.0).round();
    let size = vec2((height * 1.75).round(), height);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });
    if ui.is_rect_visible(rect) {
        let palette = Palette::for_ui(ui);
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let track = palette.track_off.lerp_to_gamma(palette.accent, t);
        let radius = rect.height() / 2.0;
        let painter = ui.painter();
        painter.rect_filled(rect, Rounding::same(radius), track);
        let knob_radius = radius - 2.0;
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        let centre = pos2(x, rect.center().y);
        // A faint shadow under the knob, so a white knob on a light track
        // still has an edge.
        painter.circle_filled(
            centre + vec2(0.0, 0.6),
            knob_radius + 0.6,
            Color32::from_black_alpha(45),
        );
        painter.circle_filled(centre, knob_radius, Color32::WHITE);
        if response.has_focus() {
            painter.rect_stroke(
                rect.expand(2.0),
                Rounding::same(radius + 2.0),
                Stroke::new(1.5_f32, palette.accent.lerp_to_gamma(palette.text, 0.3)),
            );
        }
    }
    response
}

/// A switch as the skins drew one: a checkbox. Its row's words are its
/// label, so it is the box alone.
fn checkbox(ui: &mut Ui, on: &mut bool, kit: &Kit) -> Response {
    let side = kit.px(kit.metrics().checkbox);
    let (rect, mut response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });
    if ui.is_rect_visible(rect) {
        let st = St::of(&response).on(*on).disabled(!ui.is_enabled());
        let style = kit.style();
        let painter = ui.painter();
        painter.extend(style.checkbox(kit, rect, st));
        if st.focused {
            painter.extend(style.focus(kit, rect.expand(2.0 * kit.u())));
        }
    }
    response
}

/// One choice out of a few, as the skins drew one: a row of radio buttons,
/// each with its label.
fn radios<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, &str)],
    kit: &Kit,
) -> bool {
    let style = kit.style();
    let m = kit.metrics();
    let c = kit.colors();
    let font = TextStyle::Body.resolve(ui.style());
    let side = kit.px(m.radio);
    let gap = kit.px(m.label_gap);
    let mut changed = false;
    // Laid out right to left, as a row's control is, so the first option
    // still reads first: the options are added last to first.
    for (index, (option, label)) in options.iter().enumerate().rev() {
        let ink = if ui.is_enabled() {
            c.text
        } else {
            c.text_disabled
        };
        let galley = ui
            .painter()
            .layout_no_wrap((*label).to_owned(), font.clone(), ink);
        let size = vec2(side + gap + galley.size().x, side.max(galley.size().y));
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::Default);
        if response.clicked() && *value != *option {
            *value = *option;
            changed = true;
        }
        let selected = *value == *option;
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, selected, *label)
        });
        let st = St::of(&response).on(selected).disabled(!ui.is_enabled());
        let circle = Rect::from_min_size(
            pos2(rect.left(), rect.center().y - side / 2.0),
            vec2(side, side),
        );
        let painter = ui.painter();
        painter.extend(style.radio(kit, circle, st));
        let text = pos2(
            circle.right() + gap,
            (rect.center().y - galley.size().y / 2.0).round(),
        );
        let text_rect = Rect::from_min_size(text, galley.size());
        painter.galley(text, galley, ink);
        if st.focused {
            painter.extend(style.focus(kit, text_rect.expand(kit.u())));
        }
        if index > 0 {
            ui.add_space(gap * 2.0);
        }
    }
    changed
}

/// One choice out of a few, as a row of joined buttons - GNOME's "Left |
/// Right". Returns true when the choice changed.
///
/// Each segment is a button of its own, so Tab walks them and Space picks one.
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    if let Some(kit) = Kit::of(ui.ctx()) {
        return radios(ui, value, options, &kit);
    }
    let palette = Palette::for_ui(ui);
    let font = TextStyle::Body.resolve(ui.style());
    let galleys: Vec<_> = options
        .iter()
        .map(|(_, label)| {
            ui.painter()
                .layout_no_wrap((*label).to_owned(), font.clone(), palette.text)
        })
        .collect();
    let height = ui.spacing().interact_size.y + 4.0;
    let widths: Vec<f32> = galleys.iter().map(|g| (g.size().x + 20.0).ceil()).collect();
    let total = widths.iter().sum::<f32>() + 4.0;
    let (rect, response) = ui.allocate_exact_size(vec2(total, height), Sense::hover());
    let painter = ui.painter().clone();
    painter.rect_filled(rect, Rounding::same(7.0), palette.field);

    let mut changed = false;
    let mut x = rect.left() + 2.0;
    for (index, ((option, _), galley)) in options.iter().zip(galleys).enumerate() {
        let segment =
            Rect::from_min_size(pos2(x, rect.top() + 2.0), vec2(widths[index], height - 4.0));
        x += widths[index];
        let hit = ui.interact(segment, response.id.with(index), Sense::click());
        if hit.clicked() && *value != *option {
            *value = *option;
            changed = true;
        }
        let selected = *value == *option;
        let text = if selected {
            painter.rect_filled(segment, Rounding::same(5.0), palette.accent);
            palette.on_accent
        } else {
            if hit.hovered() {
                painter.rect_filled(segment, Rounding::same(5.0), palette.hover);
            }
            palette.text
        };
        if hit.has_focus() {
            painter.rect_stroke(
                segment.shrink(1.0),
                Rounding::same(5.0),
                Stroke::new(1.5_f32, palette.accent.lerp_to_gamma(palette.text, 0.4)),
            );
        }
        let pos = segment.center() - galley.size() / 2.0;
        painter.galley(pos2(pos.x.round(), pos.y.round()), galley, text);
    }
    changed
}

/// A pop-up menu: the current value with a pair of chevrons after it, opening
/// onto `menu`. Returns what `menu` returned, when it was open.
pub fn popup<R>(
    ui: &mut Ui,
    id_source: impl Hash,
    selected: impl Into<WidgetText>,
    menu: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    if let Some(kit) = Kit::of(ui.ctx()) {
        return select(ui, Id::new(id_source), selected.into(), menu, &kit);
    }
    egui::ComboBox::from_id_source(id_source)
        .selected_text(selected)
        // Wider than egui's default, which cut theme and font names short.
        .width(200.0)
        .icon(|ui, rect, visuals, _open, _above| {
            let rect = Rect::from_center_size(rect.center(), vec2(12.0, 12.0));
            icons::symbol(
                ui.painter(),
                rect,
                Symbol::ChevronUpDown,
                visuals.fg_stroke.color,
            );
        })
        .show_ui(ui, menu)
        .inner
}

/// A drop-down as the skins drew one: the value in a field, the arrow button
/// at its right end, and the list opening under it.
fn select<R>(
    ui: &mut Ui,
    id: Id,
    selected: WidgetText,
    menu: impl FnOnce(&mut Ui) -> R,
    kit: &Kit,
) -> Option<R> {
    let style = kit.style();
    let m = kit.metrics();
    let size = vec2(200.0, kit.px(m.field_height));
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let popup = id.with("list");
    if response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup));
    }
    let open = ui.memory(|mem| mem.is_popup_open(popup));
    let st = St::of(&response).on(open).disabled(!ui.is_enabled());
    if ui.is_rect_visible(rect) {
        ui.painter().extend(style.select(kit, rect, st));
        let ink = style.select_text(kit, st);
        let galley = selected.into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            rect.width() - kit.px(m.select_arrow) - 10.0,
            TextStyle::Button,
        );
        let at = pos2(
            rect.left() + kit.px(4.0),
            (rect.center().y - galley.size().y / 2.0).round(),
        );
        // Over the colour the page's text is laid out in: the skin's field
        // ink, whatever the user's page text is.
        ui.painter()
            .galley_with_override_text_color(at, galley, ink);
    }
    egui::popup::popup_below_widget(
        ui,
        popup,
        &response,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(rect.width() - 6.0);
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, menu)
                .inner
        },
    )
}

/// A slider with what its two ends mean written at them - "Clear ... Solid" -
/// and the value in front. Drawn right to left, as a row's control is.
pub fn slider<N: egui::emath::Numeric>(
    ui: &mut Ui,
    value: &mut N,
    range: std::ops::RangeInclusive<N>,
    ends: Option<(&str, &str)>,
    configure: impl FnOnce(egui::Slider<'_>) -> egui::Slider<'_>,
) -> Response {
    let palette = Palette::for_ui(ui);
    let caption = |ui: &mut Ui, text: &str| {
        ui.label(
            egui::RichText::new(text)
                .font(subtitle_font(ui))
                .color(palette.subtle),
        );
    };
    if let Some((_, high)) = ends {
        caption(ui, high);
    }
    ui.spacing_mut().slider_width = 150.0;
    let kit = Kit::of(ui.ctx());
    let response = match &kit {
        None => ui.add(configure(egui::Slider::new(value, range).show_value(false))),
        Some(kit) => trackbar(ui, value, range, configure, kit),
    };
    if let Some((low, _)) = ends {
        caption(ui, low);
    }
    let shown = egui::emath::format_with_decimals_in_range(value.to_f64(), 0..=2);
    ui.label(egui::RichText::new(shown).color(palette.subtle).monospace());
    response
}

/// egui's slider for the dragging - its steps, its clamping, its keys - with
/// the skin's trackbar drawn in its place.
fn trackbar<N: egui::emath::Numeric>(
    ui: &mut Ui,
    value: &mut N,
    range: std::ops::RangeInclusive<N>,
    configure: impl FnOnce(egui::Slider<'_>) -> egui::Slider<'_>,
    kit: &Kit,
) -> Response {
    let (low, high) = (range.start().to_f64(), range.end().to_f64());
    let thumb = kit.metrics().slider_thumb * kit.u();
    let response = ui
        .scope(|ui| {
            // Nothing of egui's own slider is drawn: every colour it paints
            // in is clear, and the rail's room is the thumb's height.
            ui.spacing_mut().interact_size.y = thumb.y;
            let visuals = ui.visuals_mut();
            for w in [
                &mut visuals.widgets.inactive,
                &mut visuals.widgets.hovered,
                &mut visuals.widgets.active,
                &mut visuals.widgets.noninteractive,
            ] {
                w.bg_fill = Color32::TRANSPARENT;
                w.weak_bg_fill = Color32::TRANSPARENT;
                w.bg_stroke = Stroke::NONE;
                w.fg_stroke = Stroke::NONE;
            }
            visuals.selection.bg_fill = Color32::TRANSPARENT;
            ui.add(configure(egui::Slider::new(value, range).show_value(false)))
        })
        .inner;
    let rect = response.rect;
    if ui.is_rect_visible(rect) {
        let style = kit.style();
        let st = St::of(&response).disabled(!ui.is_enabled());
        let span = (high - low).abs().max(f64::EPSILON);
        let t = ((value.to_f64() - low) / span).clamp(0.0, 1.0) as f32;
        // The thumb travels between its own half-widths, as the browser's
        // does: at either end it is flush with the track's end.
        let x = egui::lerp(
            (rect.left() + thumb.x / 2.0)..=(rect.right() - thumb.x / 2.0),
            t,
        );
        let painter = ui.painter();
        painter.extend(style.slider_track(kit, rect, st));
        painter.extend(style.slider_thumb(kit, pos2(x, rect.center().y), st));
    }
    response
}

/// A rounded push button, the size of the others on a row.
pub fn button(ui: &mut Ui, text: &str) -> Response {
    if let Some(kit) = Kit::of(ui.ctx()) {
        return crate::ui::skin::button(ui, &kit, text, false);
    }
    ui.add(
        egui::Button::new(text)
            .rounding(Rounding::same(6.0))
            .min_size(vec2(0.0, ui.spacing().interact_size.y + 4.0)),
    )
}

/// [`button`] as a widget, for one that has to go through `add_enabled`.
pub fn button_widget(text: &str) -> egui::Button<'_> {
    egui::Button::new(text)
        .rounding(Rounding::same(6.0))
        .min_size(vec2(0.0, 22.0))
}

/// A line of grey text saying there is nothing here - a search that found
/// nothing, a list with no entries.
pub fn empty_state(ui: &mut Ui, text: &str) {
    let palette = Palette::for_ui(ui);
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(text).color(palette.subtle));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A control too wide to stand beside its title goes under it: the title
    /// is never drawn over.
    #[test]
    fn a_control_too_wide_for_its_row_goes_under_the_title() {
        let ctx = egui::Context::default();
        let mut control = Rect::NOTHING;
        let mut top = 0.0;
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(320.0, 400.0))),
            ..Default::default()
        };
        for _ in 0..3 {
            let _ = ctx.run(input(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    top = ui.cursor().min.y;
                    let _ = row(ui, Row::new("Style"), |ui| {
                        control = ui.allocate_exact_size(vec2(280.0, 20.0), Sense::hover()).0;
                    });
                });
            });
        }
        // The title's line is the top of the row; the control starts below it.
        let title_bottom = top + PAD_Y + 12.0;
        assert!(control.top() >= title_bottom, "{control:?} top {top}");
    }

    /// Draws one row headlessly in a panel `width` wide, a couple of frames so
    /// the control's width is remembered, and returns where its words and its
    /// control landed, with the panel's own rect.
    fn lay_out_row(width: f32, subtitle: Option<&str>) -> (Rect, Rect, Rect) {
        let ctx = egui::Context::default();
        let mut out = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING);
        for _ in 0..3 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(width, 400.0))),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut control = Rect::NOTHING;
                    let shown = row(ui, Row::new("Title").subtitle(subtitle), |ui| {
                        control = ui.allocate_exact_size(vec2(60.0, 20.0), Sense::hover()).0;
                    });
                    out = (shown.label.rect, control, ui.max_rect());
                });
            });
        }
        out
    }

    #[test]
    fn a_row_puts_its_words_on_the_left_and_its_control_against_the_right_edge() {
        let (label, control, panel) = lay_out_row(500.0, None);
        assert_eq!(label.left(), panel.left());
        assert!((control.right() - (panel.right() - PAD_X)).abs() < 0.5);
        assert!(label.right() < control.left());
        // Centred on the row, which is never shorter than the minimum.
        assert!((control.center().y - (panel.top() + MIN_ROW / 2.0)).abs() < 0.5);
    }

    #[test]
    fn a_subtitle_too_long_for_one_line_wraps_short_of_the_control_and_grows_the_row() {
        let long =
            "A sentence long enough that it cannot possibly fit on one line of a row this narrow.";
        let (label, control, _) = lay_out_row(360.0, Some(long));
        assert!(label.right() < control.left());
        assert!(label.height() > MIN_ROW);
    }

    #[test]
    fn folding_ignores_case_and_accents() {
        assert_eq!(fold("Configuração"), "configuracao");
        assert_eq!(fold("ÁÉÍÓÚ àêõü Ñ"), "aeiou aeou n");
        assert_eq!(fold("Núm. de Linhas"), "num. de linhas");
    }

    #[test]
    fn a_query_without_accents_finds_text_with_them() {
        assert!(!matches("rolagem numero", &["Núm. de Linhas de Rolagem"]));
        assert!(matches("num rolagem", &["Núm. de Linhas de Rolagem"]));
        assert!(matches("ACAO", &["Ação"]));
        assert!(matches("ação", &["acao"]));
    }

    #[test]
    fn every_word_has_to_be_found_but_each_may_be_found_anywhere() {
        let haystacks = ["Close to the tray", "bandeja"];
        assert!(matches("tray close", &haystacks));
        assert!(matches("close bandeja", &haystacks));
        assert!(!matches("close window", &haystacks));
    }

    #[test]
    fn an_empty_query_matches_everything() {
        assert!(matches("", &["anything"]));
        assert!(matches("   ", &[]));
    }

    #[test]
    fn a_word_is_matched_inside_a_longer_one() {
        assert!(matches("scroll", &["Show scrollbars"]));
    }

    fn assert_cards_stand_off_the_page(visuals: &Visuals) {
        let palette = Palette::from_visuals(visuals);
        let page = visuals.window_fill.to_opaque();
        assert_eq!(palette.card.a(), 255);
        assert_ne!(palette.card, page);
        assert_ne!(palette.sidebar, palette.card);
        assert!(palette.separator.a() > 0 && palette.separator.a() < 255);
    }

    #[test]
    fn cards_are_a_step_off_the_page_in_a_light_theme() {
        let visuals = Visuals::light();
        assert_cards_stand_off_the_page(&visuals);
        let palette = Palette::from_visuals(&visuals);
        assert!(luminance(palette.card) > luminance(visuals.window_fill));
    }

    #[test]
    fn cards_are_a_step_off_the_page_in_a_dark_theme() {
        let visuals = Visuals::dark();
        assert_cards_stand_off_the_page(&visuals);
        let palette = Palette::from_visuals(&visuals);
        assert!(luminance(palette.card) > luminance(visuals.window_fill));
        assert!(luminance(palette.sidebar) < luminance(visuals.window_fill));
    }

    #[test]
    fn a_painted_backdrop_shows_through_the_cards_and_the_sidebar() {
        for mut visuals in [Visuals::dark(), Visuals::light()] {
            visuals.panel_fill = Color32::TRANSPARENT;
            let palette = Palette::from_visuals(&visuals);
            for fill in [palette.card, palette.sidebar] {
                assert!(fill.a() < 255, "{fill:?} would cover the backdrop");
            }
            let clear = Palette::with_opacity(&visuals, 0.0);
            assert_eq!(clear.card.a(), 0);
            let solid = Palette::with_opacity(&visuals, 1.0);
            assert_eq!(solid.card.a(), 255);
        }
    }

    #[test]
    fn the_accent_is_the_themes_selection_colour() {
        let mut visuals = Visuals::dark();
        visuals.selection.bg_fill = Color32::from_rgb(0x2a, 0x4a, 0x6b);
        let palette = Palette::from_visuals(&visuals);
        assert_eq!(palette.accent, Color32::from_rgb(0x2a, 0x4a, 0x6b));
        assert_eq!(palette.on_accent, Color32::WHITE);
    }

    #[test]
    fn text_on_a_pale_accent_is_dark() {
        let mut visuals = Visuals::light();
        visuals.selection.bg_fill = Color32::from_rgb(0xcc, 0xe4, 0xff);
        assert_ne!(Palette::from_visuals(&visuals).on_accent, Color32::WHITE);
    }
}
