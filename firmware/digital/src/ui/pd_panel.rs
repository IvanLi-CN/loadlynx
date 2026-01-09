#![allow(dead_code)]

use embedded_graphics::pixelcolor::Rgb565;
use heapless::{String, Vec};
use lcd_async::raw_framebuf::RawFrameBuf;

use crate::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

use super::{Canvas, Rect, rgb, small_text_width};

const PANEL_LEFT: i32 = 20;
const PANEL_TOP: i32 = 40;
const PANEL_RIGHT: i32 = 300;
const PANEL_BOTTOM: i32 = 220;

const BORDER: i32 = 1;
const PAD_X: i32 = 10;
const PAD_Y: i32 = 10;

const HEADER_H: i32 = 28;
const ACTION_DIVIDER_Y: i32 = 185;
const ACTION_TOP: i32 = 186;

const TAB_W: i32 = 54;
const TAB_H: i32 = 20;
const TAB_GAP: i32 = 6;
const TAB_RADIUS: i32 = 6;

const ROW_H: i32 = 20;
const ROW_GAP: i32 = 2;
const ROW_COUNT: usize = 5;

const BTN_W: i32 = 42;
const BTN_H: i32 = 24;
const BTN_RADIUS: i32 = 8;

const COLOR_BG_HEADER: u32 = 0x141d2f;
const COLOR_BG_BODY: u32 = 0x171f33;
const COLOR_DIVIDER: u32 = 0x1c2a3f;
const COLOR_TAB_BG: u32 = 0x1c2638;
const COLOR_PILL_BG: u32 = 0x19243a;
const COLOR_TEXT_LABEL: u32 = 0x9ab0d8;
const COLOR_TEXT_VALUE: u32 = 0xdfe7ff;
const COLOR_TEXT_DARK: u32 = 0x080f19;
const COLOR_THEME: u32 = 0x4cc9f0;
const COLOR_ERROR: u32 = 0xff5252;

const TITLE: &str = "USB PD";
const TAB_FIXED: &str = "FIXED";
const TAB_PPS: &str = "PPS";
const APPLY_TEXT: &str = "APPLY";
const CLOSE_TEXT: &str = "CLOSE";

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PdPanelHit {
    ModeFixed,
    ModePps,
    Row(usize),
    ScrollUp,
    ScrollDown,
    TargetMinus,
    TargetPlus,
    CurrentMinus,
    CurrentPlus,
    Apply,
    Close,
}

#[derive(Clone, Debug)]
pub struct PdPanelRowVm {
    pub pos: u8,
    pub selected: bool,
    pub text: String<32>,
}

#[derive(Clone, Debug)]
pub struct PdPanelVm {
    pub is_pps: bool,
    pub attached: bool,
    pub can_scroll_up: bool,
    pub can_scroll_down: bool,
    pub show_target_adjust: bool,
    pub apply_enabled: bool,
    pub error_text: String<32>,
    pub detail_text: String<32>,
    pub target_text: String<12>,
    pub current_text: String<12>,
    pub rows: Vec<PdPanelRowVm, ROW_COUNT>,
}

pub fn render_pd_panel(frame: &mut RawFrameBuf<Rgb565, &mut [u8]>, vm: &PdPanelVm) {
    let bytes = frame.as_mut_bytes();
    let mut canvas = Canvas::new(bytes, DISPLAY_WIDTH, DISPLAY_HEIGHT);

    draw_panel_background(&mut canvas);
    draw_header(&mut canvas, vm);
    draw_rows(&mut canvas, vm);
    draw_edit_rows(&mut canvas, vm);
    draw_action_row(&mut canvas, vm);
}

pub fn hit_test_pd_panel(x: i32, y: i32, vm: &PdPanelVm) -> Option<PdPanelHit> {
    if x < PANEL_LEFT || x >= PANEL_RIGHT || y < PANEL_TOP || y >= PANEL_BOTTOM {
        return None;
    }

    if hit_in_rect(x, y, tab_rect(false)) {
        return Some(PdPanelHit::ModeFixed);
    }
    if hit_in_rect(x, y, tab_rect(true)) {
        return Some(PdPanelHit::ModePps);
    }

    if hit_in_rect(x, y, scroll_up_rect()) && vm.can_scroll_up {
        return Some(PdPanelHit::ScrollUp);
    }
    if hit_in_rect(x, y, scroll_down_rect()) && vm.can_scroll_down {
        return Some(PdPanelHit::ScrollDown);
    }

    for (i, rect) in row_rects().iter().enumerate() {
        if hit_in_rect(x, y, *rect) {
            return Some(PdPanelHit::Row(i));
        }
    }

    if vm.show_target_adjust {
        if hit_in_rect(x, y, target_minus_rect()) {
            return Some(PdPanelHit::TargetMinus);
        }
        if hit_in_rect(x, y, target_plus_rect()) {
            return Some(PdPanelHit::TargetPlus);
        }
    }
    if hit_in_rect(x, y, current_minus_rect()) {
        return Some(PdPanelHit::CurrentMinus);
    }
    if hit_in_rect(x, y, current_plus_rect()) {
        return Some(PdPanelHit::CurrentPlus);
    }

    if hit_in_rect(x, y, apply_rect()) {
        return Some(PdPanelHit::Apply);
    }
    if hit_in_rect(x, y, close_rect()) {
        return Some(PdPanelHit::Close);
    }

    None
}

fn draw_panel_background(canvas: &mut Canvas) {
    let border = rgb(COLOR_DIVIDER);
    let header_bg = rgb(COLOR_BG_HEADER);
    let body_bg = rgb(COLOR_BG_BODY);

    let outer = Rect::new(PANEL_LEFT, PANEL_TOP, PANEL_RIGHT, PANEL_BOTTOM);
    canvas.fill_rect(outer, border);

    let inner = Rect::new(
        PANEL_LEFT + BORDER,
        PANEL_TOP + BORDER,
        PANEL_RIGHT - BORDER,
        PANEL_BOTTOM - BORDER,
    );
    canvas.fill_rect(inner, body_bg);

    let header = Rect::new(inner.left, inner.top, inner.right, inner.top + HEADER_H);
    canvas.fill_rect(header, header_bg);

    canvas.fill_rect(
        Rect::new(
            inner.left,
            PANEL_TOP + ACTION_DIVIDER_Y,
            inner.right,
            PANEL_TOP + ACTION_DIVIDER_Y + 1,
        ),
        rgb(COLOR_DIVIDER),
    );
}

fn draw_header(canvas: &mut Canvas, vm: &PdPanelVm) {
    draw_small_text(
        canvas,
        TITLE,
        panel_inner_left(),
        PANEL_TOP + 8,
        rgb(COLOR_TEXT_VALUE),
        0,
    );

    draw_tab(canvas, false, !vm.is_pps);
    draw_tab(canvas, true, vm.is_pps);
}

fn draw_rows(canvas: &mut Canvas, vm: &PdPanelVm) {
    for (idx, rect) in row_rects().iter().enumerate() {
        let row = vm.rows.get(idx);
        let fill = if row.map(|r| r.selected).unwrap_or(false) {
            rgb(COLOR_THEME)
        } else {
            rgb(COLOR_PILL_BG)
        };
        canvas.fill_round_rect(*rect, 8, fill);

        if let Some(row) = row {
            let text_color = if row.selected {
                rgb(COLOR_TEXT_DARK)
            } else {
                rgb(COLOR_TEXT_VALUE)
            };
            let label = row.text.as_str();
            let w = small_text_width(label, 0);
            let x = (rect.left + 8).min(rect.right - 8 - w);
            draw_small_text(canvas, label, x, rect.top + 14, text_color, 0);
        }
    }

    draw_scroll_button(canvas, true, vm.can_scroll_up);
    draw_scroll_button(canvas, false, vm.can_scroll_down);
}

fn draw_edit_rows(canvas: &mut Canvas, vm: &PdPanelVm) {
    let detail_color = if vm.error_text.is_empty() {
        rgb(COLOR_TEXT_LABEL)
    } else {
        rgb(COLOR_ERROR)
    };
    let detail = if vm.error_text.is_empty() {
        vm.detail_text.as_str()
    } else {
        vm.error_text.as_str()
    };
    draw_small_text(
        canvas,
        detail,
        panel_inner_left(),
        PANEL_TOP + 152,
        detail_color,
        0,
    );

    // Target row (PPS only).
    if vm.show_target_adjust {
        draw_small_text(
            canvas,
            "V",
            panel_inner_left(),
            PANEL_TOP + 168,
            rgb(COLOR_TEXT_LABEL),
            0,
        );
        draw_value_pill(canvas, target_value_rect(), vm.target_text.as_str());
        draw_small_button(canvas, target_minus_rect(), "-", true);
        draw_small_button(canvas, target_plus_rect(), "+", true);
    } else {
        draw_small_text(
            canvas,
            "V",
            panel_inner_left(),
            PANEL_TOP + 168,
            rgb(COLOR_TEXT_LABEL),
            0,
        );
        draw_value_pill(canvas, target_value_rect(), vm.target_text.as_str());
    }

    // Current row.
    draw_small_text(
        canvas,
        "I",
        panel_inner_left(),
        PANEL_TOP + 184,
        rgb(COLOR_TEXT_LABEL),
        0,
    );
    draw_value_pill(canvas, current_value_rect(), vm.current_text.as_str());
    draw_small_button(canvas, current_minus_rect(), "-", true);
    draw_small_button(canvas, current_plus_rect(), "+", true);
}

fn draw_action_row(canvas: &mut Canvas, vm: &PdPanelVm) {
    draw_small_button(canvas, apply_rect(), APPLY_TEXT, vm.apply_enabled);
    draw_small_button(canvas, close_rect(), CLOSE_TEXT, true);
}

fn draw_tab(canvas: &mut Canvas, is_pps: bool, active: bool) {
    let rect = tab_rect(is_pps);
    let fill = if active {
        rgb(COLOR_THEME)
    } else {
        rgb(COLOR_TAB_BG)
    };
    canvas.fill_round_rect(rect, TAB_RADIUS, fill);
    let text = if is_pps { TAB_PPS } else { TAB_FIXED };
    let color = if active {
        rgb(COLOR_TEXT_DARK)
    } else {
        rgb(COLOR_TEXT_VALUE)
    };
    let w = small_text_width(text, 0);
    let x = rect.left + ((rect.right - rect.left - w) / 2).max(0);
    draw_small_text(canvas, text, x, rect.top + 14, color, 0);
}

fn draw_scroll_button(canvas: &mut Canvas, up: bool, enabled: bool) {
    let rect = if up {
        scroll_up_rect()
    } else {
        scroll_down_rect()
    };
    let fill = if enabled {
        rgb(COLOR_TAB_BG)
    } else {
        rgb(0x101829)
    };
    canvas.fill_round_rect(rect, 6, fill);
    let label = if up { "↑" } else { "↓" };
    let color = if enabled {
        rgb(COLOR_TEXT_VALUE)
    } else {
        rgb(COLOR_TEXT_LABEL)
    };
    let w = small_text_width(label, 0);
    let x = rect.left + ((rect.right - rect.left - w) / 2).max(0);
    draw_small_text(canvas, label, x, rect.top + 13, color, 0);
}

fn draw_value_pill(canvas: &mut Canvas, rect: Rect, text: &str) {
    canvas.fill_round_rect(rect, 8, rgb(COLOR_PILL_BG));
    let w = small_text_width(text, 0);
    let x = rect.right - 8 - w;
    draw_small_text(canvas, text, x, rect.top + 14, rgb(COLOR_TEXT_VALUE), 0);
}

fn draw_small_button(canvas: &mut Canvas, rect: Rect, label: &str, enabled: bool) {
    let fill = if enabled {
        rgb(COLOR_THEME)
    } else {
        rgb(0x101829)
    };
    let fg = if enabled {
        rgb(COLOR_TEXT_DARK)
    } else {
        rgb(COLOR_TEXT_LABEL)
    };
    canvas.fill_round_rect(rect, BTN_RADIUS, fill);
    let w = small_text_width(label, 0);
    let x = rect.left + ((rect.right - rect.left - w) / 2).max(0);
    draw_small_text(canvas, label, x, rect.top + 14, fg, 0);
}

fn tab_rect(is_pps: bool) -> Rect {
    let top = PANEL_TOP + 5;
    let right = PANEL_RIGHT - BORDER - PAD_X;
    let left_pps = right - TAB_W;
    let left_fixed = left_pps - TAB_GAP - TAB_W;
    let left = if is_pps { left_pps } else { left_fixed };
    Rect::new(left, top, left + TAB_W, top + TAB_H)
}

fn panel_inner_left() -> i32 {
    PANEL_LEFT + BORDER + PAD_X
}

fn panel_inner_right() -> i32 {
    PANEL_RIGHT - BORDER - PAD_X
}

fn row_left() -> i32 {
    panel_inner_left()
}

fn row_right() -> i32 {
    panel_inner_right() - 22
}

fn row_rects() -> [Rect; ROW_COUNT] {
    let list_top = PANEL_TOP + HEADER_H + PAD_Y;
    let mut out = [Rect::new(0, 0, 0, 0); ROW_COUNT];
    for i in 0..ROW_COUNT {
        let top = list_top + (i as i32) * (ROW_H + ROW_GAP);
        out[i] = Rect::new(row_left(), top, row_right(), top + ROW_H);
    }
    out
}

fn scroll_up_rect() -> Rect {
    let list_top = PANEL_TOP + HEADER_H + PAD_Y;
    let left = row_right() + 6;
    Rect::new(left, list_top, left + 18, list_top + 18)
}

fn scroll_down_rect() -> Rect {
    let list_top = PANEL_TOP + HEADER_H + PAD_Y;
    let bottom = list_top + (ROW_COUNT as i32) * (ROW_H + ROW_GAP) - ROW_GAP;
    let left = row_right() + 6;
    Rect::new(left, bottom - 18, left + 18, bottom)
}

fn target_value_rect() -> Rect {
    Rect::new(
        panel_inner_left() + 16,
        PANEL_TOP + 158,
        panel_inner_left() + 128,
        PANEL_TOP + 176,
    )
}

fn current_value_rect() -> Rect {
    Rect::new(
        panel_inner_left() + 16,
        PANEL_TOP + 174,
        panel_inner_left() + 128,
        PANEL_TOP + 192,
    )
}

fn target_minus_rect() -> Rect {
    Rect::new(
        panel_inner_left() + 134,
        PANEL_TOP + 156,
        panel_inner_left() + 164,
        PANEL_TOP + 180,
    )
}

fn target_plus_rect() -> Rect {
    Rect::new(
        panel_inner_left() + 168,
        PANEL_TOP + 156,
        panel_inner_left() + 198,
        PANEL_TOP + 180,
    )
}

fn current_minus_rect() -> Rect {
    Rect::new(
        panel_inner_left() + 134,
        PANEL_TOP + 172,
        panel_inner_left() + 164,
        PANEL_TOP + 196,
    )
}

fn current_plus_rect() -> Rect {
    Rect::new(
        panel_inner_left() + 168,
        PANEL_TOP + 172,
        panel_inner_left() + 198,
        PANEL_TOP + 196,
    )
}

fn apply_rect() -> Rect {
    Rect::new(
        panel_inner_right() - BTN_W * 2 - 10,
        PANEL_TOP + ACTION_TOP + 4,
        panel_inner_right() - BTN_W - 10,
        PANEL_TOP + ACTION_TOP + 4 + BTN_H,
    )
}

fn close_rect() -> Rect {
    Rect::new(
        panel_inner_right() - BTN_W,
        PANEL_TOP + ACTION_TOP + 4,
        panel_inner_right(),
        PANEL_TOP + ACTION_TOP + 4 + BTN_H,
    )
}

fn hit_in_rect(x: i32, y: i32, r: Rect) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

fn draw_small_text(
    canvas: &mut Canvas,
    s: &str,
    x: i32,
    baseline: i32,
    color: Rgb565,
    spacing: i32,
) {
    super::draw_small_text(canvas, s, x, baseline, color, spacing);
}
