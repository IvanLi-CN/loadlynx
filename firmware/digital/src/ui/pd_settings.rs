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

use crate::control::{PdMode, PdSettingsFocus};
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
const CONTROL_ROW_VREQ_Y: i32 = SELECTED_CARD_TOP + 69; // 147
const CONTROL_ROW_IREQ_FIXED_Y: i32 = SELECTED_CARD_TOP + 69; // 147
const CONTROL_ROW_IREQ_PPS_Y: i32 = SELECTED_CARD_TOP + 105; // 183

const BTN_TOP: i32 = 214;
const BTN_BOTTOM: i32 = 235;
const BACK_LEFT: i32 = CARD_LEFT;
const BACK_RIGHT: i32 = 253;
const APPLY_LEFT: i32 = 257;
const APPLY_RIGHT: i32 = 313;

// Controls sizing/placement matches `docs/assets/usb-pd-settings-panel/*.png`.
// +/- buttons are slightly taller than the value field; the focused field draws a 2px accent outline.
const VALUE_BTN_W: i32 = 24;
const VALUE_BTN_H: i32 = 22;
const VALUE_BTN_RADIUS: i32 = 6;
// The value field in the design mock is a small-radius rounded rectangle (R≈2–3px), not a pill.
const VALUE_FIELD_RADIUS: i32 = 3;
const VALUE_FIELD_GAP_LEFT: i32 = 3;
const VALUE_FIELD_GAP_RIGHT: i32 = 3;

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
const COLOR_ACCENT_TEXT: u32 = 0x47badf;
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
    VreqMinus,
    VreqPlus,
    IreqMinus,
    IreqPlus,
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

    // +/- controls live in the selected card.
    let controls = controls_layout(vm.mode);
    if let Some((v_minus, v_plus, i_minus, i_plus)) = controls {
        if hit_in_rect(x, y, v_minus) {
            return Some(PdSettingsHit::VreqMinus);
        }
        if hit_in_rect(x, y, v_plus) {
            return Some(PdSettingsHit::VreqPlus);
        }
        if hit_in_rect(x, y, i_minus) {
            return Some(PdSettingsHit::IreqMinus);
        }
        if hit_in_rect(x, y, i_plus) {
            return Some(PdSettingsHit::IreqPlus);
        }
    } else {
        // Fixed-only: only Ireq buttons.
        let (_v_minus, _v_plus, i_minus, i_plus) = fixed_controls_layout();
        if hit_in_rect(x, y, i_minus) {
            return Some(PdSettingsHit::IreqMinus);
        }
        if hit_in_rect(x, y, i_plus) {
            return Some(PdSettingsHit::IreqPlus);
        }
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
                let (i_minus, i_plus) = fixed_i_buttons();
                draw_value_row(
                    canvas,
                    "Ireq",
                    Some(IREQ_STEP_TEXT),
                    format_ma(vm.i_req_ma).as_str(),
                    i_minus,
                    i_plus,
                    vm.focus == PdSettingsFocus::Ireq,
                    vm.apply_enabled,
                );
            }
            PdMode::Pps => {
                let (v_minus, v_plus, i_minus, i_plus) = pps_value_buttons();
                draw_value_row(
                    canvas,
                    "Vreq",
                    Some(VREQ_STEP_TEXT),
                    format_v_2dp(vm.pps_target_mv).as_str(),
                    v_minus,
                    v_plus,
                    vm.focus == PdSettingsFocus::Vreq,
                    vm.apply_enabled,
                );
                draw_value_row(
                    canvas,
                    "Ireq",
                    Some(IREQ_STEP_TEXT),
                    format_ma(vm.i_req_ma).as_str(),
                    i_minus,
                    i_plus,
                    vm.focus == PdSettingsFocus::Ireq,
                    vm.apply_enabled,
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

fn draw_value_row(
    canvas: &mut Canvas,
    label: &str,
    step: Option<&str>,
    value: &str,
    minus: Rect,
    plus: Rect,
    focused: bool,
    enabled: bool,
) {
    let label_x = CARD_LEFT;
    // Match the frozen mock: keep the label visually closer to the value row.
    let label_y = minus.top - text_height("A", &FONT_REGULAR);
    let label_color = rgb(if focused {
        COLOR_ACCENT_TEXT
    } else {
        COLOR_TEXT_LABEL
    });
    draw_label_with_step(
        canvas,
        label_x,
        label_y,
        label,
        step,
        label_color,
        CARD_RIGHT - 10,
    );

    draw_value_button(canvas, minus, "-", enabled);
    draw_value_button(canvas, plus, "+", enabled);

    // Centered value between +/-.
    let mid_left = minus.right + VALUE_FIELD_GAP_LEFT;
    let mid_right = plus.left - VALUE_FIELD_GAP_RIGHT;
    // The value field is slightly shorter than the +/- buttons in the frozen mocks.
    let field = Rect::new(mid_left, minus.top + 1, mid_right, minus.bottom - 1);
    draw_value_field(canvas, field, enabled, focused);
    let text_rect = if enabled && focused {
        // Keep text inside the 2px focus ring.
        Rect::new(
            field.left + 2,
            field.top + 2,
            field.right - 2,
            field.bottom - 2,
        )
    } else {
        field
    };
    draw_centered_small_bold(canvas, value, text_rect, rgb(COLOR_TEXT_VALUE));
}

fn draw_value_button(canvas: &mut Canvas, rect: Rect, label: &str, enabled: bool) {
    let fill = rgb(if enabled {
        COLOR_INSET_BG
    } else {
        COLOR_ROW_BG
    });
    draw_round_rect_2px_border(
        canvas,
        rect,
        VALUE_BTN_RADIUS,
        rgb(COLOR_DIVIDER_MID),
        rgb(COLOR_BORDER_INNER),
        fill,
    );
    let color = if enabled {
        rgb(COLOR_TEXT_VALUE)
    } else {
        rgb(COLOR_TEXT_DIM)
    };
    draw_centered_small_bold(canvas, label, rect, color);
}

fn draw_value_field(canvas: &mut Canvas, rect: Rect, enabled: bool, focused: bool) {
    let fill = rgb(if enabled { COLOR_TOP_BG } else { COLOR_ROW_BG });
    if enabled && focused {
        // Focused value fields must be a small-radius rounded rectangle (R≈2–3px) with crisp
        // corners (no anti-alias), matching the on-device mock look.
        draw_round_rect_2px_border_hard(
            canvas,
            rect,
            VALUE_FIELD_RADIUS,
            rgb(COLOR_ACCENT),
            rgb(COLOR_ACCENT_INNER),
            fill,
        );
    } else {
        canvas.fill_round_rect(rect, VALUE_FIELD_RADIUS, fill);
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
    let mut out = String::<10>::new();
    let _ = write!(&mut out, "{}mA", ma);
    out
}

fn controls_layout(mode: PdMode) -> Option<(Rect, Rect, Rect, Rect)> {
    if mode == PdMode::Pps {
        Some(pps_value_buttons())
    } else {
        None
    }
}

fn pps_value_buttons() -> (Rect, Rect, Rect, Rect) {
    let v_y = CONTROL_ROW_VREQ_Y;
    let i_y = CONTROL_ROW_IREQ_PPS_Y;

    let left = CARD_LEFT - 1;
    let right = APPLY_RIGHT;
    let v_minus = Rect::new(left, v_y, left + VALUE_BTN_W, v_y + VALUE_BTN_H);
    let v_plus = Rect::new(right - VALUE_BTN_W, v_y, right, v_y + VALUE_BTN_H);

    let i_minus = Rect::new(left, i_y, left + VALUE_BTN_W, i_y + VALUE_BTN_H);
    let i_plus = Rect::new(right - VALUE_BTN_W, i_y, right, i_y + VALUE_BTN_H);

    (v_minus, v_plus, i_minus, i_plus)
}

fn fixed_controls_layout() -> (Rect, Rect, Rect, Rect) {
    // Dummy V buttons (unused) + I buttons.
    let v = Rect::new(0, 0, 0, 0);
    let (i_minus, i_plus) = fixed_i_buttons();
    (v, v, i_minus, i_plus)
}

fn fixed_i_buttons() -> (Rect, Rect) {
    let y = CONTROL_ROW_IREQ_FIXED_Y;
    let left = CARD_LEFT - 1;
    let right = APPLY_RIGHT;
    let minus = Rect::new(left, y, left + VALUE_BTN_W, y + VALUE_BTN_H);
    let plus = Rect::new(right - VALUE_BTN_W, y, right, y + VALUE_BTN_H);
    (minus, plus)
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
