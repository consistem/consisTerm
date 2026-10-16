//! What the snapshot test films: Settings on its pages, egui's own widgets,
//! and every control of the current skin in every state.
//!
//! Run them all with
//!
//! ```text
//! CONSISTERM_SNAPSHOT_DIR=<folder> cargo test --lib film_every_scene -- --ignored --nocapture
//! ```
//!
//! narrowed with the variables the module documentation lists.

use egui::{pos2, vec2, Context, Rect, Vec2};

use super::{Focus, Keep, Look, Scene, Stage};
use crate::ui::settings_view::{Category, Route};
use crate::ui::skin::{Dir, Glyph, Kit, St};

/// Settings, opened on `category`.
pub(crate) fn settings(look: &Look, name: &'static str, category: Category) -> Scene {
    let mut settings = look.settings.clone();
    let mut themes = look.themes.clone();
    let mut groups = Vec::new();
    let mut state = crate::ui::panels::PanelState {
        show_settings: true,
        settings_route: Route::page(category),
        font_families: Some(Vec::new()),
        ..crate::ui::panels::PanelState::default()
    };
    let mut placement = crate::ui::detach::Placement::default();
    let buttons = look.theme.window_buttons;
    let look = look.clone();
    Scene {
        name,
        stage: Stage::new(Vec2::new(900.0, 640.0)).focus(Focus::Windows),
        cues: Vec::new(),
        keep: Keep::Window,
        draw: Box::new(move |ctx: &Context| {
            look.prelude(ctx);
            let _ = crate::ui::settings_view::settings_dialog(
                ctx,
                &mut settings,
                &mut themes,
                &mut groups,
                &mut state,
                &["IRIS".to_owned()],
                &crate::config::servers::ServerList::default(),
                &mut placement,
                &buttons,
            );
        }),
    }
}

/// Settings with a search typed into its field - the field with the
/// keyboard in it, and the results.
pub(crate) fn settings_search(look: &Look) -> Scene {
    let mut scene = settings(look, "settings-search", Category::General);
    scene.stage = scene.stage.frames(8);
    scene.cues = vec![
        super::Cue::at(2, super::Act::Click(super::Target::Text("Search"))),
        super::Cue::at(4, super::Act::Type("tray".to_owned())),
    ];
    scene
}

/// egui's own widgets, each in a few states: what a dialog is made of.
pub(crate) fn widgets(look: &Look) -> Scene {
    let look = look.clone();
    let mut text = String::from("some text");
    let mut checked = true;
    let mut value = 40.0_f32;
    let mut choice = 1;
    Scene {
        name: "widgets",
        stage: Stage::new(Vec2::new(520.0, 360.0)).frames(9),
        // The menu opened, and its second row under the pointer.
        cues: vec![
            super::Cue::at(2, super::Act::Click(super::Target::Text("Menu"))),
            super::Cue::at(6, super::Act::Hover(super::Target::Text("Paste"))),
        ],
        keep: Keep::Main,
        draw: Box::new(move |ctx: &Context| {
            look.prelude(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("Menu", |ui| {
                        let _ = ui.button("Copy");
                        let _ = ui.button("Paste");
                        ui.separator();
                        ui.add_enabled(false, egui::Button::new("Select all"));
                    });
                    let _ = ui.button("Button");
                    ui.add_enabled(false, egui::Button::new("Disabled"));
                    let _ = ui.selectable_label(true, "Selected");
                    let _ = ui.selectable_label(false, "Not selected");
                });
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut text).desired_width(160.0));
                    ui.checkbox(&mut checked, "Checkbox");
                });
                ui.add(egui::Slider::new(&mut value, 0.0..=100.0));
                ui.add(egui::DragValue::new(&mut value));
                egui::ComboBox::from_label("Combo")
                    .selected_text(format!("Option {choice}"))
                    .show_ui(ui, |ui| {
                        for i in 0..3 {
                            ui.selectable_value(&mut choice, i, format!("Option {i}"));
                        }
                    });
                ui.separator();
                ui.add(egui::ProgressBar::new(0.6));
                egui::ScrollArea::vertical()
                    .max_height(110.0)
                    .show(ui, |ui| {
                        for i in 0..20 {
                            ui.label(format!("Line {i} of a list that scrolls"));
                        }
                    });
            });
        }),
    }
}

/// A dialog over the window: a field and the two buttons every prompt ends
/// with, the answer drawn as the default one.
pub(crate) fn dialog(look: &Look) -> Scene {
    let look = look.clone();
    let mut text = String::from("zw");
    Scene {
        name: "dialog",
        stage: Stage::new(Vec2::new(520.0, 300.0)).frames(4),
        cues: Vec::new(),
        keep: Keep::Main,
        draw: Box::new(move |ctx: &Context| {
            look.prelude(ctx);
            egui::CentralPanel::default().show(ctx, |_| {});
            let mut open = true;
            crate::ui::dialog::show(ctx, "scene-dialog", "mtemp", &mut open, |ui| {
                ui.horizontal(|ui| {
                    ui.label("command");
                    ui.add(egui::TextEdit::singleline(&mut text).desired_width(260.0));
                });
                ui.label("Will send:");
                crate::ui::dialog::actions(ui, |ui| {
                    let _ =
                        crate::ui::dialog::button(ui, "Send", crate::ui::dialog::Role::Suggested);
                    let _ = crate::ui::dialog::button(ui, "Cancel", crate::ui::dialog::Role::Plain);
                });
            });
        }),
    }
}

/// Every control of the skin, laid out in rows by state - the picture to
/// hold beside a reference image of the CSS it reproduces.
pub(crate) fn gallery(look: &Look) -> Scene {
    let look = look.clone();
    Scene {
        name: "gallery",
        stage: Stage::new(Vec2::new(640.0, 560.0)),
        cues: Vec::new(),
        keep: Keep::Main,
        draw: Box::new(move |ctx: &Context| {
            look.prelude(ctx);
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(look.theme.ui_background))
                .show(ctx, |ui| {
                    let Some(kit) = Kit::of(ctx) else {
                        ui.label("no skin");
                        return;
                    };
                    paint_gallery(ui, &kit);
                });
        }),
    }
}

fn states() -> [(&'static str, St); 6] {
    [
        ("normal", St::default()),
        (
            "hover",
            St {
                hovered: true,
                ..St::default()
            },
        ),
        (
            "pressed",
            St {
                hovered: true,
                pressed: true,
                ..St::default()
            },
        ),
        (
            "focus",
            St {
                focused: true,
                ..St::default()
            },
        ),
        (
            "disabled",
            St {
                disabled: true,
                ..St::default()
            },
        ),
        (
            "default",
            St {
                default: true,
                ..St::default()
            },
        ),
    ]
}

fn paint_gallery(ui: &mut egui::Ui, k: &Kit) {
    let s = k.style();
    let m = k.metrics();
    let c = k.colors();
    let painter = ui.painter().clone();
    let origin = ui.max_rect().min + vec2(10.0, 10.0);
    let font = egui::FontId::proportional(m.font_size);
    let label = |at: egui::Pos2, text: &str, colour| {
        painter.text(at, egui::Align2::LEFT_CENTER, text, font.clone(), colour);
    };
    let u = k.u();
    let mut y = origin.y;

    // A window with its title bar and buttons, active and not.
    for inactive in [false, true] {
        let win = Rect::from_min_size(pos2(origin.x, y), vec2(300.0, 70.0));
        painter.extend(s.window(k, win, St::default()));
        let inset = m.window_border * u;
        let bar = Rect::from_min_size(
            win.min + vec2(inset, inset),
            vec2(win.width() - 2.0 * inset, m.title_height * u),
        );
        let st = St {
            inactive_window: inactive,
            ..St::default()
        };
        painter.extend(s.title_bar(k, bar, st));
        painter.text(
            pos2(bar.left() + 3.0 * u, bar.center().y),
            egui::Align2::LEFT_CENTER,
            "consisTerm",
            egui::FontId::new(m.title_font_size, crate::ui::fonts::skin_title(ui.ctx())),
            if inactive {
                c.title_text_inactive
            } else {
                c.title_text
            },
        );
        let size = m.title_button * u;
        let mut x = bar.right() - 2.0 * u;
        for glyph in [
            Glyph::Close,
            Glyph::Maximize,
            Glyph::Minimize,
            Glyph::Help,
            Glyph::Gear,
            Glyph::Plus,
        ] {
            let b = Rect::from_min_size(pos2(x - size.x, bar.center().y - size.y / 2.0), size);
            painter.extend(s.title_button(k, b, glyph, st));
            x -= size.x
                + if glyph == Glyph::Close {
                    m.close_gap * u
                } else {
                    m.title_button_gap * u
                };
        }
        let body = Rect::from_min_max(
            pos2(win.left() + inset + 4.0, bar.bottom() + 6.0),
            pos2(win.right() - inset - 4.0, win.bottom() - inset),
        );
        label(pos2(body.left(), body.top() + 8.0), "Window body", c.text);
        // Title buttons pressed and hovered beside it.
        let b = Rect::from_min_size(pos2(win.right() + 12.0, bar.top()), size);
        painter.extend(s.title_button(
            k,
            b,
            Glyph::Close,
            St {
                hovered: true,
                ..st
            },
        ));
        let b2 = Rect::from_min_size(pos2(b.right() + 8.0, bar.top()), size);
        painter.extend(s.title_button(
            k,
            b2,
            Glyph::Close,
            St {
                pressed: true,
                hovered: true,
                ..st
            },
        ));
        let b3 = Rect::from_min_size(pos2(b2.right() + 8.0, bar.top()), size);
        painter.extend(s.title_button(k, b3, Glyph::Restore, st));
        y += 80.0;
    }

    // Buttons, fields, selects in every state.
    let mut x = origin.x;
    for (name, st) in states() {
        let b = Rect::from_min_size(pos2(x, y), m.button_min * u);
        painter.extend(s.button(k, b, st));
        label(
            pos2(b.left() + 8.0, b.center().y),
            name,
            s.button_text(k, st),
        );
        let f = Rect::from_min_size(pos2(x, y + 30.0), vec2(m.button_min.x, m.field_height) * u);
        painter.extend(s.field(k, f, st));
        let sel = Rect::from_min_size(
            pos2(x, y + 58.0),
            vec2(m.button_min.x + 20.0, m.field_height) * u,
        );
        painter.extend(s.select(k, sel, st));
        x += (m.button_min.x + 26.0) * u;
    }
    y += 90.0;

    // Checkboxes and radios.
    let mut x = origin.x;
    for (_, st) in states() {
        for on in [false, true] {
            let b = Rect::from_min_size(pos2(x, y), Vec2::splat(m.checkbox * u));
            painter.extend(s.checkbox(k, b, st.on(on)));
            let r = Rect::from_min_size(pos2(x, y + 20.0), Vec2::splat(m.radio * u));
            painter.extend(s.radio(k, r, st.on(on)));
            x += 22.0;
        }
        x += 10.0;
    }
    y += 46.0;

    // Tabs on a panel, a group box, a slider, a progress bar.
    let panel = Rect::from_min_size(pos2(origin.x, y + m.tab_height * u), vec2(260.0, 60.0));
    painter.extend(s.tab_panel(k, panel));
    let mut tx = panel.left() + 3.0 * u;
    for (i, name) in ["General", "Advanced", "About"].into_iter().enumerate() {
        let w = 64.0;
        let t = Rect::from_min_size(pos2(tx, y), vec2(w, m.tab_height * u));
        let st = St {
            on: i == 0,
            hovered: i == 2,
            ..St::default()
        };
        painter.extend(s.tab(k, t, st));
        label(pos2(t.left() + 8.0, t.center().y), name, c.text);
        tx += w;
    }
    let group = Rect::from_min_size(pos2(origin.x + 280.0, y + 6.0), vec2(200.0, 70.0));
    let legend = Rect::from_min_size(
        pos2(group.left() + 8.0, group.top() - 6.0),
        vec2(60.0, 13.0),
    );
    painter.extend(s.group(k, group, Some(legend)));
    label(pos2(legend.left(), legend.center().y), "Group box", c.text);
    y += 90.0;

    let track = Rect::from_min_size(pos2(origin.x, y), vec2(200.0, 24.0));
    painter.extend(s.slider_track(k, track, St::default()));
    painter.extend(s.slider_thumb(
        k,
        pos2(track.left() + 60.0, track.center().y),
        St::default(),
    ));
    painter.extend(s.slider_thumb(
        k,
        pos2(track.left() + 140.0, track.center().y),
        St {
            pressed: true,
            ..St::default()
        },
    ));
    let bar = Rect::from_min_size(pos2(origin.x + 230.0, y), vec2(200.0, 22.0));
    painter.extend(s.progress(k, bar, 0.55));
    y += 40.0;

    // Scroll bars, both ways, with a corner.
    let sb = m.scrollbar * u;
    let arrow = m.scroll_arrow * u;
    let v = Rect::from_min_size(pos2(origin.x, y), vec2(sb, 140.0));
    painter.extend(s.scroll_track(k, v, true, St::default()));
    if arrow > 0.0 {
        painter.extend(s.scroll_arrow(
            k,
            Rect::from_min_size(v.min, vec2(sb, arrow)),
            Dir::Up,
            St::default(),
        ));
        painter.extend(s.scroll_arrow(
            k,
            Rect::from_min_size(pos2(v.left(), v.bottom() - arrow), vec2(sb, arrow)),
            Dir::Down,
            St {
                pressed: true,
                ..St::default()
            },
        ));
    }
    painter.extend(s.scroll_thumb(
        k,
        Rect::from_min_size(pos2(v.left(), v.top() + arrow + 20.0), vec2(sb, 40.0)),
        true,
        St::default(),
    ));
    let h = Rect::from_min_size(pos2(origin.x + 30.0, y + 140.0 - sb), vec2(200.0, sb));
    painter.extend(s.scroll_track(k, h, false, St::default()));
    if arrow > 0.0 {
        painter.extend(s.scroll_arrow(
            k,
            Rect::from_min_size(h.min, vec2(arrow, sb)),
            Dir::Left,
            St::default(),
        ));
        painter.extend(s.scroll_arrow(
            k,
            Rect::from_min_size(pos2(h.right() - arrow, h.top()), vec2(arrow, sb)),
            Dir::Right,
            St::default(),
        ));
    }
    painter.extend(s.scroll_thumb(
        k,
        Rect::from_min_size(pos2(h.left() + arrow + 30.0, h.top()), vec2(50.0, sb)),
        false,
        St {
            hovered: true,
            ..St::default()
        },
    ));
    painter.extend(s.scroll_corner(
        k,
        Rect::from_min_size(pos2(h.right(), h.top()), vec2(sb, sb)),
    ));

    // A menu with a highlighted row and a separator, a tooltip, a list.
    let menu = Rect::from_min_size(pos2(origin.x + 260.0, y), vec2(150.0, 80.0));
    painter.extend(s.menu(k, menu));
    let row = |i: f32| {
        Rect::from_min_size(
            pos2(
                menu.left() + 3.0 * u,
                menu.top() + 3.0 * u + i * m.menu_item_height * u,
            ),
            vec2(menu.width() - 6.0 * u, m.menu_item_height * u),
        )
    };
    painter.extend(s.menu_item(
        k,
        row(1.0),
        St {
            hovered: true,
            ..St::default()
        },
    ));
    label(
        pos2(row(0.0).left() + 18.0, row(0.0).center().y),
        "Copy",
        c.text,
    );
    label(
        pos2(row(1.0).left() + 18.0, row(1.0).center().y),
        "Paste",
        c.select_text,
    );
    painter.extend(s.separator(
        k,
        Rect::from_min_size(
            pos2(row(2.0).left(), row(2.0).top()),
            vec2(row(2.0).width(), 8.0 * u),
        ),
        false,
    ));
    label(
        pos2(row(2.0).left() + 18.0, row(2.0).center().y + 6.0),
        "Select all",
        c.text_disabled,
    );
    let tip = Rect::from_min_size(pos2(origin.x + 430.0, y), vec2(120.0, 20.0));
    painter.extend(s.tooltip(k, tip));
    label(
        pos2(tip.left() + 4.0, tip.center().y),
        "A tooltip",
        c.tooltip_text,
    );
    let list = Rect::from_min_size(pos2(origin.x + 430.0, y + 30.0), vec2(150.0, 90.0));
    painter.extend(s.list(k, list));
    for (i, name) in ["General", "Appearance", "Themes", "Terminal"]
        .into_iter()
        .enumerate()
    {
        let r = Rect::from_min_size(
            pos2(
                list.left() + 3.0 * u,
                list.top() + 3.0 * u + i as f32 * 18.0,
            ),
            vec2(list.width() - 6.0 * u, 18.0),
        );
        let st = St {
            on: i == 1,
            hovered: i == 2,
            ..St::default()
        };
        painter.extend(s.list_item(k, r, st));
        label(pos2(r.left() + 4.0, r.center().y), name, s.list_text(k, st));
    }
    // The status bar along the bottom.
    let foot = Rect::from_min_size(pos2(origin.x, y + 160.0), vec2(600.0, 22.0));
    painter.extend(s.status_bar(k, foot));
    let field = Rect::from_min_size(foot.min + vec2(2.0, 2.0), vec2(300.0, 18.0));
    painter.extend(s.status_field(k, field));
    label(pos2(field.left() + 4.0, field.center().y), "Ready", c.text);
}

/// Every scene, dressed in `look`.
pub(crate) fn all(look: &Look) -> Vec<Scene> {
    vec![
        gallery(look),
        widgets(look),
        dialog(look),
        main_window(look, "main", |_| {}, None),
        main_window_inactive(look),
        main_window(
            look,
            "main-tabs-in-title",
            |s| s.tabs_in_title_bar = true,
            Some("Connected to COMP80"),
        ),
        main_window(
            look,
            "main-title-scaled",
            |s| {
                s.tabs_in_title_bar = true;
                s.title_bar_scale = 2.0;
            },
            None,
        ),
        settings(look, "settings-general", Category::General),
        settings_search(look),
        settings(look, "settings-appearance", Category::Appearance),
        settings(look, "settings-themes", Category::Themes),
        settings(look, "settings-terminal", Category::Terminal),
        settings(look, "settings-about", Category::About),
    ]
}

/// Sets `ctx` up as the app sets up its own for `look`.
pub(crate) fn dress(ctx: &Context, look: &Look) {
    crate::app::App::apply_style(ctx, &look.theme, &look.settings);
}

#[test]
#[ignore = "writes pictures for a person to look at"]
fn film_every_scene() {
    let lang = match std::env::var("CONSISTERM_SNAPSHOT_LANG").as_deref() {
        Ok("pt-br") | Ok("pt") => crate::i18n::Lang::PtBr,
        _ => crate::i18n::Lang::En,
    };
    crate::i18n::set_language(lang);
    let themes: Vec<String> = super::listed("CONSISTERM_SNAPSHOT_THEMES").unwrap_or_else(|| {
        vec![
            "Windows 98".into(),
            "Windows XP".into(),
            "Windows XP Dark".into(),
            "Tiger Aqua".into(),
            "System".into(),
        ]
    });
    let themes: Vec<String> = if themes.iter().any(|t| t == "all") {
        crate::config::theme::builtin_files()
            .into_iter()
            .map(|f| f.name)
            .collect()
    } else {
        themes
    };
    let scales: Vec<f32> = super::listed("CONSISTERM_SNAPSHOT_SCALES")
        .map(|v| v.iter().filter_map(|s| s.parse().ok()).collect())
        .unwrap_or_else(|| vec![1.0, 2.0]);
    let only = super::listed("CONSISTERM_SNAPSHOT_SCENES");
    let out = super::out_dir();
    for name in themes {
        let mut look = Look::builtin(&name).unwrap_or_else(|| panic!("no built-in theme {name:?}"));
        // `CONSISTERM_SNAPSHOT_CUSTOM=1` films a copy of the theme the way a
        // user edits one - every chrome colour changed - to see each reach
        // the controls it should.
        let custom = std::env::var("CONSISTERM_SNAPSHOT_CUSTOM").is_ok();
        let name = if custom {
            look.customise();
            format!("{name} custom")
        } else {
            name
        };
        for &ppp in &scales {
            for scene in all(&look) {
                if let Some(only) = &only {
                    if !only.iter().any(|o| scene.name.contains(o.as_str())) {
                        continue;
                    }
                }
                let dir = out.join(super::slug(&name)).join(super::scale_dir(ppp));
                let look = look.clone();
                match super::film(scene, ppp, move |ctx| dress(ctx, &look), &dir) {
                    Ok(files) => {
                        for f in files {
                            println!("{}", f.display());
                        }
                    }
                    Err(why) => println!("{why}"),
                }
            }
        }
    }
}

/// What a session looks like in the main window's scenes: the prompt, a
/// command and its answer, as in the screenshots this skin work began from.
const SESSION: &str = "N\u{f3}: CCSDESDSK095, Configura\u{e7}\u{e3}o: CONSISTEM\n\nCOMP80>w \"Duplica a partir daqui\"\nDuplica a partir daqui\nCOMP80>\n\nCOMP80>w \"ultima linha\"\nultima linha\nCOMP80>";

/// The main window in the background, for the title bar a skin draws when
/// the window does not have the keyboard.
pub(crate) fn main_window_inactive(look: &Look) -> Scene {
    let mut scene = main_window(look, "main-inactive", |_| {}, None);
    scene.stage = scene.stage.focus(Focus::Nothing);
    scene
}

/// The main window - title bar, tabs, terminal, footer - drawn by the app
/// itself, with two tabs that have no session behind them.
pub(crate) fn main_window(
    look: &Look,
    name: &'static str,
    configure: fn(&mut crate::config::Settings),
    status: Option<&'static str>,
) -> Scene {
    let mut settings = look.settings.clone();
    configure(&mut settings);
    let themes = look.themes.clone();
    let look = look.clone();
    let mut app: Option<crate::app::App> = None;
    Scene {
        name,
        stage: Stage::new(Vec2::new(760.0, 420.0)),
        cues: Vec::new(),
        keep: Keep::Main,
        draw: Box::new(move |ctx: &Context| {
            look.prelude(ctx);
            let app = app.get_or_insert_with(|| {
                let mut app = crate::app::App::for_snapshot(ctx, settings.clone(), themes.clone());
                app.add_snapshot_tab("CONSISTEM | COMP80", SESSION);
                app.add_snapshot_tab("RDB76-TR", "USER>");
                if let Some(status) = status {
                    app.snapshot_status(status);
                }
                app
            });
            app.frame(ctx);
        }),
    }
}
