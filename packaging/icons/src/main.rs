//! Writes `assets/icon-256.png` (Linux), `assets/icon.ico` (the Windows
//! executable and the window icon) and `assets/consisterm.icns` (macOS) from
//! `assets/logo.png`, the one master.
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

fn main() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let master = image::open(assets.join("logo.png"))
        .expect("reading assets/logo.png")
        .into_rgba8();
    let sized = |n: u32| image::imageops::resize(&master, n, n, FilterType::Lanczos3);

    sized(256)
        .save(assets.join("icon-256.png"))
        .expect("writing icon-256.png");

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
