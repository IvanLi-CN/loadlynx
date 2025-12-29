use std::path::{Path, PathBuf};

use embedded_graphics::pixelcolor::Rgb565;
use image::{ImageBuffer, Rgb};
use lcd_async::raw_framebuf::RawFrameBuf;
use loadlynx_protocol::LoadMode;

pub const DISPLAY_WIDTH: usize = 240;
pub const DISPLAY_HEIGHT: usize = 320;

mod control {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum AdjustDigit {
        Ones,
        Tenths,
        Hundredths,
    }

    impl AdjustDigit {
        pub const DEFAULT: Self = Self::Tenths;
    }
}

mod touch {
    #[derive(Clone, Copy, Debug)]
    pub struct TouchMarker {
        pub x: i32,
        pub y: i32,
        pub id: u8,
        pub event: u8,
    }
}

#[path = "../../../firmware/digital/src/ui/mod.rs"]
mod ui;

fn rgb565_to_rgb888(pixel: u16) -> [u8; 3] {
    let r5 = (pixel >> 11) & 0x1f;
    let g6 = (pixel >> 5) & 0x3f;
    let b5 = pixel & 0x1f;

    let r = ((r5 as u32 * 255 + 15) / 31) as u8;
    let g = ((g6 as u32 * 255 + 31) / 63) as u8;
    let b = ((b5 as u32 * 255 + 15) / 31) as u8;
    [r, g, b]
}

fn render_snapshot(
    path: &Path,
    snapshot: &ui::UiSnapshot,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut snapshot = snapshot.clone();
    snapshot.update_strings();

    let mut framebuffer = vec![0u8; DISPLAY_WIDTH * DISPLAY_HEIGHT * 2];
    let mut frame =
        RawFrameBuf::<Rgb565, _>::new(&mut framebuffer[..], DISPLAY_WIDTH, DISPLAY_HEIGHT);
    ui::render(&mut frame, &snapshot);

    // The UI renderer writes into the physical ST7789 buffer (240×320), while
    // the design mocks are documented in the logical landscape space (320×240).
    // Invert Canvas::set_pixel mapping to export a 320×240 PNG.
    let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(320, 240);
    for x in 0..320i32 {
        for y in 0..240i32 {
            let phys_x = y as usize;
            let phys_y = (DISPLAY_HEIGHT as i32 - 1 - x) as usize;
            let idx = (phys_y * DISPLAY_WIDTH + phys_x) * 2;
            let px = u16::from_be_bytes([framebuffer[idx], framebuffer[idx + 1]]);
            let rgb = rgb565_to_rgb888(px);
            img.put_pixel(x as u32, y as u32, Rgb(rgb));
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    img.save(path)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from("docs/assets/main-display");

    let mut cc = ui::UiSnapshot::demo();
    cc.set_control_overlay(1, false, LoadMode::Cc, false);
    cc.set_control_row(12_000, 'A', control::AdjustDigit::DEFAULT);
    render_snapshot(&out_dir.join("main-display-mock-cc.png"), &cc)?;

    let mut cv = ui::UiSnapshot::demo();
    cv.main_voltage = 24.50;
    cv.remote_voltage = 24.52;
    cv.local_voltage = 24.47;
    cv.set_control_overlay(1, false, LoadMode::Cv, false);
    cv.set_control_row(24_500, 'V', control::AdjustDigit::DEFAULT);
    render_snapshot(&out_dir.join("main-display-mock-cv.png"), &cv)?;

    Ok(())
}
