//! Skins: the controls of another desktop, drawn in egui.
//!
//! A theme picks colours; a skin picks *controls*. Under the Windows 98 skin
//! a button is a grey slab with a two-pixel bevel, a text field is sunk into
//! the page, a toggle is a checkbox and the title bar runs from navy to blue -
//! each drawn from the CSS library the skin reproduces (98.css, XP.css,
//! aqua-ui, system.css, snes.css), value for value.
//!
//! egui draws none of that by itself: its widgets are a fill, one stroke and a
//! rounding. So a skin is a set of painters - [`Style`] - that turn a rect and
//! a state into shapes, and the app's own furniture (the title bar, the tabs,
//! Settings, dialogs, tooltips, the scrollbars) asks the current one for its
//! shapes instead of painting its own. What is left - egui's stock buttons
//! and fields inside dialogs, its menus and popups - is reached by
//! [`post`], which finds them in the frame's shapes and swaps them.
//!
//! [`Skin::None`] is every theme that does not ask for one, and draws
//! exactly what the app always drew: every call site checks [`Kit::of`] and
//! keeps its own painting when there is no kit.

mod aqua;
pub mod post;
pub mod prim;
mod snes;
mod system;
mod tuned;
mod win98;
mod xp;

use egui::{pos2, vec2, Color32, Context, Id, Pos2, Rect, Rounding, Shape, TextureId, Vec2};

use prim::{fill, inset, outline, unit};

/// Which desktop's controls the chrome is drawn with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Skin {
    /// The app's own look.
    #[default]
    None,
    /// Windows 98, after 98.css.
    Win98,
    /// Windows XP's Luna, after XP.css.
    Xp,
    /// Mac OS X Lion's Aqua, after the Lion CSS UI kit.
    Aqua,
    /// The classic Mac OS, after system.css.
    System,
    /// The Super Nintendo, after snes.css.
    Snes,
}

impl Skin {
    pub const ALL: [Skin; 6] = [
        Skin::None,
        Skin::Win98,
        Skin::Xp,
        Skin::Aqua,
        Skin::System,
        Skin::Snes,
    ];

    /// Lenient: empty, or anything unrecognised, is no skin, so a theme file
    /// written before skins existed - or by a later version with one this
    /// build does not know - still opens, looking the way the app draws.
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "win98" | "98" | "windows98" => Skin::Win98,
            "xp" | "luna" | "windowsxp" => Skin::Xp,
            "aqua" | "lion" => Skin::Aqua,
            "system" | "macos" | "classic-mac" => Skin::System,
            "snes" | "super-nintendo" => Skin::Snes,
            _ => Skin::None,
        }
    }

    /// The name written to a theme file. Empty for none, so a theme without
    /// a skin writes nothing about one.
    pub fn name(self) -> &'static str {
        match self {
            Skin::None => "",
            Skin::Win98 => "win98",
            Skin::Xp => "xp",
            Skin::Aqua => "aqua",
            Skin::System => "system",
            Skin::Snes => "snes",
        }
    }

    /// What the picker calls it. Proper names, never translated - and never
    /// put through `tr`, whose table turns "Windows" into "Janelas".
    pub fn label(self) -> &'static str {
        match self {
            Skin::None => "",
            Skin::Win98 => "Windows 98",
            Skin::Xp => "Windows XP",
            Skin::Aqua => "Aqua",
            Skin::System => "System",
            Skin::Snes => "SNES",
        }
    }

    fn style(self) -> &'static dyn Style {
        match self {
            Skin::None => &Plain,
            Skin::Win98 => &win98::Win98,
            Skin::Xp => &xp::Xp,
            Skin::Aqua => &aqua::Aqua,
            Skin::System => &system::System,
            Skin::Snes => &snes::Snes,
        }
    }
}

/// A skin and which of its two readings - the light one it was designed in,
/// or the dark one a theme can ask for - and the colours the user gave it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Look {
    pub skin: Skin,
    pub dark: bool,
    pub paint: Paint,
}

/// What a theme of the user's own says about the chrome's colours, over the
/// skin's. Each is `None` where the theme says what a built-in theme of the
/// same skin says - a copy left as it was keeps the skin's own colours, which
/// are not always the ones its theme file spells out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Paint {
    pub face: Option<Color32>,
    pub text: Option<Color32>,
    pub select: Option<Color32>,
    pub select_text: Option<Color32>,
    /// The hue the skin's own blues are turned to: its title bars, default
    /// buttons, thumbs.
    pub accent: Option<Color32>,
    /// The scroll bar's thumb and its trough.
    pub handle: Option<Color32>,
    pub track: Option<Color32>,
    /// The window's close, minimize and maximize buttons.
    pub close: Option<Color32>,
    pub minimize: Option<Color32>,
    pub maximize: Option<Color32>,
    /// The app's own buttons on the title bar: Settings, keep on top, a
    /// new session.
    pub settings: Option<Color32>,
    pub on_top: Option<Color32>,
    pub new_tab: Option<Color32>,
    /// The mark drawn on every one of them.
    pub icon: Option<Color32>,
}

impl Look {
    /// The skin a theme asks for, in the reading its chrome's background
    /// calls for: a dark chrome gets the dark one.
    pub fn of(theme: &crate::config::Theme) -> Look {
        let own = !theme.builtin && theme.skin != Skin::None;
        Look {
            skin: theme.skin,
            dark: if own {
                reading_of(theme)
            } else {
                crate::config::theme::is_dark(theme.ui_background)
            },
            paint: if own {
                Paint::of(theme)
            } else {
                Paint::default()
            },
        }
    }
}

/// Which reading of its skin a theme of the user's wears: the one of the
/// built-in it was copied from, found by what the copy kept of it - the
/// terminal's colours, the buttons, the scroll bar. Not its background's
/// lightness: a darker background is the user's colour on the same skin,
/// not a call for the skin's dark reading, whose controls are other ones.
fn reading_of(theme: &crate::config::Theme) -> bool {
    crate::config::theme::origin(theme).map_or_else(
        || crate::config::theme::is_dark(theme.ui_background),
        |b| crate::config::theme::is_dark(b.ui_background),
    )
}

impl Paint {
    fn of(theme: &crate::config::Theme) -> Paint {
        use crate::config::Theme;
        let builtins: Vec<Theme> = crate::config::theme::builtin_files()
            .iter()
            .map(Theme::from_file)
            .filter(|t| t.skin == theme.skin)
            .collect();
        // A colour counts as the user's when no built-in of the skin has it.
        fn own<T: PartialEq + Copy>(
            mine: T,
            builtins: &[Theme],
            of: impl Fn(&Theme) -> T,
        ) -> Option<T> {
            (!builtins.iter().any(|b| of(b) == mine)).then_some(mine)
        }
        let saturated = |c: Color32| {
            let [r, g, b, _] = c.to_array();
            r.max(g).max(b) - r.min(g).min(b) > 40
        };
        let border = own(theme.ui_border, &builtins, |t| t.ui_border).flatten();
        let tab = own(theme.tab_selected, &builtins, |t| t.tab_selected).flatten();
        Paint {
            face: own(theme.ui_background, &builtins, |t| t.ui_background),
            text: own(theme.ui_foreground, &builtins, |t| t.ui_foreground),
            select: tab,
            select_text: own(theme.tab_selected_text, &builtins, |t| t.tab_selected_text).flatten(),
            accent: border
                .filter(|&c| saturated(c))
                .or(tab.filter(|&c| saturated(c))),
            handle: own(theme.scrollbar_handle, &builtins, |t| t.scrollbar_handle).flatten(),
            track: own(theme.scrollbar_track, &builtins, |t| t.scrollbar_track).flatten(),
            close: own(theme.window_buttons.close, &builtins, |t| {
                t.window_buttons.close
            })
            .flatten(),
            minimize: own(theme.window_buttons.minimize, &builtins, |t| {
                t.window_buttons.minimize
            })
            .flatten(),
            maximize: own(theme.window_buttons.maximize, &builtins, |t| {
                t.window_buttons.maximize
            })
            .flatten(),
            settings: own(theme.window_buttons.settings, &builtins, |t| {
                t.window_buttons.settings
            })
            .flatten(),
            on_top: own(theme.window_buttons.on_top, &builtins, |t| {
                t.window_buttons.on_top
            })
            .flatten(),
            new_tab: own(theme.window_buttons.new_tab, &builtins, |t| {
                t.window_buttons.new_tab
            })
            .flatten(),
            icon: own(theme.window_buttons.icon, &builtins, |t| {
                t.window_buttons.icon
            })
            .flatten(),
        }
    }
}

fn look_id() -> Id {
    Id::new("nit-skin-look")
}

/// Records the skin everything drawn after it is to wear. Kept in the
/// context, as the backdrop is, because the windows of their own - Settings -
/// are handed nothing of the theme.
pub fn set(ctx: &Context, look: Look) {
    ctx.data_mut(|d| d.insert_temp(look_id(), look));
}

/// The skin [`set`] last recorded.
pub fn look(ctx: &Context) -> Look {
    ctx.data(|d| d.get_temp::<Look>(look_id()))
        .unwrap_or_default()
}

/// Dresses `ctx` in `look`: records it, sets the skin's faces and text sizes,
/// recolours egui's own widgets to the skin's colours and markers, and makes
/// sure the end-of-frame swap is installed. Called after the theme's own
/// `Visuals` are set, since it works on top of them; with no skin it puts
/// egui's text sizes back and leaves the theme's visuals alone.
pub fn dress(ctx: &Context, look: Look) {
    set(ctx, look);
    post::install(ctx);
    crate::ui::fonts::set_skin(ctx, look.skin);
    let defaults = egui::Style::default();
    // Asked before `style_mut`, which holds the context's lock: asking
    // inside it waits on that lock for ever.
    let backdrop = crate::ui::shading::backdrop(ctx).is_some();
    ctx.style_mut(|style| {
        if look.skin == Skin::None {
            style.text_styles = defaults.text_styles.clone();
            style.spacing.button_padding = defaults.spacing.button_padding;
            style.animation_time = defaults.animation_time;
            return;
        }
        // No fades: a tooltip or a menu fading in is drawn with its fill at a
        // fraction of its alpha, which is not the marker `post` knows it by,
        // so for the length of the fade egui's own box showed - a flash of
        // another colour before the skin's. None of these desktops faded.
        style.animation_time = 0.0;
        let m = look.skin.style().metrics();
        let c = Kit::bare(look, 1.0).colors();
        let proportional = |size: f32| egui::FontId::new(size, egui::FontFamily::Proportional);
        use egui::TextStyle;
        style
            .text_styles
            .insert(TextStyle::Body, proportional(m.font_size));
        style
            .text_styles
            .insert(TextStyle::Button, proportional(m.font_size));
        // A face drawn on an 8 px grid - snes.css's - is only sharp at
        // whole multiples of it: small text stays at the body's size and a
        // heading doubles it, where any size between would blur every pixel.
        let grid = m.font_size <= 8.0;
        let small = if grid {
            m.font_size
        } else {
            (m.font_size - 2.0).max(9.0)
        };
        let heading = if grid {
            m.font_size * 2.0
        } else {
            (m.font_size * 1.5).round()
        };
        style
            .text_styles
            .insert(TextStyle::Small, proportional(small));
        style
            .text_styles
            .insert(TextStyle::Heading, proportional(heading));
        // A button as tall as the skin's: the text's row and the padding
        // round it make up the CSS minimum height.
        let row = (m.font_size * 1.2).round();
        style.spacing.button_padding =
            vec2(m.button_pad_x, ((m.button_min.y - row) / 2.0).max(1.0));
        let scroll = &mut style.spacing.scroll;
        scroll.floating = false;
        scroll.bar_width = m.scrollbar;
        scroll.bar_inner_margin = 0.0;
        scroll.bar_outer_margin = 0.0;
        scroll.foreground_color = false;
        // Not lengthened for the arrows: the swap draws the handle between
        // them at its share of the track, which shortens it with the rest.
        scroll.handle_min_length = 16.0;

        let v = &mut style.visuals;
        v.dark_mode = look.dark;
        // Aqua's window is one sheet of metal that the title bar, the tabs
        // and the status bar all show through to, as `aqua-ui` leaves them
        // `background: none`; a panel filled over it would cover it.
        // Nor over a theme's chrome gradient, which the window's body is.
        v.panel_fill = if look.skin == Skin::Aqua || backdrop {
            Color32::TRANSPARENT
        } else {
            c.face
        };
        v.faint_bg_color = c.face;
        v.code_bg_color = c.field;
        v.override_text_color = Some(c.text);
        v.selection.bg_fill = c.select;
        v.selection.stroke = egui::Stroke::new(1.0_f32, c.select_text);
        v.hyperlink_color = c.link;
        v.window_stroke = egui::Stroke::NONE;
        v.window_shadow = egui::epaint::Shadow::NONE;
        v.popup_shadow = egui::epaint::Shadow::NONE;
        v.window_rounding = Rounding::ZERO;
        v.menu_rounding = Rounding::ZERO;
        v.text_cursor.stroke.color = c.field_text;
        let ink = |w: &mut egui::style::WidgetVisuals| {
            w.bg_stroke = egui::Stroke::NONE;
            w.rounding = Rounding::ZERO;
            w.expansion = 0.0;
            w.fg_stroke.color = c.text;
        };
        ink(&mut v.widgets.inactive);
        ink(&mut v.widgets.hovered);
        ink(&mut v.widgets.active);
        ink(&mut v.widgets.open);
        // What separators are drawn in, and what disabled widgets fade to.
        v.widgets.noninteractive.bg_fill = c.face;
        v.widgets.noninteractive.weak_bg_fill = c.face;
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, c.subtle);
        v.widgets.noninteractive.fg_stroke.color = c.text;
        v.widgets.noninteractive.rounding = Rounding::ZERO;
        post::mark(v, look);
    });
}

/// A push button as the skin draws one, with `text` on it: at least the
/// skin's least size, its label pushed into the page while it is held, the
/// keyboard's dotted mark when it has the focus, and `default` drawn as the
/// button Enter presses.
pub fn button(ui: &mut egui::Ui, kit: &Kit, text: &str, default: bool) -> egui::Response {
    let style = kit.style();
    let m = style.metrics();
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, Color32::PLACEHOLDER);
    let min = m.button_min * kit.u();
    let size = vec2(
        (galley.size().x + 2.0 * m.button_pad_x * kit.u()).max(min.x),
        min.y.max(galley.size().y + 4.0),
    )
    .round();
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), text));
    if ui.is_rect_visible(rect) {
        let st = St {
            default,
            ..St::of(&response)
        }
        .disabled(!ui.is_enabled());
        let painter = ui.painter();
        painter.extend(style.button(kit, rect, st));
        let shift = if st.pressed && !st.disabled {
            style.pressed_shift() * kit.u()
        } else {
            0.0
        };
        let at = (rect.center() - galley.size() / 2.0 + vec2(shift, shift)).round();
        let c = kit.colors();
        if st.disabled {
            if let Some(emboss) = c.text_disabled_shadow {
                painter.galley(at + vec2(kit.u(), kit.u()), galley.clone(), emboss);
            }
        }
        painter.galley(at, galley, style.button_text(kit, st));
    }
    response
}

/// What a page - the body of a window, of a dialog - is filled with: the
/// skin's face, or nothing under Aqua, whose page is the window's sheet of
/// metal showing through.
pub fn page_fill(kit: &Kit) -> Color32 {
    if kit.look.skin == Skin::Aqua
        || kit
            .ctx
            .as_ref()
            .is_some_and(|c| crate::ui::shading::backdrop(c).is_some())
    {
        Color32::TRANSPARENT
    } else {
        kit.colors().face
    }
}

/// The faces `skin` sets the interface in.
pub fn fonts_of(skin: Skin) -> Fonts {
    skin.style().fonts()
}

/// What a control is doing, for the painters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct St {
    pub hovered: bool,
    /// Held down - CSS `:active`.
    pub pressed: bool,
    /// Has the keyboard.
    pub focused: bool,
    pub disabled: bool,
    /// Checked, selected, on, open.
    pub on: bool,
    /// The default button of a dialog, the one Enter presses.
    pub default: bool,
    /// The window it is in is the active one.
    pub inactive_window: bool,
}

impl St {
    /// The state of a widget from its response.
    pub fn of(response: &egui::Response) -> Self {
        St {
            hovered: response.hovered(),
            pressed: response.is_pointer_button_down_on(),
            focused: response.has_focus(),
            disabled: !response.sense.interactive() && !response.enabled(),
            ..St::default()
        }
    }

    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// The mark on a title-bar button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Glyph {
    Minimize,
    Maximize,
    Restore,
    Close,
    Help,
    /// The app's own: Settings.
    Gear,
    /// The app's own: keep the window on top, off and on.
    Pin,
    Pinned,
    /// The app's own: open a session.
    Plus,
}

/// Which way an arrow points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// The colours a skin draws text and fills in, for what the app paints
/// around the shapes - labels, titles, list rows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    /// A window's body, a dialog, a button's face.
    pub face: Color32,
    pub text: Color32,
    pub text_disabled: Color32,
    /// The emboss under disabled text, as 98 drew it; none elsewhere.
    pub text_disabled_shadow: Option<Color32>,
    /// Inside a text field or a list.
    pub field: Color32,
    pub field_text: Color32,
    /// A selected row of a list or a menu, and its text.
    pub select: Color32,
    pub select_text: Color32,
    pub title_text: Color32,
    pub title_text_inactive: Color32,
    pub title_shadow: Option<Color32>,
    pub link: Color32,
    pub tooltip_text: Color32,
    /// What faint text - a caption, a hint - is drawn in.
    pub subtle: Color32,
}

/// The sizes a skin's controls come in, in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// Body text.
    pub font_size: f32,
    /// The title bar's text.
    pub title_font_size: f32,
    pub title_height: f32,
    pub title_button: Vec2,
    /// Between two title-bar buttons, and before close.
    pub title_button_gap: f32,
    pub close_gap: f32,
    /// From a window's outer edge to its title bar and body: the border
    /// rings and the padding inside them.
    pub window_border: f32,
    /// A window's outer corners.
    pub window_radius: f32,
    pub scrollbar: f32,
    /// A scroll arrow's length along the bar; zero for a bar with none.
    pub scroll_arrow: f32,
    pub checkbox: f32,
    pub radio: f32,
    pub button_min: Vec2,
    pub button_pad_x: f32,
    pub field_height: f32,
    pub tab_height: f32,
    /// The width of a select's arrow button.
    pub select_arrow: f32,
    pub menu_item_height: f32,
    pub slider_thumb: Vec2,
    pub slider_track: f32,
    /// Between a checkbox and its label.
    pub label_gap: f32,
}

/// Where a font comes from.
#[derive(Clone, Copy, Debug)]
pub enum Face {
    /// Shipped with the app.
    Bundled(&'static [u8]),
    /// Looked for on the system, by family name, first found wins.
    System(&'static [&'static str]),
}

/// The faces a skin sets its interface in. `None` leaves egui's own.
#[derive(Clone, Copy, Debug, Default)]
pub struct Fonts {
    pub body: Option<Face>,
    pub bold: Option<Face>,
    /// The title bar, when it differs from bold body - XP's Trebuchet.
    pub title: Option<Face>,
}

/// One skin's painters.
///
/// Every method has a plain rendering built from [`Style::colors`], so a skin
/// only has to override what its library actually styles, and nothing a new
/// method adds goes undrawn in a skin that has not caught up with it.
pub trait Style: Sync {
    fn colors(&self, dark: bool) -> Colors;
    fn metrics(&self) -> Metrics;
    fn fonts(&self) -> Fonts {
        Fonts::default()
    }

    /// How far a button's label moves, right and down, while it is held - 98
    /// pushed it one pixel into the page.
    fn pressed_shift(&self) -> f32 {
        0.0
    }

    /// Whether a title bar's text is centred, as Aqua and the classic Mac OS
    /// set it, rather than at the left.
    fn title_centred(&self) -> bool {
        false
    }

    /// What sits behind the title's text, inside `text` - the classic Mac
    /// OS breaks its stripes for the title. Nothing elsewhere.
    fn title_backing(&self, k: &Kit, text: Rect, st: St) -> Vec<Shape> {
        let _ = (k, text, st);
        Vec::new()
    }

    /// The colour of a button's label.
    fn button_text(&self, k: &Kit, st: St) -> Color32 {
        let c = k.colors();
        if st.disabled {
            c.text_disabled
        } else {
            c.text
        }
    }

    /// How far a window's title bar sits from its top edge, in CSS pixels:
    /// the frame's own width unless the skin runs the bar to the edge.
    fn window_top(&self) -> f32 {
        self.metrics().window_border
    }

    /// How wide close is on the title bar, in CSS pixels - the others are
    /// `title_button`'s width.
    fn close_width(&self) -> f32 {
        self.metrics().title_button.x
    }

    /// A push button.
    fn button(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let c = k.colors();
        let fill_c = if st.pressed { c.select } else { c.face };
        vec![
            fill(rect, fill_c),
            outline(rect, Rounding::ZERO, c.subtle, k.ppp),
        ]
    }

    /// A single-line text field, or the frame of a multi-line one.
    fn field(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let c = k.colors();
        let ink = if st.focused { c.select } else { c.subtle };
        vec![
            fill(rect, if st.disabled { c.face } else { c.field }),
            outline(rect, Rounding::ZERO, ink, k.ppp),
        ]
    }

    /// A checkbox's box and, when on, its mark. `rect` is the box.
    fn checkbox(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let mut out = self.field(k, rect, st);
        if st.on {
            out.push(fill(rect.shrink(rect.width() * 0.3), k.colors().field_text));
        }
        out
    }

    /// A radio button's circle and dot. `rect` is the circle's box.
    fn radio(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let c = k.colors();
        let r = rect.width() / 2.0;
        let mut out = vec![
            Shape::circle_filled(rect.center(), r, c.field),
            Shape::circle_stroke(rect.center(), r - 0.5, egui::Stroke::new(1.0_f32, c.subtle)),
        ];
        if st.on {
            out.push(Shape::circle_filled(rect.center(), r * 0.35, c.field_text));
        }
        out
    }

    /// A closed drop-down: the field and its arrow button.
    fn select(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let mut out = self.field(k, rect, st);
        let m = self.metrics();
        let arrow = Rect::from_min_max(
            pos2(rect.right() - m.select_arrow * k.u(), rect.top()),
            rect.max,
        );
        out.extend(self.button(k, arrow, st));
        out.extend(self.arrow(k, arrow, Dir::Down, st));
        out
    }

    /// The text inside a closed drop-down, which some skins invert when it
    /// has the keyboard.
    fn select_text(&self, k: &Kit, st: St) -> Color32 {
        let c = k.colors();
        if st.disabled {
            c.text_disabled
        } else {
            c.field_text
        }
    }

    /// An arrow glyph centred in `rect` - a scroll button's, a select's.
    fn arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        let c = k.colors();
        let ink = if st.disabled { c.text_disabled } else { c.text };
        let rows: &[&str] = match dir {
            Dir::Up => &["...#...", "..###..", ".#####.", "#######"],
            Dir::Down => &["#######", ".#####.", "..###..", "...#..."],
            Dir::Left => &["...#", "..##", ".###", "####", ".###", "..##", "...#"],
            Dir::Right => &["#...", "##..", "###.", "####", "###.", "##..", "#..."],
        };
        let shift = if st.pressed { k.u() } else { 0.0 };
        prim::sprite(
            prim::centred(rect, rows, k.ppp) + vec2(shift, shift),
            rows,
            &[('#', ink)],
            k.ppp,
        )
    }

    /// A slider's track, along the middle of `rect`.
    fn slider_track(&self, k: &Kit, rect: Rect, _st: St) -> Vec<Shape> {
        let c = k.colors();
        let h = self.metrics().slider_track * k.u();
        let track = Rect::from_center_size(rect.center(), vec2(rect.width(), h));
        vec![fill(track, c.subtle)]
    }

    /// A slider's thumb, centred on `centre`.
    fn slider_thumb(&self, k: &Kit, centre: Pos2, st: St) -> Vec<Shape> {
        let size = self.metrics().slider_thumb * k.u();
        self.button(k, Rect::from_center_size(centre, size), st)
    }

    /// One tab of a strip; `st.on` is the selected one. `rect` is the tab as
    /// laid out - a selected tab may draw itself taller, over the panel's
    /// edge, as every Windows tab did.
    fn tab(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let c = k.colors();
        let mut out = vec![fill(rect, c.face)];
        out.push(outline(rect, Rounding::ZERO, c.subtle, k.ppp));
        if st.on {
            out.push(fill(
                Rect::from_min_max(pos2(rect.left(), rect.bottom() - k.u()), rect.max),
                c.face,
            ));
        }
        out
    }

    /// The colour of a tab's label; `st.on` is the selected one.
    fn tab_text(&self, k: &Kit, st: St) -> Color32 {
        let _ = st;
        k.colors().text
    }

    /// The panel a strip of tabs sits on.
    fn tab_panel(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let c = k.colors();
        vec![
            fill(rect, c.face),
            outline(rect, Rounding::ZERO, c.subtle, k.ppp),
        ]
    }

    /// A group box: its frame, broken at the top for `legend`.
    fn group(&self, k: &Kit, rect: Rect, legend: Option<Rect>) -> Vec<Shape> {
        let c = k.colors();
        let mut out = vec![outline(rect, Rounding::ZERO, c.subtle, k.ppp)];
        if let Some(legend) = legend {
            out.push(fill(legend.expand2(vec2(2.0 * k.u(), 0.0)), c.face));
        }
        out
    }

    /// A whole window: its frame and its body.
    fn window(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let _ = st;
        let c = k.colors();
        vec![
            fill(rect, c.face),
            outline(rect, Rounding::ZERO, c.subtle, k.ppp),
        ]
    }

    /// A title bar's background. `st.inactive_window` dims it.
    fn title_bar(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        let c = k.colors();
        vec![fill(
            rect,
            if st.inactive_window {
                c.subtle
            } else {
                c.select
            },
        )]
    }

    /// A button on the title bar, mark and all.
    fn title_button(&self, k: &Kit, rect: Rect, glyph: Glyph, st: St) -> Vec<Shape> {
        let mut out = self.button(k, rect, st);
        out.extend(title_glyph(k, rect, glyph, k.colors().text, st));
        out
    }

    /// The trough a scroll thumb runs in.
    fn scroll_track(&self, k: &Kit, rect: Rect, _vertical: bool, _st: St) -> Vec<Shape> {
        vec![fill(rect, k.colors().face)]
    }

    fn scroll_thumb(&self, k: &Kit, rect: Rect, _vertical: bool, st: St) -> Vec<Shape> {
        self.button(k, rect, st)
    }

    /// A scroll arrow's button, mark and all.
    fn scroll_arrow(&self, k: &Kit, rect: Rect, dir: Dir, st: St) -> Vec<Shape> {
        let mut out = self.button(k, rect, st);
        out.extend(self.arrow(k, rect, dir, st));
        out
    }

    /// Where a vertical and a horizontal bar meet.
    fn scroll_corner(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        vec![fill(rect, k.colors().face)]
    }

    /// A menu's or a pop-up's frame and background.
    fn menu(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        self.window(k, rect, St::default())
    }

    /// A menu row under the pointer, or a selected one.
    fn menu_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        if st.hovered || st.on {
            vec![fill(rect, k.colors().select)]
        } else {
            Vec::new()
        }
    }

    fn separator(&self, k: &Kit, rect: Rect, vertical: bool) -> Vec<Shape> {
        let u = k.u();
        let c = k.colors();
        let r = prim::snap_rect(rect, k.ppp);
        let line = if vertical {
            Rect::from_center_size(r.center(), vec2(u, r.height()))
        } else {
            Rect::from_center_size(r.center(), vec2(r.width(), u))
        };
        vec![fill(line, c.subtle)]
    }

    fn tooltip(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        let c = k.colors();
        vec![
            fill(rect, c.field),
            outline(rect, Rounding::ZERO, c.text, k.ppp),
        ]
    }

    /// A list or tree's sunken field.
    fn list(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        self.field(k, rect, St::default())
    }

    /// A list row; `st.on` is selected.
    fn list_item(&self, k: &Kit, rect: Rect, st: St) -> Vec<Shape> {
        if st.on {
            vec![fill(rect, k.colors().select)]
        } else if st.hovered {
            vec![fill(rect, k.colors().select.gamma_multiply(0.25))]
        } else {
            Vec::new()
        }
    }

    /// The text of a list row.
    fn list_text(&self, k: &Kit, st: St) -> Color32 {
        let c = k.colors();
        if st.on {
            c.select_text
        } else {
            c.field_text
        }
    }

    fn progress(&self, k: &Kit, rect: Rect, fraction: f32) -> Vec<Shape> {
        let mut out = self.field(k, rect, St::default());
        let inner = rect.shrink(2.0 * k.u());
        let done = Rect::from_min_size(
            inner.min,
            vec2(inner.width() * fraction.clamp(0.0, 1.0), inner.height()),
        );
        out.push(fill(done, k.colors().select));
        out
    }

    /// The strip along a window's foot.
    fn status_bar(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        vec![fill(rect, k.colors().face)]
    }

    /// One field of a status bar.
    fn status_field(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        vec![outline(rect, Rounding::ZERO, k.colors().subtle, k.ppp)]
    }

    /// The keyboard's mark on a control.
    fn focus(&self, k: &Kit, rect: Rect) -> Vec<Shape> {
        prim::dotted(rect, k.colors().text, k.ppp)
    }
}

/// The glyph of a title-bar button, drawn as small pixel art in `ink` - the
/// marks Windows drew, which every skin without marks of its own falls back
/// to.
pub fn title_glyph(k: &Kit, rect: Rect, glyph: Glyph, ink: Color32, st: St) -> Vec<Shape> {
    let rows: &[&str] = match glyph {
        Glyph::Minimize => &[
            "......", "......", "......", "......", "......", "......", "######", "######",
        ],
        Glyph::Maximize => &[
            "#########",
            "#########",
            "#.......#",
            "#.......#",
            "#.......#",
            "#.......#",
            "#.......#",
            "#.......#",
            "#########",
        ],
        Glyph::Restore => &[
            "..######", "..######", "..#....#", "######.#", "######.#", "#....###", "#....#..",
            "#....#..", "######..",
        ],
        Glyph::Close => &[
            "##....##", ".##..##.", "..####..", "...##...", "..####..", ".##..##.", "##....##",
        ],
        Glyph::Help => &[
            ".####.", "##..##", "....##", "...##.", "..##..", "..##..", "......", "..##..",
            "..##..",
        ],
        Glyph::Gear => &[
            "...#.#...",
            ".#.###.#.",
            "..#####..",
            "####.####",
            "###...###",
            "####.####",
            "..#####..",
            ".#.###.#.",
            "...#.#...",
        ],
        Glyph::Pin => &[
            "..#####..",
            "..#...#..",
            "..#...#..",
            "..#...#..",
            ".#######.",
            "....#....",
            "....#....",
            "....#....",
        ],
        Glyph::Pinned => &[
            "..#####..",
            "..#####..",
            "..#####..",
            "..#####..",
            ".#######.",
            "....#....",
            "....#....",
            "....#....",
        ],
        Glyph::Plus => &[
            "...##...", "...##...", "...##...", "########", "########", "...##...", "...##...",
            "...##...",
        ],
    };
    let shift = if st.pressed { k.u() } else { 0.0 };
    prim::sprite(
        prim::centred(rect, rows, k.ppp) + vec2(shift, shift),
        rows,
        &[('#', ink)],
        k.ppp,
    )
}

/// The look with nothing to say: what the plain renderings are built from.
/// Never drawn by the app, which keeps its own painting under no skin, but
/// it lets [`Skin::style`] be total.
struct Plain;

impl Style for Plain {
    fn colors(&self, dark: bool) -> Colors {
        let (face, text) = if dark {
            (Color32::from_gray(40), Color32::from_gray(220))
        } else {
            (Color32::from_gray(236), Color32::from_gray(20))
        };
        Colors {
            face,
            text,
            text_disabled: Color32::from_gray(128),
            text_disabled_shadow: None,
            field: if dark {
                Color32::from_gray(25)
            } else {
                Color32::WHITE
            },
            field_text: text,
            select: Color32::from_rgb(0x2a, 0x6a, 0xc8),
            select_text: Color32::WHITE,
            title_text: text,
            title_text_inactive: Color32::from_gray(128),
            title_shadow: None,
            link: Color32::from_rgb(0x2a, 0x6a, 0xc8),
            tooltip_text: text,
            subtle: Color32::from_gray(128),
        }
    }

    fn metrics(&self) -> Metrics {
        Metrics {
            font_size: 12.5,
            title_font_size: 12.5,
            title_height: 24.0,
            title_button: vec2(18.0, 18.0),
            title_button_gap: 2.0,
            close_gap: 2.0,
            window_border: 1.0,
            window_radius: 0.0,
            scrollbar: 12.0,
            scroll_arrow: 0.0,
            checkbox: 13.0,
            radio: 13.0,
            button_min: vec2(60.0, 22.0),
            button_pad_x: 10.0,
            field_height: 22.0,
            tab_height: 22.0,
            select_arrow: 18.0,
            menu_item_height: 20.0,
            slider_thumb: vec2(11.0, 19.0),
            slider_track: 4.0,
            label_gap: 6.0,
        }
    }
}

/// A skin, the scale it is drawn at, and what it needs from the context: the
/// handle the painters are called through.
#[derive(Clone)]
pub struct Kit {
    pub look: Look,
    pub ppp: f32,
    /// The 98 scroll track's dither, made once per context.
    pub dither: Option<TextureId>,
    /// The context the kit paints for, which keeps the skins' bitmaps; none
    /// for a bare kit, whose painters then fall back to drawing them.
    ctx: Option<Context>,
}

impl Kit {
    /// The kit for what `ctx` is drawing, or `None` when no skin is set - the
    /// signal every call site keeps its own painting on.
    pub fn of(ctx: &Context) -> Option<Kit> {
        let look = look(ctx);
        if look.skin == Skin::None {
            return None;
        }
        Some(Kit {
            look,
            ppp: ctx.pixels_per_point(),
            dither: Some(dither(ctx, look)),
            ctx: Some(ctx.clone()),
        })
    }

    /// The kit of another skin than the one `ctx` wears, in its reading and
    /// with `paint` - for drawing that skin's title-bar buttons on a window
    /// in another's controls.
    pub fn for_skin(ctx: &Context, skin: Skin, paint: Paint) -> Kit {
        let look = Look {
            skin,
            dark: ctx.style().visuals.dark_mode,
            paint,
        };
        Kit {
            look,
            ppp: ctx.pixels_per_point(),
            dither: Some(dither(ctx, look)),
            ctx: Some(ctx.clone()),
        }
    }

    /// This kit for a screen `by` times denser: what paints a control at
    /// `by` times its size, once its shapes are scaled back up by `by`.
    pub fn zoomed(&self, by: f32) -> Kit {
        Kit {
            ppp: self.ppp * by,
            ..self.clone()
        }
    }

    /// A kit for a given scale, with no textures - for tests and for painting
    /// shapes off a context.
    pub fn bare(look: Look, ppp: f32) -> Kit {
        Kit {
            look,
            ppp,
            dither: None,
            ctx: None,
        }
    }

    /// One of the skin's bitmaps as a texture, decoded from `png` the first
    /// time it is asked for and kept in the context after that. `None` with
    /// no context to keep it in, or when the bytes are not a PNG.
    pub fn image(&self, key: &'static str, png: &'static [u8]) -> Option<TextureId> {
        self.image_as(key, png, None, None)
    }

    /// [`Kit::image`] recoloured to `to` - a colour the user gave the
    /// control - its light and shade kept: see `tuned::retint`. Made once
    /// for each colour, and kept like the bitmap itself. `reference` is the
    /// lightness, 0 to 255, its pixels are measured against - the same for
    /// every slice of one control, so its caps and its middle stay one
    /// piece; `None` measures each bitmap against its own mean.
    pub fn image_as(
        &self,
        key: &'static str,
        png: &'static [u8],
        to: Option<Color32>,
        reference: Option<u8>,
    ) -> Option<TextureId> {
        let to = to.map(|to| (to, reference));
        self.texture_keyed((key, to), egui::TextureOptions::NEAREST, || {
            let image = image::load_from_memory(png).ok()?.into_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            let mut image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
            if let Some((to, reference)) = to {
                tuned::retint_pixels(&mut image.pixels, to, reference);
            }
            Some(image)
        })
    }

    /// A texture made by `make` the first time `key` is asked for - a
    /// pattern a skin repeats over a surface - and kept in the context.
    pub fn texture(
        &self,
        key: &'static str,
        options: egui::TextureOptions,
        make: impl FnOnce() -> Option<egui::ColorImage>,
    ) -> Option<TextureId> {
        self.texture_keyed(key, options, make)
    }

    fn texture_keyed(
        &self,
        key: impl std::hash::Hash + std::fmt::Debug,
        options: egui::TextureOptions,
        make: impl FnOnce() -> Option<egui::ColorImage>,
    ) -> Option<TextureId> {
        let ctx = self.ctx.as_ref()?;
        let id = Id::new(("nit-skin-texture", &key));
        if let Some(handle) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
            return Some(handle.id());
        }
        let handle = ctx.load_texture(format!("skin-{key:?}"), make()?, options);
        let texture = handle.id();
        ctx.data_mut(|d| d.insert_temp(id, handle));
        Some(texture)
    }

    /// The skin's painters, behind the cache and the recolouring of
    /// `tuned`.
    pub fn style(&self) -> &'static dyn Style {
        tuned::of(self.look.skin)
    }

    /// The skin's colours, with the user's laid over them: the face and the
    /// text, and the faint text and the disabled text mixed from those two
    /// so a caption stays between them; the selection, with text that reads
    /// on it when the theme names none.
    pub fn colors(&self) -> Colors {
        let mut c = self.look.skin.style().colors(self.look.dark);
        let p = self.look.paint;
        if p.face.is_some() || p.text.is_some() {
            let face = p.face.unwrap_or(c.face);
            // The skin's own ink on a face of the user's, while it reads; when
            // it would not, black or white, whichever does.
            let text = p.text.unwrap_or_else(|| {
                if crate::config::theme::contrast(c.text, face) >= 4.5 {
                    c.text
                } else if crate::config::theme::is_dark(face) {
                    Color32::WHITE
                } else {
                    Color32::BLACK
                }
            });
            c.face = face;
            c.text = text;
            c.subtle = text.lerp_to_gamma(face, 0.35);
            c.text_disabled = text.lerp_to_gamma(face, 0.55);
        }
        if let Some(select) = p.select {
            c.select = select;
            c.select_text = p.select_text.unwrap_or_else(|| {
                if crate::config::theme::is_dark(select) {
                    Color32::WHITE
                } else {
                    Color32::BLACK
                }
            });
        }
        c
    }

    pub fn metrics(&self) -> Metrics {
        self.style().metrics()
    }

    /// The colour the user gave the title-bar button showing `glyph`.
    pub fn button_colour(&self, glyph: Glyph) -> Option<Color32> {
        let p = self.look.paint;
        match glyph {
            Glyph::Close => p.close,
            Glyph::Minimize => p.minimize,
            Glyph::Maximize | Glyph::Restore => p.maximize,
            Glyph::Gear | Glyph::Help => p.settings,
            Glyph::Pin | Glyph::Pinned => p.on_top.or(p.settings),
            Glyph::Plus => p.new_tab,
        }
    }

    /// The ink of a title-bar button's mark: the user's, or the skin's
    /// `default`.
    pub fn glyph_ink(&self, default: Color32) -> Color32 {
        self.look.paint.icon.unwrap_or(default)
    }

    /// One CSS pixel, in points.
    pub fn u(&self) -> f32 {
        unit(self.ppp)
    }

    /// A metric in CSS pixels as points, on the pixel grid.
    pub fn px(&self, css: f32) -> f32 {
        css * self.u()
    }
}

fn dither_id(look: Look) -> Id {
    Id::new(("nit-skin-dither", look))
}

/// The checkerboard a Windows 98 scroll track is filled with: a 2x2 texture
/// repeated, one texel to a pixel. Made the first time it is asked for and
/// kept in the context, which owns the handle that keeps it alive.
fn dither(ctx: &Context, look: Look) -> TextureId {
    if let Some(handle) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(dither_id(look))) {
        return handle.id();
    }
    // A track colour of the user's is the dither's dark half, over its
    // light one mixed half-way to white - the checker 98 drew, in that colour.
    let (a, b) = match look.paint.track {
        Some(track) => (track, track.lerp_to_gamma(Color32::WHITE, 0.5)),
        None => win98::dither_colours(look.dark),
    };
    let image = egui::ColorImage {
        size: [2, 2],
        pixels: vec![a, b, b, a],
    };
    let handle = ctx.load_texture("skin-dither", image, egui::TextureOptions::NEAREST_REPEAT);
    let id = handle.id();
    ctx.data_mut(|d| d.insert_temp(dither_id(look), handle));
    id
}

/// The raised slab 98 draws everywhere, for skins that borrow it.
pub fn raised(
    rect: Rect,
    light: Color32,
    face_light: Color32,
    shadow: Color32,
    frame: Color32,
    ppp: f32,
) -> Vec<Shape> {
    inset(
        rect,
        &[
            (-1, -1, frame),
            (1, 1, light),
            (-2, -2, shadow),
            (2, 2, face_light),
        ],
        ppp,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skin_name_round_trips_and_an_unknown_one_is_none() {
        for skin in Skin::ALL {
            assert_eq!(Skin::from_name(skin.name()), skin);
        }
        assert_eq!(Skin::from_name("windows 3.1"), Skin::None);
        assert_eq!(Skin::from_name(""), Skin::None);
        assert_eq!(Skin::from_name(" Win98 "), Skin::Win98);
    }

    /// Every painter of every skin draws something, at every scale, without
    /// panicking - the plain fallbacks included.
    #[test]
    fn every_skin_paints_every_control() {
        let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(80.0, 24.0));
        for skin in Skin::ALL {
            for dark in [false, true] {
                // A whole and a fractional scale: the painters snap differently
                // at each, and every one more multiplies the rasters drawn.
                for ppp in [1.0, 1.25] {
                    let k = Kit::bare(
                        Look {
                            skin,
                            dark,
                            ..Look::default()
                        },
                        ppp,
                    );
                    let s = k.style();
                    let st = St::default();
                    let pressed = St {
                        pressed: true,
                        hovered: true,
                        ..St::default()
                    };
                    for st in [st, pressed, st.on(true), st.disabled(true)] {
                        assert!(!s.button(&k, rect, st).is_empty(), "{skin:?} button");
                        assert!(!s.field(&k, rect, st).is_empty(), "{skin:?} field");
                        assert!(!s.checkbox(&k, rect, st).is_empty(), "{skin:?} checkbox");
                        assert!(!s.radio(&k, rect, st).is_empty(), "{skin:?} radio");
                        assert!(!s.select(&k, rect, st).is_empty(), "{skin:?} select");
                        assert!(!s.tab(&k, rect, st).is_empty(), "{skin:?} tab");
                        assert!(!s.window(&k, rect, st).is_empty(), "{skin:?} window");
                        // A title bar may be nothing of its own - the window's
                        // surface showing through - so it is only drawn, not
                        // counted.
                        let _ = s.title_bar(&k, rect, st);
                        for glyph in [
                            Glyph::Minimize,
                            Glyph::Maximize,
                            Glyph::Restore,
                            Glyph::Close,
                            Glyph::Gear,
                            Glyph::Plus,
                            Glyph::Pin,
                            Glyph::Pinned,
                            Glyph::Help,
                        ] {
                            assert!(!s.title_button(&k, rect, glyph, st).is_empty());
                        }
                        for dir in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
                            assert!(!s.scroll_arrow(&k, rect, dir, st).is_empty());
                        }
                        assert!(!s.scroll_thumb(&k, rect, true, st).is_empty());
                        assert!(!s.scroll_track(&k, rect, true, st).is_empty());
                        assert!(!s.slider_thumb(&k, rect.center(), st).is_empty());
                        assert!(!s.slider_track(&k, rect, st).is_empty());
                        assert!(!s.menu(&k, rect).is_empty());
                        assert!(!s.tooltip(&k, rect).is_empty());
                        assert!(!s.group(&k, rect, None).is_empty());
                        assert!(!s.list(&k, rect).is_empty());
                        assert!(!s.progress(&k, rect, 0.5).is_empty());
                        assert!(!s.tab_panel(&k, rect).is_empty());
                    }
                }
            }
        }
    }

    /// The text a skin writes on its own fills has to be readable on them.
    #[test]
    fn every_skin_text_reads_on_its_own_face_field_and_selection() {
        use crate::config::theme::contrast;
        for skin in Skin::ALL {
            for dark in [false, true] {
                let c = skin.style().colors(dark);
                assert!(
                    contrast(c.text, c.face) >= 4.5,
                    "{skin:?} dark={dark} text/face"
                );
                assert!(
                    contrast(c.field_text, c.field) >= 4.5,
                    "{skin:?} dark={dark} field"
                );
                assert!(
                    contrast(c.select_text, c.select) >= 3.0,
                    "{skin:?} dark={dark} selection"
                );
            }
        }
    }

    /// A copy of a built-in theme left as it was keeps the skin's own
    /// colours; one whose colours were changed has them in every control.
    #[test]
    fn a_users_theme_colours_reach_the_skin_and_an_untouched_copy_does_not() {
        use crate::config::Theme;
        let files = crate::config::theme::builtin_files();
        let xp = files.iter().find(|f| f.name == "Windows XP").unwrap();
        let mut copy = Theme::from_file(xp);
        copy.builtin = false;
        let untouched = Look::of(&copy);
        assert_eq!(untouched.paint, Paint::default());
        assert_eq!(
            Kit::bare(untouched, 1.0).colors(),
            Skin::Xp.style().colors(false)
        );

        let green = Color32::from_rgb(0x2e, 0xa0, 0x43);
        copy.ui_background = Color32::from_rgb(0xdd, 0xee, 0xff);
        copy.ui_foreground = Color32::from_rgb(0x10, 0x20, 0x30);
        copy.ui_border = Some(green);
        copy.tab_selected = Some(Color32::from_rgb(0x80, 0x00, 0x00));
        let look = Look::of(&copy);
        let c = Kit::bare(look, 1.0).colors();
        assert_eq!(c.face, copy.ui_background);
        assert_eq!(c.text, copy.ui_foreground);
        assert_eq!(c.select, Color32::from_rgb(0x80, 0x00, 0x00));
        assert_eq!(c.select_text, Color32::WHITE);
        // The accent turns XP's blue title bar green.
        let k = Kit::bare(look, 1.0);
        let bar = k.style().title_bar(
            &k,
            Rect::from_min_size(Pos2::ZERO, vec2(200.0, 30.0)),
            St::default(),
        );
        let greenish = bar.iter().any(|s| match s {
            Shape::Mesh(m) => m.vertices.iter().any(|v| v.color.g() > v.color.b() + 40),
            Shape::Rect(r) => r.fill.g() > r.fill.b() + 40,
            _ => false,
        });
        assert!(greenish);
    }

    /// The scroll bar's and the window buttons' colours of a user's theme
    /// reach the controls: XP's thumb turns red, and so does Aqua's bitmap.
    #[test]
    fn a_users_scroll_and_window_button_colours_reach_the_controls() {
        use crate::config::Theme;
        let red = Color32::from_rgb(0xd0, 0x20, 0x20);
        let reddish = |shapes: &[Shape]| {
            shapes.iter().any(|s| match s {
                Shape::Mesh(m) => m.vertices.iter().any(|v| v.color.r() > v.color.b() + 60),
                Shape::Rect(r) => r.fill.r() > r.fill.b() + 60,
                _ => false,
            })
        };
        let files = crate::config::theme::builtin_files();
        let xp = files.iter().find(|f| f.name == "Windows XP").unwrap();
        let mut theme = Theme::from_file(xp);
        theme.builtin = false;
        theme.scrollbar_handle = Some(red);
        theme.window_buttons.close = Some(red);
        let ctx = Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            set(ctx, Look::of(&theme));
            let k = Kit::of(ctx).unwrap();
            let r = Rect::from_min_size(Pos2::ZERO, vec2(17.0, 80.0));
            assert!(reddish(&k.style().scroll_thumb(&k, r, true, St::default())));

            let aqua = files.iter().find(|f| f.name == "Tiger Aqua").unwrap();
            let mut theme = Theme::from_file(aqua);
            theme.builtin = false;
            set(ctx, Look::of(&theme));
            let plain = Kit::of(ctx).unwrap();
            let before = plain.style().scroll_thumb(&plain, r, true, St::default());
            theme.scrollbar_handle = Some(red);
            set(ctx, Look::of(&theme));
            let k = Kit::of(ctx).unwrap();
            let after = k.style().scroll_thumb(&k, r, true, St::default());
            let texture = |v: &[Shape]| match v.first() {
                Some(Shape::Rect(r)) => r.fill_texture_id,
                Some(Shape::Mesh(m)) => m.texture_id,
                _ => egui::TextureId::default(),
            };
            assert_ne!(texture(&before), texture(&after));
        });
    }

    /// A darker background on a copy of a light theme is the user's colour on
    /// the same controls, not the skin's dark reading; and the copy of a dark
    /// theme keeps its dark one with a light background.
    #[test]
    fn a_users_background_changes_the_face_not_the_skins_reading() {
        use crate::config::Theme;
        let files = crate::config::theme::builtin_files();
        for (name, dark, bg) in [
            ("Windows XP", false, Color32::from_rgb(0x30, 0x10, 0x40)),
            ("Windows XP Dark", true, Color32::from_rgb(0xf0, 0xe0, 0xff)),
        ] {
            let file = files.iter().find(|f| f.name == name).unwrap();
            let mut copy = Theme::from_file(file);
            copy.builtin = false;
            copy.ui_background = bg;
            let look = Look::of(&copy);
            assert_eq!(look.dark, dark, "{name}");
            let c = Kit::bare(look, 1.0).colors();
            assert_eq!(c.face, bg, "{name}");
            assert!(
                crate::config::theme::contrast(c.text, c.face) >= 4.5,
                "{name}"
            );
        }
    }

    /// Windows 7 is gone: a theme file that still names it opens as the
    /// app's own look rather than failing.
    #[test]
    fn a_theme_naming_the_removed_windows_7_skin_opens_unskinned() {
        assert_eq!(Skin::from_name("win7"), Skin::None);
        assert!(crate::config::theme::builtin_files()
            .iter()
            .all(|f| f.name != "Windows 7"));
    }

    #[test]
    fn the_raised_slab_has_four_rings() {
        let shapes = raised(
            Rect::from_min_size(Pos2::ZERO, vec2(20.0, 20.0)),
            Color32::WHITE,
            Color32::LIGHT_GRAY,
            Color32::GRAY,
            Color32::BLACK,
            1.0,
        );
        assert_eq!(shapes.len(), 8);
    }
}

#[cfg(test)]
mod bench {
    use super::*;

    #[test]
    #[ignore = "a stopwatch"]
    fn what_one_frame_of_each_skins_chrome_costs() {
        let ctx = Context::default();
        for skin in Skin::ALL {
            for dark in [false, true] {
                let look = Look {
                    skin,
                    dark,
                    ..Look::default()
                };
                let mut total = std::time::Duration::ZERO;
                let mut verts = 0;
                let mut parts = std::collections::BTreeMap::<&str, std::time::Duration>::new();
                for _ in 0..30 {
                    let _ = ctx.run(egui::RawInput::default(), |ctx| {
                        set(ctx, look);
                        let Some(k) = Kit::of(ctx) else { return };
                        let s = k.style();
                        let win = Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0));
                        let t0 = std::time::Instant::now();
                        let mut shapes = Vec::new();
                        let mut time =
                            |name: &'static str,
                             f: &mut dyn FnMut() -> Vec<Shape>,
                             shapes: &mut Vec<Shape>| {
                                let t = std::time::Instant::now();
                                shapes.extend(f());
                                *parts.entry(name).or_default() += t.elapsed();
                            };
                        let st = St::default();
                        time("window", &mut || s.window(&k, win, st), &mut shapes);
                        time(
                            "title",
                            &mut || {
                                s.title_bar(
                                    &k,
                                    Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 30.0)),
                                    st,
                                )
                            },
                            &mut shapes,
                        );
                        time(
                            "lights",
                            &mut || {
                                let mut v = Vec::new();
                                for i in 0..6 {
                                    v.extend(s.title_button(
                                        &k,
                                        Rect::from_min_size(
                                            pos2(10.0 + 20.0 * i as f32, 8.0),
                                            vec2(16.0, 16.0),
                                        ),
                                        Glyph::Close,
                                        st,
                                    ));
                                }
                                v
                            },
                            &mut shapes,
                        );
                        time(
                            "tabs",
                            &mut || {
                                let mut v = Vec::new();
                                for i in 0..5 {
                                    v.extend(s.tab(
                                        &k,
                                        Rect::from_min_size(
                                            pos2(10.0 + 120.0 * i as f32, 40.0),
                                            vec2(110.0, 24.0),
                                        ),
                                        st.on(i == 0),
                                    ));
                                }
                                v
                            },
                            &mut shapes,
                        );
                        time(
                            "scroll",
                            &mut || {
                                let mut v = s.scroll_track(
                                    &k,
                                    Rect::from_min_size(pos2(1180.0, 70.0), vec2(15.0, 700.0)),
                                    true,
                                    st,
                                );
                                v.extend(s.scroll_thumb(
                                    &k,
                                    Rect::from_min_size(pos2(1180.0, 200.0), vec2(15.0, 100.0)),
                                    true,
                                    st,
                                ));
                                v.extend(s.scroll_arrow(
                                    &k,
                                    Rect::from_min_size(pos2(1180.0, 770.0), vec2(15.0, 17.0)),
                                    Dir::Down,
                                    st,
                                ));
                                v
                            },
                            &mut shapes,
                        );
                        time(
                            "status",
                            &mut || {
                                s.status_bar(
                                    &k,
                                    Rect::from_min_size(pos2(0.0, 780.0), vec2(1200.0, 20.0)),
                                )
                            },
                            &mut shapes,
                        );
                        let t = std::time::Instant::now();
                        let mut tess = egui::epaint::Tessellator::new(
                            1.0,
                            Default::default(),
                            [2048, 2048],
                            vec![],
                        );
                        let mut n = 0;
                        for sh in shapes {
                            let mut mesh = egui::epaint::Mesh::default();
                            tess.tessellate_shape(sh, &mut mesh);
                            n += mesh.vertices.len();
                        }
                        *parts.entry("tessellate").or_default() += t.elapsed();
                        verts = n;
                        total += t0.elapsed();
                    });
                }
                println!(
                    "{skin:?} dark={dark}: {:?}/frame, {verts} vertices, {:?}",
                    total / 30,
                    parts.iter().map(|(k, v)| (k, *v / 30)).collect::<Vec<_>>()
                );
            }
        }
    }
}
