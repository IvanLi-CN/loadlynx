#![allow(dead_code)]

use embedded_graphics::pixelcolor::Rgb565;
use heapless::String;
use lcd_async::raw_framebuf::RawFrameBuf;
use loadlynx_protocol::LoadMode;

use crate::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

use super::fonts::SMALL_FONT;
use super::{Canvas, Rect, rgb, small_text_width};

const PANEL_LEFT: i32 = 20;
const PANEL_TOP: i32 = 40;
const PANEL_RIGHT: i32 = 300;
const PANEL_BOTTOM: i32 = 220;

const BORDER_X: i32 = 3;
const BORDER_Y: i32 = 2;
const INNER_LEFT: i32 = PANEL_LEFT + BORDER_X;
const INNER_RIGHT: i32 = PANEL_RIGHT - BORDER_X;
const INNER_TOP: i32 = PANEL_TOP + BORDER_Y;
const INNER_BOTTOM: i32 = PANEL_BOTTOM - BORDER_Y;

const TOP_PAD_BOTTOM: i32 = 44;
const TOP_DIVIDER_Y: i32 = 44;
const TABS_TOP: i32 = 45;
const TABS_BOTTOM: i32 = 63;
const TABS_DIVIDER_Y: i32 = 63;
const HEADER_TOP: i32 = 64;
const HEADER_BOTTOM: i32 = 74;
const BODY_TOP: i32 = 75;
const ACTION_DIVIDER_Y: i32 = 185;
const ACTION_TOP: i32 = 186;

const TAB_STEP: i32 = 56;
const TAB_WIDTH: i32 = 50;
const TAB_LEFT: i32 = 23;

const LABEL_X: i32 = PANEL_LEFT + 14;
const ROW0_Y: i32 = 84;
const ROW_STEP_Y: i32 = 12;

const MODE_PILL_TOP_OFFSET: i32 = -2;
const MODE_PILL_HEIGHT: i32 = 12;

const VALUE_LEN: i32 = 7;
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

    canvas.fill_rect(
        Rect::new(PANEL_LEFT, PANEL_TOP, PANEL_RIGHT, PANEL_BOTTOM),
        rgb(COLOR_DIVIDER),
    );

    canvas.fill_rect(
        Rect::new(INNER_LEFT, INNER_TOP, INNER_RIGHT, TOP_PAD_BOTTOM),
        rgb(COLOR_BG_HEADER),
    );
    canvas.fill_rect(
        Rect::new(INNER_LEFT, TOP_DIVIDER_Y, INNER_RIGHT, TOP_DIVIDER_Y + 1),
        rgb(COLOR_DIVIDER),
    );

    canvas.fill_rect(
        Rect::new(INNER_LEFT, TABS_TOP, INNER_RIGHT, TABS_BOTTOM),
        rgb(COLOR_BG_HEADER),
    );
    canvas.fill_rect(
        Rect::new(INNER_LEFT, TABS_DIVIDER_Y, INNER_RIGHT, TABS_DIVIDER_Y + 1),
        rgb(COLOR_DIVIDER),
    );

    canvas.fill_rect(
        Rect::new(INNER_LEFT, HEADER_TOP, INNER_RIGHT, HEADER_BOTTOM),
        rgb(COLOR_BG_HEADER),
    );
    canvas.fill_rect(
        Rect::new(INNER_LEFT, HEADER_BOTTOM, INNER_RIGHT, HEADER_BOTTOM + 1),
        rgb(COLOR_DIVIDER),
    );

    canvas.fill_rect(
        Rect::new(INNER_LEFT, BODY_TOP, INNER_RIGHT, ACTION_DIVIDER_Y),
        rgb(COLOR_BG_BODY),
    );
    canvas.fill_rect(
        Rect::new(
            INNER_LEFT,
            ACTION_DIVIDER_Y,
            INNER_RIGHT,
            ACTION_DIVIDER_Y + 1,
        ),
        rgb(COLOR_DIVIDER),
    );

    canvas.fill_rect(
        Rect::new(INNER_LEFT, ACTION_TOP, INNER_RIGHT, INNER_BOTTOM),
        rgb(COLOR_BG_HEADER),
    );

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

    if y >= TABS_TOP && y < TABS_BOTTOM {
        for preset_id in 1u8..=5u8 {
            let rect = tab_rect(preset_id);
            if hit_in_rect(x, y, rect) {
                return Some(PresetPanelHit::Tab(preset_id));
            }
        }
        return None;
    }

    // Treat the whole MODE row as the hit target (not just the text),
    // so the segmented control remains easy to use on hardware.
    let mode_row_y = row_y(PresetPanelField::Mode, normalize_mode(vm.editing_mode));
    let mode_pill = mode_pill_rect_at(mode_row_y);
    let mode_hit = Rect::new(
        PANEL_LEFT + 10,
        mode_pill.top - 2,
        PANEL_RIGHT - 8,
        mode_pill.bottom + 2,
    );
    if hit_in_rect(x, y, mode_hit) {
        let sep = mode_pill_separator_x();
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

    let mode = normalize_mode(vm.editing_mode);
    if hit_in_rect(x, y, row_hit_rect(row_y(PresetPanelField::Target, mode))) {
        return Some(PresetPanelHit::Target);
    }
    if hit_in_rect(x, y, row_hit_rect(row_y(PresetPanelField::VLim, mode))) {
        return Some(PresetPanelHit::VLim);
    }
    if hit_in_rect(x, y, row_hit_rect(row_y(PresetPanelField::ILim, mode))) {
        return Some(PresetPanelHit::ILim);
    }
    if hit_in_rect(x, y, row_hit_rect(row_y(PresetPanelField::PLim, mode))) {
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
    for preset_id in 1u8..=5u8 {
        let rect = tab_rect(preset_id);
        let selected = preset_id == vm.editing_preset_id;
        let fill = if selected { COLOR_THEME } else { COLOR_TAB_BG };
        canvas.fill_rect(rect, rgb(fill));
        draw_rect_outline(canvas, rect, rgb(COLOR_DIVIDER));

        if vm.active_preset_id == preset_id && vm.active_preset_id != vm.editing_preset_id {
            let underline = Rect::new(
                rect.left + 2,
                rect.bottom - 3,
                rect.right - 2,
                rect.bottom - 1,
            );
            canvas.fill_rect(underline, rgb(COLOR_THEME));
        }

        let mut label = String::<3>::new();
        let _ = label.push('M');
        let _ = label.push(char::from(b'0' + preset_id));
        let text_w = small_text_width(label.as_str(), 0);
        let x = rect.left + ((rect.right - rect.left) - text_w) / 2;
        let y = rect.top + 6;
        let color = if selected {
            rgb(COLOR_TEXT_DARK)
        } else {
            rgb(COLOR_TEXT_VALUE)
        };
        super::draw_small_text(canvas, label.as_str(), x, y, color, 0);
    }
}

fn draw_fields(canvas: &mut Canvas, vm: &PresetPanelVm) {
    let label_color = rgb(COLOR_TEXT_LABEL);
    let value_color = rgb(COLOR_TEXT_VALUE);
    let mode = normalize_mode(vm.editing_mode);

    super::draw_small_text(canvas, "MODE", LABEL_X, ROW0_Y, label_color, 0);
    draw_mode_segmented(
        canvas,
        ROW0_Y,
        mode,
        vm.selected_field == PresetPanelField::Mode,
    );

    let mut row = ROW0_Y + ROW_STEP_Y;
    draw_value_row(
        canvas,
        "TARGET",
        vm.target_text.as_str(),
        row,
        vm.selected_field == PresetPanelField::Target,
        vm.selected_digit,
        false,
        value_color,
    );

    row += ROW_STEP_Y;

    draw_value_row(
        canvas,
        "UVLO",
        vm.v_lim_text.as_str(),
        row,
        vm.selected_field == PresetPanelField::VLim,
        vm.selected_digit,
        false,
        value_color,
    );
    row += ROW_STEP_Y;

    draw_value_row(
        canvas,
        "OCP",
        vm.i_lim_text.as_str(),
        row,
        vm.selected_field == PresetPanelField::ILim,
        vm.selected_digit,
        false,
        value_color,
    );
    row += ROW_STEP_Y;

    draw_value_row(
        canvas,
        "OPP",
        vm.p_lim_text.as_str(),
        row,
        vm.selected_field == PresetPanelField::PLim,
        vm.selected_digit,
        true,
        value_color,
    );
}

fn draw_value_row(
    canvas: &mut Canvas,
    label: &str,
    value: &str,
    y: i32,
    selected: bool,
    digit: PresetPanelDigit,
    is_power: bool,
    value_color: Rgb565,
) {
    super::draw_small_text(canvas, label, LABEL_X, y, rgb(COLOR_TEXT_LABEL), 0);
    let value_x = value_x();
    let glyph = SMALL_FONT.width() as i32;

    let highlight_idx = if selected {
        selected_digit_index(digit, is_power)
    } else {
        None
    };

    if let Some(idx) = highlight_idx {
        let cell_x = value_x + idx as i32 * glyph;
        let highlight = digit_highlight_rect(cell_x, y);
        canvas.fill_rect(highlight, rgb(COLOR_THEME));
    }

    super::draw_small_text(canvas, value, value_x, y, value_color, 0);

    if let Some(idx) = highlight_idx {
        if let Some(ch) = value.chars().nth(idx) {
            let cell_x = value_x + idx as i32 * glyph;
            SMALL_FONT.draw_char(
                ch,
                |px, py| canvas.set_pixel(px + cell_x, py + y, rgb(COLOR_TEXT_DARK)),
                0,
                0,
            );
        }
    }
}

fn draw_mode_segmented(canvas: &mut Canvas, y: i32, mode: LoadMode, selected: bool) {
    let rect = mode_pill_rect_at(y);
    canvas.fill_round_rect(rect, 6, rgb(COLOR_PILL_BG));
    let rect_inner = Rect::new(rect.left + 1, rect.top + 1, rect.right - 1, rect.bottom - 1);
    canvas.fill_round_rect(rect_inner, 5, rgb(COLOR_PILL_BG));

    let sep = mode_pill_separator_x_at(y);
    canvas.draw_line(sep, rect.top + 1, sep, rect.bottom - 2, rgb(COLOR_DIVIDER));

    if selected {
        draw_rect_outline(canvas, rect, rgb(COLOR_THEME));
    }

    let (cv_color, cc_color) = match mode {
        LoadMode::Cv => (rgb(COLOR_MODE_CV), rgb(COLOR_MODE_OFF)),
        LoadMode::Cc => (rgb(COLOR_MODE_OFF), rgb(COLOR_MODE_CC)),
        LoadMode::Reserved(_) => (rgb(COLOR_MODE_OFF), rgb(COLOR_MODE_CC)),
    };

    let cv_w = small_text_width("CV", 0);
    let cc_w = small_text_width("CC", 0);
    let left_x0 = rect.left + 2;
    let left_x1 = sep - 2;
    let right_x0 = sep + 2;
    let right_x1 = rect.right - 2;
    let cv_x = left_x0 + ((left_x1 - left_x0) - cv_w) / 2;
    let cc_x = right_x0 + ((right_x1 - right_x0) - cc_w) / 2;
    super::draw_small_text(canvas, "CV", cv_x, y, cv_color, 0);
    super::draw_small_text(canvas, "CC", cc_x, y, cc_color, 0);
}

fn draw_action_row(canvas: &mut Canvas, vm: &PresetPanelVm) {
    super::draw_small_text(
        canvas,
        LOAD_LABEL,
        LABEL_X,
        ACTION_TOP + 6,
        rgb(COLOR_TEXT_LABEL),
        0,
    );
    draw_load_switch(canvas, vm.load_enabled);

    let save = save_rect();
    canvas.fill_rect(save, rgb(COLOR_DIVIDER));
    let save_inner = Rect::new(save.left + 1, save.top + 1, save.right - 1, save.bottom - 1);
    canvas.fill_rect(save_inner, rgb(COLOR_THEME));
    let w = small_text_width(SAVE_TEXT, 0);
    let x = save.left + ((save.right - save.left) - w) / 2;
    let y = save.top + 7;
    super::draw_small_text(canvas, SAVE_TEXT, x, y, rgb(COLOR_TEXT_DARK), 0);
}

fn draw_load_switch(canvas: &mut Canvas, enabled: bool) {
    let rect = load_switch_rect();
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

fn tab_rect(preset_id: u8) -> Rect {
    let idx = preset_id.saturating_sub(1) as i32;
    let left = TAB_LEFT + idx * TAB_STEP;
    Rect::new(left, TABS_TOP, left + TAB_WIDTH, TABS_BOTTOM)
}

fn save_rect() -> Rect {
    Rect::new(
        PANEL_RIGHT - 84,
        ACTION_TOP + 2,
        PANEL_RIGHT - 12,
        ACTION_TOP + 26,
    )
}

fn load_switch_rect() -> Rect {
    Rect::new(
        PANEL_LEFT + 77,
        ACTION_TOP + 8,
        PANEL_LEFT + 103,
        ACTION_TOP + 20,
    )
}

fn load_hit_rect() -> Rect {
    Rect::new(PANEL_LEFT + 30, ACTION_TOP, PANEL_LEFT + 140, INNER_BOTTOM)
}

fn mode_pill_rect() -> Rect {
    mode_pill_rect_at(ROW0_Y)
}

fn mode_pill_rect_at(y: i32) -> Rect {
    let top = y + MODE_PILL_TOP_OFFSET;
    let left = value_x() - 5;
    let right = value_right_x() - 2;
    Rect::new(left, top, right, top + MODE_PILL_HEIGHT)
}

fn mode_pill_separator_x() -> i32 {
    mode_pill_separator_x_at(ROW0_Y)
}

fn mode_pill_separator_x_at(y: i32) -> i32 {
    let rect = mode_pill_rect_at(y);
    rect.left + (rect.right - rect.left) / 2
}

fn value_x() -> i32 {
    value_right_x() - (SMALL_FONT.width() as i32) * VALUE_LEN
}

fn value_right_x() -> i32 {
    PANEL_RIGHT - 11
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

fn digit_highlight_rect(cell_x: i32, y: i32) -> Rect {
    let digit_bbox_min_x = 0;
    let digit_bbox_max_x = 4;
    let digit_bbox_min_y = 2;
    let digit_bbox_max_y = 9;
    let pad_x = 1;
    let pad_y = 2;
    Rect::new(
        cell_x + digit_bbox_min_x - pad_x,
        y + digit_bbox_min_y - pad_y,
        cell_x + digit_bbox_max_x + 1 + pad_x,
        y + digit_bbox_max_y + 1 + pad_y,
    )
}

fn row_y(field: PresetPanelField, _mode: LoadMode) -> i32 {
    let base = ROW0_Y + ROW_STEP_Y;
    match field {
        PresetPanelField::Target => base,
        PresetPanelField::VLim => base + ROW_STEP_Y,
        PresetPanelField::ILim => base + ROW_STEP_Y * 2,
        PresetPanelField::PLim => base + ROW_STEP_Y * 3,
        PresetPanelField::Mode => ROW0_Y,
    }
}

fn row_hit_rect(y: i32) -> Rect {
    Rect::new(
        PANEL_LEFT + 10,
        y - 2,
        PANEL_RIGHT - 8,
        y + SMALL_FONT.height() as i32 + 2,
    )
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
