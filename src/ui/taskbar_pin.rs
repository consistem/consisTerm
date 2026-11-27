//! A shortcut pinned to the taskbar follows the icon chosen in Settings.
//!
//! A pinned button shows its shortcut's icon, not the window's - while the app
//! runs as well as while it is closed - so changing only the window's icon
//! changed everything but the one place it is most seen. The pin is a `.lnk`
//! the shell keeps in a folder of the user's; pointed at an `.ico` written
//! from the chosen picture, it shows that instead.
//!
//! Only shortcuts to this very executable are touched, and only their icon.
//! The shell caches icons, so the taskbar may keep the old one until it next
//! redraws; it is told the shortcut changed, which is usually enough.

use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::imageops::FilterType;
use image::{ColorType, RgbaImage};

use crate::config::Settings;

/// The sizes the shell draws a taskbar icon at, from 100% to 250% scale, and
/// the large one Explorer shows in its biggest views.
const SIDES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

/// Points every pinned shortcut of this executable at the icon `settings`
/// choose. On a thread of its own: COM, a few files and a word to the shell
/// are nothing a frame should wait on.
pub fn follow(settings: &Settings) {
    #[cfg(windows)]
    {
        let settings = settings.clone();
        std::thread::spawn(move || {
            if let Err(e) = imp::follow(&settings) {
                eprintln!("could not update the taskbar pin's icon: {e:#}");
            }
        });
    }
    #[cfg(not(windows))]
    let _ = settings;
}

/// `image` as an `.ico` holding every size in `SIDES`, each shrunk with a
/// proper filter rather than left to the shell's.
pub fn ico_bytes(image: &RgbaImage) -> image::ImageResult<Vec<u8>> {
    let frames = SIDES
        .iter()
        .map(|&side| {
            let small = image::imageops::resize(image, side, side, FilterType::Lanczos3);
            IcoFrame::as_png(small.as_raw(), side, side, ColorType::Rgba8)
        })
        .collect::<image::ImageResult<Vec<_>>>()?;
    let mut out = Vec::new();
    IcoEncoder::new(&mut out).encode_images(&frames)?;
    Ok(out)
}

/// The name an icon is written under: from its contents, so a new picture -
/// a file of the user's changed in place included - is a new name. The shell
/// caches icons by path, and would go on showing the old picture at an old
/// name. Only the Windows side writes one; elsewhere it would be dead code,
/// which `-D warnings` refuses.
#[cfg(any(windows, test))]
fn ico_name(bytes: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("taskbar-{:016x}.ico", hasher.finish())
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use anyhow::Context as _;
    use windows::core::{ComInterface, HSTRING, PCWSTR};
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::System::Com::{
        CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READWRITE,
    };
    use windows::Win32::UI::Shell::{
        IShellLinkW, SHChangeNotify, ShellLink, SHCNE_ASSOCCHANGED, SHCNE_UPDATEITEM, SHCNF_IDLIST,
        SHCNF_PATHW,
    };

    use crate::config::{AppIcon, Settings};

    pub(super) fn follow(settings: &Settings) -> anyhow::Result<()> {
        let Some(folder) = pinned_folder() else {
            return Ok(());
        };
        let Ok(entries) = std::fs::read_dir(&folder) else {
            return Ok(());
        };
        let links: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")))
            .collect();
        if links.is_empty() {
            return Ok(());
        }
        let exe = std::env::current_exe().context("finding the executable")?;
        let icon = point(&links, &exe, true, || {
            // The original is the executable's own, which is where a pin's
            // icon came from before it was ever changed.
            if settings.app_icon == AppIcon::Original {
                Ok(exe.clone())
            } else {
                write_ico(settings)
            }
        })?;
        if let Some(icon) = icon {
            forget_other_icons(&icon);
        }
        Ok(())
    }

    /// Points those of `links` that start `exe` at the icon `icon` gives,
    /// asked for only when there is one, and tells the shell when `notify`.
    /// Returns that icon, if any was.
    pub(super) fn point(
        links: &[PathBuf],
        exe: &Path,
        notify: bool,
        icon: impl FnOnce() -> anyhow::Result<PathBuf>,
    ) -> anyhow::Result<Option<PathBuf>> {
        let _com = Com::init();
        let ours: Vec<Link> = links
            .iter()
            .filter_map(|path| Link::open(path).ok())
            .filter(|link| link.target().is_some_and(|t| same_file(&t, exe)))
            .collect();
        if ours.is_empty() {
            return Ok(None);
        }
        let icon = icon()?;
        let mut changed = false;
        for link in &ours {
            if link.icon().is_some_and(|now| same_file(&now, &icon)) {
                continue;
            }
            link.set_icon(&icon)
                .with_context(|| format!("updating {}", link.path.display()))?;
            changed = true;
            if !notify {
                continue;
            }
            let wide = wide(&link.path);
            unsafe {
                SHChangeNotify(
                    SHCNE_UPDATEITEM,
                    SHCNF_PATHW,
                    Some(wide.as_ptr().cast()),
                    None,
                );
            }
        }
        if changed && notify {
            // What makes the taskbar drop its cached picture of the pin.
            unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
        }
        Ok(Some(icon))
    }

    /// Where the shell keeps the shortcuts pinned to the taskbar.
    fn pinned_folder() -> Option<PathBuf> {
        Some(
            dirs::config_dir()?
                .join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar"),
        )
    }

    fn icon_folder() -> PathBuf {
        crate::config::config_dir().join("icons")
    }

    /// The chosen picture written as an `.ico` in the config folder, unless
    /// it already is.
    fn write_ico(settings: &Settings) -> anyhow::Result<PathBuf> {
        let (image, _) = crate::ui::app_icon::chosen(settings);
        let bytes = super::ico_bytes(&image).context("encoding the icon")?;
        let folder = icon_folder();
        std::fs::create_dir_all(&folder)
            .with_context(|| format!("creating {}", folder.display()))?;
        let path = folder.join(super::ico_name(&bytes));
        if !path.exists() {
            std::fs::write(&path, &bytes).with_context(|| format!("writing {}", path.display()))?;
        }
        Ok(path)
    }

    /// Every icon written for an earlier choice. Only those this module
    /// names: the folder is the user's config folder.
    fn forget_other_icons(keep: &Path) {
        let Ok(entries) = std::fs::read_dir(icon_folder()) else {
            return;
        };
        for path in entries.filter_map(|e| e.ok().map(|e| e.path())) {
            let ours = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("taskbar-") && n.ends_with(".ico"));
            if ours && !same_file(&path, keep) {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    fn same_file(a: &Path, b: &Path) -> bool {
        let canonical = |p: &Path| {
            p.canonicalize()
                .unwrap_or_else(|_| p.to_path_buf())
                .to_string_lossy()
                .to_lowercase()
        };
        canonical(a) == canonical(b)
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    /// A UTF-16 buffer as written by the shell, up to its terminating NUL.
    fn from_wide(buffer: &[u16]) -> Option<PathBuf> {
        let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        (end > 0).then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..end])))
    }

    /// COM on this thread for as long as it is held.
    struct Com(bool);

    impl Com {
        fn init() -> Com {
            use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            let hr = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
            Com(hr >= 0)
        }
    }

    impl Drop for Com {
        fn drop(&mut self) {
            if self.0 {
                unsafe { windows_sys::Win32::System::Com::CoUninitialize() };
            }
        }
    }

    /// One shortcut, loaded.
    struct Link {
        path: PathBuf,
        shell: IShellLinkW,
        file: IPersistFile,
    }

    impl Link {
        fn open(path: &Path) -> windows::core::Result<Link> {
            unsafe {
                let shell: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
                let file: IPersistFile = shell.cast()?;
                file.Load(&HSTRING::from(path.as_os_str()), STGM_READWRITE)?;
                Ok(Link {
                    path: path.to_path_buf(),
                    shell,
                    file,
                })
            }
        }

        fn target(&self) -> Option<PathBuf> {
            let mut buffer = [0u16; 1024];
            unsafe {
                self.shell
                    .GetPath(&mut buffer, std::ptr::null_mut(), 0)
                    .ok()?;
            }
            from_wide(&buffer)
        }

        fn icon(&self) -> Option<PathBuf> {
            let mut buffer = [0u16; 1024];
            let mut index = 0;
            unsafe { self.shell.GetIconLocation(&mut buffer, &mut index).ok()? };
            from_wide(&buffer)
        }

        fn set_icon(&self, icon: &Path) -> windows::core::Result<()> {
            unsafe {
                self.shell
                    .SetIconLocation(&HSTRING::from(icon.as_os_str()), 0)?;
                self.file.Save(PCWSTR::null(), BOOL::from(true))
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use windows::Win32::System::Com::IPersistFile;

        /// A shortcut at `path` to `target`, as the shell writes one.
        fn make(path: &Path, target: &Path) {
            unsafe {
                let shell: IShellLinkW =
                    CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
                shell.SetPath(&HSTRING::from(target.as_os_str())).unwrap();
                let file: IPersistFile = shell.cast().unwrap();
                file.Save(&HSTRING::from(path.as_os_str()), BOOL::from(true))
                    .unwrap();
            }
        }

        #[test]
        fn only_a_pin_of_this_executable_is_pointed_at_the_chosen_icon() {
            let dir = std::env::temp_dir().join(format!("consisterm-pin-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let exe = dir.join("consisterm.exe");
            let other = dir.join("other.exe");
            let icon = dir.join("chosen.ico");
            for file in [&exe, &other, &icon] {
                std::fs::write(file, b"").unwrap();
            }
            let ours = dir.join("ours.lnk");
            let theirs = dir.join("theirs.lnk");
            let _com = Com::init();
            make(&ours, &exe);
            make(&theirs, &other);
            let links = [ours.clone(), theirs.clone()];

            let given = point(&links, &exe, false, || Ok(icon.clone())).unwrap();
            assert_eq!(given.as_deref(), Some(icon.as_path()));
            let icon_of = |p: &Path| Link::open(p).unwrap().icon();
            assert!(icon_of(&ours).is_some_and(|i| same_file(&i, &icon)));
            assert_eq!(icon_of(&theirs), None);

            // With no pin of it, the icon is not even made.
            let none = point(std::slice::from_ref(&theirs), &exe, false, || {
                panic!("not asked for")
            })
            .unwrap();
            assert_eq!(none, None);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pins_icon_holds_every_size_the_taskbar_draws() {
        let (picture, _) = crate::ui::app_icon::chosen(&Settings::default());
        let bytes = ico_bytes(&picture).unwrap();
        let decoded = image::codecs::ico::IcoDecoder::new(std::io::Cursor::new(&bytes)).unwrap();
        // The decoder picks the largest; the count is in the header.
        let count = u16::from_le_bytes([bytes[4], bytes[5]]);
        assert_eq!(usize::from(count), SIDES.len());
        use image::ImageDecoder;
        assert_eq!(decoded.dimensions(), (256, 256));
    }

    #[test]
    fn a_different_picture_is_written_under_a_different_name() {
        let a = ico_bytes(&RgbaImage::from_pixel(8, 8, image::Rgba([255, 0, 0, 255]))).unwrap();
        let b = ico_bytes(&RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 255, 255]))).unwrap();
        assert_ne!(ico_name(&a), ico_name(&b));
        assert_eq!(ico_name(&a), ico_name(&a.clone()));
    }
}
