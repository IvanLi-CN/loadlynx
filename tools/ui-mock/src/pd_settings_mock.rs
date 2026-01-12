use std::path::Path;

use embedded_graphics::pixelcolor::Rgb565;
use lcd_async::raw_framebuf::RawFrameBuf;
use loadlynx_protocol::{FixedPdo, FixedPdoList, PpsPdo, PpsPdoList};

use crate::ui;

fn render_panel(
    path: &Path,
    vm: &ui::pd_settings::PdSettingsVm,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut framebuffer = vec![0u8; crate::DISPLAY_WIDTH * crate::DISPLAY_HEIGHT * 2];
    let mut frame = RawFrameBuf::<Rgb565, _>::new(
        &mut framebuffer[..],
        crate::DISPLAY_WIDTH,
        crate::DISPLAY_HEIGHT,
    );
    ui::pd_settings::render_pd_settings(&mut frame, vm);

    // Export as a 320×240 PNG in logical landscape space (same as other mocks).
    let mut img: image::ImageBuffer<image::Rgb<u8>, std::vec::Vec<u8>> =
        image::ImageBuffer::new(320, 240);
    for x in 0..320i32 {
        for y in 0..240i32 {
            let phys_x = y as usize;
            let phys_y = (crate::DISPLAY_HEIGHT as i32 - 1 - x) as usize;
            let idx = (phys_y * crate::DISPLAY_WIDTH + phys_x) * 2;
            let px = u16::from_be_bytes([framebuffer[idx], framebuffer[idx + 1]]);
            let rgb = crate::rgb565_to_rgb888(px);
            img.put_pixel(x as u32, y as u32, image::Rgb(rgb));
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    img.save(path)?;
    Ok(())
}

fn fixed_list() -> FixedPdoList {
    let mut out = FixedPdoList::new();
    let _ = out.push(FixedPdo {
        pos: 1,
        mv: 5_000,
        max_ma: 3_000,
    });
    let _ = out.push(FixedPdo {
        pos: 2,
        mv: 9_000,
        max_ma: 3_000,
    });
    let _ = out.push(FixedPdo {
        pos: 3,
        mv: 15_000,
        max_ma: 3_000,
    });
    let _ = out.push(FixedPdo {
        pos: 4,
        mv: 20_000,
        max_ma: 5_000,
    });
    out
}

fn pps_list_full() -> PpsPdoList {
    let mut out = PpsPdoList::new();
    let _ = out.push(PpsPdo {
        pos: 1,
        min_mv: 3_300,
        max_mv: 11_000,
        max_ma: 3_000,
    });
    let _ = out.push(PpsPdo {
        pos: 2,
        min_mv: 3_300,
        max_mv: 21_000,
        max_ma: 5_000,
    });
    let _ = out.push(PpsPdo {
        pos: 3,
        min_mv: 5_000,
        max_mv: 20_000,
        max_ma: 3_000,
    });
    out
}

fn pps_list_missing_selected() -> PpsPdoList {
    // Missing APDO2, matches the "unavailable" mock.
    let mut out = PpsPdoList::new();
    let _ = out.push(PpsPdo {
        pos: 1,
        min_mv: 3_300,
        max_mv: 11_000,
        max_ma: 3_000,
    });
    let _ = out.push(PpsPdo {
        pos: 3,
        min_mv: 5_000,
        max_mv: 20_000,
        max_ma: 3_000,
    });
    out
}

pub fn render_pd_settings_mocks(repo_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = repo_root.join("docs/assets/usb-pd-settings-panel-rendered");

    // Fixed
    let fixed = ui::pd_settings::PdSettingsVm {
        attached: true,
        mode: crate::control::PdMode::Fixed,
        focus: crate::control::PdSettingsFocus::None,
        fixed_pdos: fixed_list(),
        pps_pdos: PpsPdoList::new(),
        contract_mv: 20_000,
        contract_ma: 2_900,
        fixed_object_pos: 4,
        pps_object_pos: 0,
        pps_target_mv: 0,
        i_req_ma: 3_000,
        apply_enabled: true,
        message: ui::pd_settings::PdSettingsMessage::None,
    };
    render_panel(&out_dir.join("pd-settings-fixed.png"), &fixed)?;

    // PPS
    let pps = ui::pd_settings::PdSettingsVm {
        attached: true,
        mode: crate::control::PdMode::Pps,
        focus: crate::control::PdSettingsFocus::None,
        fixed_pdos: FixedPdoList::new(),
        pps_pdos: pps_list_full(),
        contract_mv: 9_000,
        contract_ma: 2_000,
        fixed_object_pos: 0,
        pps_object_pos: 2,
        pps_target_mv: 9_000,
        i_req_ma: 2_000,
        apply_enabled: true,
        message: ui::pd_settings::PdSettingsMessage::None,
    };
    render_panel(&out_dir.join("pd-settings-pps.png"), &pps)?;

    // PPS (Ireq selected)
    let pps_ireq_selected = ui::pd_settings::PdSettingsVm {
        focus: crate::control::PdSettingsFocus::Ireq,
        ..pps.clone()
    };
    render_panel(
        &out_dir.join("pd-settings-pps-ireq-selected.png"),
        &pps_ireq_selected,
    )?;

    // Unavailable
    let unavailable = ui::pd_settings::PdSettingsVm {
        attached: true,
        mode: crate::control::PdMode::Pps,
        focus: crate::control::PdSettingsFocus::None,
        fixed_pdos: FixedPdoList::new(),
        pps_pdos: pps_list_missing_selected(),
        contract_mv: 5_000,
        contract_ma: 0,
        fixed_object_pos: 0,
        pps_object_pos: 2,
        pps_target_mv: 0,
        i_req_ma: 0,
        apply_enabled: false,
        message: ui::pd_settings::PdSettingsMessage::Unavailable,
    };
    render_panel(&out_dir.join("pd-settings-unavailable.png"), &unavailable)?;

    Ok(())
}
