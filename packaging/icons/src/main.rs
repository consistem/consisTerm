//! Writes `assets/icon-256.png` (Linux), `assets/icon.ico` (the Windows
//! executable, the tray and the About page), `assets/icon-window.png` (the
//! window's own icon, which the taskbar shows) and `assets/consisterm.icns`
//! (macOS) from `assets/logo.png`, the one master.
//!
//! The `.ico` and `.icns` hold PNG images rather than bitmaps: both formats
//! allow it, Windows has read PNG entries since Vista, and it keeps a 256 px
//! glow from turning into a megabyte of raw pixels.

use std::io::Cursor;
use std::path::Path;

use image::{imageops::FilterType, RgbaImage};

fn png(img: &RgbaImage) -> Vec<u8> {
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), image::ImageOutputFormat::Png)
        .expect("encoding a PNG in memory");
    out
}

/// The square of `img` round the pixels at least `alpha` opaque, grown by
/// `margin` of its side and centred on them.
///
/// The logo's glow fades out over a wide margin, which is right for the logo
/// and wrong for an icon: drawn at the size of a taskbar button or a desktop
/// shortcut, the ring inside the glow came out smaller than every icon next
/// to it. So an icon is cut round what can actually be seen at its size.
fn crop(img: &RgbaImage, alpha: u8, margin: f32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (mut left, mut top, mut right, mut bottom) = (w, h, 0, 0);
    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel[3] >= alpha {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    if left > right {
        return img.clone();
    }
    let side = ((right - left).max(bottom - top) as f32 * (1.0 + 2.0 * margin)).ceil() as u32;
    let cx = (left + right) as i64 / 2;
    let cy = (top + bottom) as i64 / 2;
    let mut out = RgbaImage::new(side, side);
    image::imageops::overlay(
        &mut out,
        img,
        side as i64 / 2 - cx,
        side as i64 / 2 - cy,
    );
    out
}

/// `img` a little brighter: each channel lifted by `gain`, toward white
/// rather than past it. At tray size the ring reads darker than it does
/// large, against a taskbar that is often dark itself.
fn brighten(img: &RgbaImage, gain: f32) -> RgbaImage {
    let mut out = img.clone();
    for pixel in out.pixels_mut() {
        for channel in &mut pixel.0[..3] {
            *channel = (f32::from(*channel) * gain).round().min(255.0) as u8;
        }
    }
    out
}

fn main() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let master = image::open(assets.join("logo.png"))
        .expect("reading assets/logo.png")
        .into_rgba8();
    // Large sizes are the whole logo, glow and all, which they are big enough
    // to show; small ones are cut close to the ring, where a glow would only
    // be a blur taking room from it - the tray, the taskbar.
    let small = brighten(&crop(&master, 140, 0.03), 1.12);
    println!(
        "logo {}px; small icons cut to {}px",
        master.width(),
        small.width()
    );
    let sized = |n: u32| {
        let from = if n <= 64 { &small } else { &master };
        image::imageops::resize(from, n, n, FilterType::Lanczos3)
    };

    sized(256)
        .save(assets.join("icon-256.png"))
        .expect("writing icon-256.png");
    // The window's icon, which the system scales down for the taskbar: from
    // the close cut, at a size that stays sharp at 200%.
    image::imageops::resize(&small, 128, 128, FilterType::Lanczos3)
        .save(assets.join("icon-window.png"))
        .expect("writing icon-window.png");

    // ICO: a directory of entries, then the images, smallest first.
    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
    let images: Vec<Vec<u8>> = sizes.iter().map(|&n| png(&sized(n))).collect();
    let mut ico = vec![0u8, 0, 1, 0];
    ico.extend((sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&n, data) in sizes.iter().zip(&images) {
        // 256 is written as 0: the field is one byte.
        let side = if n >= 256 { 0 } else { n as u8 };
        ico.extend([side, side, 0, 0]);
        ico.extend(1u16.to_le_bytes());
        ico.extend(32u16.to_le_bytes());
        ico.extend((data.len() as u32).to_le_bytes());
        ico.extend(offset.to_le_bytes());
        offset += data.len() as u32;
    }
    for data in &images {
        ico.extend(data);
    }
    std::fs::write(assets.join("icon.ico"), ico).expect("writing icon.ico");

    // ICNS: typed chunks, each a PNG. The @2x types reuse the larger sizes.
    let types: [(&[u8; 4], u32); 10] = [
        (b"icp4", 16),
        (b"icp5", 32),
        (b"ic11", 32),
        (b"icp6", 64),
        (b"ic12", 64),
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic13", 256),
        (b"ic09", 512),
        (b"ic10", 1024),
    ];
    let mut body = Vec::new();
    for (kind, n) in types {
        let data = png(&sized(n));
        body.extend(kind.as_slice());
        body.extend(((data.len() + 8) as u32).to_be_bytes());
        body.extend(data);
    }
    let mut icns = b"icns".to_vec();
    icns.extend(((body.len() + 8) as u32).to_be_bytes());
    icns.extend(body);
    std::fs::write(assets.join("consisterm.icns"), icns).expect("writing consisterm.icns");
}
