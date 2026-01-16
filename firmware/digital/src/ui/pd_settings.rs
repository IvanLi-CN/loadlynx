#![allow(dead_code)]

use core::fmt::Write as _;

use embedded_graphics::pixelcolor::raw::{RawData, RawU16};
use embedded_graphics::pixelcolor::{Rgb565, Rgb888};
use embedded_graphics::prelude::Point;
use heapless::String;
use lcd_async::raw_framebuf::RawFrameBuf;
use loadlynx_protocol::{FixedPdoList, PpsPdoList};
use u8g2_fonts::FontRenderer;
use u8g2_fonts::fonts;
use u8g2_fonts::types::{FontColor, HorizontalAlignment, VerticalPosition};

use crate::control::{AdjustDigit, PdMode, PdSettingsFocus};
use crate::{DISPLAY_HEIGHT as PHYS_HEIGHT, DISPLAY_WIDTH as PHYS_WIDTH};

use super::{Canvas, Rect, rgb};

// Supersampling factor for text anti-aliasing. Paired fonts use 8px (base) and 24px (AA).
const TEXT_AA_SCALE: usize = 3;

// Text fonts: use a proportional sans family (Helvetica) to match the frozen UI mocks.
const UI_TEXT_FONT: FontRenderer =
    FontRenderer::new::<fonts::u8g2_font_helvR08_tf>().with_ignore_unknown_chars(true);
const UI_TEXT_FONT_BOLD: FontRenderer =
    FontRenderer::new::<fonts::u8g2_font_helvB08_tf>().with_ignore_unknown_chars(true);
const UI_TEXT_FONT_AA: FontRenderer =
    FontRenderer::new::<fonts::u8g2_font_helvR24_tf>().with_ignore_unknown_chars(true);
const UI_TEXT_FONT_AA_BOLD: FontRenderer =
    FontRenderer::new::<fonts::u8g2_font_helvB24_tf>().with_ignore_unknown_chars(true);

struct FontPair {
    base: &'static FontRenderer,
    aa: &'static FontRenderer,
}

const FONT_REGULAR: FontPair = FontPair {
    base: &UI_TEXT_FONT,
    aa: &UI_TEXT_FONT_AA,
};
const FONT_BOLD: FontPair = FontPair {
    base: &UI_TEXT_FONT_BOLD,
    aa: &UI_TEXT_FONT_AA_BOLD,
};

// PD settings panel layout (320x240). Design baseline:
// `docs/assets/usb-pd-settings-panel/*.png`.
const LEFT_COL_RIGHT: i32 = 190;
const RIGHT_COL_LEFT: i32 = LEFT_COL_RIGHT;

const DOT_GAP_PX: i32 = 4;

const TOP_BAR_H: i32 = 24;

// Top-bar text baseline for the frozen mock assets.
const HEADER_Y: i32 = 7;
const HEADER_X: i32 = 12;
const HEADER_RIGHT_PAD: i32 = 22;
const HEADER_ATTACH_GAP_PX: i32 = 6;

// NOTE: Pixel-aligned to frozen mock assets.
const MODE_PILL_LEFT: i32 = 11;
const MODE_PILL_TOP: i32 = 31;
const MODE_PILL_RIGHT: i32 = 171;
const MODE_PILL_BOTTOM: i32 = 53;
const MODE_PILL_RADIUS: i32 = 8;
const MODE_SEG_GAP: i32 = 4;

const LIST_LEFT: i32 = MODE_PILL_LEFT;
const LIST_RIGHT: i32 = 179;
const LIST_TITLE_Y: i32 = 58;
const LIST_TOP: i32 = 73;
const LIST_ROW_H_FIXED: i32 = 24;
const LIST_ROW_GAP_FIXED: i32 = 2;
// PPS rows carry two lines, so the frozen mock uses a taller row + larger gap.
const LIST_ROW_H_PPS: i32 = 30;
const LIST_ROW_GAP_PPS: i32 = 4;
const LIST_ROW_RADIUS: i32 = 8;
const LIST_ROW_LINE1_Y_OFF: i32 = 8;
// Two-line PPS rows: match frozen mock baseline. Line 2 must start low enough
// to avoid overlapping the range line.
const LIST_ROW_LINE2_Y_OFF: i32 = 19;

const CARD_LEFT: i32 = RIGHT_COL_LEFT + 8;
const CARD_RIGHT: i32 = 312;
const CONTRACT_CARD_TOP: i32 = 32;
const CONTRACT_CARD_BOTTOM: i32 = 72;
const SELECTED_CARD_TOP: i32 = 78;
const SELECTED_SUMMARY_BOTTOM: i32 = 134;
const CARD_RADIUS: i32 = 10;

// Controls area (outside the selected-summary card).
// NOTE: These are pixel-matched to the frozen mocks in `docs/assets/usb-pd-settings-panel/*.png`.
const CONTROL_ROW_VREQ_Y: i32 = 144;
const CONTROL_ROW_IREQ_FIXED_Y: i32 = 156;
const CONTROL_ROW_IREQ_PPS_Y: i32 = 170;

const BTN_TOP: i32 = 214;
const BTN_BOTTOM: i32 = 235;
const BACK_LEFT: i32 = CARD_LEFT;
const BACK_RIGHT: i32 = 253;
const APPLY_LEFT: i32 = 257;
const APPLY_RIGHT: i32 = 313;

// Controls sizing/placement matches `docs/assets/usb-pd-settings-panel/*.png`.
// Target value editor (tap + swipe + encoder).
const VALUE_PILL_W: i32 = 63;
const VALUE_PILL_H: i32 = 21;
const VALUE_PILL_RADIUS: i32 = 8;
const VALUE_PILL_RIGHT_PAD: i32 = 7;
// Keep a small right padding so the unit doesn't visually touch the pill edge.
const VALUE_PILL_TEXT_RIGHT_PAD: i32 = 8;
const VALUE_PILL_TEXT_MIN_LEFT_PAD: i32 = 0;

// Frozen palette sampled from `docs/assets/usb-pd-settings-panel/*.png`.
const COLOR_TOP_BG: u32 = 0x1c2638;
const COLOR_LEFT_BG: u32 = 0x101829;
const COLOR_RIGHT_BG: u32 = 0x0b111e;
const COLOR_INSET_BG: u32 = 0x141d2f;
const COLOR_ROW_BG: u32 = 0x111828;
const COLOR_CARD_BG: u32 = 0x1c2638;
const COLOR_BORDER: u32 = 0x162134;
const COLOR_BORDER_INNER: u32 = 0x182337;
const COLOR_DIVIDER_MID: u32 = 0x131d2f;
// Secondary text (labels/details) in the frozen mocks is noticeably brighter than the old value.
// Keep a separate "disabled" text color for greyed-out Apply.
const COLOR_TEXT_DIM: u32 = 0x899ec2;
const COLOR_TEXT_LABEL: u32 = 0x899ec2;
const COLOR_TEXT_DISABLED: u32 = 0x4f5d7b;
const COLOR_TEXT_VALUE: u32 = 0xdfe7ff;
// Unit glyph color sampled from the frozen mock (darker than label text).
const COLOR_TEXT_UNIT: u32 = 0x666d7f;
const COLOR_ACCENT_TEXT: u32 = 0x47badf;
const COLOR_SETPOINT_SHADOW: u32 = 0xafb6cc;
const COLOR_ACCENT: u32 = 0x367d9a;
const COLOR_ACCENT_DARK: u32 = 0x2e718c;
const COLOR_ACCENT_INNER: u32 = 0x347894;
const COLOR_ACCENT_SHADOW: u32 = 0x32728e;
const COLOR_ACCENT_OUTER_TOP: u32 = 0x2c6b86;
const COLOR_ACCENT_OUTER_BOTTOM: u32 = 0x307693;
const COLOR_ACCENT_BTN_OUTER_BASE: u32 = 0x2b6d87;
const COLOR_ACCENT_BTN_OUTER_BOTTOM: u32 = 0x2e738e;
const COLOR_BUTTON_DISABLED_BORDER_OUTER: u32 = 0x131d2f;
const COLOR_WARN: u32 = 0xff5252;

const VREQ_STEP_TEXT: &str = "20mV";
const IREQ_STEP_TEXT: &str = "50mA";
const VALUE_UNIT_GAP: i32 = 1;

// Value pill: sampled from `docs/assets/usb-pd-settings-panel/*.png`.
const COLOR_VALUE_PILL_FOCUSED_OUTER: u32 = 0x4cc9f0;
const COLOR_VALUE_PILL_FOCUSED_INNER: u32 = 0x0f1d2b;
const COLOR_VALUE_PILL_IDLE_OUTER: u32 = 0x243257;
const COLOR_VALUE_PILL_IDLE_INNER: u32 = 0x0d1322;

const SELECTED_SUM_LINE1_Y_OFF: i32 = 22;
// Slightly increase line spacing in the selected-summary card to match the frozen mock.
const SELECTED_SUM_LINE2_Y_OFF: i32 = 40;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PdSettingsMessage {
    None,
    Unavailable,
    Applying,
    ApplyOk,
    ApplyFailed,
}

#[derive(Clone, Debug)]
pub struct PdSettingsVm {
    pub attached: bool,
    pub mode: PdMode,
    pub focus: PdSettingsFocus,
    pub focused_digit: AdjustDigit,
    pub fixed_pdos: FixedPdoList,
    pub pps_pdos: PpsPdoList,
    pub contract_mv: u32,
    pub contract_ma: u32,
    pub fixed_object_pos: u8,
    pub pps_object_pos: u8,
    pub pps_target_mv: u32,
    pub i_req_ma: u32,
    pub apply_enabled: bool,
    pub message: PdSettingsMessage,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PdSettingsHit {
    ModeFixed,
    ModePps,
    ListRow(usize),
    VreqValue,
    IreqValue,
    Back,
    Apply,
}

pub fn render_pd_settings(frame: &mut RawFrameBuf<Rgb565, &mut [u8]>, vm: &PdSettingsVm) {
    let bytes = frame.as_mut_bytes();
    let mut canvas = Canvas::new(bytes, PHYS_WIDTH, PHYS_HEIGHT);

    // Top bar (full width), then per-column backgrounds (below top bar).
    canvas.fill_rect(
        Rect::new(0, 0, super::LOGICAL_WIDTH, TOP_BAR_H),
        rgb(COLOR_TOP_BG),
    );
    canvas.fill_rect(
        Rect::new(0, TOP_BAR_H, LEFT_COL_RIGHT, super::LOGICAL_HEIGHT),
        rgb(COLOR_LEFT_BG),
    );
    canvas.fill_rect(
        Rect::new(
            LEFT_COL_RIGHT,
            TOP_BAR_H,
            super::LOGICAL_WIDTH,
            super::LOGICAL_HEIGHT,
        ),
        rgb(COLOR_RIGHT_BG),
    );

    // Divider (2px) only in the content area; the top bar remains seamless.
    canvas.fill_rect(
        Rect::new(
            LEFT_COL_RIGHT - 1,
            TOP_BAR_H,
            LEFT_COL_RIGHT,
            super::LOGICAL_HEIGHT,
        ),
        rgb(COLOR_BORDER),
    );
    canvas.fill_rect(
        Rect::new(
            LEFT_COL_RIGHT,
            TOP_BAR_H,
            LEFT_COL_RIGHT + 1,
            super::LOGICAL_HEIGHT,
        ),
        rgb(COLOR_DIVIDER_MID),
    );

    draw_header(&mut canvas, vm);
    draw_mode_toggle(&mut canvas, vm);
    draw_caps_list(&mut canvas, vm);
    draw_contract_card(&mut canvas, vm);
    draw_selected_card(&mut canvas, vm);
    draw_message_bar(&mut canvas, vm);
    draw_action_buttons(&mut canvas, vm);
}

pub fn hit_test_pd_settings(x: i32, y: i32, vm: &PdSettingsVm) -> Option<PdSettingsHit> {
    if hit_in_rect(
        x,
        y,
        Rect::new(
            MODE_PILL_LEFT,
            MODE_PILL_TOP,
            MODE_PILL_RIGHT,
            MODE_PILL_BOTTOM,
        ),
    ) {
        let mid = (MODE_PILL_LEFT + MODE_PILL_RIGHT) / 2;
        return if x < mid {
            Some(PdSettingsHit::ModeFixed)
        } else {
            Some(PdSettingsHit::ModePps)
        };
    }

    if hit_in_rect(x, y, Rect::new(BACK_LEFT, BTN_TOP, BACK_RIGHT, BTN_BOTTOM)) {
        return Some(PdSettingsHit::Back);
    }
    if hit_in_rect(
        x,
        y,
        Rect::new(APPLY_LEFT, BTN_TOP, APPLY_RIGHT, BTN_BOTTOM),
    ) {
        return Some(PdSettingsHit::Apply);
    }

    // Target value editor areas (tap to focus/select digit).
    if vm.mode == PdMode::Pps && hit_in_rect(x, y, vreq_value_rect()) {
        return Some(PdSettingsHit::VreqValue);
    }
    if hit_in_rect(x, y, ireq_value_rect(vm.mode)) {
        return Some(PdSettingsHit::IreqValue);
    }

    // List rows.
    let (row_h, row_gap) = list_row_metrics(vm.mode);
    let (row_count, row_top) = match vm.mode {
        PdMode::Fixed => (vm.fixed_pdos.len() as i32, LIST_TOP),
        PdMode::Pps => {
            let extra = if pps_selection_missing(vm) { 1 } else { 0 };
            ((vm.pps_pdos.len() as i32) + extra, LIST_TOP)
        }
    };
    if row_count > 0 {
        let list_bottom = row_top + row_count * (row_h + row_gap) - row_gap;
        if x >= LIST_LEFT && x < LIST_RIGHT && y >= row_top && y < list_bottom {
            let rel = y - row_top;
            let stride = row_h + row_gap;
            let idx = rel / stride;
            let in_row = (rel % stride) < row_h;
            if in_row && idx >= 0 && idx < row_count {
                let idx = idx as usize;
                if vm.mode == PdMode::Pps && pps_selection_missing(vm) && idx == vm.pps_pdos.len() {
                    return None;
                }
                return Some(PdSettingsHit::ListRow(idx));
            }
        }
    }

    None
}

fn draw_header(canvas: &mut Canvas, vm: &PdSettingsVm) {
    draw_small_bold(
        canvas,
        "USB-PD Settings",
        HEADER_X,
        HEADER_Y,
        rgb(COLOR_TEXT_VALUE),
    );

    // Right-aligned attach state.
    let attach_state = if vm.attached { "YES" } else { "NO" };
    let label = "ATTACH:";
    let label_w = text_width(label);
    let state_w = text_width(attach_state);
    let total_w = label_w + HEADER_ATTACH_GAP_PX + state_w;
    let x0 = (super::LOGICAL_WIDTH - HEADER_RIGHT_PAD - total_w).max(0);
    draw_small(canvas, label, x0, HEADER_Y, rgb(COLOR_TEXT_DIM));
    draw_small(
        canvas,
        attach_state,
        x0 + label_w + HEADER_ATTACH_GAP_PX,
        HEADER_Y,
        rgb(COLOR_TEXT_VALUE),
    );
}

fn draw_mode_toggle(canvas: &mut Canvas, vm: &PdSettingsVm) {
    let rect = Rect::new(
        MODE_PILL_LEFT,
        MODE_PILL_TOP,
        MODE_PILL_RIGHT,
        MODE_PILL_BOTTOM,
    );
    let total_w = rect.right - rect.left;
    let seg_w = ((total_w - MODE_SEG_GAP).max(0)) / 2;
    let fixed_rect = Rect::new(rect.left, rect.top, rect.left + seg_w, rect.bottom);
    let pps_rect = Rect::new(
        fixed_rect.right + MODE_SEG_GAP,
        rect.top,
        rect.right,
        rect.bottom,
    );

    let (fixed_selected, fixed_text) = if vm.mode == PdMode::Fixed {
        (true, rgb(COLOR_TEXT_VALUE))
    } else {
        (false, rgb(COLOR_TEXT_DIM))
    };
    let (pps_selected, pps_text) = if vm.mode == PdMode::Pps {
        (true, rgb(COLOR_TEXT_VALUE))
    } else {
        (false, rgb(COLOR_TEXT_DIM))
    };

    if fixed_selected {
        draw_round_rect_2px_border_accent_row(
            canvas,
            fixed_rect,
            MODE_PILL_RADIUS,
            rgb(COLOR_TOP_BG),
        );
    } else {
        draw_round_rect_2px_border(
            canvas,
            fixed_rect,
            MODE_PILL_RADIUS,
            rgb(COLOR_BORDER),
            rgb(COLOR_BORDER_INNER),
            rgb(COLOR_INSET_BG),
        );
    }

    if pps_selected {
        draw_round_rect_2px_border_accent_row(
            canvas,
            pps_rect,
            MODE_PILL_RADIUS,
            rgb(COLOR_TOP_BG),
        );
    } else {
        draw_round_rect_2px_border(
            canvas,
            pps_rect,
            MODE_PILL_RADIUS,
            rgb(COLOR_BORDER),
            rgb(COLOR_BORDER_INNER),
            rgb(COLOR_INSET_BG),
        );
    }

    draw_centered_small_bold(canvas, "Fixed", fixed_rect, fixed_text);
    draw_centered_small_bold(canvas, "PPS", pps_rect, pps_text);
}

fn draw_caps_list(canvas: &mut Canvas, vm: &PdSettingsVm) {
    let (row_h, row_gap) = list_row_metrics(vm.mode);
    let fixed_missing = vm.mode == PdMode::Fixed && fixed_selection_missing(vm);
    let pps_missing = vm.mode == PdMode::Pps && pps_selection_missing(vm);
    let title = match vm.mode {
        PdMode::Fixed => {
            if fixed_missing {
                "Fixed PDOs"
            } else {
                "Fixed PDOs (tap to select)"
            }
        }
        PdMode::Pps => {
            if pps_missing {
                "PPS APDOs"
            } else {
                "PPS APDOs (tap to select)"
            }
        }
    };

    draw_small_title(
        canvas,
        title,
        LIST_LEFT,
        LIST_TITLE_Y,
        rgb(COLOR_TEXT_LABEL),
        LEFT_COL_RIGHT - 8,
    );

    match vm.mode {
        PdMode::Fixed => {
            for (idx, pdo) in vm.fixed_pdos.iter().enumerate() {
                let top = LIST_TOP + idx as i32 * (row_h + row_gap);
                let rect = Rect::new(LIST_LEFT, top, LIST_RIGHT, top + row_h);
                let pos = effective_pos(pdo.pos, idx);
                let selected = vm.fixed_object_pos != 0 && vm.fixed_object_pos == pos;
                draw_list_row_fixed(canvas, rect, idx as u8, *pdo, selected);
            }
        }
        PdMode::Pps => {
            for (idx, pdo) in vm.pps_pdos.iter().enumerate() {
                let top = LIST_TOP + idx as i32 * (row_h + row_gap);
                let rect = Rect::new(LIST_LEFT, top, LIST_RIGHT, top + row_h);
                let pos = effective_pos(pdo.pos, idx);
                let selected = vm.pps_object_pos != 0 && vm.pps_object_pos == pos;
                draw_list_row_pps(canvas, rect, idx as u8, *pdo, selected);
            }
            if pps_missing && vm.pps_object_pos != 0 {
                let idx = vm.pps_pdos.len();
                let top = LIST_TOP + idx as i32 * (row_h + row_gap);
                let rect = Rect::new(LIST_LEFT, top, LIST_RIGHT, top + row_h);
                let mut note = String::<32>::new();
                let _ = write!(&mut note, "(selected APDO{} missing)", vm.pps_object_pos);
                draw_list_row_missing(canvas, rect, note.as_str());
            }
        }
    }
}

fn draw_contract_card(canvas: &mut Canvas, vm: &PdSettingsVm) {
    let rect = Rect::new(
        CARD_LEFT,
        CONTRACT_CARD_TOP,
        CARD_RIGHT,
        CONTRACT_CARD_BOTTOM,
    );
    canvas.fill_round_rect_aa(rect, CARD_RADIUS, rgb(COLOR_CARD_BG));

    draw_small_title(
        canvas,
        "Active contract",
        rect.left + 10,
        rect.top + 8,
        rgb(COLOR_TEXT_LABEL),
        rect.right - 10,
    );

    if vm.attached {
        let v = format_v_contract_1dp(vm.contract_mv);
        let a = format_a_contract_1dp(vm.contract_ma);
        draw_dot_joined_small(
            canvas,
            rect.left + 10,
            rect.top + 24,
            v.as_str(),
            a.as_str(),
            rgb(COLOR_TEXT_VALUE),
        );
    } else {
        draw_small(
            canvas,
            "--",
            rect.left + 10,
            rect.top + 24,
            rgb(COLOR_TEXT_VALUE),
        );
    }
}

fn draw_selected_card(canvas: &mut Canvas, vm: &PdSettingsVm) {
    let rect = Rect::new(
        CARD_LEFT,
        SELECTED_CARD_TOP,
        CARD_RIGHT,
        SELECTED_SUMMARY_BOTTOM,
    );
    canvas.fill_round_rect_aa(rect, CARD_RADIUS, rgb(COLOR_CARD_BG));

    draw_small(
        canvas,
        "Selected",
        rect.left + 10,
        rect.top + 8,
        rgb(COLOR_TEXT_LABEL),
    );

    match vm.mode {
        PdMode::Fixed => draw_selected_fixed_summary(canvas, vm, rect),
        PdMode::Pps => draw_selected_pps_summary(canvas, vm, rect),
    }

    if vm.message == PdSettingsMessage::Unavailable {
        let msg_left = rect.left + 10;
        let msg_right = rect.right - 10;
        let line_h = text_height("A", &FONT_REGULAR);
        let y1 = CONTROL_ROW_VREQ_Y + 3;
        let y2 = y1 + line_h;
        let y3 = y2 + line_h;

        draw_small_title(
            canvas,
            "Unavailable",
            msg_left,
            y1,
            rgb(COLOR_WARN),
            msg_right,
        );
        let (m1, m2) = match vm.mode {
            PdMode::Fixed => ("Selected PDO not", "present in caps."),
            PdMode::Pps => ("Selected APDO not", "present in caps."),
        };
        draw_small_title(canvas, m1, msg_left, y2, rgb(COLOR_TEXT_LABEL), msg_right);
        draw_small_title(canvas, m2, msg_left, y3, rgb(COLOR_TEXT_LABEL), msg_right);
    } else {
        match vm.mode {
            PdMode::Fixed => {
                draw_target_value_row(
                    canvas,
                    PdSettingsFocus::Ireq,
                    format_req_a_2dp_digits(vm.i_req_ma).as_str(),
                    'A',
                    ireq_value_rect(PdMode::Fixed),
                    vm.focus == PdSettingsFocus::Ireq,
                    vm.focused_digit,
                );
            }
            PdMode::Pps => {
                draw_target_value_row(
                    canvas,
                    PdSettingsFocus::Vreq,
                    format_req_v_2dp_digits(vm.pps_target_mv).as_str(),
                    'V',
                    vreq_value_rect(),
                    vm.focus == PdSettingsFocus::Vreq,
                    vm.focused_digit,
                );
                draw_target_value_row(
                    canvas,
                    PdSettingsFocus::Ireq,
                    format_req_a_2dp_digits(vm.i_req_ma).as_str(),
                    'A',
                    ireq_value_rect(PdMode::Pps),
                    vm.focus == PdSettingsFocus::Ireq,
                    vm.focused_digit,
                );
            }
        }
    }
}

fn draw_message_bar(canvas: &mut Canvas, vm: &PdSettingsVm) {
    let (text, color) = match vm.message {
        PdSettingsMessage::Applying => ("Applying...", rgb(COLOR_TEXT_LABEL)),
        PdSettingsMessage::ApplyOk => ("Apply OK", rgb(COLOR_ACCENT)),
        PdSettingsMessage::ApplyFailed => ("Apply failed", rgb(COLOR_WARN)),
        _ => return,
    };

    // Keep it near the bottom so it doesn't fight with the controls region (esp. PPS).
    let rect = Rect::new(CARD_LEFT, BTN_TOP - 18, CARD_RIGHT, BTN_TOP - 2);
    draw_centered_small(canvas, text, rect, color);
}

fn draw_action_buttons(canvas: &mut Canvas, vm: &PdSettingsVm) {
    // Back
    draw_button_back(
        canvas,
        Rect::new(BACK_LEFT, BTN_TOP, BACK_RIGHT, BTN_BOTTOM),
        "Back",
        true,
    );
    // Apply
    draw_button_apply(
        canvas,
        Rect::new(APPLY_LEFT, BTN_TOP, APPLY_RIGHT, BTN_BOTTOM),
        "Apply",
        vm.apply_enabled,
    );
}

fn draw_button_back(canvas: &mut Canvas, rect: Rect, label: &str, enabled: bool) {
    let (border, fill) = if enabled {
        (rgb(COLOR_BORDER_INNER), rgb(COLOR_INSET_BG))
    } else {
        (rgb(COLOR_BORDER), rgb(COLOR_ROW_BG))
    };
    draw_round_rect_1px_border(canvas, rect, 8, border, fill);
    let text_color = if enabled {
        rgb(COLOR_TEXT_VALUE)
    } else {
        rgb(COLOR_TEXT_DIM)
    };
    draw_centered_small_bold(canvas, label, rect, text_color);
}

fn draw_button_apply(canvas: &mut Canvas, rect: Rect, label: &str, enabled: bool) {
    if enabled {
        draw_round_rect_2px_border_accent_button(canvas, rect, 8, rgb(COLOR_TOP_BG));
    } else {
        draw_round_rect_2px_border(
            canvas,
            rect,
            8,
            rgb(COLOR_BUTTON_DISABLED_BORDER_OUTER),
            rgb(COLOR_BORDER),
            // Disabled Apply uses a much darker fill in the frozen mocks.
            rgb(COLOR_ROW_BG),
        );
    }
    let text_color = if enabled {
        rgb(COLOR_TEXT_VALUE)
    } else {
        rgb(COLOR_TEXT_DISABLED)
    };
    draw_centered_small_bold(canvas, label, rect, text_color);
}

fn draw_target_value_row(
    canvas: &mut Canvas,
    field: PdSettingsFocus,
    digits: &str,
    unit: char,
    value_pill: Rect,
    focused: bool,
    focused_digit: AdjustDigit,
) {
    // Align with other right-column card content (e.g. "Active contract"/"Selected").
    let label_x = CARD_LEFT + 10;
    let label_y = value_pill.top;
    let label = match field {
        PdSettingsFocus::Vreq => "Vreq",
        PdSettingsFocus::Ireq => "Ireq",
        PdSettingsFocus::None => "",
    };
    let step = match field {
        PdSettingsFocus::Vreq => VREQ_STEP_TEXT,
        PdSettingsFocus::Ireq => IREQ_STEP_TEXT,
        PdSettingsFocus::None => "",
    };

    draw_label_two_line(
        canvas,
        label_x,
        label_y,
        label,
        step,
        rgb(COLOR_TEXT_VALUE),
        rgb(COLOR_TEXT_LABEL),
        value_pill.left - 4,
    );

    draw_value_pill(canvas, value_pill, focused);
    draw_value_pill_text(canvas, value_pill, digits, unit, focused, focused_digit);
}

fn draw_value_pill(canvas: &mut Canvas, rect: Rect, focused: bool) {
    if focused {
        draw_round_rect_2px_border_hard(
            canvas,
            rect,
            VALUE_PILL_RADIUS,
            rgb(COLOR_VALUE_PILL_FOCUSED_OUTER),
            rgb(COLOR_VALUE_PILL_FOCUSED_INNER),
            rgb(COLOR_RIGHT_BG),
        );
    } else {
        draw_round_rect_2px_border_hard(
            canvas,
            rect,
            VALUE_PILL_RADIUS,
            rgb(COLOR_VALUE_PILL_IDLE_OUTER),
            rgb(COLOR_VALUE_PILL_IDLE_INNER),
            rgb(COLOR_RIGHT_BG),
        );
    }
}

fn draw_value_pill_text(
    canvas: &mut Canvas,
    rect: Rect,
    digits: &str,
    unit: char,
    focused: bool,
    focused_digit: AdjustDigit,
) {
    let inner = if focused {
        Rect::new(rect.left + 2, rect.top + 2, rect.right - 2, rect.bottom - 2)
    } else {
        Rect::new(rect.left + 2, rect.top + 2, rect.right - 2, rect.bottom - 2)
    };

    let cell_w = SETPOINT_CELL_W as i32;
    let num_w = (digits.chars().count() as i32) * cell_w;

    let mut unit_buf = [0u8; 4];
    let unit_s = unit.encode_utf8(&mut unit_buf);
    // Unit in the frozen mock is not the UTFT SmallFont; use the regular UI font
    // to get the correct glyph width and tone.
    // Use a slightly conservative width here: the font's advance can be narrower
    // than its rendered bounds (especially with AA), which would otherwise let the
    // glyph spill into the pill border.
    let unit_w = text_bbox_scaled(unit_s, &FONT_REGULAR)
        .map(|(w, _)| w)
        .unwrap_or_else(|| text_width(unit_s));
    let total_w = num_w + VALUE_UNIT_GAP + unit_w;

    let num_x0 = (inner.right - VALUE_PILL_TEXT_RIGHT_PAD - total_w)
        .max(inner.left + VALUE_PILL_TEXT_MIN_LEFT_PAD);

    // The frozen PD settings mocks use the setpoint digits at a slightly smaller optical size
    // than the raw 10x18 bitmap, so they fit within the pill's 2px border.
    let num_h = SETPOINT_PILL_DIGIT_H as i32;
    let unit_h = text_height("A", &FONT_REGULAR);
    let num_y = inner.top + ((inner.bottom - inner.top - num_h).max(0)) / 2;
    // Unit in the frozen mock is optically centered a bit higher than strict bottom-alignment.
    let unit_y = num_y + num_h - unit_h - 1;

    draw_setpoint_text_scaled(
        canvas,
        digits,
        num_x0,
        num_y,
        rgb(COLOR_TEXT_VALUE),
        0,
        Rect::new(inner.left, inner.top, inner.right, inner.bottom),
    );
    let unit_x = num_x0 + num_w + VALUE_UNIT_GAP;
    // Clip the unit to the pill's inner rect to prevent glyphs from bleeding into the border.
    draw_small_clipped_with_font(
        canvas,
        unit_s,
        unit_x + 1,
        unit_y + 1,
        rgb(COLOR_SETPOINT_SHADOW),
        inner.right - 1,
        &FONT_REGULAR,
    );
    draw_small_clipped_with_font(
        canvas,
        unit_s,
        unit_x,
        unit_y,
        rgb(COLOR_TEXT_UNIT),
        inner.right - 1,
        &FONT_REGULAR,
    );

    if focused {
        let idx = match focused_digit {
            AdjustDigit::Ones => Some(1),
            AdjustDigit::Tenths => Some(3),
            AdjustDigit::Hundredths => Some(4),
            _ => None,
        };
        if let Some(idx) = idx {
            let cell_x = num_x0 + idx as i32 * cell_w;
            // The frozen mock uses a 1px underline, aligned ~2px above the outer border.
            let underline_top = rect.bottom - 3;
            let ul_pad = ((SETPOINT_CELL_W - SETPOINT_PILL_DIGIT_W) / 2) as i32;
            canvas.fill_rect(
                Rect::new(
                    cell_x + ul_pad,
                    underline_top,
                    cell_x + ul_pad + SETPOINT_PILL_DIGIT_W as i32,
                    underline_top + 1,
                ),
                rgb(COLOR_VALUE_PILL_FOCUSED_OUTER),
            );
        }
    }
}

const SETPOINT_SRC_W: usize = 10;
const SETPOINT_SRC_H: usize = 18;
// The PD settings value pill uses a slightly condensed setpoint layout compared to
// the raw 10px UTFT glyph cell, matching the frozen mock assets.
const SETPOINT_CELL_W: usize = 8;
const SETPOINT_PILL_DIGIT_W: usize = 8;
const SETPOINT_PILL_DIGIT_H: usize = 15;

fn draw_setpoint_text_scaled(
    canvas: &mut Canvas,
    text: &str,
    x: i32,
    y: i32,
    color: Rgb565,
    spacing: i32,
    clip: Rect,
) {
    const HR_SCALE: usize = 4;
    const HR_W: usize = SETPOINT_SRC_W * HR_SCALE;
    const HR_H: usize = SETPOINT_SRC_H * HR_SCALE;

    debug_assert_eq!(super::fonts::SETPOINT_FONT.width() as usize, SETPOINT_SRC_W);
    debug_assert_eq!(
        super::fonts::SETPOINT_FONT.height() as usize,
        SETPOINT_SRC_H
    );

    let glyph = SETPOINT_CELL_W as i32 + spacing;
    let x_pad = ((SETPOINT_CELL_W - SETPOINT_PILL_DIGIT_W) / 2) as i32;
    let mut cursor_x = x;

    for ch in text.chars() {
        if ch == ' ' {
            cursor_x += glyph;
            continue;
        }

        let mut src = [0u8; SETPOINT_SRC_W * SETPOINT_SRC_H];
        super::fonts::SETPOINT_FONT.draw_char(
            ch,
            |px, py| {
                if px < 0 || py < 0 {
                    return;
                }
                let px = px as usize;
                let py = py as usize;
                if px >= SETPOINT_SRC_W || py >= SETPOINT_SRC_H {
                    return;
                }
                src[py * SETPOINT_SRC_W + px] = 1;
            },
            0,
            0,
        );

        let mut hr = [0u8; HR_W * HR_H];
        for py in 0..SETPOINT_SRC_H {
            for px in 0..SETPOINT_SRC_W {
                if src[py * SETPOINT_SRC_W + px] == 0 {
                    continue;
                }
                let hx0 = px * HR_SCALE;
                let hy0 = py * HR_SCALE;
                for sy in 0..HR_SCALE {
                    for sx in 0..HR_SCALE {
                        hr[(hy0 + sy) * HR_W + (hx0 + sx)] = 1;
                    }
                }
            }
        }

        // Second pass: compute per-pixel alpha grid for the scaled glyph, then render a subtle halo.
        let mut alpha_grid = [0u8; SETPOINT_PILL_DIGIT_W * SETPOINT_PILL_DIGIT_H];
        for oy in 0..SETPOINT_PILL_DIGIT_H {
            let hy0 = oy * HR_H / SETPOINT_PILL_DIGIT_H;
            let hy1 = (oy + 1) * HR_H / SETPOINT_PILL_DIGIT_H;
            if hy1 <= hy0 {
                continue;
            }
            for ox in 0..SETPOINT_PILL_DIGIT_W {
                let hx0 = ox * HR_W / SETPOINT_PILL_DIGIT_W;
                let hx1 = (ox + 1) * HR_W / SETPOINT_PILL_DIGIT_W;
                if hx1 <= hx0 {
                    continue;
                }

                let mut inside: u16 = 0;
                for hy in hy0..hy1 {
                    for hx in hx0..hx1 {
                        inside += hr[hy * HR_W + hx] as u16;
                    }
                }
                if inside == 0 {
                    continue;
                }
                let denom = ((hx1 - hx0) * (hy1 - hy0)) as u16;
                if denom == 0 {
                    continue;
                }
                let alpha = ((inside * 255 + (denom / 2)) / denom) as u8;
                alpha_grid[oy * SETPOINT_PILL_DIGIT_W + ox] = alpha;
            }
        }

        // Halo pass: blend into 1px neighbors where the glyph is empty.
        let mut halo_grid = [0u8; SETPOINT_PILL_DIGIT_W * SETPOINT_PILL_DIGIT_H];
        for oy in 0..SETPOINT_PILL_DIGIT_H {
            for ox in 0..SETPOINT_PILL_DIGIT_W {
                let a = alpha_grid[oy * SETPOINT_PILL_DIGIT_W + ox];
                if a < 200 {
                    continue;
                }
                for (dx, dy) in [
                    (-1i32, -1i32),
                    (0, -1),
                    (1, -1),
                    (-1, 0),
                    (1, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                ] {
                    let nx = ox as i32 + dx;
                    let ny = oy as i32 + dy;
                    if nx < 0
                        || ny < 0
                        || nx >= SETPOINT_PILL_DIGIT_W as i32
                        || ny >= SETPOINT_PILL_DIGIT_H as i32
                    {
                        continue;
                    }
                    let nidx = ny as usize * SETPOINT_PILL_DIGIT_W + nx as usize;
                    if alpha_grid[nidx] != 0 {
                        continue;
                    }
                    let ha = ((a as u16 * 2) / 5).min(110) as u8;
                    halo_grid[nidx] = halo_grid[nidx].max(ha);
                }
            }
        }

        for oy in 0..SETPOINT_PILL_DIGIT_H {
            for ox in 0..SETPOINT_PILL_DIGIT_W {
                let ha = halo_grid[oy * SETPOINT_PILL_DIGIT_W + ox];
                if ha == 0 {
                    continue;
                }
                let gx = cursor_x + x_pad + ox as i32;
                let gy = y + oy as i32;
                if gx < clip.left || gx >= clip.right || gy < clip.top || gy >= clip.bottom {
                    continue;
                }
                canvas.blend_pixel(gx, gy, rgb(COLOR_SETPOINT_SHADOW), ha);
            }
        }

        // Main pass.
        for oy in 0..SETPOINT_PILL_DIGIT_H {
            for ox in 0..SETPOINT_PILL_DIGIT_W {
                let a = alpha_grid[oy * SETPOINT_PILL_DIGIT_W + ox];
                if a == 0 {
                    continue;
                }
                let gx = cursor_x + x_pad + ox as i32;
                let gy = y + oy as i32;
                if gx < clip.left || gx >= clip.right || gy < clip.top || gy >= clip.bottom {
                    continue;
                }
                canvas.blend_pixel(gx, gy, color, a);
            }
        }

        cursor_x += glyph;
    }
}

fn draw_round_rect_2px_border_accent_field(
    canvas: &mut Canvas,
    rect: Rect,
    radius: i32,
    fill: Rgb565,
) {
    draw_round_rect_2px_border_accent_core(
        canvas,
        rect,
        radius,
        fill,
        rgb(COLOR_ACCENT_BTN_OUTER_BASE),
        rgb(COLOR_ACCENT_OUTER_TOP),
        rgb(COLOR_ACCENT_BTN_OUTER_BOTTOM),
        rgb(COLOR_ACCENT),
        rgb(COLOR_ACCENT_SHADOW),
    );
}

fn draw_round_rect_2px_border_hard(
    canvas: &mut Canvas,
    rect: Rect,
    radius: i32,
    outer: Rgb565,
    inner_border: Rgb565,
    fill: Rgb565,
) {
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    if w <= 0 || h <= 0 {
        return;
    }

    canvas.fill_round_rect(rect, radius, outer);
    let mid = Rect::new(rect.left + 1, rect.top + 1, rect.right - 1, rect.bottom - 1);
    canvas.fill_round_rect(mid, (radius - 1).max(0), inner_border);
    let inner = Rect::new(rect.left + 2, rect.top + 2, rect.right - 2, rect.bottom - 2);
    canvas.fill_round_rect(inner, (radius - 2).max(0), fill);
}

fn draw_list_row_fixed(
    canvas: &mut Canvas,
    rect: Rect,
    idx: u8,
    pdo: loadlynx_protocol::FixedPdo,
    selected: bool,
) {
    if selected {
        draw_round_rect_2px_border_accent_row(canvas, rect, LIST_ROW_RADIUS, rgb(COLOR_TOP_BG));
    } else {
        draw_round_rect_2px_border(
            canvas,
            rect,
            LIST_ROW_RADIUS,
            rgb(COLOR_BORDER),
            rgb(COLOR_BORDER),
            rgb(COLOR_ROW_BG),
        );
    }

    let v = format_v_short(pdo.mv);
    let mut right = String::<8>::new();
    let pos = effective_pos(pdo.pos, idx as usize);
    let _ = write!(&mut right, "PDO{}", pos);

    draw_small(
        canvas,
        v.as_str(),
        rect.left + 10,
        rect.top + 8,
        rgb(COLOR_TEXT_VALUE),
    );

    let mut mid = String::<16>::new();
    let _ = write!(&mut mid, "Imax {}", format_a_short(pdo.max_ma).as_str());
    let text_y = rect.top + 8;
    let info_x = rect.left + 58;
    let dim = rgb(COLOR_TEXT_DIM);
    draw_dot_joined_small(canvas, info_x, text_y, mid.as_str(), right.as_str(), dim);
}

fn draw_list_row_pps(
    canvas: &mut Canvas,
    rect: Rect,
    idx: u8,
    pdo: loadlynx_protocol::PpsPdo,
    selected: bool,
) {
    if selected {
        draw_round_rect_2px_border_accent_row(canvas, rect, LIST_ROW_RADIUS, rgb(COLOR_TOP_BG));
    } else {
        draw_round_rect_2px_border(
            canvas,
            rect,
            LIST_ROW_RADIUS,
            rgb(COLOR_BORDER),
            rgb(COLOR_BORDER),
            rgb(COLOR_ROW_BG),
        );
    }

    let range = format_v_range(pdo.min_mv, pdo.max_mv);
    let pos = effective_pos(pdo.pos, idx as usize);
    let mut line2_left = String::<16>::new();
    let _ = write!(
        &mut line2_left,
        "Imax {}",
        format_a_short(pdo.max_ma).as_str()
    );
    let mut line2_right = String::<8>::new();
    let _ = write!(&mut line2_right, "APDO{}", pos);

    let y1 = rect.top + LIST_ROW_LINE1_Y_OFF;
    let y2 = rect.top + LIST_ROW_LINE2_Y_OFF;

    draw_small(
        canvas,
        range.as_str(),
        rect.left + 10,
        y1,
        rgb(COLOR_TEXT_VALUE),
    );
    let dim = rgb(COLOR_TEXT_DIM);
    draw_dot_joined_small(
        canvas,
        rect.left + 10,
        y2,
        line2_left.as_str(),
        line2_right.as_str(),
        dim,
    );
}

fn draw_list_row_missing(canvas: &mut Canvas, rect: Rect, note: &str) {
    draw_round_rect_2px_border(
        canvas,
        rect,
        LIST_ROW_RADIUS,
        rgb(COLOR_BORDER),
        rgb(COLOR_BORDER),
        rgb(COLOR_ROW_BG),
    );

    let y1 = rect.top + LIST_ROW_LINE1_Y_OFF;
    let y2 = rect.top + LIST_ROW_LINE2_Y_OFF;

    draw_missing_dash(canvas, rect.left + 10, y1, rgb(COLOR_TEXT_VALUE));
    draw_small_title(
        canvas,
        note,
        rect.left + 10,
        y2,
        rgb(COLOR_TEXT_DIM),
        rect.right - 10,
    );
}

fn list_row_metrics(mode: PdMode) -> (i32, i32) {
    match mode {
        PdMode::Fixed => (LIST_ROW_H_FIXED, LIST_ROW_GAP_FIXED),
        PdMode::Pps => (LIST_ROW_H_PPS, LIST_ROW_GAP_PPS),
    }
}

fn draw_selected_fixed_summary(canvas: &mut Canvas, vm: &PdSettingsVm, rect: Rect) {
    let selected = vm
        .fixed_pdos
        .iter()
        .enumerate()
        .find(|(idx, pdo)| effective_pos(pdo.pos, *idx) == vm.fixed_object_pos)
        .map(|(idx, pdo)| (idx, *pdo));

    let mut line1 = String::<20>::new();
    let mut line2_left = String::<8>::new();
    let mut line2_right = String::<16>::new();

    if let Some((idx, pdo)) = selected {
        let _ = write!(&mut line1, "Fixed {}", format_v_short(pdo.mv).as_str());
        let pos = effective_pos(pdo.pos, idx);
        let _ = write!(&mut line2_left, "PDO{}", pos);
        let _ = write!(
            &mut line2_right,
            "Imax {}",
            format_a_short(pdo.max_ma).as_str()
        );
    } else if vm.fixed_object_pos == 0 {
        let _ = line1.push_str("Fixed (select PDO)");
        let _ = line2_left.push_str("Tap a row to select");
    } else {
        let _ = line1.push_str("Fixed (missing)");
        let _ = write!(&mut line2_left, "PDO{}", vm.fixed_object_pos);
    }

    draw_small(
        canvas,
        line1.as_str(),
        rect.left + 10,
        rect.top + SELECTED_SUM_LINE1_Y_OFF,
        rgb(COLOR_TEXT_VALUE),
    );
    if !line2_right.is_empty() {
        draw_dot_joined_small(
            canvas,
            rect.left + 10,
            rect.top + SELECTED_SUM_LINE2_Y_OFF,
            line2_left.as_str(),
            line2_right.as_str(),
            rgb(COLOR_TEXT_DIM),
        );
    } else {
        draw_small(
            canvas,
            line2_left.as_str(),
            rect.left + 10,
            rect.top + SELECTED_SUM_LINE2_Y_OFF,
            rgb(COLOR_TEXT_DIM),
        );
    }
}

fn draw_selected_pps_summary(canvas: &mut Canvas, vm: &PdSettingsVm, rect: Rect) {
    let selected = vm
        .pps_pdos
        .iter()
        .enumerate()
        .find(|(idx, pdo)| effective_pos(pdo.pos, *idx) == vm.pps_object_pos)
        .map(|(idx, pdo)| (idx, *pdo));

    let mut line1 = String::<24>::new();
    let mut line2_left = String::<24>::new();
    let mut line2_right = String::<16>::new();

    if let Some((idx, pdo)) = selected {
        let _ = write!(
            &mut line1,
            "PPS {}",
            format_v_range(pdo.min_mv, pdo.max_mv).as_str()
        );
        let pos = effective_pos(pdo.pos, idx);
        let _ = write!(&mut line2_left, "APDO{}", pos);
        let _ = write!(
            &mut line2_right,
            "Imax {}",
            format_a_short(pdo.max_ma).as_str()
        );
    } else if vm.pps_object_pos == 0 {
        let _ = line1.push_str("PPS (select APDO)");
        let _ = line2_left.push_str("Tap a row to select");
    } else {
        let _ = line1.push_str("PPS (missing)");
        let _ = write!(&mut line2_left, "APDO{}", vm.pps_object_pos);
    }

    draw_small(
        canvas,
        line1.as_str(),
        rect.left + 10,
        rect.top + SELECTED_SUM_LINE1_Y_OFF,
        rgb(COLOR_TEXT_VALUE),
    );
    if !line2_right.is_empty() {
        draw_dot_joined_small(
            canvas,
            rect.left + 10,
            rect.top + SELECTED_SUM_LINE2_Y_OFF,
            line2_left.as_str(),
            line2_right.as_str(),
            rgb(COLOR_TEXT_DIM),
        );
    } else {
        draw_small(
            canvas,
            line2_left.as_str(),
            rect.left + 10,
            rect.top + SELECTED_SUM_LINE2_Y_OFF,
            rgb(COLOR_TEXT_DIM),
        );
    }
}

fn draw_small(canvas: &mut Canvas, s: &str, x: i32, y: i32, color: Rgb565) {
    draw_small_clipped(canvas, s, x, y, color, super::LOGICAL_WIDTH);
}

fn draw_small_bold(canvas: &mut Canvas, s: &str, x: i32, y: i32, color: Rgb565) {
    draw_small_clipped_with_font(canvas, s, x, y, color, super::LOGICAL_WIDTH, &FONT_BOLD);
}

fn draw_small_title(canvas: &mut Canvas, s: &str, x: i32, y: i32, color: Rgb565, clip_right: i32) {
    draw_small_clipped(canvas, s, x, y, color, clip_right);
}

fn draw_centered_small(canvas: &mut Canvas, s: &str, rect: Rect, color: Rgb565) {
    draw_centered_small_with_font(canvas, s, rect, color, &FONT_REGULAR);
}

fn draw_centered_small_bold(canvas: &mut Canvas, s: &str, rect: Rect, color: Rgb565) {
    draw_centered_small_with_font(canvas, s, rect, color, &FONT_BOLD);
}

fn draw_centered_small_with_font(
    canvas: &mut Canvas,
    s: &str,
    rect: Rect,
    color: Rgb565,
    font: &FontPair,
) {
    if s.is_empty() {
        return;
    }

    // Use the font engine's own centered positioning to avoid per-string bbox drift
    // (which tends to place short strings like "Apply" slightly too high).
    let cx = rect.left + ((rect.right - rect.left).max(0) / 2);
    let cy = rect.top + ((rect.bottom - rect.top).max(0) / 2);

    let mut clipped = ClipCanvas {
        canvas,
        clip_right: rect.right,
    };
    let _ = font.base.render_aligned(
        s,
        Point::new(cx, cy),
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(color),
        &mut clipped,
    );
}

fn draw_small_clipped(
    canvas: &mut Canvas,
    s: &str,
    x: i32,
    y: i32,
    color: Rgb565,
    clip_right: i32,
) {
    draw_small_clipped_with_font(canvas, s, x, y, color, clip_right, &FONT_REGULAR);
}

struct ClipCanvas<'a, 'b> {
    canvas: &'a mut Canvas<'b>,
    clip_right: i32,
}

impl embedded_graphics::geometry::OriginDimensions for ClipCanvas<'_, '_> {
    fn size(&self) -> embedded_graphics::prelude::Size {
        embedded_graphics::prelude::Size::new(
            super::LOGICAL_WIDTH as u32,
            super::LOGICAL_HEIGHT as u32,
        )
    }
}

impl embedded_graphics::draw_target::DrawTarget for ClipCanvas<'_, '_> {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::prelude::Pixel<Self::Color>>,
    {
        for embedded_graphics::prelude::Pixel(coord, color) in pixels {
            if coord.x < self.clip_right {
                self.canvas.set_pixel(coord.x, coord.y, color);
            }
        }
        Ok(())
    }
}

fn draw_small_clipped_with_font(
    canvas: &mut Canvas,
    s: &str,
    x: i32,
    y: i32,
    color: Rgb565,
    clip_right: i32,
    font: &FontPair,
) {
    if x >= clip_right || s.is_empty() {
        return;
    }

    if draw_small_aa_clipped(canvas, s, x, y, color, clip_right, font) {
        return;
    }

    let mut clipped = ClipCanvas { canvas, clip_right };
    let (off_x, off_y) = text_bbox_offset(s, font);
    let pos = Point::new(x - off_x, y - off_y);
    let _ = font.base.render(
        s,
        pos,
        VerticalPosition::Top,
        FontColor::Transparent(color),
        &mut clipped,
    );
}

fn text_width(text: &str) -> i32 {
    text_advance_scaled(text, &FONT_REGULAR)
}

fn text_height(text: &str, font: &FontPair) -> i32 {
    text_bbox_scaled(text, font).map(|(_, h)| h).unwrap_or(0)
}

fn text_bbox_offset(text: &str, font: &FontPair) -> (i32, i32) {
    match font
        .base
        .get_rendered_dimensions(text, Point::new(0, 0), VerticalPosition::Top)
    {
        Ok(dims) => dims
            .bounding_box
            .map(|b| (b.top_left.x, b.top_left.y))
            .unwrap_or((0, 0)),
        Err(_) => (0, 0),
    }
}

fn text_bbox_scaled(text: &str, font: &FontPair) -> Option<(i32, i32)> {
    if text.is_empty() {
        return Some((0, 0));
    }
    let dims = font
        .aa
        .get_rendered_dimensions(text, Point::new(0, 0), VerticalPosition::Top)
        .ok()?;
    let bbox = dims.bounding_box?;
    let w = ((bbox.size.width as i32) + (TEXT_AA_SCALE as i32 - 1)) / (TEXT_AA_SCALE as i32);
    let h = ((bbox.size.height as i32) + (TEXT_AA_SCALE as i32 - 1)) / (TEXT_AA_SCALE as i32);
    Some((w, h))
}

fn text_advance_scaled(text: &str, font: &FontPair) -> i32 {
    if text.is_empty() {
        return 0;
    }
    let Ok(dims) = font
        .aa
        .get_rendered_dimensions(text, Point::new(0, 0), VerticalPosition::Top)
    else {
        return 0;
    };
    let w = dims.advance.x.max(0);
    (w + (TEXT_AA_SCALE as i32 - 1)) / (TEXT_AA_SCALE as i32)
}

struct MaskCanvas<'a> {
    buf: &'a mut [u8],
    width: usize,
    height: usize,
}

impl embedded_graphics::geometry::OriginDimensions for MaskCanvas<'_> {
    fn size(&self) -> embedded_graphics::prelude::Size {
        embedded_graphics::prelude::Size::new(self.width as u32, self.height as u32)
    }
}

impl embedded_graphics::draw_target::DrawTarget for MaskCanvas<'_> {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::prelude::Pixel<Self::Color>>,
    {
        for embedded_graphics::prelude::Pixel(coord, _color) in pixels {
            if coord.x < 0 || coord.y < 0 {
                continue;
            }
            let x = coord.x as usize;
            let y = coord.y as usize;
            if x >= self.width || y >= self.height {
                continue;
            }
            let bit = y * self.width + x;
            let byte = bit >> 3;
            let mask = 1u8 << (bit & 7);
            if byte < self.buf.len() {
                self.buf[byte] |= mask;
            }
        }
        Ok(())
    }
}

fn draw_small_aa_clipped(
    canvas: &mut Canvas,
    s: &str,
    x: i32,
    y: i32,
    color: Rgb565,
    clip_right: i32,
    font: &FontPair,
) -> bool {
    const MASK_MAX: usize = 4096;

    let dims = match font
        .aa
        .get_rendered_dimensions(s, Point::new(0, 0), VerticalPosition::Top)
    {
        Ok(dims) => dims,
        Err(_) => return false,
    };
    let Some(bbox) = dims.bounding_box else {
        return true;
    };

    let w = bbox.size.width as usize;
    let h = bbox.size.height as usize;
    if w == 0 || h == 0 {
        return true;
    }

    let needed_bits = w.saturating_mul(h);
    let needed = (needed_bits + 7) / 8;
    if needed == 0 || needed > MASK_MAX {
        return false;
    }

    let mut mask = [0u8; MASK_MAX];
    for v in &mut mask[..needed] {
        *v = 0;
    }

    let mut target = MaskCanvas {
        buf: &mut mask[..needed],
        width: w,
        height: h,
    };

    let pos = Point::new(-bbox.top_left.x, -bbox.top_left.y);
    let _ = font.aa.render(
        s,
        pos,
        VerticalPosition::Top,
        FontColor::Transparent(color),
        &mut target,
    );

    let out_w = (w + (TEXT_AA_SCALE - 1)) / TEXT_AA_SCALE;
    let out_h = (h + (TEXT_AA_SCALE - 1)) / TEXT_AA_SCALE;
    let denom = (TEXT_AA_SCALE * TEXT_AA_SCALE) as u16;

    for oy in 0..out_h {
        let dy = y + oy as i32;
        for ox in 0..out_w {
            let dx = x + ox as i32;
            if dx >= clip_right {
                continue;
            }
            let mut inside = 0u8;
            for sy in 0..TEXT_AA_SCALE {
                let hy = oy * TEXT_AA_SCALE + sy;
                if hy >= h {
                    continue;
                }
                for sx in 0..TEXT_AA_SCALE {
                    let hx = ox * TEXT_AA_SCALE + sx;
                    if hx >= w {
                        continue;
                    }
                    let bit = hy * w + hx;
                    let byte = bit >> 3;
                    let mask_bit = 1u8 << (bit & 7);
                    if (mask[byte] & mask_bit) != 0 {
                        inside = inside.saturating_add(1);
                    }
                }
            }
            if inside == 0 {
                continue;
            }
            let alpha = (((inside as u16) * 255 + (denom / 2)) / denom) as u8;
            canvas.blend_pixel(dx, dy, color, alpha);
        }
    }

    true
}

fn draw_label_with_step(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    label: &str,
    step: Option<&str>,
    label_color: Rgb565,
    clip_right: i32,
) {
    draw_small_clipped(canvas, label, x, y, label_color, clip_right);
    let Some(step) = step else {
        return;
    };

    let label_w = text_width(label);
    let dot = 2;
    let dot_x = x + label_w + DOT_GAP_PX;
    let dot_y = y + (text_height("A", &FONT_REGULAR) / 2) - (dot / 2);
    if dot_x + dot >= clip_right {
        return;
    }
    canvas.fill_rect(
        Rect::new(dot_x, dot_y, dot_x + dot, dot_y + dot),
        label_color,
    );
    draw_small_clipped(
        canvas,
        step,
        dot_x + dot + DOT_GAP_PX,
        y,
        label_color,
        clip_right,
    );
}

fn draw_label_two_line(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    line1: &str,
    line2: &str,
    line1_color: Rgb565,
    line2_color: Rgb565,
    clip_right: i32,
) {
    if !line1.is_empty() {
        draw_small_clipped(canvas, line1, x, y, line1_color, clip_right);
    }
    if !line2.is_empty() {
        let line_h = text_height("A", &FONT_REGULAR);
        draw_small_clipped(canvas, line2, x, y + line_h, line2_color, clip_right);
    }
}

fn draw_dot_joined_small(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    left: &str,
    right: &str,
    color: Rgb565,
) {
    if left.is_empty() {
        draw_small(canvas, right, x, y, color);
        return;
    }
    if right.is_empty() {
        draw_small(canvas, left, x, y, color);
        return;
    }

    draw_small(canvas, left, x, y, color);
    let left_w = text_width(left);
    let dot = 2;
    let dot_x = x + left_w + DOT_GAP_PX;
    let dot_y = y + (text_height("A", &FONT_REGULAR) / 2) - (dot / 2);
    canvas.fill_rect(Rect::new(dot_x, dot_y, dot_x + dot, dot_y + dot), color);
    draw_small(canvas, right, dot_x + dot + DOT_GAP_PX, y, color);
}

fn draw_missing_dash(canvas: &mut Canvas, x: i32, y: i32, color: Rgb565) {
    let w = 18;
    let h = 2;
    let y = y + (text_height("A", &FONT_REGULAR) / 2) - (h / 2);
    canvas.fill_rect(Rect::new(x, y, x + w, y + h), color);
}

fn draw_round_rect_2px_border(
    canvas: &mut Canvas,
    rect: Rect,
    radius: i32,
    outer: Rgb565,
    inner_border: Rgb565,
    fill: Rgb565,
) {
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    if w <= 0 || h <= 0 {
        return;
    }

    canvas.fill_round_rect_aa(rect, radius, outer);
    let mid = Rect::new(rect.left + 1, rect.top + 1, rect.right - 1, rect.bottom - 1);
    canvas.fill_round_rect_aa(mid, (radius - 1).max(0), inner_border);
    let inner = Rect::new(rect.left + 2, rect.top + 2, rect.right - 2, rect.bottom - 2);
    canvas.fill_round_rect_aa(inner, (radius - 2).max(0), fill);
}

fn draw_round_rect_2px_border_accent_row(
    canvas: &mut Canvas,
    rect: Rect,
    radius: i32,
    fill: Rgb565,
) {
    // 2px bevelled accent border: outer stroke + inner stroke, with top/bottom tinting.
    draw_round_rect_2px_border_accent_core(
        canvas,
        rect,
        radius,
        fill,
        rgb(COLOR_ACCENT_DARK),
        rgb(COLOR_ACCENT_OUTER_TOP),
        rgb(COLOR_ACCENT_OUTER_BOTTOM),
        rgb(COLOR_ACCENT),
        rgb(COLOR_ACCENT_SHADOW),
    );
}

fn draw_round_rect_2px_border_accent_button(
    canvas: &mut Canvas,
    rect: Rect,
    radius: i32,
    fill: Rgb565,
) {
    draw_round_rect_2px_border_accent_core(
        canvas,
        rect,
        radius,
        fill,
        rgb(COLOR_ACCENT_BTN_OUTER_BASE),
        // Frozen mock uses a bright top highlight for focused fields/buttons.
        rgb(COLOR_ACCENT),
        rgb(COLOR_ACCENT_BTN_OUTER_BOTTOM),
        // Inner top edge is not stroked in the frozen mock; keep it filled.
        fill,
        rgb(COLOR_ACCENT_SHADOW),
    );
}

fn draw_round_rect_2px_border_accent_core(
    canvas: &mut Canvas,
    rect: Rect,
    radius: i32,
    fill: Rgb565,
    outer_base: Rgb565,
    outer_top: Rgb565,
    outer_bottom: Rgb565,
    inner_top: Rgb565,
    inner_bottom: Rgb565,
) {
    canvas.fill_round_rect_aa(rect, radius, outer_base);
    let mid = Rect::new(rect.left + 1, rect.top + 1, rect.right - 1, rect.bottom - 1);
    canvas.fill_round_rect_aa(mid, (radius - 1).max(0), rgb(COLOR_ACCENT_INNER));
    let inner = Rect::new(rect.left + 2, rect.top + 2, rect.right - 2, rect.bottom - 2);
    canvas.fill_round_rect_aa(inner, (radius - 2).max(0), fill);

    // Outer stroke shading.
    overlay_round_rect_edge_h(canvas, rect, radius, rect.top, outer_top);
    overlay_round_rect_edge_h(canvas, rect, radius, rect.bottom - 1, outer_bottom);

    // Inner stroke shading.
    overlay_round_rect_edge_h(canvas, mid, (radius - 1).max(0), mid.top, inner_top);
    overlay_round_rect_edge_h(
        canvas,
        mid,
        (radius - 1).max(0),
        mid.bottom - 1,
        inner_bottom,
    );
}

fn overlay_round_rect_edge_h(canvas: &mut Canvas, rect: Rect, radius: i32, y: i32, color: Rgb565) {
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    if w <= 0 || h <= 0 {
        return;
    }
    if y < rect.top || y >= rect.bottom {
        return;
    }

    let mut r = radius.max(0);
    r = r.min(w / 2).min(h / 2);
    if r == 0 {
        canvas.fill_rect(Rect::new(rect.left, y, rect.right, y + 1), color);
        return;
    }

    // Apply edge tint including the rounded corners. Use coverage-based blending so the
    // highlighted edge stays anti-aliased in the corner arcs.
    const SUB: i32 = 4;
    const SCALE: i64 = (SUB * 2) as i64;
    let r_u = (r as i64) * SCALE;
    let r_u2 = r_u * r_u;

    let left_core = (rect.left + r) as i64 * SCALE;
    let right_core = (rect.right - r) as i64 * SCALE;
    let top_core = (rect.top + r) as i64 * SCALE;
    let bottom_core = (rect.bottom - r) as i64 * SCALE;

    let y_u_base = (y as i64) * SCALE;

    for x0 in rect.left..rect.right {
        let x_u_base = (x0 as i64) * SCALE;
        let mut inside: u16 = 0;

        for sy in 0..SUB {
            let yy = y_u_base + (2 * sy + 1) as i64;
            let in_center_y = yy >= top_core && yy < bottom_core;

            for sx in 0..SUB {
                let xx = x_u_base + (2 * sx + 1) as i64;
                let in_center_x = xx >= left_core && xx < right_core;
                let mut ok = in_center_x || in_center_y;

                if !ok {
                    let (cx, cy) = if xx < left_core {
                        if yy < top_core {
                            (left_core, top_core)
                        } else {
                            (left_core, bottom_core)
                        }
                    } else if yy < top_core {
                        (right_core, top_core)
                    } else {
                        (right_core, bottom_core)
                    };

                    let dx = (xx - cx).abs();
                    let dy = (yy - cy).abs();
                    let dx2 = dx * dx;
                    let dy2 = dy * dy;
                    ok = dx2 + dy2 <= r_u2;
                }

                if ok {
                    inside += 1;
                }
            }
        }

        if inside == 0 {
            continue;
        }
        let alpha = ((inside * 255 + (SUB * SUB / 2) as u16) / (SUB * SUB) as u16) as u8;
        canvas.blend_pixel(x0, y, color, alpha);
    }
}

fn draw_round_rect_1px_border(
    canvas: &mut Canvas,
    rect: Rect,
    radius: i32,
    border: Rgb565,
    fill: Rgb565,
) {
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    if w <= 0 || h <= 0 {
        return;
    }
    canvas.fill_round_rect_aa(rect, radius, border);
    let inner = Rect::new(rect.left + 1, rect.top + 1, rect.right - 1, rect.bottom - 1);
    canvas.fill_round_rect_aa(inner, (radius - 1).max(0), fill);
}

fn draw_round_rect_1px_border_accent_field(
    canvas: &mut Canvas,
    rect: Rect,
    radius: i32,
    fill: Rgb565,
) {
    // 1px accent border with top/bottom shading, as used by focused value fields in the frozen mocks.
    canvas.fill_round_rect_aa(rect, radius, rgb(COLOR_ACCENT_INNER));
    let inner = Rect::new(rect.left + 1, rect.top + 1, rect.right - 1, rect.bottom - 1);
    canvas.fill_round_rect_aa(inner, (radius - 1).max(0), fill);

    overlay_round_rect_edge_h(canvas, rect, radius, rect.top, rgb(COLOR_ACCENT));
    overlay_round_rect_edge_h(
        canvas,
        rect,
        radius,
        rect.bottom - 1,
        rgb(COLOR_ACCENT_SHADOW),
    );
}

fn hit_in_rect(x: i32, y: i32, rect: Rect) -> bool {
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
}

fn effective_pos(pos: u8, idx: usize) -> u8 {
    if pos != 0 {
        pos
    } else {
        (idx + 1).min(u8::MAX as usize) as u8
    }
}

fn fixed_selection_missing(vm: &PdSettingsVm) -> bool {
    if vm.mode != PdMode::Fixed {
        return false;
    }
    if vm.fixed_object_pos == 0 {
        return false;
    }
    !vm.fixed_pdos
        .iter()
        .enumerate()
        .any(|(idx, pdo)| effective_pos(pdo.pos, idx) == vm.fixed_object_pos)
}

fn pps_selection_missing(vm: &PdSettingsVm) -> bool {
    if vm.mode != PdMode::Pps {
        return false;
    }
    if vm.pps_object_pos == 0 {
        return false;
    }
    !vm.pps_pdos
        .iter()
        .enumerate()
        .any(|(idx, pdo)| effective_pos(pdo.pos, idx) == vm.pps_object_pos)
}

fn format_v_contract_1dp(mv: u32) -> String<8> {
    let mut out = String::<8>::new();
    let v10 = mv / 100;
    let _ = write!(&mut out, "{}.{}V", v10 / 10, v10 % 10);
    out
}

fn format_a_contract_1dp(ma: u32) -> String<8> {
    let mut out = String::<8>::new();
    let a10 = ma / 100;
    let _ = write!(&mut out, "{}.{}A", a10 / 10, a10 % 10);
    out
}

fn format_v_short(mv: u32) -> String<8> {
    let mut out = String::<8>::new();
    if mv % 1000 == 0 {
        let _ = write!(&mut out, "{}V", mv / 1000);
    } else {
        let v10 = mv / 100;
        let _ = write!(&mut out, "{}.{}V", v10 / 10, v10 % 10);
    }
    out
}

fn format_v_2dp(mv: u32) -> String<10> {
    let mut out = String::<10>::new();
    let v100 = mv / 10;
    let _ = write!(
        &mut out,
        "{}.{}{}V",
        v100 / 100,
        (v100 / 10) % 10,
        v100 % 10
    );
    out
}

fn format_v_range(min_mv: u32, max_mv: u32) -> String<16> {
    let mut out = String::<16>::new();
    let min = format_v_end(min_mv);
    let max = format_v_end(max_mv);
    let _ = write!(&mut out, "{}-{}V", min.as_str(), max.as_str());
    out
}

fn format_v_end(mv: u32) -> String<8> {
    let mut out = String::<8>::new();
    if mv % 1000 == 0 {
        let _ = write!(&mut out, "{}", mv / 1000);
    } else {
        let v10 = mv / 100;
        let _ = write!(&mut out, "{}.{}", v10 / 10, v10 % 10);
    }
    out
}

fn format_a_short(ma: u32) -> String<8> {
    let mut out = String::<8>::new();
    if ma % 1000 == 0 {
        let _ = write!(&mut out, "{}A", ma / 1000);
    } else {
        let a10 = ma / 100;
        let _ = write!(&mut out, "{}.{}A", a10 / 10, a10 % 10);
    }
    out
}

fn format_ma(ma: u32) -> String<10> {
    // Kept for older call sites; PD target values now format in A/V.
    let mut out = String::<10>::new();
    let _ = write!(&mut out, "{}mA", ma);
    out
}

fn format_req_v_2dp_digits(mv: u32) -> String<5> {
    let mv = mv.min(99_990);
    let int_part = (mv / 1000).min(99);
    let frac = ((mv % 1000) / 10).min(99); // 2dp

    let mut out = String::<5>::new();
    let tens = (int_part / 10) as u8;
    let ones = (int_part % 10) as u8;
    let frac_tens = (frac / 10) as u8;
    let frac_ones = (frac % 10) as u8;

    // Keep the 2-digit integer field width (DD.dd) but don't draw a leading '0' when < 10.
    // Use a "non-glyph" placeholder that still advances the setpoint font cursor.
    let _ = out.push(if tens == 0 {
        ' '
    } else {
        (b'0' + tens) as char
    });
    let _ = out.push((b'0' + ones) as char);
    let _ = out.push('.');
    let _ = out.push((b'0' + frac_tens) as char);
    let _ = out.push((b'0' + frac_ones) as char);
    out
}

fn format_req_a_2dp_digits(ma: u32) -> String<5> {
    let ma = ma.min(99_990);
    let int_part = (ma / 1000).min(99);
    let frac = ((ma % 1000) / 10).min(99); // 2dp

    let mut out = String::<5>::new();
    let tens = (int_part / 10) as u8;
    let ones = (int_part % 10) as u8;
    let frac_tens = (frac / 10) as u8;
    let frac_ones = (frac % 10) as u8;

    let _ = out.push(if tens == 0 {
        ' '
    } else {
        (b'0' + tens) as char
    });
    let _ = out.push((b'0' + ones) as char);
    let _ = out.push('.');
    let _ = out.push((b'0' + frac_tens) as char);
    let _ = out.push((b'0' + frac_ones) as char);
    out
}

fn vreq_value_rect() -> Rect {
    let right = CARD_RIGHT - VALUE_PILL_RIGHT_PAD;
    Rect::new(
        right - VALUE_PILL_W,
        CONTROL_ROW_VREQ_Y,
        right,
        CONTROL_ROW_VREQ_Y + VALUE_PILL_H,
    )
}

fn ireq_value_rect(mode: PdMode) -> Rect {
    let y = match mode {
        PdMode::Fixed => CONTROL_ROW_IREQ_FIXED_Y,
        PdMode::Pps => CONTROL_ROW_IREQ_PPS_Y,
    };
    let right = CARD_RIGHT - VALUE_PILL_RIGHT_PAD;
    Rect::new(right - VALUE_PILL_W, y, right, y + VALUE_PILL_H)
}

pub fn pick_value_digit(field: PdSettingsFocus, x: i32, mode: PdMode) -> AdjustDigit {
    let cell_w = SETPOINT_CELL_W as i32;
    let num_w = cell_w * 5; // "DD.dd"

    let mut unit_buf = [0u8; 4];
    let unit = match field {
        PdSettingsFocus::Vreq => 'V',
        PdSettingsFocus::Ireq => 'A',
        PdSettingsFocus::None => ' ',
    };
    let unit_s = unit.encode_utf8(&mut unit_buf);
    let unit_w = text_bbox_scaled(unit_s, &FONT_REGULAR)
        .map(|(w, _)| w)
        .unwrap_or_else(|| text_width(unit_s));

    let total_w = num_w + VALUE_UNIT_GAP + unit_w;
    let pill = match field {
        PdSettingsFocus::Vreq => vreq_value_rect(),
        PdSettingsFocus::Ireq => ireq_value_rect(mode),
        PdSettingsFocus::None => ireq_value_rect(mode),
    };
    let value_right = pill.right - 2 - VALUE_PILL_TEXT_RIGHT_PAD;
    let num_left = (value_right - total_w).max(pill.left + 2 + VALUE_PILL_TEXT_MIN_LEFT_PAD);
    let rel = x - num_left;

    let (cell_idx, cell_off) = if rel < 0 {
        (0, 0)
    } else if rel >= num_w {
        (4, cell_w.saturating_sub(1))
    } else {
        (rel / cell_w, rel % cell_w)
    };

    match cell_idx {
        0 | 1 => AdjustDigit::Ones, // tens is non-selectable; snap to ones
        2 => {
            // Decimal point: snap to nearest adjacent selectable digit.
            if cell_off < cell_w / 2 {
                AdjustDigit::Ones
            } else {
                AdjustDigit::Tenths
            }
        }
        3 => AdjustDigit::Tenths,
        _ => AdjustDigit::Hundredths,
    }
}

trait CanvasAaExt {
    fn get_pixel_raw(&self, x: i32, y: i32) -> Option<u16>;
    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgb565, alpha: u8);
    fn fill_round_rect_aa(&mut self, rect: Rect, radius: i32, color: Rgb565);
}

impl CanvasAaExt for Canvas<'_> {
    fn get_pixel_raw(&self, x: i32, y: i32) -> Option<u16> {
        if x < 0 || x >= super::LOGICAL_WIDTH || y < 0 || y >= super::LOGICAL_HEIGHT {
            return None;
        }
        let actual_x = y as usize;
        let actual_y = (self.phys_height as i32 - 1 - x) as usize;
        let idx = (actual_y * self.phys_width + actual_x) * 2;
        if idx + 1 >= self.bytes.len() {
            return None;
        }
        Some(u16::from_be_bytes([self.bytes[idx], self.bytes[idx + 1]]))
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgb565, alpha: u8) {
        if alpha == 0 {
            return;
        }
        if alpha == 255 {
            self.set_pixel(x, y, color);
            return;
        }
        let Some(dst_raw) = self.get_pixel_raw(x, y) else {
            return;
        };

        let src_raw = RawU16::from(color).into_inner();
        let (sr, sg, sb) = rgb565_to_rgb888(src_raw);
        let (dr, dg, db) = rgb565_to_rgb888(dst_raw);

        let a = alpha as u16;
        let ia = 255u16 - a;
        let r = ((sr as u16 * a + dr as u16 * ia + 127) / 255) as u8;
        let g = ((sg as u16 * a + dg as u16 * ia + 127) / 255) as u8;
        let b = ((sb as u16 * a + db as u16 * ia + 127) / 255) as u8;
        self.set_pixel(x, y, Rgb888::new(r, g, b).into());
    }

    fn fill_round_rect_aa(&mut self, rect: Rect, radius: i32, color: Rgb565) {
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

        // Fill interior rectangles (everything but the corners).
        self.fill_rect(
            Rect::new(rect.left + r, rect.top, rect.right - r, rect.bottom),
            color,
        );
        self.fill_rect(
            Rect::new(rect.left, rect.top + r, rect.left + r, rect.bottom - r),
            color,
        );
        self.fill_rect(
            Rect::new(rect.right - r, rect.top + r, rect.right, rect.bottom - r),
            color,
        );

        // Anti-aliased corners by supersampling.
        // Rounded-corner coverage by supersampling a circular arc.
        const SUB: i32 = 4;
        const SCALE: i32 = SUB * 2; // sample points at 1/(2*SUB) pixel.
        let r_u = (r * SCALE) as i64;
        let r_u2 = r_u * r_u;

        let corners = [
            (rect.left + r, rect.top + r, rect.left, rect.top), // TL
            (rect.right - r, rect.top + r, rect.right - r, rect.top), // TR
            (rect.left + r, rect.bottom - r, rect.left, rect.bottom - r), // BL
            (
                rect.right - r,
                rect.bottom - r,
                rect.right - r,
                rect.bottom - r,
            ), // BR
        ];

        for (cx, cy, sx0, sy0) in corners {
            let cx_u = (cx * SCALE) as i64;
            let cy_u = (cy * SCALE) as i64;
            for py in 0..r {
                for px in 0..r {
                    let x0 = sx0 + px;
                    let y0 = sy0 + py;
                    let mut inside = 0u8;
                    for sy in 0..SUB {
                        let sy_u = (2 * sy + 1) as i64;
                        let yy = (y0 * SCALE) as i64 + sy_u;
                        let dy = yy - cy_u;
                        let dy2 = dy * dy;
                        for sx in 0..SUB {
                            let sx_u = (2 * sx + 1) as i64;
                            let xx = (x0 * SCALE) as i64 + sx_u;
                            let dx = xx - cx_u;
                            let dx2 = dx * dx;
                            if dx2 + dy2 <= r_u2 {
                                inside = inside.saturating_add(1);
                            }
                        }
                    }
                    if inside == 0 {
                        continue;
                    }
                    let alpha =
                        ((inside as u16 * 255 + (SUB * SUB / 2) as u16) / (SUB * SUB) as u16) as u8;
                    self.blend_pixel(x0, y0, color, alpha);
                }
            }
        }
    }
}

fn rgb565_to_rgb888(raw: u16) -> (u8, u8, u8) {
    let r5 = (raw >> 11) & 0x1f;
    let g6 = (raw >> 5) & 0x3f;
    let b5 = raw & 0x1f;

    let r = ((r5 as u32 * 255 + 15) / 31) as u8;
    let g = ((g6 as u32 * 255 + 31) / 63) as u8;
    let b = ((b5 as u32 * 255 + 15) / 31) as u8;
    (r, g, b)
}
