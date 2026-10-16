//! Finding the monospace fonts installed on this machine, and handing one to
//! egui.
//!
//! egui only knows the families that have been registered with it, and asking
//! for an unregistered one is not a soft failure — it panics inside glyph
//! measurement. So nothing may name a family that [`install`] has not
//! confirmed; that is the whole reason this module reports success rather than
//! just doing the work.

use std::sync::OnceLock;

use egui::{Context, FontData, FontDefinitions, FontFamily};

/// Key the terminal font is registered under. Fixed, so switching fonts
/// replaces the entry instead of accumulating one per family ever chosen.
const TERMINAL_FONT: &str = "nit-terminal-font";

/// The system font list, loaded once.
///
/// Enumerating installed fonts costs on the order of 100 ms, which is fine at
/// startup and not fine every time the Settings combo box is opened — the same
/// reason instance discovery is cached in [`crate::pty::launcher`].
fn database() -> &'static fontdb::Database {
    static DB: OnceLock<fontdb::Database> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        db
    })
}

/// Installed monospace families, sorted, without duplicates.
///
/// Restricted to monospace because a proportional font in a character grid is
/// unusable rather than merely ugly: every column is one cell wide, so the
/// glyphs would not line up with anything.
pub fn monospace_families() -> Vec<String> {
    let mut names: Vec<String> = database()
        .faces()
        .filter(|face| face.monospaced)
        .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
        .collect();
    names.sort_by_key(|name| name.to_lowercase());
    names.dedup();
    names
}

/// Bumped every time the terminal font is replaced.
///
/// `FontFamily::Monospace` is redefined by each install, so the same [`FontId`]
/// can mean a different face before and after one. Anything caching per-glyph
/// work has to be able to tell that apart, and the [`FontId`] alone cannot.
///
/// [`FontId`]: egui::FontId
static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Which set of faces is installed right now. Only ever compared, never read
/// for meaning.
pub fn generation() -> u64 {
    GENERATION.load(std::sync::atomic::Ordering::Relaxed)
}

/// Registers `family` as the terminal font, returning whether it worked.
///
/// An empty name means "egui's bundled monospace", which is always available
/// and needs no work. A name that is not installed, or whose file cannot be
/// read, returns `false` — the caller must then fall back rather than let the
/// name reach a `FontFamily::Name`.
pub fn install(ctx: &Context, family: &str) -> bool {
    if family.is_empty() || family == "monospace" {
        let mut defs = FontDefinitions::default();
        add_display_faces(&mut defs);
        add_skin_faces(&mut defs);
        ctx.set_fonts(defs);
        GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        remember_terminal_font("");
        return true;
    }

    let Some((bytes, index)) = face_data(family) else {
        log::warn!("font family {family:?} is not installed; keeping the built-in monospace");
        return false;
    };

    let mut defs = FontDefinitions::default();
    add_display_faces(&mut defs);
    add_skin_faces(&mut defs);
    defs.font_data.insert(
        TERMINAL_FONT.to_owned(),
        FontData {
            font: std::borrow::Cow::Owned(bytes),
            // Collections (.ttc) hold several faces in one file, so the index
            // fontdb reported has to come along or we would install whichever
            // face happens to be first.
            index,
            tweak: Default::default(),
        },
    );
    // Registered under both names: its own, which the terminal asks for, and at
    // the front of Monospace so `ui.code` and the macro previews match the
    // terminal. The bundled font stays behind it as the fallback that covers
    // whatever glyphs the chosen font is missing.
    defs.families
        .entry(FontFamily::Name(family.into()))
        .or_default()
        .insert(0, TERMINAL_FONT.to_owned());
    defs.families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, TERMINAL_FONT.to_owned());

    ctx.set_fonts(defs);
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    remember_terminal_font(family);
    true
}

/// The family the last successful [`install`] put in place, so the display
/// faces can be added later without changing the terminal's font under it.
fn remember_terminal_font(family: &str) {
    if let Ok(mut last) = TERMINAL_FAMILY.lock() {
        family.clone_into(&mut last);
    }
}

static TERMINAL_FAMILY: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Set once something has asked for a [`Display`] face. Until then none is
/// loaded: they are only for the screen saver's logos, and reading the system
/// font list for them would cost every start-up a tenth of a second.
static DISPLAY_WANTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A lettering face for the screen saver's logos, which egui's bundled light
/// sans cannot do: there is no bold in it and no serif at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Display {
    /// The XP wordmark's "Windows".
    Heavy,
    /// Its "xp", which leans.
    HeavyItalic,
    /// "Pirated Edition": an old-style serif.
    Serif,
}

impl Display {
    const ALL: [Display; 3] = [Display::Heavy, Display::HeavyItalic, Display::Serif];

    fn key(self) -> &'static str {
        match self {
            Display::Heavy => "nit-display-heavy",
            Display::HeavyItalic => "nit-display-heavy-italic",
            Display::Serif => "nit-display-serif",
        }
    }

    /// Families tried in order, and the weight and slant wanted of each.
    /// Franklin Gothic is what the XP wordmark is lettered in, and Palatino
    /// the nearest old-style serif Windows ships; the rest are for a machine
    /// without them.
    fn candidates(self) -> (&'static [&'static str], u16, bool) {
        const HEAVY: &[&str] = &["Franklin Gothic Medium", "Franklin Gothic Demi", "Arial"];
        match self {
            Display::Heavy => (HEAVY, 700, false),
            Display::HeavyItalic => (HEAVY, 700, true),
            Display::Serif => (
                &[
                    "Palatino Linotype",
                    "Book Antiqua",
                    "Georgia",
                    "Times New Roman",
                ],
                700,
                false,
            ),
        }
    }
}

/// Registers whichever display faces this machine has, each as a family of
/// its own with the bundled proportional font behind it for missing glyphs.
fn add_display_faces(defs: &mut FontDefinitions) {
    if !DISPLAY_WANTED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let fallback = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for display in Display::ALL {
        let (families, weight, italic) = display.candidates();
        let Some((bytes, index)) = families
            .iter()
            .find_map(|family| face_matching(family, weight, italic))
        else {
            continue;
        };
        defs.font_data.insert(
            display.key().to_owned(),
            FontData {
                font: std::borrow::Cow::Owned(bytes),
                index,
                tweak: Default::default(),
            },
        );
        let mut chain = vec![display.key().to_owned()];
        chain.extend(fallback.iter().cloned());
        defs.families
            .insert(FontFamily::Name(display.key().into()), chain);
    }
}

/// The family to letter `display` in, or `None` for the bundled proportional
/// font - which is what the first frame that asks gets, because that request
/// is what loads the faces, and egui only takes new fonts at the next frame.
///
/// Never hands out a family egui does not have yet: asking for one panics.
pub fn display_family(ctx: &Context, display: Display) -> Option<FontFamily> {
    if !DISPLAY_WANTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        let family = TERMINAL_FAMILY
            .lock()
            .map(|f| f.clone())
            .unwrap_or_default();
        install(ctx, &family);
        return None;
    }
    let family = FontFamily::Name(display.key().into());
    ctx.fonts(|f| f.families().contains(&family))
        .then_some(family)
}

/// The skin whose faces the interface is set in. Remembered beside the
/// terminal's family, because every install starts again from egui's
/// defaults and has to put both back.
static SKIN: std::sync::Mutex<crate::ui::skin::Skin> =
    std::sync::Mutex::new(crate::ui::skin::Skin::None);

/// The interface's bold face under a skin - title bars, headings, a default
/// button's label - registered as a family of its own.
pub const SKIN_BOLD: &str = "nit-skin-bold";
/// The title bar's face, when it is not the bold one: XP's Trebuchet.
pub const SKIN_TITLE: &str = "nit-skin-title";

/// Sets the interface in `skin`'s faces, reinstalling the fonts only when the
/// skin has changed: an install throws egui's whole glyph atlas away.
pub fn set_skin(ctx: &Context, skin: crate::ui::skin::Skin) {
    if let Ok(mut current) = SKIN.lock() {
        *current = skin;
    }
    // Which skin this context's fonts were last installed for. Kept in the
    // context rather than beside `SKIN`, because a second context - a test,
    // a snapshot - starts from egui's defaults whatever the first one got;
    // and not found by asking egui what it has, which panics before the
    // context's first frame, when the app dresses it.
    let id = egui::Id::new("nit-skin-fonts");
    let installed = ctx.data(|d| d.get_temp::<crate::ui::skin::Skin>(id));
    if installed != Some(skin) {
        ctx.data_mut(|d| d.insert_temp(id, skin));
        let family = TERMINAL_FAMILY
            .lock()
            .map(|f| f.clone())
            .unwrap_or_default();
        install(ctx, &family);
    }
}

/// The family to set bold interface text in: the skin's bold face, or the
/// regular interface when egui has not taken the fonts yet - it does a frame
/// after they are set, and naming a family it does not have panics.
pub fn skin_bold(ctx: &Context) -> FontFamily {
    let family = FontFamily::Name(SKIN_BOLD.into());
    if ctx.fonts(|f| f.families().contains(&family)) {
        family
    } else {
        FontFamily::Proportional
    }
}

/// The family for a title bar's text, by the same rules as [`skin_bold`].
pub fn skin_title(ctx: &Context) -> FontFamily {
    let family = FontFamily::Name(SKIN_TITLE.into());
    if ctx.fonts(|f| f.families().contains(&family)) {
        family
    } else {
        skin_bold(ctx)
    }
}

/// A skin's face as egui takes it, from the binary or from the system.
fn face_bytes(face: crate::ui::skin::Face, weight: u16) -> Option<FontData> {
    match face {
        crate::ui::skin::Face::Bundled(bytes) => Some(FontData::from_static(bytes)),
        crate::ui::skin::Face::System(names) => names
            .iter()
            .find_map(|name| face_matching(name, weight, false))
            .map(|(bytes, index)| owned(bytes, index)),
    }
}

fn owned(bytes: Vec<u8>, index: u32) -> FontData {
    FontData {
        font: std::borrow::Cow::Owned(bytes),
        index,
        tweak: Default::default(),
    }
}

/// Puts the current skin's faces at the front of the interface's family, with
/// Arial behind them - the fallback the CSS names, and what covers whatever a
/// pixel font of the period never had; the Latin-1 accents have been added to
/// the bundled faces themselves (`assets/fonts/*/MODIFIED.txt`), since Arial
/// between two pixel letters is the wrong size - and registers the bold and title
/// families. Under no skin egui's own faces stay as they are, and the two
/// families are the regular one, so a name asked for always exists.
fn add_skin_faces(defs: &mut FontDefinitions) {
    let skin = SKIN.lock().map(|s| *s).unwrap_or_default();
    let fonts = crate::ui::skin::fonts_of(skin);
    let skinned = skin != crate::ui::skin::Skin::None;
    let mut proportional = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut bold = proportional.clone();
    if skinned {
        if let Some((bytes, index)) = face_matching("Arial", 400, false) {
            defs.font_data
                .insert("nit-skin-arial".to_owned(), owned(bytes, index));
            proportional.insert(0, "nit-skin-arial".to_owned());
        }
        if let Some((bytes, index)) = face_matching("Arial", 700, false) {
            defs.font_data
                .insert("nit-skin-arial-bold".to_owned(), owned(bytes, index));
            bold.insert(0, "nit-skin-arial-bold".to_owned());
        }
    }
    if let Some(data) = fonts.body.and_then(|f| face_bytes(f, 400)) {
        defs.font_data.insert("nit-skin-body".to_owned(), data);
        proportional.insert(0, "nit-skin-body".to_owned());
    }
    if let Some(data) = fonts.bold.and_then(|f| face_bytes(f, 700)) {
        defs.font_data.insert("nit-skin-bold-face".to_owned(), data);
        bold.insert(0, "nit-skin-bold-face".to_owned());
    }
    let mut title = bold.clone();
    if let Some(data) = fonts.title.and_then(|f| face_bytes(f, 700)) {
        defs.font_data
            .insert("nit-skin-title-face".to_owned(), data);
        title.insert(0, "nit-skin-title-face".to_owned());
    }
    defs.families.insert(FontFamily::Proportional, proportional);
    defs.families
        .insert(FontFamily::Name(SKIN_BOLD.into()), bold);
    defs.families
        .insert(FontFamily::Name(SKIN_TITLE.into()), title);
}

/// Bytes and face index of the regular face of `family`.
///
/// The lightest upright face wins. Matching on the family name alone would
/// happily pick Bold or Italic, whichever the enumeration reached first, and
/// the terminal would then render everything bold with no way to undo it.
fn face_data(family: &str) -> Option<(Vec<u8>, u32)> {
    face_matching(family, fontdb::Weight::NORMAL.0, false)
}

/// Bytes and face index of the face of `family` nearest `weight`, upright or
/// italic as asked; the slant counts before the weight does.
fn face_matching(family: &str, weight: u16, italic: bool) -> Option<(Vec<u8>, u32)> {
    let db = database();
    let wanted = family.to_lowercase();

    let best = db
        .faces()
        .filter(|face| {
            face.families
                .iter()
                .any(|(name, _)| name.to_lowercase() == wanted)
        })
        .min_by_key(|face| {
            let slant = (face.style != fontdb::Style::Normal) != italic;
            // Distance from what was asked, so a family shipping only Light or
            // Medium still resolves instead of being rejected.
            let weight = face.weight.0.abs_diff(weight);
            (slant, weight)
        })?;

    let index = best.index;
    let bytes = db.with_face_data(best.id, |data, _| data.to_vec())?;
    Some((bytes, index))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every pixel face a skin ships has the letters Portuguese is written
    /// in, on its own - not borrowed from the Arial behind it, which sets
    /// them at another size between two pixel letters.
    #[test]
    fn every_bundled_skin_face_writes_portuguese_by_itself() {
        let faces: [&'static [u8]; 5] = [
            include_bytes!("../../assets/fonts/press-start-2p/SNESPixel8px-Regular.ttf"),
            include_bytes!("../../assets/fonts/ms-sans-serif/MS Sans Serif.ttf"),
            include_bytes!("../../assets/fonts/ms-sans-serif-bold/MS Sans Serif Bold.ttf"),
            include_bytes!("../../assets/fonts/chikarego2/ChiKareGo2.ttf"),
            include_bytes!("../../assets/fonts/finderskeepers/FindersKeepers.ttf"),
        ];
        for (i, bytes) in faces.into_iter().enumerate() {
            let mut defs = FontDefinitions::empty();
            defs.font_data
                .insert("face".to_owned(), FontData::from_static(bytes));
            defs.families
                .insert(FontFamily::Proportional, vec!["face".to_owned()]);
            let fonts = egui::epaint::text::Fonts::new(1.0, 1024, defs);
            let id = egui::FontId::proportional(16.0);
            for c in "áàâãéêíóôõúüçÁÀÂÃÉÊÍÓÔÕÚÇ".chars() {
                assert!(fonts.has_glyph(&id, c), "face {i} lacks {c}");
            }
        }
    }
}
