//! The icon the window goes by - on the taskbar, in the window switcher and on
//! the About page - chosen in Settings from the ones bundled or a file of the
//! user's.
//!
//! The executable keeps the icon it was built with: it is a resource inside
//! the file, and rewriting a running program's own file is not something to
//! do behind anyone's back. A shortcut pinned to the taskbar shows its own
//! icon rather than the window's, so [`crate::ui::taskbar_pin`] points it at
//! the chosen one.

use egui::{Color32, ColorImage, Context, IconData, Rect, Sense, TextureHandle, Ui, Vec2};
use image::imageops::FilterType;
use image::RgbaImage;

use crate::config::{AppIcon, Settings};
use crate::i18n::tr;

/// Largest side kept of a picture: the About page draws its icon at 168
/// points, which on a display at twice the scale is 336 pixels.
const MAX_SIDE: u32 = 512;

/// Side of the icon handed to the window. Windows scales it to whatever the
/// taskbar wants, and it does that badly from a large picture: a hairline
/// ring came out broken. Shrunk here with a proper filter instead, to about
/// the size of the largest it draws.
const WINDOW_SIDE: u32 = 128;

/// Side, in pixels, of a picture in the Settings gallery: twice what it is
/// drawn at, so it stays sharp at twice the scale.
const THUMB_SIDE: u32 = 80;

/// The bundled picture `icon` stands for, `None` for a file of the user's.
fn bundled(icon: AppIcon) -> Option<&'static [u8]> {
    Some(match icon {
        AppIcon::White => &include_bytes!("../../assets/app-icons/white.png")[..],
        AppIcon::Neon => &include_bytes!("../../assets/app-icons/neon.png")[..],
        AppIcon::Water => &include_bytes!("../../assets/app-icons/water.png")[..],
        AppIcon::Christmas => &include_bytes!("../../assets/app-icons/christmas.png")[..],
        AppIcon::Metal => &include_bytes!("../../assets/app-icons/metal.png")[..],
        AppIcon::Original => &include_bytes!("../../assets/icon-window.png")[..],
        AppIcon::Custom => return None,
    })
}

/// The picture `icon` stands for, square and no larger than `MAX_SIDE`, or
/// why it cannot be had.
pub fn picture(icon: AppIcon, file: &str) -> Result<RgbaImage, String> {
    let decoded = match bundled(icon) {
        Some(bytes) => image::load_from_memory(bytes),
        None => {
            let file = file.trim();
            if file.is_empty() {
                return Err(tr("With no file, the original icon is shown.").to_owned());
            }
            image::open(file)
        }
    };
    let image = decoded.map_err(|_| {
        tr("This file could not be read as a picture; the original icon is shown instead.")
            .to_owned()
    })?;
    Ok(square(image.into_rgba8(), MAX_SIDE))
}

/// The picture the settings ask for, or the original when a file of the
/// user's will not load, with why it would not.
pub fn chosen(settings: &Settings) -> (RgbaImage, Option<String>) {
    match picture(settings.app_icon, &settings.app_icon_file) {
        Ok(image) => (image, None),
        Err(why) => (
            picture(AppIcon::default(), "").expect("the bundled icon decodes"),
            Some(why),
        ),
    }
}

/// What tells one choice from another: a different file is a different icon
/// even when the choice is Custom both times.
pub fn key(settings: &Settings) -> (AppIcon, String) {
    let file = if settings.app_icon == AppIcon::Custom {
        settings.app_icon_file.trim().to_owned()
    } else {
        String::new()
    };
    (settings.app_icon, file)
}

/// The icon to give the window.
pub fn window_icon(settings: &Settings) -> IconData {
    let image = square(chosen(settings).0, WINDOW_SIDE);
    IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

/// `image` shrunk to fit `side` and centred on a transparent square of its
/// size or smaller: a window icon is square, and one that is not is
/// stretched.
fn square(image: RgbaImage, side: u32) -> RgbaImage {
    let (w, h) = image.dimensions();
    let longest = w.max(h).max(1);
    let image = if longest > side {
        let scale = side as f32 / longest as f32;
        let w = ((w as f32 * scale).round() as u32).max(1);
        let h = ((h as f32 * scale).round() as u32).max(1);
        image::imageops::resize(&image, w, h, FilterType::Lanczos3)
    } else {
        image
    };
    let (w, h) = image.dimensions();
    if w == h {
        return image;
    }
    let edge = w.max(h);
    let mut canvas = RgbaImage::new(edge, edge);
    image::imageops::overlay(
        &mut canvas,
        &image,
        i64::from((edge - w) / 2),
        i64::from((edge - h) / 2),
    );
    canvas
}

fn texture(ctx: &Context, name: &str, image: &RgbaImage) -> TextureHandle {
    let size = [image.width() as usize, image.height() as usize];
    ctx.load_texture(
        name,
        ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        egui::TextureOptions::LINEAR,
    )
}

/// The chosen icon as a texture, and why it is not the one asked for when it
/// is not.
#[derive(Clone)]
struct Loaded {
    key: (AppIcon, String),
    texture: TextureHandle,
    problem: Option<String>,
}

/// The chosen icon, loaded once per choice. One slot, not one per choice: a
/// path typed a letter at a time would otherwise leave a texture behind for
/// every letter.
fn loaded(ctx: &Context, settings: &Settings) -> Loaded {
    let id = egui::Id::new("app-icon");
    let key = key(settings);
    if let Some(found) = ctx.data(|d| d.get_temp::<Loaded>(id)) {
        if found.key == key {
            return found;
        }
    }
    let (image, problem) = chosen(settings);
    let found = Loaded {
        key,
        texture: texture(ctx, "app-icon", &image),
        problem,
    };
    ctx.data_mut(|d| d.insert_temp(id, found.clone()));
    found
}

/// Why the chosen file is not the icon showing, when it is not.
pub fn problem(ctx: &Context, settings: &Settings) -> Option<String> {
    loaded(ctx, settings).problem
}

/// The icon on the About page, `side` points square. The original keeps its
/// living glow; any other is drawn as it is, since the glow is cut out of
/// that one picture's own halo and the others have none to cut.
pub fn show(ui: &mut Ui, side: f32, settings: &Settings) {
    if settings.app_icon == AppIcon::Original {
        crate::ui::logo::show(ui, side);
        return;
    }
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let texture = loaded(ui.ctx(), settings).texture;
    ui.painter()
        .image(texture.id(), rect, full_uv(), Color32::WHITE);
}

/// A bundled icon, small, for the gallery in Settings.
pub fn thumbnail(ctx: &Context, icon: AppIcon) -> Option<TextureHandle> {
    let bytes = bundled(icon)?;
    let id = egui::Id::new(("app-icon-thumb", icon));
    if let Some(found) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return Some(found);
    }
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let found = texture(ctx, "app-icon-thumb", &square(image, THUMB_SIDE));
    ctx.data_mut(|d| d.insert_temp(id, found.clone()));
    Some(found)
}

pub fn full_uv() -> Rect {
    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of the test's own under the system's temporary one.
    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("consisterm-app-icon-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn every_bundled_icon_decodes_square_and_within_bounds() {
        for icon in AppIcon::ALL {
            if icon == AppIcon::Custom {
                continue;
            }
            let image = picture(icon, "").unwrap();
            assert_eq!(image.width(), image.height(), "{icon:?}");
            assert!(image.width() <= MAX_SIDE, "{icon:?}");
        }
    }

    #[test]
    fn a_custom_icon_with_no_file_or_a_broken_one_falls_back_to_the_original() {
        let original = picture(AppIcon::Original, "").unwrap();
        let mut settings = Settings {
            app_icon: AppIcon::Custom,
            ..Settings::default()
        };
        let (image, problem) = chosen(&settings);
        assert_eq!(image, original);
        assert!(problem.is_some());

        let broken = temp_dir("broken").join("broken.png");
        std::fs::write(&broken, b"not a picture").unwrap();
        settings.app_icon_file = broken.display().to_string();
        let (image, problem) = chosen(&settings);
        assert_eq!(image, original);
        assert!(problem.is_some());
    }

    #[test]
    fn a_custom_picture_that_is_not_square_is_centred_on_a_square() {
        let path = temp_dir("wide").join("wide.png");
        RgbaImage::from_pixel(40, 20, image::Rgba([255, 0, 0, 255]))
            .save(&path)
            .unwrap();
        let settings = Settings {
            app_icon: AppIcon::Custom,
            app_icon_file: path.display().to_string(),
            ..Settings::default()
        };
        let (image, problem) = chosen(&settings);
        assert_eq!(problem, None);
        assert_eq!(image.dimensions(), (40, 40));
        // Transparent above and below, the picture across the middle.
        assert_eq!(image.get_pixel(20, 2)[3], 0);
        assert_eq!(*image.get_pixel(20, 20), image::Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn the_window_gets_a_small_square_icon_whatever_the_picture() {
        let icon = window_icon(&Settings::default());
        assert_eq!((icon.width, icon.height), (WINDOW_SIDE, WINDOW_SIDE));
        assert_eq!(icon.rgba.len(), (WINDOW_SIDE * WINDOW_SIDE * 4) as usize);
    }

    #[test]
    fn the_file_only_tells_choices_apart_when_the_choice_is_a_file() {
        let neon = Settings {
            app_icon: AppIcon::Neon,
            app_icon_file: "C:\\x.png".into(),
            ..Settings::default()
        };
        assert_eq!(key(&neon), (AppIcon::Neon, String::new()));
        let custom = Settings {
            app_icon: AppIcon::Custom,
            ..neon
        };
        assert_eq!(key(&custom), (AppIcon::Custom, "C:\\x.png".into()));
    }
}
