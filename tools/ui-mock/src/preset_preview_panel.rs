use embedded_graphics::pixelcolor::{
    Rgb565, Rgb888,
    raw::{RawData, RawU16},
};
use heapless::String;
use lcd_async::raw_framebuf::RawFrameBuf;
use loadlynx_protocol::LoadMode;

#[path = "../../../firmware/digital/src/ui/fonts.rs"]
mod ui_fonts;

const LOGICAL_WIDTH: i32 = 320;
const LOGICAL_HEIGHT: i32 = 240;

const PANEL_LEFT: i32 = 154;
const PANEL_RIGHT: i32 = 314;
const PANEL_TOP: i32 = 44;

const BORDER: i32 = 1;
const RADIUS: i32 = 6;
const PAD_X: i32 = 10;
const PAD_Y: i32 = 8;
const ROW_H: i32 = 24;
const UNIT_GAP: i32 = 1;

const COLOR_BG: u32 = 0x1c2638;
const COLOR_BORDER: u32 = 0x1c2a3f;
const COLOR_TEXT_LABEL: u32 = 0x9ab0d8;
const COLOR_TEXT_VALUE: u32 = 0xdfe7ff;

#[derive(Clone, Debug)]
pub struct PresetPreviewPanelVm {
    pub mode: LoadMode,
    pub target_text: String<8>,
    pub v_lim_text: String<8>,
    pub i_lim_text: String<8>,
    pub p_lim_text: String<8>,
}

pub fn render_preset_preview_panel(
    frame: &mut RawFrameBuf<Rgb565, &mut [u8]>,
    vm: &PresetPreviewPanelVm,
) {
    let mode = normalize_mode(vm.mode);
    let rows = match mode {
        LoadMode::Cv => 4,
        _ => 3,
    };
    let panel_h = BORDER * 2 + PAD_Y * 2 + rows * ROW_H;

    let bytes = frame.as_mut_bytes();
    let mut canvas = Canvas::new(bytes, crate::DISPLAY_WIDTH, crate::DISPLAY_HEIGHT);

    let outer = Rect::new(PANEL_LEFT, PANEL_TOP, PANEL_RIGHT, PANEL_TOP + panel_h);
    canvas.fill_round_rect(outer, RADIUS, rgb(COLOR_BORDER));

    let inner = Rect::new(
        PANEL_LEFT + BORDER,
        PANEL_TOP + BORDER,
        PANEL_RIGHT - BORDER,
        PANEL_TOP + panel_h - BORDER,
    );
    canvas.fill_round_rect(inner, (RADIUS - BORDER).max(0), rgb(COLOR_BG));

    let label_x = PANEL_LEFT + PAD_X;
    let value_right = PANEL_RIGHT - PAD_X;
    let small_h = ui_fonts::SMALL_FONT.height() as i32;
    let num_h = ui_fonts::SETPOINT_FONT.height() as i32;

    let label_color = rgb(COLOR_TEXT_LABEL);
    let value_color = rgb(COLOR_TEXT_VALUE);

    let mut row_idx = 0;
    for (label, value) in fields(mode, vm).into_iter().take(rows as usize) {
        let row_top = PANEL_TOP + BORDER + PAD_Y + row_idx * ROW_H;
        let row_bottom = row_top + ROW_H;

        let label_y = row_top + (ROW_H - small_h).max(0) / 2;
        draw_text(
            &mut canvas,
            &ui_fonts::SMALL_FONT,
            label,
            label_x,
            label_y,
            label_color,
            0,
        );

        let (num, unit) = split_value(value);
        let num_w = text_width(&ui_fonts::SETPOINT_FONT, num, 0);
        let unit_w = text_width(&ui_fonts::SMALL_FONT, unit, 0);
        let total_w = num_w + UNIT_GAP + unit_w;
        let value_x0 = (value_right - total_w).max(label_x);

        let num_y = row_top + (ROW_H - num_h).max(0) / 2;
        let unit_y = num_y + num_h - small_h;

        draw_text(
            &mut canvas,
            &ui_fonts::SETPOINT_FONT,
            num,
            value_x0,
            num_y,
            value_color,
            0,
        );
        draw_text(
            &mut canvas,
            &ui_fonts::SMALL_FONT,
            unit,
            value_x0 + num_w + UNIT_GAP,
            unit_y,
            label_color,
            0,
        );

        row_idx += 1;
        if row_idx < rows {
            canvas.fill_rect(
                Rect::new(
                    PANEL_LEFT + BORDER,
                    row_bottom,
                    PANEL_RIGHT - BORDER,
                    row_bottom + 1,
                ),
                rgb(COLOR_BORDER),
            );
        }
    }
}

fn normalize_mode(mode: LoadMode) -> LoadMode {
    match mode {
        LoadMode::Cv => LoadMode::Cv,
        _ => LoadMode::Cc,
    }
}

fn fields<'a>(mode: LoadMode, vm: &'a PresetPreviewPanelVm) -> [(&'static str, &'a str); 4] {
    match mode {
        LoadMode::Cv => [
            ("TARGET", vm.target_text.as_str()),
            ("I-LIM", vm.i_lim_text.as_str()),
            ("V-LIM", vm.v_lim_text.as_str()),
            ("P-LIM", vm.p_lim_text.as_str()),
        ],
        _ => [
            ("TARGET", vm.target_text.as_str()),
            ("V-LIM", vm.v_lim_text.as_str()),
            ("P-LIM", vm.p_lim_text.as_str()),
            ("", ""),
        ],
    }
}

fn split_value(value: &str) -> (&str, &str) {
    if value.len() < 2 {
        return ("", "");
    }
    value.split_at(value.len() - 1)
}

fn rgb(hex: u32) -> Rgb565 {
    let r = ((hex >> 16) & 0xFF) as u8;
    let g = ((hex >> 8) & 0xFF) as u8;
    let b = (hex & 0xFF) as u8;
    Rgb888::new(r, g, b).into()
}

fn text_width(font: &ui_fonts::UtftFont, text: &str, spacing: i32) -> i32 {
    let mut w = 0;
    let mut any = false;
    for _ch in text.chars() {
        any = true;
        w += font.width() as i32 + spacing;
    }
    if any { w - spacing } else { 0 }
}

fn draw_text(
    canvas: &mut Canvas,
    font: &ui_fonts::UtftFont,
    text: &str,
    mut x: i32,
    y: i32,
    color: Rgb565,
    spacing: i32,
) {
    for ch in text.chars() {
        font.draw_char(ch, |px, py| canvas.set_pixel(px + x, py + y, color), 0, 0);
        x += font.width() as i32 + spacing;
    }
}

#[derive(Copy, Clone, Debug)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Rect {
    const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

struct Canvas<'a> {
    bytes: &'a mut [u8],
    phys_width: usize,
    phys_height: usize,
}

impl<'a> Canvas<'a> {
    fn new(bytes: &'a mut [u8], phys_width: usize, phys_height: usize) -> Self {
        Self {
            bytes,
            phys_width,
            phys_height,
        }
    }

    fn set_pixel(&mut self, x: i32, y: i32, color: Rgb565) {
        if x < 0 || x >= LOGICAL_WIDTH || y < 0 || y >= LOGICAL_HEIGHT {
            return;
        }
        let actual_x = y as usize;
        let actual_y = (self.phys_height as i32 - 1 - x) as usize;
        let idx = (actual_y * self.phys_width + actual_x) * 2;
        let raw = RawU16::from(color).into_inner().to_be_bytes();
        self.bytes[idx] = raw[0];
        self.bytes[idx + 1] = raw[1];
    }

    fn fill_rect(&mut self, rect: Rect, color: Rgb565) {
        for yy in rect.top..rect.bottom {
            for xx in rect.left..rect.right {
                self.set_pixel(xx, yy, color);
            }
        }
    }

    fn fill_round_rect(&mut self, rect: Rect, radius: i32, color: Rgb565) {
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w <= 0 || h <= 0 {
            return;
        }
        let mut r = radius.max(0);
        r = r.min(w / 2).min(h / 2);
        if r == 0 {
            self.fill_rect(rect, color);
            return;
        }
        let r2 = r * r;

        let tl_cx = rect.left + r;
        let tl_cy = rect.top + r;
        let tr_cx = rect.right - r - 1;
        let tr_cy = rect.top + r;
        let bl_cx = rect.left + r;
        let bl_cy = rect.bottom - r - 1;
        let br_cx = rect.right - r - 1;
        let br_cy = rect.bottom - r - 1;

        let left_r = rect.left + r;
        let right_r = rect.right - r;
        let top_r = rect.top + r;
        let bottom_r = rect.bottom - r;

        for yy in rect.top..rect.bottom {
            for xx in rect.left..rect.right {
                let inside = if xx < left_r && yy < top_r {
                    let dx = xx - tl_cx;
                    let dy = yy - tl_cy;
                    dx * dx + dy * dy <= r2
                } else if xx >= right_r && yy < top_r {
                    let dx = xx - tr_cx;
                    let dy = yy - tr_cy;
                    dx * dx + dy * dy <= r2
                } else if xx < left_r && yy >= bottom_r {
                    let dx = xx - bl_cx;
                    let dy = yy - bl_cy;
                    dx * dx + dy * dy <= r2
                } else if xx >= right_r && yy >= bottom_r {
                    let dx = xx - br_cx;
                    let dy = yy - br_cy;
                    dx * dx + dy * dy <= r2
                } else {
                    true
                };
                if inside {
                    self.set_pixel(xx, yy, color);
                }
            }
        }
    }
}
