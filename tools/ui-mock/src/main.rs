use std::path::{Path, PathBuf};

use embedded_graphics::pixelcolor::Rgb565;
use image::{ImageBuffer, Rgb};
use lcd_async::raw_framebuf::RawFrameBuf;
use loadlynx_protocol::LoadMode;

pub const DISPLAY_WIDTH: usize = 240;
pub const DISPLAY_HEIGHT: usize = 320;

mod preset_panel_mock;
mod preset_preview_panel;

mod control {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum AdjustDigit {
        Ones,
        Tenths,
        Hundredths,
        Thousandths,
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
    preset_panel_vm: Option<&preset_panel_mock::PresetPanelVm>,
    preset_preview_vm: Option<&preset_preview_panel::PresetPreviewPanelVm>,
    fps_overlay: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut snapshot = snapshot.clone();
    snapshot.update_strings();

    let mut framebuffer = vec![0u8; DISPLAY_WIDTH * DISPLAY_HEIGHT * 2];
    let mut frame =
        RawFrameBuf::<Rgb565, _>::new(&mut framebuffer[..], DISPLAY_WIDTH, DISPLAY_HEIGHT);
    ui::render(&mut frame, &snapshot);
    if let Some(vm) = preset_panel_vm {
        preset_panel_mock::render_preset_panel(&mut frame, vm);
    }
    if let Some(vm) = preset_preview_vm {
        preset_preview_panel::render_preset_preview_panel(&mut frame, vm);
    }
    if let Some(fps) = fps_overlay {
        ui::render_fps_overlay(&mut frame, fps);
    }

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

fn render_pd_toggle_mocks(repo_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = repo_root.join("docs/assets/usb-pd-sink-toggle");

    let mut base = ui::UiSnapshot::demo();
    base.main_voltage = 20.07;
    base.main_current = 0.09;
    base.main_power = 1.9;
    base.remote_voltage = 20.08;
    base.local_voltage = 20.09;
    base.set_control_overlay(2, false, LoadMode::Cc, false, true, false, true, None, None);
    base.set_control_row(3_000, 'A', control::AdjustDigit::DEFAULT);
    base.run_time.clear();
    let _ = base.run_time.push_str("01:03:48");
    base.sink_core_temp = 18.0;
    base.sink_exhaust_temp = 17.8;
    base.mcu_temp = 35.0;
    base.pd_desired_mv = 20_000;
    base.pd_20v_available = true;

    let mut standby = base.clone();
    standby.pd_state = ui::PdButtonState::Standby;
    render_snapshot(
        &out_dir.join("dashboard-pd-standby-20v.png"),
        &standby,
        None,
        None,
        Some(7),
    )?;

    let mut active = base;
    active.pd_state = ui::PdButtonState::Active;
    render_snapshot(
        &out_dir.join("dashboard-pd-active-20v.png"),
        &active,
        None,
        None,
        Some(7),
    )?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let mode = std::env::args().nth(1);
    if mode.as_deref() == Some("pd") {
        return render_pd_toggle_mocks(&repo_root);
    }
    let out_dir = repo_root.join("docs/assets/main-display");
    let preset_dir = repo_root.join("docs/assets/on-device-preset-ui");

    let mut cc = ui::UiSnapshot::demo();
    cc.set_control_overlay(2, true, LoadMode::Cc, false, true, false, true, None, None);
    cc.set_control_row(12_000, 'A', control::AdjustDigit::DEFAULT);
    render_snapshot(
        &out_dir.join("main-display-mock-cc.png"),
        &cc,
        None,
        None,
        None,
    )?;
    render_snapshot(&preset_dir.join("dashboard.png"), &cc, None, None, None)?;

    let mut cc_blocked_lnk = cc.clone();
    cc_blocked_lnk.set_control_overlay(
        2,
        false,
        LoadMode::Cc,
        false,
        false,
        false,
        true,
        None,
        None,
    );
    render_snapshot(
        &preset_dir.join("dashboard-blocked-lnk.png"),
        &cc_blocked_lnk,
        None,
        None,
        None,
    )?;

    let mut cc_blocked_uv = cc.clone();
    cc_blocked_uv.set_control_overlay(
        2,
        false,
        LoadMode::Cc,
        true,
        true,
        false,
        true,
        Some("UVLO"),
        None,
    );
    render_snapshot(
        &preset_dir.join("dashboard-blocked-uv.png"),
        &cc_blocked_uv,
        None,
        None,
        None,
    )?;

    let vm_off = preset_panel_mock::PresetPanelVm {
        active_preset_id: 2,
        editing_preset_id: 2,
        editing_mode: LoadMode::Cc,
        load_enabled: false,
        blocked_save: false,
        dirty: false,
        selected_field: ui::preset_panel::PresetPanelField::Target,
        selected_digit: ui::preset_panel::PresetPanelDigit::Tenths,
        target_text: ui::preset_panel::format_av_3dp(12_000, 'A'),
        v_lim_text: ui::preset_panel::format_av_3dp(10_000, 'V'),
        i_lim_text: ui::preset_panel::format_av_3dp(15_000, 'A'),
        p_lim_text: ui::preset_panel::format_power_2dp(300_000),
    };
    render_snapshot(
        &preset_dir.join("preset-panel-output-off.png"),
        &cc,
        Some(&vm_off),
        None,
        None,
    )?;

    let mut cc_active_other = cc.clone();
    cc_active_other.set_control_overlay(
        4,
        false,
        LoadMode::Cc,
        false,
        true,
        false,
        true,
        None,
        None,
    );

    let vm_on = preset_panel_mock::PresetPanelVm {
        active_preset_id: 4,
        load_enabled: true,
        ..vm_off
    };
    render_snapshot(
        &preset_dir.join("preset-panel-output-on.png"),
        &cc_active_other,
        Some(&vm_on),
        None,
        None,
    )?;

    let preview_cc = preset_preview_panel::PresetPreviewPanelVm {
        preset_id: 2,
        mode: LoadMode::Cc,
        target_text: ui::preset_panel::format_av_3dp(12_000, 'A'),
        v_lim_text: ui::preset_panel::format_av_3dp(10_000, 'V'),
        i_lim_text: ui::preset_panel::format_av_3dp(15_000, 'A'),
        p_lim_text: ui::preset_panel::format_power_2dp(300_000),
    };
    render_snapshot(
        &preset_dir.join("preset-preview-panel-cc.png"),
        &cc,
        None,
        Some(&preview_cc),
        None,
    )?;

    let mut cv = ui::UiSnapshot::demo();
    cv.main_voltage = 24.50;
    cv.remote_voltage = 24.52;
    cv.local_voltage = 24.47;
    cv.set_control_overlay(2, false, LoadMode::Cv, false, true, false, true, None, None);
    cv.set_control_row(24_500, 'V', control::AdjustDigit::DEFAULT);
    render_snapshot(
        &out_dir.join("main-display-mock-cv.png"),
        &cv,
        None,
        None,
        None,
    )?;

    let preview_cv = preset_preview_panel::PresetPreviewPanelVm {
        preset_id: 2,
        mode: LoadMode::Cv,
        target_text: ui::preset_panel::format_av_3dp(24_500, 'V'),
        v_lim_text: ui::preset_panel::format_av_3dp(10_000, 'V'),
        i_lim_text: ui::preset_panel::format_av_3dp(12_000, 'A'),
        p_lim_text: ui::preset_panel::format_power_2dp(300_000),
    };
    render_snapshot(
        &preset_dir.join("preset-preview-panel-cv.png"),
        &cv,
        None,
        Some(&preview_cv),
        None,
    )?;

    Ok(())
}
