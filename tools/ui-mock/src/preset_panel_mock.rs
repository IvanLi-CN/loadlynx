use embedded_graphics::pixelcolor::{
    Rgb565, Rgb888,
    raw::{RawData, RawU16},
};
use heapless::String;
use lcd_async::raw_framebuf::RawFrameBuf;
use loadlynx_protocol::LoadMode;

use crate::ui::preset_panel::{PresetPanelDigit, PresetPanelField};

#[path = "../../../firmware/digital/src/ui/fonts.rs"]
mod ui_fonts;

const LOGICAL_WIDTH: i32 = 320;
const LOGICAL_HEIGHT: i32 = 240;

const PANEL_LEFT: i32 = 20;
const PANEL_TOP: i32 = 40;
const PANEL_RIGHT: i32 = 300;
const PANEL_BOTTOM: i32 = 220;

const BORDER: i32 = 1;
const PAD_X: i32 = 10;
const PAD_Y: i32 = 10;

const ACTION_DIVIDER_Y: i32 = 185;
const ACTION_TOP: i32 = 186;

const TAB_W: i32 = 34;
const TAB_H: i32 = 20;
const TAB_GAP: i32 = 6;
const TAB_RADIUS: i32 = 6;

const ROW_H: i32 = 26;
const ROW_GAP: i32 = 0;

const VALUE_PILL_RADIUS: i32 = 8;
const UNIT_GAP: i32 = 1;

const SAVE_TEXT: &str = "SAVE";
const LOAD_LABEL: &str = "LOAD";

const COLOR_BG_HEADER: u32 = 0x141d2f;
const COLOR_BG_BODY: u32 = 0x171f33;
const COLOR_DIVIDER: u32 = 0x1c2a3f;
const COLOR_TAB_BG: u32 = 0x1c2638;
const COLOR_PILL_BG: u32 = 0x19243a;
const COLOR_TEXT_LABEL: u32 = 0x9ab0d8;
const COLOR_TEXT_VALUE: u32 = 0xdfe7ff;
const COLOR_TEXT_DARK: u32 = 0x080f19;
const COLOR_THEME: u32 = 0x4cc9f0;
const COLOR_MODE_CV: u32 = 0xffb24a;
const COLOR_MODE_CC: u32 = 0xff5252;
const COLOR_MODE_OFF: u32 = 0x7a7f8c;

const COLOR_LOAD_TRACK_OFF: u32 = 0x4a1824;
const COLOR_LOAD_TRACK_ON: u32 = 0x134b2d;
const COLOR_LOAD_THUMB_OFF: u32 = 0x555f75;

#[derive(Clone, Debug)]
pub struct PresetPanelVm {
    pub active_preset_id: u8,
    pub editing_preset_id: u8,
    pub editing_mode: LoadMode,
    pub load_enabled: bool,
    pub blocked_save: bool,
    pub dirty: bool,
    pub selected_field: PresetPanelField,
    pub selected_digit: PresetPanelDigit,
    pub target_text: String<8>,
    pub v_lim_text: String<8>,
    pub i_lim_text: String<8>,
    pub p_lim_text: String<8>,
}

pub fn render_preset_panel(frame: &mut RawFrameBuf<Rgb565, &mut [u8]>, vm: &PresetPanelVm) {
    let bytes = frame.as_mut_bytes();
    let mut canvas = Canvas::new(bytes, crate::DISPLAY_WIDTH, crate::DISPLAY_HEIGHT);

    draw_panel_background(&mut canvas);
    draw_tabs(&mut canvas, vm);
    draw_fields(&mut canvas, vm);
    draw_action_row(&mut canvas, vm);

    if vm.blocked_save {
        draw_blocked_save_copy(&mut canvas);
    }
}

fn draw_panel_background(canvas: &mut Canvas) {
    let outer = Rect::new(PANEL_LEFT, PANEL_TOP, PANEL_RIGHT, PANEL_BOTTOM);
    canvas.fill_rect(outer, rgb(COLOR_DIVIDER));

    let inner = Rect::new(
        PANEL_LEFT + BORDER,
        PANEL_TOP + BORDER,
        PANEL_RIGHT - BORDER,
        PANEL_BOTTOM - BORDER,
    );
    canvas.fill_rect(inner, rgb(COLOR_BG_BODY));

    // Action row background + divider.
    canvas.fill_rect(
        Rect::new(inner.left, ACTION_TOP, inner.right, inner.bottom),
        rgb(COLOR_BG_HEADER),
    );
    canvas.fill_rect(
        Rect::new(
            inner.left,
            ACTION_DIVIDER_Y,
            inner.right,
            ACTION_DIVIDER_Y + 1,
        ),
        rgb(COLOR_DIVIDER),
    );

    // Vertical divider between tabs and fields.
    let x = tabs_right() + 8;
    canvas.fill_rect(
        Rect::new(x, inner.top, x + 1, ACTION_DIVIDER_Y),
        rgb(COLOR_DIVIDER),
    );
}

fn draw_tabs(canvas: &mut Canvas, vm: &PresetPanelVm) {
    let tabs_h = TAB_H * 5 + TAB_GAP * 4;
    let avail_h = (ACTION_DIVIDER_Y - panel_inner_top()).max(0);
    let top_pad = ((avail_h - tabs_h) / 2).max(0);
    let mut y = panel_inner_top() + top_pad;

    for preset_id in 1u8..=5u8 {
        let rect = Rect::new(tabs_left(), y, tabs_right(), y + TAB_H);
        let selected = preset_id == vm.editing_preset_id;

        let fill = if selected { COLOR_THEME } else { COLOR_TAB_BG };
        canvas.fill_round_rect(rect, TAB_RADIUS, rgb(fill));
        draw_rect_outline(canvas, rect, rgb(COLOR_DIVIDER));

        if vm.active_preset_id == preset_id && vm.active_preset_id != vm.editing_preset_id {
            let marker = Rect::new(
                rect.right - 6,
                rect.top + 2,
                rect.right - 2,
                rect.bottom - 2,
            );
            canvas.fill_round_rect(marker, 2, rgb(COLOR_THEME));
        }

        let label = preset_label(preset_id);
        let text_w = text_width(&ui_fonts::SMALL_FONT, label.as_str(), 0);
        let x = rect.left + ((rect.right - rect.left) - text_w).max(0) / 2;
        let y_text = rect.top + ((TAB_H - ui_fonts::SMALL_FONT.height() as i32).max(0)) / 2;
        let color = if selected {
            rgb(COLOR_TEXT_DARK)
        } else {
            rgb(COLOR_TEXT_VALUE)
        };
        draw_text(
            canvas,
            &ui_fonts::SMALL_FONT,
            label.as_str(),
            x,
            y_text,
            color,
            0,
        );

        y += TAB_H + TAB_GAP;
    }
}

fn draw_fields(canvas: &mut Canvas, vm: &PresetPanelVm) {
    let mode = normalize_mode(vm.editing_mode);
    let label_x = fields_left();

    let rows = [
        (PresetPanelField::Mode, "MODE"),
        (PresetPanelField::Target, "TARGET"),
        (PresetPanelField::VLim, "UVLO"),
        (PresetPanelField::ILim, "OCP"),
        (PresetPanelField::PLim, "OPP"),
    ];

    let mut row_top = fields_top();
    for (idx, (field, label)) in rows.iter().enumerate() {
        let row_bottom = row_top + ROW_H;
        let label_y = row_top + ((ROW_H - ui_fonts::SMALL_FONT.height() as i32).max(0)) / 2;

        draw_text(
            canvas,
            &ui_fonts::SMALL_FONT,
            label,
            label_x,
            label_y,
            rgb(COLOR_TEXT_LABEL),
            0,
        );

        match *field {
            PresetPanelField::Mode => {
                draw_mode_segmented(canvas, row_top, mode, vm.selected_field == *field);
            }
            PresetPanelField::Target => {
                draw_value_row(
                    canvas,
                    row_top,
                    vm.target_text.as_str(),
                    vm.selected_field == *field,
                    vm.selected_digit,
                    false,
                );
            }
            PresetPanelField::VLim => {
                draw_value_row(
                    canvas,
                    row_top,
                    vm.v_lim_text.as_str(),
                    vm.selected_field == *field,
                    vm.selected_digit,
                    false,
                );
            }
            PresetPanelField::ILim => {
                draw_value_row(
                    canvas,
                    row_top,
                    vm.i_lim_text.as_str(),
                    vm.selected_field == *field,
                    vm.selected_digit,
                    false,
                );
            }
            PresetPanelField::PLim => {
                draw_value_row(
                    canvas,
                    row_top,
                    vm.p_lim_text.as_str(),
                    vm.selected_field == *field,
                    vm.selected_digit,
                    true,
                );
            }
        }

        if idx + 1 < rows.len() {
            canvas.fill_rect(
                Rect::new(
                    panel_inner_left(),
                    row_bottom,
                    panel_inner_right(),
                    row_bottom + 1,
                ),
                rgb(COLOR_DIVIDER),
            );
        }

        row_top = row_bottom + ROW_GAP;
    }
}

fn draw_value_row(
    canvas: &mut Canvas,
    row_top: i32,
    value: &str,
    selected: bool,
    digit: PresetPanelDigit,
    is_power: bool,
) {
    let pill = value_pill_rect(row_top);
    canvas.fill_round_rect(pill, VALUE_PILL_RADIUS, rgb(COLOR_PILL_BG));
    draw_rect_outline(canvas, pill, rgb(COLOR_DIVIDER));
    if selected {
        draw_rect_outline(canvas, pill, rgb(COLOR_THEME));
    }

    let (num, unit) = split_value(value);
    let num_w = text_width(&ui_fonts::SETPOINT_FONT, num, 0);
    let unit_w = text_width(&ui_fonts::SMALL_FONT, unit, 0);
    let total_w = num_w + UNIT_GAP + unit_w;

    let value_right = pill.right - 8;
    let value_x0 = (value_right - total_w).max(pill.left + 6);

    let num_h = ui_fonts::SETPOINT_FONT.height() as i32;
    let small_h = ui_fonts::SMALL_FONT.height() as i32;
    let num_y = row_top + ((ROW_H - num_h).max(0)) / 2;
    let unit_y = num_y + num_h - small_h;

    draw_text(
        canvas,
        &ui_fonts::SETPOINT_FONT,
        num,
        value_x0,
        num_y,
        rgb(COLOR_TEXT_VALUE),
        0,
    );
    draw_text(
        canvas,
        &ui_fonts::SMALL_FONT,
        unit,
        value_x0 + num_w + UNIT_GAP,
        unit_y,
        rgb(COLOR_TEXT_LABEL),
        0,
    );

    if selected {
        if let Some(idx) = selected_digit_index(digit, is_power) {
            let glyph_w = ui_fonts::SETPOINT_FONT.width() as i32;
            let cell_x = value_x0 + idx as i32 * glyph_w;
            let underline_top = (num_y + num_h + 1).min(row_top + ROW_H - 3);
            let underline_bottom = underline_top + 2;
            canvas.fill_rect(
                Rect::new(
                    cell_x + 1,
                    underline_top,
                    cell_x + glyph_w - 1,
                    underline_bottom,
                ),
                rgb(COLOR_THEME),
            );
        }
    }
}

fn draw_mode_segmented(canvas: &mut Canvas, row_top: i32, mode: LoadMode, selected: bool) {
    let rect = mode_pill_rect(row_top);
    canvas.fill_round_rect(rect, 8, rgb(COLOR_PILL_BG));
    draw_rect_outline(canvas, rect, rgb(COLOR_DIVIDER));
    if selected {
        draw_rect_outline(canvas, rect, rgb(COLOR_THEME));
    }

    let sep = rect.left + (rect.right - rect.left) / 2;
    canvas.fill_rect(
        Rect::new(sep, rect.top + 2, sep + 1, rect.bottom - 2),
        rgb(COLOR_DIVIDER),
    );

    let (cv_color, cc_color) = match mode {
        LoadMode::Cv => (rgb(COLOR_MODE_CV), rgb(COLOR_MODE_OFF)),
        _ => (rgb(COLOR_MODE_OFF), rgb(COLOR_MODE_CC)),
    };

    let cv_w = text_width(&ui_fonts::SMALL_FONT, "CV", 0);
    let cc_w = text_width(&ui_fonts::SMALL_FONT, "CC", 0);
    let small_h = ui_fonts::SMALL_FONT.height() as i32;
    let text_y = rect.top + ((rect.bottom - rect.top) - small_h).max(0) / 2;

    let left_x0 = rect.left + 2;
    let left_x1 = sep - 2;
    let right_x0 = sep + 2;
    let right_x1 = rect.right - 2;

    let cv_x = left_x0 + ((left_x1 - left_x0) - cv_w).max(0) / 2;
    let cc_x = right_x0 + ((right_x1 - right_x0) - cc_w).max(0) / 2;

    draw_text(
        canvas,
        &ui_fonts::SMALL_FONT,
        "CV",
        cv_x,
        text_y,
        cv_color,
        0,
    );
    draw_text(
        canvas,
        &ui_fonts::SMALL_FONT,
        "CC",
        cc_x,
        text_y,
        cc_color,
        0,
    );
}

fn draw_action_row(canvas: &mut Canvas, vm: &PresetPanelVm) {
    let label_x = fields_left();
    let label_y = ACTION_TOP + 6;
    draw_text(
        canvas,
        &ui_fonts::SMALL_FONT,
        LOAD_LABEL,
        label_x,
        label_y,
        rgb(COLOR_TEXT_LABEL),
        0,
    );

    draw_load_switch(canvas, vm.load_enabled, label_x + 44, ACTION_TOP + 8);

    let save = save_rect();
    canvas.fill_rect(save, rgb(COLOR_DIVIDER));
    let save_inner = Rect::new(save.left + 1, save.top + 1, save.right - 1, save.bottom - 1);

    if vm.dirty && !vm.blocked_save {
        canvas.fill_rect(save_inner, rgb(COLOR_THEME));
        let w = text_width(&ui_fonts::SMALL_FONT, SAVE_TEXT, 0);
        let x = save.left + ((save.right - save.left) - w).max(0) / 2;
        let y = save.top + 7;
        draw_text(
            canvas,
            &ui_fonts::SMALL_FONT,
            SAVE_TEXT,
            x,
            y,
            rgb(COLOR_TEXT_DARK),
            0,
        );
    } else {
        canvas.fill_rect(save_inner, rgb(COLOR_PILL_BG));
        let w = text_width(&ui_fonts::SMALL_FONT, SAVE_TEXT, 0);
        let x = save.left + ((save.right - save.left) - w).max(0) / 2;
        let y = save.top + 7;
        draw_text(
            canvas,
            &ui_fonts::SMALL_FONT,
            SAVE_TEXT,
            x,
            y,
            rgb(COLOR_TEXT_LABEL),
            0,
        );
    }
}

fn draw_load_switch(canvas: &mut Canvas, enabled: bool, x: i32, y: i32) {
    let rect = Rect::new(x, y, x + 26, y + 12);
    canvas.fill_round_rect(rect, 6, rgb(COLOR_DIVIDER));
    let track_rect = Rect::new(rect.left + 1, rect.top + 1, rect.right - 1, rect.bottom - 1);
    let track_color = if enabled {
        rgb(COLOR_LOAD_TRACK_ON)
    } else {
        rgb(COLOR_LOAD_TRACK_OFF)
    };
    canvas.fill_round_rect(track_rect, 5, track_color);

    let r = 5;
    let cx = if enabled {
        track_rect.right - 1 - r
    } else {
        track_rect.left + r
    };
    let cy = track_rect.top + r;
    let thumb_color = if enabled {
        rgb(COLOR_THEME)
    } else {
        rgb(COLOR_LOAD_THUMB_OFF)
    };
    fill_circle(canvas, cx, cy, r, thumb_color);
}

fn draw_blocked_save_copy(canvas: &mut Canvas) {
    const LINE1: &str = "SAVE FAILED";
    const LINE2: &str = "RETRY SAVE";

    let w1 = text_width(&ui_fonts::SMALL_FONT, LINE1, 0);
    let w2 = text_width(&ui_fonts::SMALL_FONT, LINE2, 0);
    let w = w1.max(w2) + 16;
    let h = (ui_fonts::SMALL_FONT.height() as i32) * 2 + 8;
    let x = (PANEL_LEFT + PANEL_RIGHT - w) / 2;
    let y = ACTION_DIVIDER_Y - h - 10;

    let box_rect = Rect::new(x, y, x + w, y + h);
    canvas.fill_rect(box_rect, rgb(COLOR_PILL_BG));
    draw_rect_outline(canvas, box_rect, rgb(COLOR_DIVIDER));

    let line1_x = x + (w - w1) / 2;
    let line1_y = y + 4;
    draw_text(
        canvas,
        &ui_fonts::SMALL_FONT,
        LINE1,
        line1_x,
        line1_y,
        rgb(COLOR_MODE_CC),
        0,
    );
    let line2_x = x + (w - w2) / 2;
    let line2_y = line1_y + ui_fonts::SMALL_FONT.height() as i32;
    draw_text(
        canvas,
        &ui_fonts::SMALL_FONT,
        LINE2,
        line2_x,
        line2_y,
        rgb(COLOR_TEXT_VALUE),
        0,
    );
}

fn panel_inner_left() -> i32 {
    PANEL_LEFT + BORDER
}

fn panel_inner_right() -> i32 {
    PANEL_RIGHT - BORDER
}

fn panel_inner_top() -> i32 {
    PANEL_TOP + BORDER
}

fn fields_left() -> i32 {
    tabs_right() + 8 + 10
}

fn fields_top() -> i32 {
    panel_inner_top() + PAD_Y
}

fn tabs_left() -> i32 {
    panel_inner_left() + 3
}

fn tabs_right() -> i32 {
    tabs_left() + TAB_W
}

fn mode_pill_rect(row_top: i32) -> Rect {
    let rect = value_pill_rect(row_top);
    Rect::new(rect.left, rect.top + 3, rect.right, rect.bottom - 3)
}

fn value_pill_rect(row_top: i32) -> Rect {
    let top = row_top + 2;
    let bottom = row_top + ROW_H - 2;
    Rect::new(fields_left() + 70, top, panel_inner_right() - PAD_X, bottom)
}

fn save_rect() -> Rect {
    Rect::new(
        PANEL_RIGHT - 84,
        ACTION_TOP + 2,
        PANEL_RIGHT - 12,
        ACTION_TOP + 26,
    )
}

fn preset_label(preset_id: u8) -> String<3> {
    let mut out = String::<3>::new();
    let _ = out.push('M');
    if (1..=9).contains(&preset_id) {
        let _ = out.push((b'0' + preset_id) as char);
    } else {
        let _ = out.push('?');
    }
    out
}

fn split_value(value: &str) -> (&str, &str) {
    if value.len() < 2 {
        return ("", "");
    }
    value.split_at(value.len() - 1)
}

fn normalize_mode(mode: LoadMode) -> LoadMode {
    match mode {
        LoadMode::Cv => LoadMode::Cv,
        _ => LoadMode::Cc,
    }
}

fn selected_digit_index(digit: PresetPanelDigit, is_power: bool) -> Option<usize> {
    if is_power {
        match digit {
            PresetPanelDigit::Tens => Some(1),
            PresetPanelDigit::Ones => Some(2),
            PresetPanelDigit::Tenths => Some(4),
            PresetPanelDigit::Hundredths => Some(5),
            _ => None,
        }
    } else {
        match digit {
            PresetPanelDigit::Ones => Some(1),
            PresetPanelDigit::Tenths => Some(3),
            PresetPanelDigit::Hundredths => Some(4),
            PresetPanelDigit::Thousandths => Some(5),
            _ => None,
        }
    }
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

fn draw_rect_outline(canvas: &mut Canvas, rect: Rect, color: Rgb565) {
    if rect.right - rect.left <= 0 || rect.bottom - rect.top <= 0 {
        return;
    }
    let left = rect.left;
    let right = rect.right - 1;
    let top = rect.top;
    let bottom = rect.bottom - 1;
    canvas.draw_line(left, top, right, top, color);
    canvas.draw_line(left, bottom, right, bottom, color);
    canvas.draw_line(left, top, left, bottom, color);
    canvas.draw_line(right, top, right, bottom, color);
}

fn fill_circle(canvas: &mut Canvas, cx: i32, cy: i32, r: i32, color: Rgb565) {
    let r2 = r * r;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r2 {
                canvas.set_pixel(cx + dx, cy + dy, color);
            }
        }
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

    fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgb565) {
        let mut x0 = x0;
        let mut y0 = y0;
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.set_pixel(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}
