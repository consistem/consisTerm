//! A camera for egui: frames run without a window, and what they draw is
//! painted into PNG files the way `egui_glow` would have put it on screen.
//! For looking at a theme - and comparing it with a picture of what it
//! imitates - without opening the app. Test-only.
//!
//! [`shoot`] runs any `FnMut(&egui::Context)` for a few frames and paints the
//! last of them, once for every window it drew: the main window, and each
//! window of its own the frame asked for, which is how eframe shows Settings.
//! Input is scripted with [`Cue`]s aimed at the words on screen rather than at
//! coordinates, so a script outlives a change of layout.
//!
//! The scenes are in [`scenes`] - the main window, Settings, egui's own
//! widgets and every control of the skin - with the test that films them all,
//! which dresses each context exactly as the app dresses its own:
//!
//! ```text
//! CONSISTERM_SNAPSHOT_DIR=<folder> CONSISTERM_CONFIG_DIR=<scratch folder> \
//!     cargo test --lib film_every_scene -- --ignored --nocapture
//! ```
//!
//! `CONSISTERM_SNAPSHOT_THEMES` (built-in names, comma-separated, or `all`),
//! `CONSISTERM_SNAPSHOT_SCALES` (pixels per point, `1,2` by default),
//! `CONSISTERM_SNAPSHOT_SCENES` (parts of scene names) and
//! `CONSISTERM_SNAPSHOT_LANG` (`en` or `pt-br`) narrow it down. The pictures
//! go to `<folder>/<theme>/<scale>x/<scene>.png`.
//!
//! Run it on its own: it sets the interface language, which every other test
//! reading `tr` would race with.

mod raster;
pub(crate) mod scenes;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use egui::epaint::ClippedShape;
use egui::{
    Context, Event, FullOutput, ImmediateViewport, Modifiers, PointerButton, Pos2, RawInput, Rect,
    Shape, Vec2, ViewportBuilder, ViewportId, ViewportIdMap, ViewportIdPair, ViewportInfo,
};

use crate::config::{Settings, Theme};

pub(crate) use raster::{Canvas, Textures};

/// What a desktop GPU reports as its largest texture, which is what decides
/// how wide egui makes the font atlas. Any value from 8192 up gives the same
/// atlas.
const MAX_TEXTURE_SIDE: usize = 16384;

/// Where the main window sits on the monitor. Only the windows' positions
/// relative to each other matter, and nothing here is near an edge.
const MAIN_AT: Pos2 = Pos2::new(0.0, 0.0);

/// How a scene is filmed, whatever the scale.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Stage {
    /// The main window's inner size, in points.
    pub size: Vec2,
    /// Frames run before the picture is taken. egui settles a layout over a
    /// few of them - much of it is measured on the frame before - and a click
    /// is answered on the frame after it.
    pub frames: usize,
    /// Seconds between two frames. Long enough that an animation started on
    /// one frame has finished by the next, and that a pointer left still is
    /// still long enough for a tooltip; short enough that a press and the
    /// release on the next frame are still a click.
    pub step: f64,
    /// Which window has the keyboard, which is what an active title bar is
    /// drawn for.
    pub focus: Focus,
}

impl Stage {
    /// A main window the size of a small laptop's, filmed over six frames.
    pub(crate) fn new(size: Vec2) -> Self {
        Stage {
            size,
            frames: 6,
            step: 0.25,
            focus: Focus::Main,
        }
    }

    pub(crate) fn frames(mut self, frames: usize) -> Self {
        self.frames = frames;
        self
    }

    pub(crate) fn focus(mut self, focus: Focus) -> Self {
        self.focus = focus;
        self
    }
}

/// Which window is the active one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Focus {
    /// The main window.
    Main,
    /// The windows of their own - Settings - and not the main one.
    Windows,
    /// Neither: the app is in the background.
    Nothing,
}

/// Something the pointer or the keyboard does, on a given frame.
#[derive(Clone, Debug)]
pub(crate) struct Cue {
    pub frame: usize,
    pub act: Act,
}

impl Cue {
    pub(crate) fn at(frame: usize, act: Act) -> Self {
        Cue { frame, act }
    }
}

/// What a [`Cue`] does.
#[derive(Clone, Debug)]
pub(crate) enum Act {
    /// The pointer comes to rest over the target, and stays.
    Hover(Target),
    /// The primary button, pressed on the cue's frame and released on the
    /// next.
    Click(Target),
    /// Text typed into the window with the keyboard.
    Type(String),
}

/// Where a pointer cue lands.
#[derive(Clone, Debug)]
pub(crate) enum Target {
    /// The middle of a text on screen on the frame before: one that reads
    /// exactly this if there is one, and otherwise one that contains it. Of
    /// several, a window of its own wins over the main window and the last
    /// drawn over the rest, since that is the one on top.
    Text(&'static str),
}

/// One window's picture.
pub(crate) struct Shot {
    /// `None` for the main window; the title of any other.
    pub window: Option<String>,
    pub image: image::RgbaImage,
    /// Whatever could not be painted, which leaves a hole in the picture.
    pub missed: Vec<String>,
}

/// Which windows' pictures a scene keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keep {
    /// The main window.
    Main,
    /// The window of its own the scene opened, such as Settings.
    Window,
    /// All of them.
    Every,
}

/// A theme, and the settings around it: what a scene is dressed in.
#[derive(Clone)]
pub(crate) struct Look {
    pub theme: Theme,
    /// Every built-in, as a first start has them, so the gallery shows them
    /// all.
    pub themes: Vec<Theme>,
    /// The defaults, with this theme chosen.
    pub settings: Settings,
}

impl Look {
    /// The built-in theme called `name`, if there is one.
    pub(crate) fn builtin(name: &str) -> Option<Self> {
        let themes: Vec<Theme> = crate::config::theme::builtin_files()
            .iter()
            .map(|file| Theme::from_file(file).as_builtin())
            .collect();
        let theme = themes.iter().find(|theme| theme.name == name)?.clone();
        let settings = Settings {
            theme: theme.name.clone(),
            ..Settings::default()
        };
        Some(Look {
            theme,
            themes,
            settings,
        })
    }

    /// Turns the theme into a copy of the user's with every chrome colour
    /// changed, chosen in the settings as a first start would have it.
    pub(crate) fn customise(&mut self) {
        let c = |r, g, b| egui::Color32::from_rgb(r, g, b);
        let t = &mut self.theme;
        t.name = format!("{} custom", t.name);
        t.builtin = false;
        t.ui_background = c(0x6a, 0x3d, 0x9a);
        t.ui_foreground = c(0xff, 0xf0, 0xa0);
        t.ui_border = Some(c(0xe0, 0x80, 0x20));
        t.tab_selected = Some(c(0x2e, 0x8b, 0x57));
        t.tab_selected_text = Some(c(0xff, 0xff, 0xff));
        t.scrollbar_handle = Some(c(0xd0, 0x20, 0x20));
        t.scrollbar_track = Some(c(0x30, 0x30, 0x60));
        t.window_buttons.close = Some(c(0x20, 0xa0, 0xd0));
        t.window_buttons.minimize = Some(c(0xd0, 0x40, 0xc0));
        t.window_buttons.maximize = Some(c(0xe0, 0xe0, 0x20));
        t.window_buttons.settings = Some(c(0x40, 0xd0, 0x60));
        t.window_buttons.icon = Some(c(0x10, 0x10, 0x10));
        // `CONSISTERM_SNAPSHOT_GLASS=<0..1>` gives the copy a chrome gradient
        // and that much window glass over it.
        if let Some(glass) = std::env::var("CONSISTERM_SNAPSHOT_GLASS")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
        {
            t.ui_gradient = Some(crate::config::theme::UiGradient {
                direction: crate::config::theme::GradientDirection::Diagonal,
                from: c(0xd0, 0x40, 0x40),
                to: c(0x20, 0x40, 0xa0),
            });
            t.ui_glass = Some(glass);
        }
        self.themes.push(t.clone());
        self.settings.theme = t.name.clone();
    }

    /// What the frame loop does to the context before it draws anything -
    /// the interface zoom, the glass over the backdrop, and the backdrop -
    /// in step with `App::update`.
    pub(crate) fn prelude(&self, ctx: &Context) {
        let scale = if self.settings.ui_scale.is_finite() {
            self.settings
                .ui_scale
                .clamp(1.0, crate::ui::settings_view::UI_SCALE_MAX as f32 / 100.0)
        } else {
            1.0
        };
        if (ctx.zoom_factor() - scale).abs() > f32::EPSILON {
            ctx.set_zoom_factor(scale);
        }
        let glass = self.theme.ui_glass.unwrap_or(self.settings.sheet_opacity);
        crate::ui::prefs::set_opacity(ctx, glass);
        crate::ui::shading::paint_backdrop(ctx);
    }
}

/// Something to film: a stage, a script, and what to draw on every frame.
pub(crate) struct Scene {
    /// The file the picture is written to, less `.png`.
    pub name: &'static str,
    pub stage: Stage,
    pub cues: Vec<Cue>,
    pub keep: Keep,
    pub draw: Box<dyn FnMut(&Context)>,
}

/// Runs `draw` for `stage.frames` frames at `pixels_per_point`, playing
/// `cues`, and returns the last frame of every window it drew whose picture
/// `keep` asks for. `dress` is handed the fresh context first, to set it up
/// the way the app sets up its own.
pub(crate) fn shoot(
    stage: &Stage,
    pixels_per_point: f32,
    cues: &[Cue],
    keep: Keep,
    dress: impl FnOnce(&Context),
    mut draw: impl FnMut(&Context),
) -> Vec<Shot> {
    let ctx = Context::default();
    // What eframe sets wherever it can open windows: a window of its own is
    // then one, drawn by the renderer below, rather than a window drawn inside
    // the main one.
    ctx.set_embed_viewports(false);
    dress(&ctx);
    let studio = Rc::new(RefCell::new(Studio::new(*stage, pixels_per_point, keep)));
    STUDIO.with(|current| *current.borrow_mut() = Some(studio.clone()));
    Context::set_immediate_viewport_renderer(render_window);
    for frame in 0..stage.frames {
        studio.borrow_mut().begin(frame, cues);
        let input = studio.borrow_mut().main_input();
        let output = ctx.run(input, &mut draw);
        studio
            .borrow_mut()
            .develop(&ctx, ViewportId::ROOT, None, output);
    }
    STUDIO.with(|current| *current.borrow_mut() = None);
    let shots = std::mem::take(&mut studio.borrow_mut().shots);
    shots
}

thread_local! {
    /// The shoot in progress on this thread, which the renderer egui calls
    /// for a window of its own films into. egui keeps that renderer per
    /// thread too.
    static STUDIO: RefCell<Option<Rc<RefCell<Studio>>>> = const { RefCell::new(None) };
}

/// What eframe does for a window of its own, minus the window: runs its
/// frame with input of its own, then paints it.
fn render_window(ctx: &Context, viewport: ImmediateViewport<'_>) {
    let ImmediateViewport {
        ids,
        builder,
        viewport_ui_cb,
    } = viewport;
    let Some(studio) = STUDIO.with(|current| current.borrow().clone()) else {
        // Nothing filming: the frame still has to run, or the code that asked
        // for the window would wait on it forever.
        let input = RawInput {
            viewport_id: ids.this,
            ..RawInput::default()
        };
        let _ = ctx.run(input, |ctx| viewport_ui_cb(ctx));
        return;
    };
    let input = studio.borrow_mut().window_input(ids, &builder);
    let output = ctx.run(input, |ctx| viewport_ui_cb(ctx));
    studio
        .borrow_mut()
        .develop(ctx, ids.this, builder.title.clone(), output);
}

/// Everything a shoot keeps between frames and across its windows.
struct Studio {
    stage: Stage,
    pixels_per_point: f32,
    keep: Keep,
    frame: usize,
    textures: Textures,
    /// Every window, as the platform describes it to egui: size, place,
    /// scale, focus.
    infos: ViewportIdMap<ViewportInfo>,
    /// Each window's framebuffer, in physical pixels.
    pixels: HashMap<ViewportId, [usize; 2]>,
    /// Events for the next frame of each window.
    queued: HashMap<ViewportId, Vec<Event>>,
    /// Events for a later frame - the release of a click.
    later: Vec<(usize, ViewportId, Event)>,
    /// The texts each window drew on its last frame, and where: what a cue
    /// aims at. Windows in the order they were first drawn.
    words: Vec<(ViewportId, Vec<(String, Rect)>)>,
    shots: Vec<Shot>,
}

impl Studio {
    fn new(stage: Stage, pixels_per_point: f32, keep: Keep) -> Self {
        let mut studio = Studio {
            stage,
            pixels_per_point,
            keep,
            frame: 0,
            textures: Textures::default(),
            infos: ViewportIdMap::default(),
            pixels: HashMap::new(),
            queued: HashMap::new(),
            later: Vec::new(),
            words: Vec::new(),
            shots: Vec::new(),
        };
        let size = studio.place(ViewportId::ROOT, MAIN_AT, stage.size);
        let info = ViewportInfo {
            title: Some(crate::APP_NAME.to_owned()),
            focused: Some(stage.focus == Focus::Main),
            ..studio.window_info(None, MAIN_AT, size)
        };
        studio.infos.insert(ViewportId::ROOT, info);
        studio
    }

    /// Records a window's framebuffer for `size` points - a whole number of
    /// pixels, as a real window has - and returns the size in points egui is
    /// told, which that whole number of pixels makes.
    fn place(&mut self, id: ViewportId, _at: Pos2, size: Vec2) -> Vec2 {
        let ppp = self.pixels_per_point;
        let width = (size.x * ppp).round().max(1.0) as usize;
        let height = (size.y * ppp).round().max(1.0) as usize;
        self.pixels.insert(id, [width, height]);
        Vec2::new(width as f32 / ppp, height as f32 / ppp)
    }

    /// A window without decorations: its outer rectangle is its inner one.
    fn window_info(&self, parent: Option<ViewportId>, at: Pos2, size: Vec2) -> ViewportInfo {
        let rect = Rect::from_min_size(at, size);
        ViewportInfo {
            parent,
            native_pixels_per_point: Some(self.pixels_per_point),
            monitor_size: Some(Vec2::new(3840.0, 2160.0) / self.pixels_per_point),
            inner_rect: Some(rect),
            outer_rect: Some(rect),
            minimized: Some(false),
            maximized: Some(false),
            fullscreen: Some(false),
            ..ViewportInfo::default()
        }
    }

    fn time(&self) -> f64 {
        self.frame as f64 * self.stage.step
    }

    /// Plays the cues due on `frame`, and the events left over from earlier
    /// ones.
    fn begin(&mut self, frame: usize, cues: &[Cue]) {
        self.frame = frame;
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.later)
            .into_iter()
            .partition(|(at, _, _)| *at <= frame);
        self.later = later;
        for (_, viewport, event) in due {
            self.queued.entry(viewport).or_default().push(event);
        }
        for cue in cues.iter().filter(|cue| cue.frame == frame) {
            self.play(&cue.act);
        }
    }

    fn play(&mut self, act: &Act) {
        let none = Modifiers::NONE;
        let button = |pos, button, pressed| Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: none,
        };
        match act {
            Act::Hover(target) => {
                let (viewport, pos) = self.find(target);
                self.send(viewport, Event::PointerMoved(pos));
            }
            Act::Click(target) => {
                let which = PointerButton::Primary;
                let (viewport, pos) = self.find(target);
                self.send(viewport, Event::PointerMoved(pos));
                self.send(viewport, button(pos, which, true));
                self.later
                    .push((self.frame + 1, viewport, button(pos, which, false)));
            }
            Act::Type(text) => {
                let viewport = self.keyboard();
                self.send(viewport, Event::Text(text.clone()));
            }
        }
    }

    fn send(&mut self, viewport: ViewportId, event: Event) {
        self.queued.entry(viewport).or_default().push(event);
    }

    /// The window the keyboard is in.
    fn keyboard(&self) -> ViewportId {
        match self.stage.focus {
            Focus::Windows => self
                .words
                .iter()
                .map(|(viewport, _)| *viewport)
                .find(|viewport| *viewport != ViewportId::ROOT)
                .unwrap_or(ViewportId::ROOT),
            Focus::Main | Focus::Nothing => ViewportId::ROOT,
        }
    }

    /// Where `target` is, and in which window.
    fn find(&self, target: &Target) -> (ViewportId, Pos2) {
        let Target::Text(wanted) = target;
        let wanted = *wanted;
        // Windows of their own first, being on top of the main one; within a
        // window, the last drawn.
        let windows = self
            .words
            .iter()
            .filter(|(viewport, _)| *viewport != ViewportId::ROOT)
            .chain(
                self.words
                    .iter()
                    .filter(|(viewport, _)| *viewport == ViewportId::ROOT),
            );
        let candidates: Vec<(ViewportId, &str, Rect)> = windows
            .flat_map(|(viewport, words)| {
                words
                    .iter()
                    .rev()
                    .map(|(text, rect)| (*viewport, text.as_str(), *rect))
            })
            .collect();
        let exact = candidates.iter().find(|(_, text, _)| text.trim() == wanted);
        let within = || candidates.iter().find(|(_, text, _)| text.contains(wanted));
        match exact.or_else(within) {
            Some((viewport, _, rect)) => (*viewport, rect.center()),
            None => {
                let seen: Vec<&str> = candidates.iter().map(|(_, text, _)| *text).collect();
                panic!(
                    "frame {}: no text {wanted:?} on screen to aim at. On screen: {seen:?}",
                    self.frame
                );
            }
        }
    }

    fn input_for(&mut self, viewport: ViewportId, size: Vec2) -> RawInput {
        RawInput {
            viewport_id: viewport,
            viewports: self.infos.clone(),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            max_texture_side: Some(MAX_TEXTURE_SIDE),
            time: Some(self.time()),
            predicted_dt: self.stage.step as f32,
            events: self.queued.remove(&viewport).unwrap_or_default(),
            focused: self
                .infos
                .get(&viewport)
                .and_then(|info| info.focused)
                .unwrap_or(false),
            ..RawInput::default()
        }
    }

    fn main_input(&mut self) -> RawInput {
        let size = self.points(ViewportId::ROOT);
        self.input_for(ViewportId::ROOT, size)
    }

    /// The input for a window of its own, placed where the builder asks or,
    /// failing that, in the middle of the main window - where
    /// `detach::shell` opens Settings.
    fn window_input(&mut self, ids: ViewportIdPair, builder: &ViewportBuilder) -> RawInput {
        let wanted = builder.inner_size.unwrap_or(Vec2::new(800.0, 600.0));
        let main = self.points(ViewportId::ROOT);
        let at = builder.position.unwrap_or(MAIN_AT + (main - wanted) / 2.0);
        let size = self.place(ids.this, at, wanted);
        let info = ViewportInfo {
            title: builder.title.clone(),
            focused: Some(self.stage.focus == Focus::Windows),
            ..self.window_info(Some(ids.parent), at, size)
        };
        self.infos.insert(ids.this, info);
        self.input_for(ids.this, size)
    }

    /// A window's size in points, as last placed.
    fn points(&self, viewport: ViewportId) -> Vec2 {
        let [width, height] = self.pixels[&viewport];
        Vec2::new(width as f32, height as f32) / self.pixels_per_point
    }

    /// Takes in what a window's frame produced: its textures, its words, and
    /// on the last frame, its picture.
    fn develop(
        &mut self,
        ctx: &Context,
        viewport: ViewportId,
        title: Option<String>,
        output: FullOutput,
    ) {
        self.textures.set(&output.textures_delta);
        let words = words(&output.shapes);
        match self.words.iter_mut().find(|(id, _)| *id == viewport) {
            Some((_, seen)) => *seen = words,
            None => self.words.push((viewport, words)),
        }
        let main = viewport == ViewportId::ROOT;
        let wanted = match self.keep {
            Keep::Main => main,
            Keep::Window => !main,
            Keep::Every => true,
        };
        if wanted && self.frame + 1 == self.stage.frames {
            let [width, height] = self.pixels[&viewport];
            // eframe clears the main window to its `App::clear_color`, which
            // the app leaves at eframe's own, and a window of its own to
            // nothing at all. Only a gap in what egui paints would show it.
            let clear = if main {
                egui::Color32::from_rgba_unmultiplied(12, 12, 12, 180).to_normalized_gamma_f32()
            } else {
                [0.0; 4]
            };
            let mut canvas = Canvas::new(width, height, clear);
            let ppp = output.pixels_per_point;
            let primitives = ctx.tessellate(output.shapes, ppp);
            let missed = canvas.paint(&primitives, ppp, &self.textures);
            self.shots.push(Shot {
                window: (!main).then(|| title.unwrap_or_default()),
                image: canvas.into_image(),
                missed,
            });
        }
        self.textures.free(&output.textures_delta);
    }
}

/// Every text a frame draws, and the part of the screen it shows in.
fn words(shapes: &[ClippedShape]) -> Vec<(String, Rect)> {
    fn walk(shape: &Shape, clip: Rect, out: &mut Vec<(String, Rect)>) {
        match shape {
            Shape::Vec(shapes) => {
                for shape in shapes {
                    walk(shape, clip, out);
                }
            }
            Shape::Text(text) => {
                let rect = text
                    .galley
                    .rect
                    .translate(text.pos.to_vec2())
                    .intersect(clip);
                if rect.is_positive() && !text.galley.text().trim().is_empty() {
                    out.push((text.galley.text().to_owned(), rect));
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in shapes {
        walk(&clipped.shape, clipped.clip_rect, &mut out);
    }
    out
}

// ---------------------------------------------------------------------------
// Filming a scene into files
// ---------------------------------------------------------------------------

/// Where the pictures go: `CONSISTERM_SNAPSHOT_DIR`, or a folder in the
/// system's temporary directory. Never the config folder.
pub(crate) fn out_dir() -> PathBuf {
    std::env::var_os("CONSISTERM_SNAPSHOT_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("consisterm-snapshots"))
}

/// The comma-separated values of `var`, trimmed, or `None` when it is unset.
pub(crate) fn listed(var: &str) -> Option<Vec<String>> {
    let value = std::env::var(var).ok()?;
    let items: Vec<String> = value
        .split([',', ';'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect();
    (!items.is_empty()).then_some(items)
}

/// A name as a file name: lower case, unaccented, words joined by hyphens.
pub(crate) fn slug(name: &str) -> String {
    let folded = crate::ui::prefs::fold(name);
    let mut slug = String::with_capacity(folded.len());
    for c in folded.chars() {
        if c.is_ascii_alphanumeric() || c == '.' {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_owned()
}

/// The folder for a scale: `1x`, `2x`, `1.5x`.
pub(crate) fn scale_dir(pixels_per_point: f32) -> String {
    format!("{pixels_per_point}x")
}

/// Films `scene` at `pixels_per_point` into `dir`, returning the files
/// written. The kept window is `<scene>.png`; with [`Keep::Every`], any window
/// besides the main one is `<scene>.<its title>.png`.
pub(crate) fn film(
    scene: Scene,
    pixels_per_point: f32,
    dress: impl FnOnce(&Context),
    dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let Scene {
        name,
        stage,
        cues,
        keep,
        draw,
        ..
    } = scene;
    let shots = shoot(&stage, pixels_per_point, &cues, keep, dress, draw);
    if shots.is_empty() {
        return Err(format!("{name}: no window to film - did it open?"));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut written = Vec::new();
    for shot in shots {
        if !shot.missed.is_empty() {
            eprintln!("{name}: could not paint {:?}", shot.missed);
        }
        let file = match (&shot.window, keep) {
            (Some(title), Keep::Every) => format!("{name}.{}.png", slug(title)),
            _ => format!("{name}.png"),
        };
        let path = dir.join(file);
        shot.image
            .save(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Color32;

    fn red_square(ctx: &Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::splat(10.0));
                ui.painter().rect_filled(rect, 0.0, Color32::RED);
            });
    }

    /// A shape a whole number of points from the edge lands on whole pixels
    /// at either scale: ten points in is ten pixels at 1x and twenty at 2x.
    #[test]
    fn a_square_is_painted_where_it_was_drawn_at_every_scale() {
        for ppp in [1.0, 2.0] {
            let stage = Stage::new(Vec2::new(40.0, 30.0)).frames(2);
            let shots = shoot(&stage, ppp, &[], Keep::Main, |_| {}, red_square);
            assert_eq!(shots.len(), 1);
            let image = &shots[0].image;
            assert_eq!(image.dimensions(), (40 * ppp as u32, 30 * ppp as u32));
            let at = |x: f32, y: f32| image.get_pixel((x * ppp) as u32, (y * ppp) as u32).0;
            assert_eq!(at(10.0, 10.0), [255, 0, 0, 255]);
            assert_eq!(at(19.9, 19.9), [255, 0, 0, 255]);
            assert_eq!(at(9.9, 10.0), [0, 0, 0, 255]);
            assert_eq!(at(20.0, 15.0), [0, 0, 0, 255]);
        }
    }

    /// Text comes out as glyphs - the atlas reached the painter - and a cue
    /// can find it by what it says.
    #[test]
    fn a_click_aimed_at_a_word_reaches_the_button_that_shows_it() {
        let stage = Stage::new(Vec2::new(200.0, 100.0)).frames(4);
        let cues = [Cue::at(1, Act::Click(Target::Text("Press me")))];
        let mut clicked = false;
        let shots = shoot(
            &stage,
            1.0,
            &cues,
            Keep::Main,
            |_| {},
            |ctx: &Context| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    clicked |= ui.button("Press me").clicked();
                });
            },
        );
        assert!(clicked);
        let image = &shots[0].image;
        let colours: std::collections::HashSet<[u8; 4]> =
            image.pixels().map(|pixel| pixel.0).collect();
        assert!(colours.len() > 4, "the label's glyphs are painted");
    }

    /// A window of its own is filmed as a picture of its own, the size it
    /// asked for, as eframe shows Settings.
    #[test]
    fn a_window_of_its_own_is_filmed_apart_from_the_main_one() {
        let stage = Stage::new(Vec2::new(300.0, 200.0))
            .frames(3)
            .focus(Focus::Windows);
        let shots = shoot(
            &stage,
            1.0,
            &[],
            Keep::Every,
            |_| {},
            |ctx: &Context| {
                let builder = ViewportBuilder::default()
                    .with_title("Other")
                    .with_inner_size([120.0, 80.0]);
                ctx.show_viewport_immediate(
                    ViewportId::from_hash_of("other"),
                    builder,
                    |ctx, _| {
                        egui::CentralPanel::default().show(ctx, |ui| ui.label("inside"));
                    },
                );
            },
        );
        let titles: Vec<Option<&str>> = shots.iter().map(|s| s.window.as_deref()).collect();
        assert_eq!(titles, [Some("Other"), None]);
        assert_eq!(shots[0].image.dimensions(), (120, 80));
        assert_eq!(shots[1].image.dimensions(), (300, 200));
    }

    #[test]
    fn a_name_becomes_a_file_name_without_accents_or_spaces() {
        assert_eq!(slug("Windows 98"), "windows-98");
        assert_eq!(slug("Configurações"), "configuracoes");
        assert_eq!(slug("IRIS Dark"), "iris-dark");
        assert_eq!(scale_dir(2.0), "2x");
        assert_eq!(scale_dir(1.5), "1.5x");
    }
}
