#![allow(dead_code)]

use embedded_graphics::pixelcolor::Rgb565;
use heapless::String;
use lcd_async::raw_framebuf::RawFrameBuf;
use loadlynx_protocol::LoadMode;

use crate::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

use super::fonts::{SETPOINT_FONT, SMALL_FONT};
use super::{Canvas, Rect, rgb, small_text_width};

// Mirror `tools/ui-mock/src/preset_panel_mock.rs` for pixel-perfect layout.
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

const LOAD_LABEL: &str = "LOAD";
const SAVE_FAILED_LINE1: &str = "SAVE FAILED";
const SAVE_FAILED_LINE2: &str = "RETRY SAVE";

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PresetPanelField {
    Mode,
    Target,
    VLim,
    ILim,
    PLim,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PresetPanelDigit {
    Tens,
    Ones,
    Tenths,
    Hundredths,
    Thousandths,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PresetPanelHit {
    Tab(u8),
    ModeCv,
    ModeCc,
    Target,
    VLim,
    ILim,
    PLim,
    LoadToggle,
    Save,
}

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
    let mut canvas = Canvas::new(bytes, DISPLAY_WIDTH, DISPLAY_HEIGHT);

    draw_panel_background(&mut canvas);
    draw_tabs(&mut canvas, vm);
    draw_fields(&mut canvas, vm);
    draw_action_row(&mut canvas, vm);

    if vm.blocked_save {
        draw_blocked_save_copy(&mut canvas);
    }
}

pub fn hit_test_preset_panel(x: i32, y: i32, vm: &PresetPanelVm) -> Option<PresetPanelHit> {
    if x < PANEL_LEFT || x >= PANEL_RIGHT || y < PANEL_TOP || y >= PANEL_BOTTOM {
        return None;
    }

    if vm.blocked_save {
        if hit_in_rect(x, y, save_rect()) {
            return Some(PresetPanelHit::Save);
        }
        return None;
    }

    for preset_id in 1u8..=5u8 {
        if hit_in_rect(x, y, tab_rect(preset_id)) {
            return Some(PresetPanelHit::Tab(preset_id));
        }
    }

    // Treat the whole MODE row as the hit target (not just the text),
    // so the segmented control remains easy to use on hardware.
    let mode_row_top = row_top(PresetPanelField::Mode);
    let mode_pill = mode_pill_rect(mode_row_top);
    let mode_hit = Rect::new(
        fields_left() - 6,
        mode_row_top,
        panel_inner_right() - PAD_X + 6,
        mode_row_top + ROW_H,
    );
    if hit_in_rect(x, y, mode_hit) {
        let sep = mode_pill.left + (mode_pill.right - mode_pill.left) / 2;
        return if x < sep {
            Some(PresetPanelHit::ModeCv)
        } else {
            Some(PresetPanelHit::ModeCc)
        };
    }

    if hit_in_rect(x, y, load_hit_rect()) {
        return Some(PresetPanelHit::LoadToggle);
    }
    if hit_in_rect(x, y, save_rect()) {
        return Some(PresetPanelHit::Save);
    }

    if hit_in_rect(x, y, row_hit_rect(PresetPanelField::Target)) {
        return Some(PresetPanelHit::Target);
    }
    if hit_in_rect(x, y, row_hit_rect(PresetPanelField::VLim)) {
        return Some(PresetPanelHit::VLim);
    }
    if hit_in_rect(x, y, row_hit_rect(PresetPanelField::ILim)) {
        return Some(PresetPanelHit::ILim);
    }
    if hit_in_rect(x, y, row_hit_rect(PresetPanelField::PLim)) {
        return Some(PresetPanelHit::PLim);
    }

    None
}

pub fn format_av_3dp(value_milli: i32, unit: char) -> String<8> {
    let clamped = value_milli.clamp(0, 99_999) as u32;
    let int_part = clamped / 1000;
    let frac = clamped % 1000;

    let mut out = String::<8>::new();
    append_u32_2dp(&mut out, int_part);
    let _ = out.push('.');
    append_u32_fixed(&mut out, frac, 3);
    let _ = out.push(unit);
    out
}

pub fn format_power_2dp(value_mw: i32) -> String<8> {
    let mw = value_mw.max(0) as u32;
    let centi_w = (mw + 5) / 10;
    let centi_w = centi_w.min(99_999);
    let int_part = centi_w / 100;
    let frac = centi_w % 100;

    let mut out = String::<8>::new();
    append_u32_fixed(&mut out, int_part, 3);
    let _ = out.push('.');
    append_u32_fixed(&mut out, frac, 2);
    let _ = out.push('W');
    out
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

        let mut label = String::<3>::new();
        let _ = label.push('M');
        let _ = label.push(char::from(b'0' + preset_id));
        let text_w = small_text_width(label.as_str(), 0);
        let x = rect.left + ((rect.right - rect.left) - text_w).max(0) / 2;
        let y_text = rect.top + ((TAB_H - SMALL_FONT.height() as i32).max(0)) / 2;
        let color = if selected {
            rgb(COLOR_TEXT_DARK)
        } else {
            rgb(COLOR_TEXT_VALUE)
        };
        super::draw_small_text(canvas, label.as_str(), x, y_text, color, 0);

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
        let label_y = row_top + ((ROW_H - SMALL_FONT.height() as i32).max(0)) / 2;
        super::draw_small_text(canvas, label, label_x, label_y, rgb(COLOR_TEXT_LABEL), 0);

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
            let divider_x = tabs_right() + 8;
            canvas.fill_rect(
                Rect::new(divider_x, row_bottom, panel_inner_right(), row_bottom + 1),
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
    let num_w = super::setpoint_text_width(num, 0);
    let unit_w = small_text_width(unit, 0);
    let total_w = num_w + UNIT_GAP + unit_w;

    let value_right = pill.right - 8;
    let value_x0 = (value_right - total_w).max(pill.left + 6);

    let num_h = SETPOINT_FONT.height() as i32;
    let small_h = SMALL_FONT.height() as i32;
    let num_y = row_top + ((ROW_H - num_h).max(0)) / 2;
    let unit_y = num_y + num_h - small_h;

    super::draw_setpoint_text(canvas, num, value_x0, num_y, rgb(COLOR_TEXT_VALUE), 0);
    super::draw_small_text(
        canvas,
        unit,
        value_x0 + num_w + UNIT_GAP,
        unit_y,
        rgb(COLOR_TEXT_LABEL),
        0,
    );

    if selected {
        if let Some(idx) = selected_digit_index(digit, is_power) {
            let glyph_w = SETPOINT_FONT.width() as i32;
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
    canvas.fill_round_rect(rect, VALUE_PILL_RADIUS, rgb(COLOR_PILL_BG));
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
        LoadMode::Cc => (rgb(COLOR_MODE_OFF), rgb(COLOR_MODE_CC)),
        LoadMode::Reserved(_) => (rgb(COLOR_MODE_OFF), rgb(COLOR_MODE_CC)),
    };

    let cv_w = small_text_width("CV", 0);
    let cc_w = small_text_width("CC", 0);
    let small_h = SMALL_FONT.height() as i32;
    let text_y = rect.top + ((rect.bottom - rect.top) - small_h).max(0) / 2;

    let left_x0 = rect.left + 2;
    let left_x1 = sep - 2;
    let right_x0 = sep + 2;
    let right_x1 = rect.right - 2;
    let cv_x = left_x0 + ((left_x1 - left_x0) - cv_w).max(0) / 2;
    let cc_x = right_x0 + ((right_x1 - right_x0) - cc_w).max(0) / 2;
    super::draw_small_text(canvas, "CV", cv_x, text_y, cv_color, 0);
    super::draw_small_text(canvas, "CC", cc_x, text_y, cc_color, 0);
}

fn draw_action_row(canvas: &mut Canvas, vm: &PresetPanelVm) {
    let label_x = fields_left();
    let label_y = ACTION_TOP + 6;
    super::draw_small_text(
        canvas,
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
    let enabled = vm.dirty || vm.blocked_save;
    if enabled {
        canvas.fill_rect(save_inner, rgb(COLOR_THEME));
        let w = small_text_width(SAVE_TEXT, 0);
        let x = save.left + ((save.right - save.left) - w).max(0) / 2;
        let y = save.top + 7;
        super::draw_small_text(canvas, SAVE_TEXT, x, y, rgb(COLOR_TEXT_DARK), 0);
    } else {
        canvas.fill_rect(save_inner, rgb(COLOR_PILL_BG));
        let w = small_text_width(SAVE_TEXT, 0);
        let x = save.left + ((save.right - save.left) - w).max(0) / 2;
        let y = save.top + 7;
        super::draw_small_text(canvas, SAVE_TEXT, x, y, rgb(COLOR_TEXT_LABEL), 0);
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
    let w1 = small_text_width(SAVE_FAILED_LINE1, 0);
    let w2 = small_text_width(SAVE_FAILED_LINE2, 0);
    let w = w1.max(w2) + 16;
    let h = (SMALL_FONT.height() as i32) * 2 + 8;
    let x = (PANEL_LEFT + PANEL_RIGHT - w) / 2;
    let y = ACTION_DIVIDER_Y - h - 10;

    let box_rect = Rect::new(x, y, x + w, y + h);
    canvas.fill_rect(box_rect, rgb(COLOR_PILL_BG));
    draw_rect_outline(canvas, box_rect, rgb(COLOR_DIVIDER));

    let line1_x = x + (w - w1) / 2;
    let line1_y = y + 4;
    super::draw_small_text(
        canvas,
        SAVE_FAILED_LINE1,
        line1_x,
        line1_y,
        rgb(COLOR_MODE_CC),
        0,
    );
    let line2_x = x + (w - w2) / 2;
    let line2_y = line1_y + SMALL_FONT.height() as i32;
    super::draw_small_text(
        canvas,
        SAVE_FAILED_LINE2,
        line2_x,
        line2_y,
        rgb(COLOR_TEXT_VALUE),
        0,
    );
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

    // Two-column layout: tabs column (left) + content column (right).
    let divider_x = tabs_right() + 8;

    // Tabs column background spans full panel height (including action row).
    canvas.fill_rect(
        Rect::new(inner.left, inner.top, divider_x, inner.bottom),
        rgb(COLOR_BG_HEADER),
    );

    // Vertical divider between tabs and content spans full panel height.
    canvas.fill_rect(
        Rect::new(divider_x, inner.top, divider_x + 1, inner.bottom),
        rgb(COLOR_DIVIDER),
    );

    // Divider between fields and action row should only live in the content column.
    canvas.fill_rect(
        Rect::new(
            divider_x,
            ACTION_DIVIDER_Y,
            inner.right,
            ACTION_DIVIDER_Y + 1,
        ),
        rgb(COLOR_DIVIDER),
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

fn tabs_left() -> i32 {
    panel_inner_left() + 3
}

fn tabs_right() -> i32 {
    tabs_left() + TAB_W
}

fn fields_left() -> i32 {
    tabs_right() + 8 + 10
}

fn fields_top() -> i32 {
    panel_inner_top() + PAD_Y
}

fn tab_rect(preset_id: u8) -> Rect {
    let tabs_h = TAB_H * 5 + TAB_GAP * 4;
    let avail_h = (ACTION_DIVIDER_Y - panel_inner_top()).max(0);
    let top_pad = ((avail_h - tabs_h) / 2).max(0);
    let idx = preset_id.saturating_sub(1) as i32;
    let top = panel_inner_top() + top_pad + idx * (TAB_H + TAB_GAP);
    Rect::new(tabs_left(), top, tabs_right(), top + TAB_H)
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

fn split_value(value: &str) -> (&str, &str) {
    if value.len() < 2 {
        return ("", "");
    }
    value.split_at(value.len() - 1)
}

fn save_rect() -> Rect {
    Rect::new(
        PANEL_RIGHT - 84,
        ACTION_TOP + 2,
        PANEL_RIGHT - 12,
        ACTION_TOP + 26,
    )
}

fn load_hit_rect() -> Rect {
    let label_x = fields_left();
    Rect::new(label_x, ACTION_TOP, label_x + 110, PANEL_BOTTOM - BORDER)
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

fn row_top(field: PresetPanelField) -> i32 {
    let idx = match field {
        PresetPanelField::Mode => 0,
        PresetPanelField::Target => 1,
        PresetPanelField::VLim => 2,
        PresetPanelField::ILim => 3,
        PresetPanelField::PLim => 4,
    };
    fields_top() + idx * (ROW_H + ROW_GAP)
}

fn row_hit_rect(field: PresetPanelField) -> Rect {
    let top = row_top(field);
    Rect::new(fields_left(), top, panel_inner_right(), top + ROW_H)
}

fn normalize_mode(mode: LoadMode) -> LoadMode {
    match mode {
        LoadMode::Cv => LoadMode::Cv,
        LoadMode::Cc => LoadMode::Cc,
        LoadMode::Reserved(_) => LoadMode::Cc,
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

fn hit_in_rect(x: i32, y: i32, rect: Rect) -> bool {
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
}

fn append_u32_fixed<const N: usize>(buf: &mut String<N>, value: u32, width: u32) {
    let mut tmp = [b'0'; 10];
    let mut n = 0;
    let mut v = value;
    loop {
        tmp[n] = b'0' + (v % 10) as u8;
        n += 1;
        v /= 10;
        if v == 0 {
            break;
        }
        if n >= tmp.len() {
            break;
        }
    }
    while n < width as usize {
        tmp[n] = b'0';
        n += 1;
    }
    for ch in tmp[..n].iter().rev() {
        let _ = buf.push(*ch as char);
    }
}

fn append_u32_2dp<const N: usize>(buf: &mut String<N>, value: u32) {
    let tens = ((value / 10) % 10) as u8;
    let ones = (value % 10) as u8;
    let _ = buf.push((b'0' + tens) as char);
    let _ = buf.push((b'0' + ones) as char);
}
