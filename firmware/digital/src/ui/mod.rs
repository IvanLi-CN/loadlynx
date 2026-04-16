#[cfg(feature = "audio_menu")]
pub mod audio_menu;
mod fonts;
pub mod pd_settings;
pub mod preset_panel;

use core::fmt::Write;
use embedded_graphics::pixelcolor::{
    Rgb565,
    raw::{RawData, RawU16},
};
use heapless::{String, Vec};
use lcd_async::raw_framebuf::RawFrameBuf;
use loadlynx_protocol::{
    CalKind, FAULT_MCU_OVER_TEMP, FAULT_OVERCURRENT, FAULT_OVERVOLTAGE, FAULT_SINK_OVER_TEMP,
    LoadMode,
};

use crate::control::AdjustDigit;
use crate::touch::TouchMarker;
use crate::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

use self::fonts::{SETPOINT_FONT, SEVEN_SEG_FONT, SMALL_FONT};

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum AnalogState {
    Offline = 0,
    CalMissing = 1,
    Faulted = 2,
    Ready = 3,
}

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum CalibrationUiMode {
    Off = 0,
    Voltage = 1,
    CurrentCh1 = 2,
    CurrentCh2 = 3,
}

impl CalibrationUiMode {
    pub fn from_cal_kind(kind: CalKind) -> Self {
        match kind {
            CalKind::Off => CalibrationUiMode::Off,
            CalKind::Voltage => CalibrationUiMode::Voltage,
            CalKind::CurrentCh1 => CalibrationUiMode::CurrentCh1,
            CalKind::CurrentCh2 => CalibrationUiMode::CurrentCh2,
        }
    }

    pub fn status_text(self) -> &'static str {
        match self {
            CalibrationUiMode::Off => "",
            CalibrationUiMode::Voltage => "CAL V",
            CalibrationUiMode::CurrentCh1 => "CAL C1",
            CalibrationUiMode::CurrentCh2 => "CAL C2",
        }
    }

    pub fn pill_top_text(self) -> Option<&'static str> {
        match self {
            CalibrationUiMode::Off => None,
            CalibrationUiMode::Voltage => Some("V"),
            CalibrationUiMode::CurrentCh1 => Some("C1"),
            CalibrationUiMode::CurrentCh2 => Some("C2"),
        }
    }
}

impl AnalogState {
    pub fn from_u8(raw: u8) -> Self {
        match raw {
            x if x == AnalogState::Offline as u8 => AnalogState::Offline,
            x if x == AnalogState::CalMissing as u8 => AnalogState::CalMissing,
            x if x == AnalogState::Faulted as u8 => AnalogState::Faulted,
            x if x == AnalogState::Ready as u8 => AnalogState::Ready,
            _ => AnalogState::Offline,
        }
    }
}

const LOGICAL_WIDTH: i32 = 320;
const LOGICAL_HEIGHT: i32 = 240;
const DEBUG_OVERLAY: bool = false;

pub fn touch_marker_overlay_enabled() -> bool {
    DEBUG_OVERLAY
}

// 左侧三张主卡片的布局参数，便于在全帧/局部渲染中保持一致。
const CARD_TOPS: [i32; 3] = [0, 80, 160];
const CARD_COLORS: [u32; 3] = [0x171f33, 0x141d2f, 0x111828];
const CARD_BG_LEFT: i32 = 8;
const CARD_BG_RIGHT: i32 = 182;
const MAIN_LABEL_X: i32 = 16;
const MAIN_DIGITS_RIGHT: i32 = 170;
// 背景在 Y 方向的偏移：与原设计保持上边距 6px，同时向下扩展到 +80，
// 以完全覆盖 32x50 的七段字体（area.top = top+28，高度 50 → bottom=top+78）。
const CARD_BG_TOP_OFFSET: i32 = 6;
const CARD_BG_BOTTOM_OFFSET: i32 = 80;
// `draw_seven_seg_value()` right-aligns `99.99` / `999.9` inside `MAIN_DIGITS_RIGHT`.
// The widest main metric can start around x=18, so partial clears must extend left of 24
// or stale leading segments remain visible when the value shrinks.
const MAIN_VALUE_LEFT: i32 = 16;
const MAIN_VALUE_RIGHT: i32 = 182;
const MAIN_VALUE_TOP_OFFSET: i32 = 26;
const MAIN_VALUE_BOTTOM_OFFSET: i32 = 76;
const CURRENT_BAR_LEFT: i32 = 66;
const CURRENT_BAR_RIGHT: i32 = 180;
const CURRENT_BAR_TOP: i32 = CARD_TOPS[1] + 10;
const CURRENT_BAR_BOTTOM: i32 = CURRENT_BAR_TOP + 12;

// Right-side layout (logical 320×240 coordinate space).
const VOLTAGE_PAIR_TOP: i32 = 50;
const VOLTAGE_PAIR_BOTTOM: i32 = 96;
const LOAD_ROW_TOP: i32 = 118;
const LOAD_ROW_BOTTOM: i32 = LOAD_ROW_TOP + LOAD_BUTTON_SIZE;
const TELEMETRY_TOP: i32 = 172;
const TELEMETRY_LINE_HEIGHT: i32 = 12;
const TELEMETRY_LINE_COUNT: usize = 5;
const PRESET_PREVIEW_PANEL_LEFT: i32 = 154;
const PRESET_PREVIEW_PANEL_RIGHT: i32 = 314;
const PRESET_PREVIEW_PANEL_TOP: i32 = 44;
const PRESET_PREVIEW_PANEL_BOTTOM: i32 = 170;
const VOLTAGE_PAIR_VALUE_LEFT: i32 = 198;
const VOLTAGE_PAIR_VALUE_RIGHT: i32 = 314;
const VOLTAGE_PAIR_VALUE_TOP: i32 = VOLTAGE_PAIR_TOP + 12;
const VOLTAGE_PAIR_VALUE_BOTTOM: i32 = VOLTAGE_PAIR_BOTTOM;

// Control row layout: <M#><MODE> entry + fixed-width target summary.
pub(crate) const CONTROL_ROW_TOP: i32 = 10;
pub(crate) const CONTROL_ROW_BOTTOM: i32 = 38;
pub(crate) const CONTROL_MODE_PILL_LEFT: i32 = 198;
pub(crate) const CONTROL_MODE_PILL_RIGHT: i32 = 228;
pub(crate) const CONTROL_VALUE_PILL_LEFT: i32 = 232;
pub(crate) const CONTROL_VALUE_PILL_RIGHT: i32 = 314;
const CONTROL_PILL_RADIUS: i32 = 6;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ControlRowHit {
    PresetEntry,
    TargetEntry,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct SetpointDigitPick {
    pub digit: AdjustDigit,
    pub attempted_left: bool,
    pub attempted_right: bool,
}

pub fn hit_test_control_row(x: i32, y: i32) -> Option<ControlRowHit> {
    // Slightly expand the touch hit box to tolerate touch calibration offsets.
    const HIT_PAD_X: i32 = 2;
    const HIT_PAD_Y: i32 = 8;

    if y < CONTROL_ROW_TOP - HIT_PAD_Y || y > CONTROL_ROW_BOTTOM + HIT_PAD_Y {
        return None;
    }

    if x < CONTROL_MODE_PILL_LEFT - HIT_PAD_X || x > CONTROL_VALUE_PILL_RIGHT + HIT_PAD_X {
        return None;
    }

    // The gap between the two pills should not become a dead zone: attribute it
    // to the nearest half.
    let split_x = (CONTROL_MODE_PILL_RIGHT + CONTROL_VALUE_PILL_LEFT) / 2;
    if x <= split_x {
        Some(ControlRowHit::PresetEntry)
    } else {
        Some(ControlRowHit::TargetEntry)
    }
}

pub fn pick_control_row_setpoint_digit(x: i32, unit: char) -> SetpointDigitPick {
    // Mirror the `draw_control_row()` layout so hit-testing matches what is rendered:
    // numeric is right-aligned inside the pill, followed by the unit in SmallFont.
    let glyph_w = SETPOINT_FONT.width() as i32;
    let num_w = glyph_w * 6;

    let mut unit_buf = [0u8; 4];
    let unit_s = unit.encode_utf8(&mut unit_buf);
    let unit_w = small_text_width(unit_s, 0);

    let unit_gap = 1;
    let total_w = num_w + unit_gap + unit_w;

    let right_pad = 3;
    let value_right = CONTROL_VALUE_PILL_RIGHT - right_pad;
    let num_left = (value_right - total_w).max(CONTROL_VALUE_PILL_LEFT);
    let num_right = num_left + num_w;

    let attempted_left = x < num_left + glyph_w;
    let attempted_right = x >= num_right;

    let rel = x - num_left;
    let (cell_idx, cell_off) = if rel < 0 {
        (0, 0)
    } else if rel >= num_w {
        (5, glyph_w.saturating_sub(1))
    } else {
        (rel / glyph_w, rel % glyph_w)
    };

    let digit = if unit == 'W' {
        match cell_idx {
            0 | 1 | 2 => AdjustDigit::Ones, // hundreds/tens are non-selectable; snap to ones
            3 => {
                // Decimal point: snap to nearest adjacent selectable digit.
                if cell_off < glyph_w / 2 {
                    AdjustDigit::Ones
                } else {
                    AdjustDigit::Tenths
                }
            }
            4 | 5 => AdjustDigit::Tenths, // 0.1 W resolution
            _ => AdjustDigit::Tenths,
        }
    } else {
        match cell_idx {
            0 | 1 => AdjustDigit::Ones, // tens is non-selectable; snap to ones
            2 => {
                // Decimal point: snap to nearest adjacent selectable digit.
                if cell_off < glyph_w / 2 {
                    AdjustDigit::Ones
                } else {
                    AdjustDigit::Tenths
                }
            }
            3 => AdjustDigit::Tenths,
            4 => AdjustDigit::Hundredths,
            _ => AdjustDigit::Thousandths,
        }
    };

    SetpointDigitPick {
        digit,
        attempted_left,
        attempted_right,
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum WifiUiStatus {
    Disabled,
    Connecting,
    Ok,
    Error,
}

// Dashboard settings entry (reuses the old round button footprint).
// Scaled up from the original 21px design to ~1.3× while keeping an odd diameter.
const LOAD_BUTTON_SIZE: i32 = 27;
const LOAD_BUTTON_RIGHT: i32 = CONTROL_VALUE_PILL_RIGHT;
const LOAD_BUTTON_LEFT: i32 = LOAD_BUTTON_RIGHT - LOAD_BUTTON_SIZE;
const LOAD_BUTTON_BOTTOM: i32 = LOAD_ROW_TOP + LOAD_BUTTON_SIZE;
// Dashboard extended-voltage toggle (keeps the original PD button footprint).
const PD_BUTTON_LEFT: i32 = 198;
const PD_BUTTON_GAP_TO_POWER: i32 = 10;
const PD_BUTTON_RIGHT: i32 = LOAD_BUTTON_LEFT - PD_BUTTON_GAP_TO_POWER;
const PD_BUTTON_BOTTOM: i32 = LOAD_BUTTON_BOTTOM;
const PD_BUTTON_RADIUS: i32 = 6;

#[derive(Copy, Clone, Eq, PartialEq, Debug, defmt::Format)]
pub enum PdButtonState {
    Safe5vOnly,
    ExtendedAllowed,
    ExtendedFailed,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PdButtonDisplayMode {
    /// Unattached / no PD source detected.
    Detach,
    /// Fixed PDO mode (label uses `PD`).
    Fixed,
    /// PPS APDO mode (label uses `PPS`).
    Pps,
}

/// Bitmask describing which logical UI regions need to be updated for a frame.
#[derive(Copy, Clone, Default)]
pub struct UiChangeMask {
    pub main_voltage: bool,
    pub main_current: bool,
    pub main_power: bool,
    pub voltage_pair: bool,
    pub load_row: bool,
    pub channel_currents: bool,
    pub control_row: bool,
    pub telemetry_line_mask: u8,
    pub wifi_status: bool,
    pub touch_marker: bool,
}

impl UiChangeMask {
    pub fn has_main_metrics(&self) -> bool {
        self.main_voltage || self.main_current || self.main_power
    }

    pub fn has_telemetry_lines(&self) -> bool {
        self.telemetry_line_mask != 0
    }

    pub fn mark_telemetry_line_dirty(&mut self, line_idx: usize) {
        self.telemetry_line_mask |= telemetry_line_bit(line_idx);
    }

    pub fn is_empty(&self) -> bool {
        !(self.has_main_metrics()
            || self.voltage_pair
            || self.load_row
            || self.channel_currents
            || self.control_row
            || self.has_telemetry_lines()
            || self.wifi_status
            || self.touch_marker)
    }
}

pub const DIRTY_RECT_CAPACITY: usize = 12;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct DirtyRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub needs_base_copy: bool,
}

impl DirtyRect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
            needs_base_copy: false,
        }
    }

    pub const fn with_base_copy(mut self, needs_base_copy: bool) -> Self {
        self.needs_base_copy = needs_base_copy;
        self
    }

    pub fn same_region(&self, other: &Self) -> bool {
        self.x == other.x
            && self.y == other.y
            && self.width == other.width
            && self.height == other.height
    }
}

pub type DirtyRects = Vec<DirtyRect, DIRTY_RECT_CAPACITY>;

pub const fn telemetry_line_bit(line_idx: usize) -> u8 {
    1u8 << line_idx
}

pub const fn telemetry_line_all_mask() -> u8 {
    (1u8 << TELEMETRY_LINE_COUNT) - 1
}

pub const fn telemetry_alert_line_bit() -> u8 {
    telemetry_line_bit(TELEMETRY_LINE_COUNT - 1)
}

fn logical_to_physical_dirty_rect(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> Option<DirtyRect> {
    let left = left.clamp(0, LOGICAL_WIDTH);
    let top = top.clamp(0, LOGICAL_HEIGHT);
    let right = right.clamp(left, LOGICAL_WIDTH);
    let bottom = bottom.clamp(top, LOGICAL_HEIGHT);

    let phys_left = top;
    let phys_top = DISPLAY_HEIGHT as i32 - right;
    let phys_right = bottom;
    let phys_bottom = DISPLAY_HEIGHT as i32 - left;

    let width = phys_right - phys_left;
    let height = phys_bottom - phys_top;
    if width <= 0 || height <= 0 {
        return None;
    }

    Some(DirtyRect::new(
        phys_left as u16,
        phys_top as u16,
        width as u16,
        height as u16,
    ))
}

fn push_logical_dirty_rect_with_copy(
    rects: &mut DirtyRects,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    needs_base_copy: bool,
) {
    if let Some(rect) = logical_to_physical_dirty_rect(left, top, right, bottom) {
        let _ = rects.push(rect.with_base_copy(needs_base_copy));
    }
}

pub(super) fn push_logical_dirty_rect(
    rects: &mut DirtyRects,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) {
    push_logical_dirty_rect_with_copy(rects, left, top, right, bottom, false);
}

pub fn full_screen_dirty_rect() -> DirtyRect {
    DirtyRect::new(0, 0, DISPLAY_WIDTH as u16, DISPLAY_HEIGHT as u16)
}

pub fn push_full_screen_dirty_rect(rects: &mut DirtyRects) {
    let _ = rects.push(full_screen_dirty_rect());
}

pub fn push_fps_overlay_dirty_rect(rects: &mut DirtyRects) {
    push_logical_dirty_rect(rects, 0, 0, 80, 16);
}

fn push_main_metric_value_dirty_rect(rects: &mut DirtyRects, top: i32) {
    push_logical_dirty_rect(
        rects,
        MAIN_VALUE_LEFT,
        top + MAIN_VALUE_TOP_OFFSET,
        MAIN_VALUE_RIGHT,
        top + MAIN_VALUE_BOTTOM_OFFSET,
    );
}

fn push_telemetry_line_dirty_rect(rects: &mut DirtyRects, line_idx: usize) {
    let top = TELEMETRY_TOP + (line_idx as i32) * TELEMETRY_LINE_HEIGHT;
    push_logical_dirty_rect(rects, 198, top, LOGICAL_WIDTH, top + TELEMETRY_LINE_HEIGHT);
}

pub fn collect_partial_dirty_rects(
    curr: &UiSnapshot,
    mask: &UiChangeMask,
    include_fps_overlay: bool,
    dirty_rects: &mut DirtyRects,
) {
    if mask.main_voltage {
        push_main_metric_value_dirty_rect(dirty_rects, CARD_TOPS[0]);
    }

    if mask.main_current {
        push_main_metric_value_dirty_rect(dirty_rects, CARD_TOPS[1]);
    }

    if mask.main_power {
        push_main_metric_value_dirty_rect(dirty_rects, CARD_TOPS[2]);
    }

    if mask.voltage_pair {
        push_logical_dirty_rect(
            dirty_rects,
            VOLTAGE_PAIR_VALUE_LEFT,
            VOLTAGE_PAIR_VALUE_TOP,
            VOLTAGE_PAIR_VALUE_RIGHT,
            VOLTAGE_PAIR_VALUE_BOTTOM,
        );
    }

    if mask.load_row {
        push_logical_dirty_rect(
            dirty_rects,
            190,
            LOAD_ROW_TOP,
            LOGICAL_WIDTH,
            LOAD_ROW_BOTTOM,
        );
    }

    if mask.channel_currents {
        push_logical_dirty_rect(
            dirty_rects,
            CURRENT_BAR_LEFT,
            CURRENT_BAR_TOP,
            CURRENT_BAR_RIGHT,
            CURRENT_BAR_BOTTOM,
        );
    }

    if mask.control_row {
        push_logical_dirty_rect(dirty_rects, 190, 0, LOGICAL_WIDTH, VOLTAGE_PAIR_TOP);
    }

    if mask.has_telemetry_lines() {
        for line_idx in 0..TELEMETRY_LINE_COUNT {
            if (mask.telemetry_line_mask & telemetry_line_bit(line_idx)) != 0 {
                push_telemetry_line_dirty_rect(dirty_rects, line_idx);
            }
        }
    }

    let redraw_preview = curr.preset_preview_active
        && (mask.control_row || mask.voltage_pair || mask.load_row || mask.has_telemetry_lines());
    if redraw_preview {
        push_logical_dirty_rect(
            dirty_rects,
            PRESET_PREVIEW_PANEL_LEFT,
            PRESET_PREVIEW_PANEL_TOP,
            PRESET_PREVIEW_PANEL_RIGHT,
            PRESET_PREVIEW_PANEL_BOTTOM,
        );
    }

    if mask.wifi_status && !mask.control_row {
        push_logical_dirty_rect(dirty_rects, LOGICAL_WIDTH - 32, 0, LOGICAL_WIDTH, 10);
    }

    if include_fps_overlay {
        push_fps_overlay_dirty_rect(dirty_rects);
    }
}

pub fn render(frame: &mut RawFrameBuf<Rgb565, &mut [u8]>, data: &UiSnapshot) {
    let bytes = frame.as_mut_bytes();
    let mut canvas = Canvas::new(bytes, DISPLAY_WIDTH, DISPLAY_HEIGHT);

    // Background blocks
    canvas.fill_rect(Rect::new(0, 0, 190, LOGICAL_HEIGHT), rgb(0x101829));
    canvas.fill_rect(
        Rect::new(190, 0, LOGICAL_WIDTH, LOGICAL_HEIGHT),
        rgb(0x080f19),
    );

    for (idx, &top) in CARD_TOPS.iter().enumerate() {
        canvas.fill_rect(
            Rect::new(
                CARD_BG_LEFT,
                top + CARD_BG_TOP_OFFSET,
                CARD_BG_RIGHT,
                top + CARD_BG_BOTTOM_OFFSET,
            ),
            rgb(CARD_COLORS[idx]),
        );
    }

    // Digits colors per design palette
    draw_main_metric(
        &mut canvas,
        "VOLTAGE",
        data.main_voltage_text.as_str(),
        "V",
        0,
        rgb(0xFFB347),
    );
    draw_main_metric(
        &mut canvas,
        "CURRENT",
        data.main_current_text.as_str(),
        "A",
        80,
        rgb(0xFF5252),
    );
    draw_current_mirror_bar(&mut canvas, data);
    draw_main_metric(
        &mut canvas,
        "POWER",
        data.main_power_text.as_str(),
        "W",
        160,
        rgb(0x6EF58C),
    );

    draw_control_row(&mut canvas, data);
    draw_voltage_pair(
        &mut canvas,
        data,
        data.remote_voltage_text.as_str(),
        data.local_voltage_text.as_str(),
    );
    draw_dashboard_load_row(&mut canvas, data);
    draw_preset_preview_panel(&mut canvas, data);
    draw_telemetry(&mut canvas, data);

    render_wifi_status(&mut canvas, data.wifi_status);

    if DEBUG_OVERLAY {
        draw_debug_overlay(&mut canvas);
    }
}

/// Partially update the framebuffer based on a change mask and the current UI
/// snapshot. 静态布局（背景、标签、单位等）假定已经通过首帧 `render()` 绘制。
pub fn render_partial(
    frame: &mut RawFrameBuf<Rgb565, &mut [u8]>,
    curr: &UiSnapshot,
    mask: &UiChangeMask,
) {
    if mask.is_empty() {
        return;
    }

    let bytes = frame.as_mut_bytes();
    let mut canvas = Canvas::new(bytes, DISPLAY_WIDTH, DISPLAY_HEIGHT);

    if mask.main_voltage {
        clear_main_metric_value(&mut canvas, CARD_TOPS[0], rgb(CARD_COLORS[0]));
        draw_main_metric_value(
            &mut canvas,
            curr.main_voltage_text.as_str(),
            "V",
            CARD_TOPS[0],
            rgb(0xFFB347),
        );
    }

    if mask.main_current {
        clear_main_metric_value(&mut canvas, CARD_TOPS[1], rgb(CARD_COLORS[1]));
        draw_main_metric_value(
            &mut canvas,
            curr.main_current_text.as_str(),
            "A",
            CARD_TOPS[1],
            rgb(0xFF5252),
        );
    }

    if mask.main_power {
        clear_main_metric_value(&mut canvas, CARD_TOPS[2], rgb(CARD_COLORS[2]));
        draw_main_metric_value(
            &mut canvas,
            curr.main_power_text.as_str(),
            "W",
            CARD_TOPS[2],
            rgb(0x6EF58C),
        );
    }

    if mask.voltage_pair {
        canvas.fill_rect(
            Rect::new(
                VOLTAGE_PAIR_VALUE_LEFT,
                VOLTAGE_PAIR_VALUE_TOP,
                VOLTAGE_PAIR_VALUE_RIGHT,
                VOLTAGE_PAIR_VALUE_BOTTOM,
            ),
            rgb(0x080f19),
        );
        let remote_text = curr.remote_voltage_text.as_str();
        let local_text = curr.local_voltage_text.as_str();
        draw_voltage_pair(&mut canvas, curr, remote_text, local_text);
    }

    if mask.load_row {
        canvas.fill_rect(
            Rect::new(190, LOAD_ROW_TOP, LOGICAL_WIDTH, LOAD_ROW_BOTTOM),
            rgb(0x080f19),
        );
        draw_dashboard_load_row(&mut canvas, curr);
    }

    if mask.channel_currents {
        canvas.fill_rect(
            Rect::new(
                CURRENT_BAR_LEFT,
                CURRENT_BAR_TOP,
                CURRENT_BAR_RIGHT,
                CURRENT_BAR_BOTTOM,
            ),
            rgb(CARD_COLORS[1]),
        );
        draw_current_mirror_bar(&mut canvas, curr);
    }

    if mask.control_row {
        // 控制行（CC/CV + 目标值 + 选中位背景高亮）：独立刷新，避免与其它区域清屏互相干扰。
        canvas.fill_rect(
            Rect::new(190, 0, LOGICAL_WIDTH, VOLTAGE_PAIR_TOP),
            rgb(0x080f19),
        );
        draw_control_row(&mut canvas, curr);
    }

    if mask.has_telemetry_lines() {
        // 底部状态文本逐行擦除，避免 run time 这类短字符串更新时整块闪动。
        for line_idx in 0..TELEMETRY_LINE_COUNT {
            if (mask.telemetry_line_mask & telemetry_line_bit(line_idx)) == 0 {
                continue;
            }

            let top = TELEMETRY_TOP + (line_idx as i32) * TELEMETRY_LINE_HEIGHT;
            canvas.fill_rect(
                Rect::new(198, top, LOGICAL_WIDTH, top + TELEMETRY_LINE_HEIGHT),
                rgb(0x080f19),
            );
            draw_telemetry_line(&mut canvas, curr, line_idx);
        }
    }

    let redraw_preview = curr.preset_preview_active
        && (mask.control_row || mask.voltage_pair || mask.load_row || mask.has_telemetry_lines());
    if redraw_preview {
        // Preset preview info panel overlays part of the right-side info column; redraw it last so
        // partial updates do not accidentally wipe it while the gesture is still held.
        draw_preset_preview_panel(&mut canvas, curr);
    }

    let redraw_wifi = mask.wifi_status || mask.control_row;
    if redraw_wifi {
        // Wi‑Fi 状态标记始终在最后绘制一层小覆盖，避免被右侧其它元素重绘时“擦掉”。
        render_wifi_status(&mut canvas, curr.wifi_status);
    }
}

/// 在左上角叠加显示 FPS 信息。
/// 参数 `fps` 来自 display task 的 present 完成统计窗口。
pub fn render_fps_overlay(frame: &mut RawFrameBuf<Rgb565, &mut [u8]>, fps: u32) {
    let bytes = frame.as_mut_bytes();
    let mut canvas = Canvas::new(bytes, DISPLAY_WIDTH, DISPLAY_HEIGHT);

    let mut text = String::<12>::new();
    let _ = text.push_str("FPS ");
    append_u32(&mut text, fps);

    // 清理左上角一小块区域，使用与左侧背景一致的底色。
    canvas.fill_rect(Rect::new(0, 0, 80, 16), rgb(0x101829));
    // 叠加白色小字体文本。
    draw_small_text(&mut canvas, text.as_str(), 4, 4, rgb(0xFFFFFF), 0);
}

pub fn render_touch_marker(
    frame: &mut RawFrameBuf<Rgb565, &mut [u8]>,
    marker: Option<TouchMarker>,
) {
    if !DEBUG_OVERLAY {
        return;
    }

    let Some(marker) = marker else {
        return;
    };

    let bytes = frame.as_mut_bytes();
    let mut canvas = Canvas::new(bytes, DISPLAY_WIDTH, DISPLAY_HEIGHT);

    let color = match marker.event {
        0 => rgb(0x00ff00), // down
        1 => rgb(0xff4040), // up
        2 => rgb(0xffd000), // contact/move
        _ => rgb(0xffffff),
    };

    let x = marker.x;
    let y = marker.y;
    let r = 7i32;
    canvas.draw_line(x - r, y, x + r, y, color);
    canvas.draw_line(x, y - r, x, y + r, color);
    canvas.fill_rect(Rect::new(x - 1, y - 1, x + 2, y + 2), rgb(0xffffff));
}

/// 在右上角叠加显示简要 Wi‑Fi 状态。
fn render_wifi_status(canvas: &mut Canvas, status: WifiUiStatus) {
    // 固定在屏幕最右上角的一小块区域，尽量避免覆盖 REMOTE/LOCAL 文本。
    // LOGICAL_WIDTH=320，因此这里占用 [288,320)×[0,10) 这一条窄带。
    let area = Rect::new(LOGICAL_WIDTH - 32, 0, LOGICAL_WIDTH, 10);
    // 使用与右侧卡片相同的背景色。
    canvas.fill_rect(area, rgb(0x080f19));

    // 使用至多 4 个字符的状态缩写，保证在窄区域内完整可见：
    //   W:OK / W:.. / W:ER / W:--
    let (text, color) = match status {
        WifiUiStatus::Ok => ("W:OK", rgb(0x6EF58C)),
        WifiUiStatus::Connecting => ("W:..", rgb(0xFFB347)),
        WifiUiStatus::Error => ("W:ER", rgb(0xFF5252)),
        WifiUiStatus::Disabled => ("W:--", rgb(0x6d7fa4)),
    };

    let x = area.left + 2;
    let y = area.top + 1;
    draw_small_text(canvas, text, x, y, color, 0);
}

fn draw_main_metric(
    canvas: &mut Canvas,
    label: &str,
    value: &str,
    unit: &str,
    top: i32,
    digit_color: Rgb565,
) {
    draw_small_text(canvas, label, MAIN_LABEL_X, top + 10, rgb(0x9ab0d8), 0);
    let area = Rect::new(24, top + 28, MAIN_DIGITS_RIGHT, top + 72);
    draw_seven_seg_value(canvas, value, &area, digit_color);
    draw_small_text(canvas, unit, area.right, top + 56, rgb(0x9ab0d8), 1);
}

fn clear_main_metric_value(canvas: &mut Canvas, top: i32, bg: Rgb565) {
    canvas.fill_rect(
        Rect::new(
            MAIN_VALUE_LEFT,
            top + MAIN_VALUE_TOP_OFFSET,
            MAIN_VALUE_RIGHT,
            top + MAIN_VALUE_BOTTOM_OFFSET,
        ),
        bg,
    );
}

fn draw_main_metric_value(
    canvas: &mut Canvas,
    value: &str,
    unit: &str,
    top: i32,
    digit_color: Rgb565,
) {
    let area = Rect::new(24, top + 28, MAIN_DIGITS_RIGHT, top + 72);
    draw_seven_seg_value(canvas, value, &area, digit_color);
    draw_small_text(canvas, unit, area.right, top + 56, rgb(0x9ab0d8), 1);
}

fn draw_current_mirror_bar(canvas: &mut Canvas, data: &UiSnapshot) {
    let top = CARD_TOPS[1];
    let label_width = small_text_width("CURRENT", 0);
    let bar_left = (MAIN_LABEL_X + label_width + 4).min(CARD_BG_RIGHT - 4);
    // Keep the bar inside the current card slab; do not spill into the right column.
    let bar_right = CARD_BG_RIGHT - 2;
    let bar_top = top + 12;
    draw_mirror_bar_in_bounds(
        canvas,
        bar_top,
        bar_left,
        bar_right,
        data.ch1_current / 5.0,
        data.ch2_current / 5.0,
    );
}

fn draw_voltage_pair(canvas: &mut Canvas, data: &UiSnapshot, left_value: &str, right_value: &str) {
    draw_pair_header(
        canvas,
        ("REMOTE", left_value),
        ("LOCAL", right_value),
        VOLTAGE_PAIR_TOP,
    );
    let remote_bar = if data.remote_active {
        data.remote_voltage / 40.0
    } else {
        0.0
    };
    draw_mirror_bar_in_bounds(
        canvas,
        VOLTAGE_PAIR_TOP + 34,
        198,
        314,
        remote_bar,
        data.local_voltage / 40.0,
    );
}

fn small_text_width(text: &str, spacing: i32) -> i32 {
    let glyph = SMALL_FONT.width() as i32 + spacing;
    (text.chars().count() as i32) * glyph
}

fn setpoint_text_width(text: &str, spacing: i32) -> i32 {
    let glyph = SETPOINT_FONT.width() as i32 + spacing;
    (text.chars().count() as i32) * glyph
}

fn draw_control_row(canvas: &mut Canvas, data: &UiSnapshot) {
    // Two independent pills: preset/mode (left) and setpoint summary (right).
    canvas.fill_round_rect(
        Rect::new(
            CONTROL_MODE_PILL_LEFT,
            CONTROL_ROW_TOP,
            CONTROL_MODE_PILL_RIGHT,
            CONTROL_ROW_BOTTOM,
        ),
        CONTROL_PILL_RADIUS,
        rgb(0x1c2638),
    );
    canvas.fill_round_rect(
        Rect::new(
            CONTROL_VALUE_PILL_LEFT,
            CONTROL_ROW_TOP,
            CONTROL_VALUE_PILL_RIGHT,
            CONTROL_ROW_BOTTOM,
        ),
        CONTROL_PILL_RADIUS,
        rgb(0x1c2638),
    );

    // <M#><MODE> entry: show active/preview preset id + mode, with mode-specific colors.
    let (preset_text, mode_text, mode_color) =
        if let Some(cal_top) = data.calibration_mode.pill_top_text() {
            let mut preset_text = String::<3>::new();
            let _ = preset_text.push_str(cal_top);
            let mut mode_text = String::<3>::new();
            let _ = mode_text.push_str("CAL");
            (preset_text, mode_text, rgb(0x4CC9F0))
        } else {
            let (mode_text, mode_color) = match data.active_mode {
                LoadMode::Cc | LoadMode::Reserved(_) => ("CC", rgb(0xFF5252)),
                LoadMode::Cv => ("CV", rgb(0xFFB347)),
                LoadMode::Cp => ("CP", rgb(0xB27BFF)),
            };

            let mut preset_text = String::<3>::new();
            let _ = preset_text.push('M');
            if (1..=9).contains(&data.active_preset_id) {
                let _ = preset_text.push((b'0' + data.active_preset_id) as char);
            } else {
                let _ = preset_text.push('?');
            }
            let mut mode_buf = String::<3>::new();
            let _ = mode_buf.push_str(mode_text);
            (preset_text, mode_buf, mode_color)
        };

    // Two-line preset label inside the left half: top = "M#", bottom = "CC"/"CV".
    let small_h = SMALL_FONT.height() as i32;
    let lines_h = small_h * 2;
    let label_y0 = CONTROL_ROW_TOP + ((CONTROL_ROW_BOTTOM - CONTROL_ROW_TOP) - lines_h).max(0) / 2;

    let label_left = CONTROL_MODE_PILL_LEFT;
    let label_right = CONTROL_MODE_PILL_RIGHT;
    let label_w = (label_right - label_left).max(1);

    let preset_w = small_text_width(preset_text.as_ref(), 0);
    let mode_w = small_text_width(mode_text.as_ref(), 0);
    let preset_x = label_left + (label_w - preset_w).max(0) / 2;
    let mode_x = label_left + (label_w - mode_w).max(0) / 2;

    draw_small_text(
        canvas,
        preset_text.as_ref(),
        preset_x,
        label_y0,
        rgb(0xdfe7ff),
        0,
    );
    draw_small_text(
        canvas,
        mode_text.as_ref(),
        mode_x,
        label_y0 + small_h,
        mode_color,
        0,
    );

    // Target summary: big digits + small unit, right-aligned in the right half.
    let target = data.control_target_text.as_str();
    let (num, unit) = target.split_at(target.len().saturating_sub(1));

    let num_w = setpoint_text_width(num, 0);
    let unit_w = small_text_width(unit, 0);
    let unit_gap = 1;
    let total_w = num_w + unit_gap + unit_w;

    // Keep a small right padding so the unit doesn't visually touch the pill edge.
    let right_pad = 3;
    let value_right = CONTROL_VALUE_PILL_RIGHT - right_pad;
    let value_x0 = (value_right - total_w).max(CONTROL_VALUE_PILL_LEFT);

    let num_h = SETPOINT_FONT.height() as i32;
    let num_y = CONTROL_ROW_TOP + ((CONTROL_ROW_BOTTOM - CONTROL_ROW_TOP) - num_h).max(0) / 2;
    // Baseline-align the unit with the larger numeric font by matching bottom edges.
    let unit_y = num_y + num_h - (SMALL_FONT.height() as i32);

    draw_setpoint_text(canvas, num, value_x0, num_y, rgb(0xdfe7ff), 0);
    draw_small_text(
        canvas,
        unit,
        value_x0 + num_w + unit_gap,
        unit_y,
        rgb(0x9ab0d8),
        0,
    );

    // Indicate which digit is currently selected for encoder adjustment.
    // Format is fixed-width and unit-dependent:
    // - A/V: "DD.ddd" (0 tens, 1 ones, 2 '.', 3 tenths, 4 hundredths, 5 thousandths)
    // - W:   "DDD.dd" (0 hundreds, 1 tens, 2 ones, 3 '.', 4 tenths, 5 hundredths)
    let idx = if data.control_target_unit == 'W' {
        match data.adjust_digit {
            AdjustDigit::Tenths => 4,
            _ => 2,
        }
    } else {
        match data.adjust_digit {
            AdjustDigit::Ones => 1,
            AdjustDigit::Tenths => 3,
            AdjustDigit::Hundredths => 4,
            AdjustDigit::Thousandths => 5,
        }
    };
    let glyph_w = SETPOINT_FONT.width() as i32;
    let cell_x = value_x0 + idx as i32 * glyph_w;
    // Place a short underline inside the pill, below the digit baseline.
    let underline_top = (num_y + num_h + 1).min(CONTROL_ROW_BOTTOM - 3);
    let underline_bottom = underline_top + 2;
    if underline_bottom <= CONTROL_ROW_BOTTOM {
        let left = cell_x + 1;
        let right = cell_x + glyph_w - 1;
        if right > left {
            canvas.fill_rect(
                Rect::new(left, underline_top, right, underline_bottom),
                rgb(0x4cc9f0),
            );
        }
    }
}

fn draw_preset_preview_panel(canvas: &mut Canvas, data: &UiSnapshot) {
    if !data.preset_preview_active {
        return;
    }

    // A1 preset preview info panel: mirror `tools/ui-mock/src/preset_preview_panel.rs`
    // for pixel-perfect constants/layout (logical 320x240 coordinate space).
    const BORDER: i32 = 1;
    const RADIUS: i32 = 6;
    const PAD_X: i32 = 10;
    const PAD_Y: i32 = 8;
    // Keep the preview panel above the telemetry/status region (TELEMETRY_TOP=172).
    // With 6 rows, ROW_H=18 yields panel bottom at y=170 (PANEL_TOP=44), avoiding overlap.
    const ROW_H: i32 = 18;
    const UNIT_GAP: i32 = 1;

    const COLOR_BG: u32 = 0x1c2638;
    const COLOR_BORDER: u32 = 0x1c2a3f;
    const COLOR_TEXT_LABEL: u32 = 0x9ab0d8;
    const COLOR_TEXT_VALUE: u32 = 0xdfe7ff;
    const COLOR_MODE_CV: u32 = 0xffb24a;
    const COLOR_MODE_CC: u32 = 0xff5252;
    const COLOR_MODE_CP: u32 = 0xb27bff;

    let mode = match data.active_mode {
        LoadMode::Cv => LoadMode::Cv,
        LoadMode::Cp => LoadMode::Cp,
        _ => LoadMode::Cc,
    };
    let rows = 6;
    let panel_h = BORDER * 2 + PAD_Y * 2 + rows * ROW_H;

    let outer = Rect::new(
        PRESET_PREVIEW_PANEL_LEFT,
        PRESET_PREVIEW_PANEL_TOP,
        PRESET_PREVIEW_PANEL_RIGHT,
        PRESET_PREVIEW_PANEL_TOP + panel_h,
    );
    canvas.fill_round_rect(outer, RADIUS, rgb(COLOR_BORDER));

    let inner = Rect::new(
        PRESET_PREVIEW_PANEL_LEFT + BORDER,
        PRESET_PREVIEW_PANEL_TOP + BORDER,
        PRESET_PREVIEW_PANEL_RIGHT - BORDER,
        PRESET_PREVIEW_PANEL_TOP + panel_h - BORDER,
    );
    canvas.fill_round_rect(inner, (RADIUS - BORDER).max(0), rgb(COLOR_BG));

    let label_x = PRESET_PREVIEW_PANEL_LEFT + PAD_X;
    let value_right = PRESET_PREVIEW_PANEL_RIGHT - PAD_X;
    let small_h = SMALL_FONT.height() as i32;
    let num_h = SETPOINT_FONT.height() as i32;

    let label_color = rgb(COLOR_TEXT_LABEL);
    let value_color = rgb(COLOR_TEXT_VALUE);

    let mut row_idx = 0;
    while row_idx < rows {
        let row_top = PRESET_PREVIEW_PANEL_TOP + BORDER + PAD_Y + row_idx * ROW_H;
        let row_bottom = row_top + ROW_H;

        let label_y = row_top + (ROW_H - small_h).max(0) / 2;

        match row_idx {
            0 => {
                draw_small_text(canvas, "PRESET", label_x, label_y, label_color, 0);

                let mut preset_value = String::<3>::new();
                let _ = preset_value.push('M');
                if (1..=9).contains(&data.active_preset_id) {
                    let _ = preset_value.push(char::from(b'0' + data.active_preset_id));
                } else {
                    let _ = preset_value.push('?');
                }
                let value_w = small_text_width(preset_value.as_str(), 0);
                let value_x0 = (value_right - value_w).max(label_x);
                draw_small_text(
                    canvas,
                    preset_value.as_str(),
                    value_x0,
                    label_y,
                    value_color,
                    0,
                );
            }
            1 => {
                draw_small_text(canvas, "MODE", label_x, label_y, label_color, 0);

                let (mode_text, mode_color) = match mode {
                    LoadMode::Cv => ("CV", rgb(COLOR_MODE_CV)),
                    LoadMode::Cp => ("CP", rgb(COLOR_MODE_CP)),
                    _ => ("CC", rgb(COLOR_MODE_CC)),
                };
                let value_w = small_text_width(mode_text, 0);
                let value_x0 = (value_right - value_w).max(label_x);
                draw_small_text(canvas, mode_text, value_x0, label_y, mode_color, 0);
            }
            _ => {
                let (field_label, field_value) = match row_idx - 2 {
                    0 => ("TARGET", data.preset_preview_target_text.as_str()),
                    1 => ("UVLO", data.preset_preview_v_lim_text.as_str()),
                    2 => ("OCP", data.preset_preview_i_lim_text.as_str()),
                    _ => ("OPP", data.preset_preview_p_lim_text.as_str()),
                };

                draw_small_text(canvas, field_label, label_x, label_y, label_color, 0);

                let (num, unit) = split_unit(field_value);
                let num_w = setpoint_text_width(num, 0);
                let unit_w = small_text_width(unit, 0);
                let total_w = num_w + UNIT_GAP + unit_w;
                let value_x0 = (value_right - total_w).max(label_x);

                let num_y = row_top + (ROW_H - num_h).max(0) / 2;
                let unit_y = num_y + num_h - small_h;

                draw_setpoint_text(canvas, num, value_x0, num_y, value_color, 0);
                draw_small_text(
                    canvas,
                    unit,
                    value_x0 + num_w + UNIT_GAP,
                    unit_y,
                    label_color,
                    0,
                );
            }
        }

        row_idx += 1;
        if row_idx < rows {
            canvas.fill_rect(
                Rect::new(
                    PRESET_PREVIEW_PANEL_LEFT + BORDER,
                    row_bottom,
                    PRESET_PREVIEW_PANEL_RIGHT - BORDER,
                    row_bottom + 1,
                ),
                rgb(COLOR_BORDER),
            );
        }
    }
}

fn split_unit(value: &str) -> (&str, &str) {
    if value.len() < 2 {
        return ("", "");
    }
    value.split_at(value.len() - 1)
}

fn draw_pair_header(canvas: &mut Canvas, left: (&str, &str), right: (&str, &str), top: i32) {
    draw_small_text(canvas, left.0, 198, top, rgb(0x6d7fa4), 0);
    draw_small_text(canvas, left.1, 198, top + 12, rgb(0xdfe7ff), 0);
    draw_small_text(canvas, right.0, 258, top, rgb(0x6d7fa4), 0);
    draw_small_text(canvas, right.1, 258, top + 12, rgb(0xdfe7ff), 0);
}

fn draw_mirror_bar_in_bounds(
    canvas: &mut Canvas,
    top: i32,
    left: i32,
    right: i32,
    left_ratio: f32,
    right_ratio: f32,
) {
    let bar_height = 8;
    let center = (left + right) / 2;
    canvas.fill_rect(Rect::new(left, top, right, top + bar_height), rgb(0x1c2638));
    canvas.draw_line(center, top - 2, center, top + bar_height + 2, rgb(0x6d7fa4));

    let half_width = (right - left) / 2;
    let left_fill = (half_width as f32 * left_ratio.clamp(0.0, 1.0) + 0.5) as i32;
    let right_fill = (half_width as f32 * right_ratio.clamp(0.0, 1.0) + 0.5) as i32;
    if left_fill > 0 {
        canvas.fill_rect(
            Rect::new(center - left_fill, top, center, top + bar_height),
            rgb(0x4cc9f0),
        );
    }
    if right_fill > 0 {
        canvas.fill_rect(
            Rect::new(center, top, center + right_fill, top + bar_height),
            rgb(0x4cc9f0),
        );
    }
}

fn draw_telemetry_line(canvas: &mut Canvas, data: &UiSnapshot, line_idx: usize) {
    let top = TELEMETRY_TOP + (line_idx as i32) * TELEMETRY_LINE_HEIGHT;
    let line = data.status_lines[line_idx].as_str();
    let is_alert_line = line_idx + 1 == TELEMETRY_LINE_COUNT;
    let mut color = rgb(0xdfe7ff);
    if is_alert_line {
        // Bottom-right "reason" line should blink on any abnormal condition.
        let ctl_alert = data.fault_flags != 0
            || data.link_alarm_latched
            || data.trip_alarm_abbrev.is_some()
            || data.blocked_enable_abbrev.is_some()
            || data.uv_latched
            || !data.link_up;
        if ctl_alert && !line.is_empty() {
            color = if data.blink_on {
                rgb(0xff5252)
            } else {
                rgb(0xffffff)
            };
        }
    }
    draw_small_text(canvas, line, 198, top, color, 0);
}

fn draw_telemetry(canvas: &mut Canvas, data: &UiSnapshot) {
    for line_idx in 0..TELEMETRY_LINE_COUNT {
        draw_telemetry_line(canvas, data, line_idx);
    }
}

pub fn hit_test_dashboard_load_button(x: i32, y: i32) -> bool {
    let radius = (LOAD_BUTTON_SIZE / 2) + 1;
    let cx = LOAD_BUTTON_LEFT + (LOAD_BUTTON_SIZE / 2);
    let cy = LOAD_ROW_TOP + (LOAD_BUTTON_SIZE / 2);
    let dx = x - cx;
    let dy = y - cy;
    dx.saturating_mul(dx) + dy.saturating_mul(dy) <= radius.saturating_mul(radius)
}

pub fn hit_test_dashboard_pd_button(x: i32, y: i32) -> bool {
    x >= PD_BUTTON_LEFT && x < PD_BUTTON_RIGHT && y >= LOAD_ROW_TOP && y < PD_BUTTON_BOTTOM
}

fn draw_dashboard_load_row(canvas: &mut Canvas, data: &UiSnapshot) {
    draw_dashboard_pd_button(canvas, data);
    draw_settings_button(canvas, LOAD_BUTTON_LEFT, LOAD_ROW_TOP);
}

fn draw_dashboard_pd_button(canvas: &mut Canvas, data: &UiSnapshot) {
    let rect = Rect::new(
        PD_BUTTON_LEFT,
        LOAD_ROW_TOP,
        PD_BUTTON_RIGHT,
        PD_BUTTON_BOTTOM,
    );

    let accent = match data.pd_state {
        PdButtonState::Safe5vOnly => rgb(0x555f75),
        PdButtonState::ExtendedAllowed => rgb(0x4cc9f0),
        PdButtonState::ExtendedFailed => rgb(0xff5252),
    };

    // Match the approved mock: neutral dark body, with state color only on the text + corner accents.
    canvas.fill_round_rect(rect, PD_BUTTON_RADIUS, rgb(0x1c2638));
    if !matches!(data.pd_state, PdButtonState::Safe5vOnly) {
        overlay_pd_button_corner_highlights(canvas, rect, accent);
    }

    let pad_top = 3;
    let line1_y = rect.top + pad_top;
    let line2_y = line1_y + 11;

    let spacing = 0;
    let line1 = "PD";
    let line1_w = small_text_width(line1, spacing);
    let line1_x = rect.left + ((rect.right - rect.left - line1_w).max(0) / 2);
    draw_small_text(canvas, line1, line1_x, line1_y, accent, spacing);

    let mut line2_buf = String::<8>::new();
    if let Some(mv) = data.pd_target_mv {
        let v10 = (mv + 50) / 100;
        if v10 % 10 == 0 {
            let _ = write!(&mut line2_buf, "{}V", v10 / 10);
        } else {
            let _ = write!(&mut line2_buf, "{}.{}V", v10 / 10, v10 % 10);
        }
    } else {
        let _ = line2_buf.push_str("N/A");
    }

    let line2 = line2_buf.as_str();
    let line2_w = small_text_width(line2, spacing);
    let line2_x = rect.left + ((rect.right - rect.left - line2_w).max(0) / 2);
    draw_small_text(canvas, line2, line2_x, line2_y, accent, spacing);
}

fn overlay_pd_button_corner_highlights(canvas: &mut Canvas, rect: Rect, color: Rgb565) {
    // Pixel-perfect corner accents from the existing design mock:
    // - 3px segments on each side near each corner
    // - 1px diagonal dot in each corner pocket
    //
    // This is intentionally *not* a rounded-rect outline.
    let left = rect.left;
    let right = rect.right;
    let top = rect.top;
    let bottom = rect.bottom;

    let x_l = left + 1;
    let x_r = right - 2;
    let y_t = top + 1;
    let y_b = bottom - 2;

    // Top/bottom short segments.
    for x in (left + 3)..=(left + 5) {
        canvas.set_pixel(x, y_t, color);
        canvas.set_pixel(x, y_b, color);
    }
    for x in (right - 6)..=(right - 4) {
        canvas.set_pixel(x, y_t, color);
        canvas.set_pixel(x, y_b, color);
    }

    // Left/right short segments.
    for y in (top + 3)..=(top + 5) {
        canvas.set_pixel(x_l, y, color);
        canvas.set_pixel(x_r, y, color);
    }
    for y in (bottom - 6)..=(bottom - 4) {
        canvas.set_pixel(x_l, y, color);
        canvas.set_pixel(x_r, y, color);
    }

    // Diagonal dots.
    canvas.set_pixel(left + 2, top + 2, color);
    canvas.set_pixel(right - 3, top + 2, color);
    canvas.set_pixel(left + 2, bottom - 3, color);
    canvas.set_pixel(right - 3, bottom - 3, color);
}

fn overlay_round_rect_corner_outline(canvas: &mut Canvas, rect: Rect, radius: i32, color: Rgb565) {
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    if w <= 0 || h <= 0 {
        return;
    }
    let mut r = radius.max(0);
    r = r.min(w / 2).min(h / 2);
    if r == 0 {
        return;
    }

    let left_r = rect.left + r;
    let right_r = rect.right - r;
    let top_r = rect.top + r;
    let bottom_r = rect.bottom - r;

    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            let in_corner = (x < left_r && y < top_r)
                || (x >= right_r && y < top_r)
                || (x < left_r && y >= bottom_r)
                || (x >= right_r && y >= bottom_r);
            if !in_corner {
                continue;
            }
            if !point_in_round_rect(rect, radius, x, y) {
                continue;
            }
            // Single-outline highlight (avoid the "double arc" look from inner/outer rings).
            let edge = !point_in_round_rect(rect, radius, x - 1, y)
                || !point_in_round_rect(rect, radius, x + 1, y)
                || !point_in_round_rect(rect, radius, x, y - 1)
                || !point_in_round_rect(rect, radius, x, y + 1);
            if edge {
                canvas.set_pixel(x, y, color);
            }
        }
    }
}

fn draw_settings_button(canvas: &mut Canvas, left: i32, top: i32) {
    let border = rgb(0x1c2a3f);
    let shadow = rgb(0x19243a);
    let fill = rgb(0x1c2638);
    let icon = rgb(0xf4f7ff);

    let size = LOAD_BUTTON_SIZE;
    let center = (size - 1) / 2;
    let outer_r = center;

    for y in 0..size {
        for x in 0..size {
            let dx = x - center;
            let dy = y - center;
            let d2 = dx * dx + dy * dy;
            let d2_4 = d2 * 4;

            let mut px = None;
            if in_circle_ring(d2_4, outer_r) {
                px = Some(border);
            } else if in_circle_ring(d2_4, outer_r - 1) {
                px = Some(shadow);
            } else if d2_4 <= circle_fill_limit_4(outer_r - 2) {
                px = Some(fill);
            }

            let line1 = dx == -4 && dy >= -5 && dy <= 5;
            let line2 = dx == 0 && dy >= -5 && dy <= 5;
            let line3 = dx == 4 && dy >= -5 && dy <= 5;
            let knob1 = dy >= -3 && dy <= -1 && dx >= -5 && dx <= -3;
            let knob2 = dy >= 0 && dy <= 2 && dx >= -1 && dx <= 1;
            let knob3 = dy >= 2 && dy <= 4 && dx >= 3 && dx <= 5;
            if line1 || line2 || line3 || knob1 || knob2 || knob3 {
                px = Some(icon);
            }

            if let Some(px) = px {
                canvas.set_pixel(left + x, top + y, px);
            }
        }
    }
}

fn in_circle_ring(d2_4: i32, r: i32) -> bool {
    if r <= 0 {
        return false;
    }
    // Distance-to-radius check using (2*dist)^2 to avoid floating point.
    let lo = (2 * r - 1) * (2 * r - 1);
    let hi = (2 * r + 1) * (2 * r + 1);
    d2_4 >= lo && d2_4 <= hi
}

fn circle_fill_limit_4(r: i32) -> i32 {
    if r <= 0 {
        return 0;
    }
    (2 * r + 1) * (2 * r + 1)
}

fn draw_debug_overlay(canvas: &mut Canvas) {
    // Corner labels for orientation
    draw_small_text(canvas, "TOP", 4, 4, rgb(0xFFFFFF), 0);
    draw_small_text(canvas, "BOTTOM", 4, LOGICAL_HEIGHT - 12, rgb(0xFFFFFF), 0);
    draw_small_text(canvas, "LEFT", 4, LOGICAL_HEIGHT / 2, rgb(0xFFFFFF), 0);
    draw_small_text(
        canvas,
        "RIGHT",
        LOGICAL_WIDTH - 48,
        LOGICAL_HEIGHT / 2,
        rgb(0xFFFFFF),
        0,
    );

    // Draw axis arrows
    canvas.draw_line(150, 4, 170, 4, rgb(0xFF0000));
    canvas.draw_line(170, 4, 164, 0, rgb(0xFF0000));
    canvas.draw_line(170, 4, 164, 8, rgb(0xFF0000));
    draw_small_text(canvas, "+X", 172, 0, rgb(0xFF0000), 0);
    canvas.draw_line(150, 4, 150, 24, rgb(0x00FF00));
    canvas.draw_line(150, 24, 146, 18, rgb(0x00FF00));
    canvas.draw_line(150, 24, 154, 18, rgb(0x00FF00));
    draw_small_text(canvas, "+Y", 138, 24, rgb(0x00FF00), 0);

    // Color swatches for RGB565 verification
    const SWATCHES: [(&str, u32); 6] = [
        ("R", 0xFF0000),
        ("G", 0x00FF00),
        ("B", 0x0000FF),
        ("Y", 0xFFFF00),
        ("C", 0x00FFFF),
        ("W", 0xFFFFFF),
    ];
    let mut x = 194;
    for &(label, hex) in SWATCHES.iter() {
        canvas.fill_rect(Rect::new(x, 0, x + 10, 10), rgb(hex));
        draw_small_text(canvas, label, x, 10, rgb(0xFFFFFF), 0);
        x += 12;
    }
}

fn draw_seven_seg_value(canvas: &mut Canvas, value: &str, area: &Rect, color: Rgb565) {
    let spacing = 4;
    let mut total_width = 0;
    for ch in value.chars() {
        total_width += match ch {
            '.' => 8,
            _ => SEVEN_SEG_FONT.width() as i32,
        } + spacing;
    }
    if !value.is_empty() {
        total_width -= spacing;
    }
    let mut cursor_x = area.right - total_width;
    for ch in value.chars() {
        if ch == '.' {
            canvas.fill_rect(
                Rect::new(cursor_x, area.bottom - 10, cursor_x + 6, area.bottom - 4),
                color,
            );
            cursor_x += 8 + spacing;
            continue;
        }
        SEVEN_SEG_FONT.draw_char(
            ch,
            |x, y| canvas.set_pixel(x + cursor_x, y + area.top, color),
            0,
            0,
        );
        cursor_x += SEVEN_SEG_FONT.width() as i32 + spacing;
    }
}

fn draw_small_text(
    canvas: &mut Canvas,
    text: &str,
    mut x: i32,
    y: i32,
    color: Rgb565,
    spacing: i32,
) {
    for ch in text.chars() {
        if ch == ' ' {
            x += SMALL_FONT.width() as i32 + spacing;
            continue;
        }
        SMALL_FONT.draw_char(ch, |px, py| canvas.set_pixel(px + x, py + y, color), 0, 0);
        x += SMALL_FONT.width() as i32 + spacing;
    }
}

fn draw_setpoint_text(
    canvas: &mut Canvas,
    text: &str,
    mut x: i32,
    y: i32,
    color: Rgb565,
    spacing: i32,
) {
    let glyph = SETPOINT_FONT.width() as i32 + spacing;
    for ch in text.chars() {
        SETPOINT_FONT.draw_char(ch, |px, py| canvas.set_pixel(px + x, py + y, color), 0, 0);
        x += glyph;
    }
}

fn format_fixed_2dp(value: f32) -> String<8> {
    // 固定格式：DD.dd（总计 4 个数字 + 1 个小数点），四舍五入到 0.01。
    // 用于主电压/电流、右侧电压/电流对、以及控制行 setpoint 的数值部分。
    let mut s = String::<8>::new();

    if !value.is_finite() {
        let _ = s.push_str("99.99");
        return s;
    }

    let v = value.abs();
    let scaled = (v * 100.0 + 0.5) as u32; // 0.01 units, half-up rounding
    if scaled > 9_999 {
        let _ = s.push_str("99.99");
        return s;
    }

    let int_part = scaled / 100; // 0..99
    let frac_part = scaled % 100; // 0..99

    let _ = s.push((b'0' + (int_part / 10) as u8) as char);
    let _ = s.push((b'0' + (int_part % 10) as u8) as char);
    let _ = s.push('.');
    let _ = s.push((b'0' + (frac_part / 10) as u8) as char);
    let _ = s.push((b'0' + (frac_part % 10) as u8) as char);
    s
}

fn format_fixed_1dp_3i(value: f32) -> String<8> {
    // 固定格式：DDD.d（总计 4 个数字 + 1 个小数点），四舍五入到 0.1。
    // 用于主功率显示。
    let mut s = String::<8>::new();

    if !value.is_finite() {
        let _ = s.push_str("999.9");
        return s;
    }

    let v = value.abs();
    let scaled = (v * 10.0 + 0.5) as u32; // 0.1 units, half-up rounding
    if scaled > 9_999 {
        let _ = s.push_str("999.9");
        return s;
    }

    let int_part = scaled / 10; // 0..999
    let frac_part = scaled % 10; // 0..9

    let _ = s.push((b'0' + ((int_part / 100) % 10) as u8) as char);
    let _ = s.push((b'0' + ((int_part / 10) % 10) as u8) as char);
    let _ = s.push((b'0' + (int_part % 10) as u8) as char);
    let _ = s.push('.');
    let _ = s.push((b'0' + frac_part as u8) as char);
    s
}

fn format_pair_value(value: f32, unit: char) -> String<6> {
    let digits = format_fixed_2dp(value);
    let mut s = String::<6>::new();
    let _ = s.push_str(digits.as_str());
    let _ = s.push(unit);
    s
}

fn format_setpoint_milli(value_milli: i32, unit: char) -> String<7> {
    // Fixed-width numeric text for the control row:
    // - A/V: "DD.dddU" (7 chars)
    // - W:   "DDD.ddU" (7 chars)
    // Matches `docs/plan/0005:on-device-preset-ui/PLAN.md` (+ CP extension).
    let mut s = String::<7>::new();
    let v = value_milli.max(0) as u32;

    if unit == 'W' {
        if v > 999_990 {
            let _ = s.push_str("---.--");
            let _ = s.push(unit);
            return s;
        }

        // mW -> centi-watts (0.01 W).
        let centi_w = v / 10;
        let int_part = centi_w / 100; // 0..999
        let frac_part = centi_w % 100; // 0..99

        let _ = s.push((b'0' + ((int_part / 100) % 10) as u8) as char);
        let _ = s.push((b'0' + ((int_part / 10) % 10) as u8) as char);
        let _ = s.push((b'0' + (int_part % 10) as u8) as char);
        let _ = s.push('.');
        let _ = s.push((b'0' + (frac_part / 10) as u8) as char);
        let _ = s.push((b'0' + (frac_part % 10) as u8) as char);
        let _ = s.push(unit);
        return s;
    }

    if v > 99_999 {
        let _ = s.push_str("--.---");
        let _ = s.push(unit);
        return s;
    }

    let int_part = v / 1000; // 0..99
    let frac_part = v % 1000; // 0..999

    let _ = s.push((b'0' + (int_part / 10) as u8) as char);
    let _ = s.push((b'0' + (int_part % 10) as u8) as char);
    let _ = s.push('.');
    append_frac(&mut s, frac_part, 3);
    let _ = s.push(unit);
    s
}

fn append_u32<const N: usize>(buf: &mut String<N>, mut value: u32) {
    // 把无符号整数按十进制追加到 buf（不做左侧补零）。
    let mut tmp = [0u8; 10];
    let mut i = 0;
    if value == 0 {
        let _ = buf.push('0');
        return;
    }
    while value > 0 && i < tmp.len() {
        tmp[i] = b'0' + (value % 10) as u8;
        value /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        let _ = buf.push(tmp[i] as char);
    }
}

fn append_frac<const N: usize>(buf: &mut String<N>, mut value: u32, digits: u8) {
    // 以固定位数输出小数部分，必要时左侧补零。
    let mut tmp = [b'0'; 4];
    let mut i = digits as usize;
    while i > 0 {
        i -= 1;
        tmp[i] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    for i in 0..(digits as usize) {
        let _ = buf.push(tmp[i] as char);
    }
}

fn rgb(hex: u32) -> Rgb565 {
    let r = ((hex >> 16) & 0xFF) as u8;
    let g = ((hex >> 8) & 0xFF) as u8;
    let b = (hex & 0xFF) as u8;
    embedded_graphics::pixelcolor::Rgb888::new(r, g, b).into()
}

fn rgb888(r: u8, g: u8, b: u8) -> Rgb565 {
    embedded_graphics::pixelcolor::Rgb888::new(r, g, b).into()
}

fn lerp_u8(a: u8, b: u8, t255: u32) -> u8 {
    let a = a as u32;
    let b = b as u32;
    ((a * (255 - t255) + b * t255 + 127) / 255) as u8
}

fn point_in_round_rect(rect: Rect, radius: i32, x: i32, y: i32) -> bool {
    if x < rect.left || x >= rect.right || y < rect.top || y >= rect.bottom {
        return false;
    }
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    if w <= 0 || h <= 0 {
        return false;
    }
    let mut r = radius.max(0);
    r = r.min(w / 2).min(h / 2);
    if r == 0 {
        return true;
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

    if x < left_r && y < top_r {
        let dx = x - tl_cx;
        let dy = y - tl_cy;
        dx * dx + dy * dy <= r2
    } else if x >= right_r && y < top_r {
        let dx = x - tr_cx;
        let dy = y - tr_cy;
        dx * dx + dy * dy <= r2
    } else if x < left_r && y >= bottom_r {
        let dx = x - bl_cx;
        let dy = y - bl_cy;
        dx * dx + dy * dy <= r2
    } else if x >= right_r && y >= bottom_r {
        let dx = x - br_cx;
        let dy = y - br_cy;
        dx * dx + dy * dy <= r2
    } else {
        true
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

    fn fill_round_rect_vgradient(
        &mut self,
        rect: Rect,
        radius: i32,
        top_rgb: (u8, u8, u8),
        bottom_rgb: (u8, u8, u8),
    ) {
        let h = (rect.bottom - rect.top).max(1) as u32;
        for y in rect.top..rect.bottom {
            let t = if h <= 1 {
                0
            } else {
                ((y - rect.top) as u32 * 255) / (h - 1)
            };
            let r = lerp_u8(top_rgb.0, bottom_rgb.0, t);
            let g = lerp_u8(top_rgb.1, bottom_rgb.1, t);
            let b = lerp_u8(top_rgb.2, bottom_rgb.2, t);
            let row = rgb888(r, g, b);
            for x in rect.left..rect.right {
                if point_in_round_rect(rect, radius, x, y) {
                    self.set_pixel(x, y, row);
                }
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

        for y in rect.top..rect.bottom {
            for x in rect.left..rect.right {
                let inside = if x < left_r && y < top_r {
                    let dx = x - tl_cx;
                    let dy = y - tl_cy;
                    dx * dx + dy * dy <= r2
                } else if x >= right_r && y < top_r {
                    let dx = x - tr_cx;
                    let dy = y - tr_cy;
                    dx * dx + dy * dy <= r2
                } else if x < left_r && y >= bottom_r {
                    let dx = x - bl_cx;
                    let dy = y - bl_cy;
                    dx * dx + dy * dy <= r2
                } else if x >= right_r && y >= bottom_r {
                    let dx = x - br_cx;
                    let dy = y - br_cy;
                    dx * dx + dy * dy <= r2
                } else {
                    true
                };
                if inside {
                    self.set_pixel(x, y, color);
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

#[derive(Debug, Clone)]
pub struct UiSnapshot {
    pub main_voltage: f32,
    pub main_current: f32,
    pub main_power: f32,
    pub remote_voltage: f32,
    pub local_voltage: f32,
    pub ch1_current: f32,
    pub ch2_current: f32,
    pub control_target_milli: i32,
    pub control_target_unit: char,
    pub adjust_digit: AdjustDigit,
    pub run_time: String<16>,
    pub sink_core_temp: f32,
    pub sink_exhaust_temp: f32,
    pub mcu_temp: f32,
    pub energy_wh: f32,
    pub remote_active: bool,
    pub fault_flags: u32,
    pub analog_state: AnalogState,
    pub wifi_status: WifiUiStatus,
    // Control overlay (active preset + mode + output + UV latch), driven by the
    // digital-side preset/control model.
    pub calibration_mode: CalibrationUiMode,
    pub active_preset_id: u8,
    pub output_enabled: bool,
    pub active_mode: LoadMode,
    pub uv_latched: bool,
    pub link_up: bool,
    pub link_alarm_latched: bool,
    pub hello_seen: bool,
    pub trip_alarm_abbrev: Option<&'static str>,
    pub blocked_enable_abbrev: Option<&'static str>,
    pub blink_on: bool,
    pub pd_state: PdButtonState,
    pub pd_display_mode: PdButtonDisplayMode,
    pub pd_target_mv: Option<u32>,
    pub pd_target_available: bool,
    pub preset_preview_active: bool,
    pub preset_preview_target_text: String<8>,
    pub preset_preview_v_lim_text: String<8>,
    pub preset_preview_i_lim_text: String<8>,
    pub preset_preview_p_lim_text: String<8>,
    // Preformatted strings for on-demand, character-aware updates.
    pub main_voltage_text: String<8>,
    pub main_current_text: String<8>,
    pub main_power_text: String<8>,
    pub remote_voltage_text: String<6>,
    pub local_voltage_text: String<6>,
    pub ch1_current_text: String<6>,
    pub ch2_current_text: String<6>,
    pub control_target_text: String<7>,
    pub status_lines: [String<20>; 5],
}

fn fault_flags_abbrev(flags: u32) -> &'static str {
    if flags & FAULT_OVERVOLTAGE != 0 {
        "OVP"
    } else if flags & (FAULT_MCU_OVER_TEMP | FAULT_SINK_OVER_TEMP) != 0 {
        "OTP"
    } else if flags & FAULT_OVERCURRENT != 0 {
        "OCF"
    } else {
        "FLT"
    }
}

impl UiSnapshot {
    pub fn demo() -> Self {
        let mut run_time = String::<16>::new();
        let _ = run_time.push_str("01:32:10");
        Self {
            main_voltage: 24.50,
            main_current: 12.00,
            main_power: 294.0,
            remote_voltage: 24.52,
            local_voltage: 24.47,
            ch1_current: 4.20,
            ch2_current: 3.50,
            control_target_milli: 12_000,
            control_target_unit: 'A',
            adjust_digit: AdjustDigit::DEFAULT,
            run_time,
            sink_core_temp: 42.3,
            sink_exhaust_temp: 38.1,
            mcu_temp: 35.0,
            energy_wh: 125.4,
            remote_active: true,
            fault_flags: 0,
            analog_state: AnalogState::Ready,
            wifi_status: WifiUiStatus::Disabled,
            calibration_mode: CalibrationUiMode::Off,
            active_preset_id: 1,
            output_enabled: false,
            active_mode: LoadMode::Cc,
            uv_latched: false,
            link_up: true,
            link_alarm_latched: false,
            hello_seen: true,
            trip_alarm_abbrev: None,
            blocked_enable_abbrev: None,
            blink_on: false,
            pd_state: PdButtonState::Safe5vOnly,
            pd_display_mode: PdButtonDisplayMode::Fixed,
            pd_target_mv: Some(5_000),
            pd_target_available: true,
            preset_preview_active: false,
            preset_preview_target_text: String::new(),
            preset_preview_v_lim_text: String::new(),
            preset_preview_i_lim_text: String::new(),
            preset_preview_p_lim_text: String::new(),
            main_voltage_text: String::new(),
            main_current_text: String::new(),
            main_power_text: String::new(),
            remote_voltage_text: String::new(),
            local_voltage_text: String::new(),
            ch1_current_text: String::new(),
            ch2_current_text: String::new(),
            control_target_text: String::new(),
            status_lines: Default::default(),
        }
    }

    pub fn set_control_overlay(
        &mut self,
        calibration_mode: CalibrationUiMode,
        active_preset_id: u8,
        output_enabled: bool,
        mode: LoadMode,
        uv_latched: bool,
        link_up: bool,
        link_alarm_latched: bool,
        hello_seen: bool,
        trip_alarm_abbrev: Option<&'static str>,
        blocked_enable_abbrev: Option<&'static str>,
    ) {
        self.calibration_mode = calibration_mode;
        self.active_preset_id = active_preset_id;
        self.output_enabled = output_enabled;
        self.active_mode = match mode {
            LoadMode::Cc => LoadMode::Cc,
            LoadMode::Cv => LoadMode::Cv,
            LoadMode::Cp => LoadMode::Cp,
            LoadMode::Reserved(_) => LoadMode::Cc,
        };
        self.uv_latched = uv_latched;
        self.link_up = link_up;
        self.link_alarm_latched = link_alarm_latched;
        self.hello_seen = hello_seen;
        self.trip_alarm_abbrev = trip_alarm_abbrev;
        self.blocked_enable_abbrev = blocked_enable_abbrev;
    }

    pub fn set_control_row(&mut self, target_milli: i32, unit: char, adjust_digit: AdjustDigit) {
        self.control_target_milli = target_milli;
        self.control_target_unit = unit;
        self.adjust_digit = adjust_digit;
    }

    /// Recompute all preformatted strings from the current numeric snapshot.
    pub fn update_strings(&mut self) {
        self.main_voltage_text = format_fixed_2dp(self.main_voltage);
        self.main_current_text = format_fixed_2dp(self.main_current);
        self.main_power_text = format_fixed_1dp_3i(self.main_power);

        if self.remote_active {
            self.remote_voltage_text = format_pair_value(self.remote_voltage, 'V');
        } else {
            self.remote_voltage_text.clear();
            let _ = self.remote_voltage_text.push_str("--.--");
        }
        self.local_voltage_text = format_pair_value(self.local_voltage, 'V');
        self.ch1_current_text = format_pair_value(self.ch1_current, 'A');
        self.ch2_current_text = format_pair_value(self.ch2_current, 'A');
        self.control_target_text =
            format_setpoint_milli(self.control_target_milli, self.control_target_unit);

        self.status_lines = self.compute_status_lines();
    }

    // CORE = NTC near MOSFETs (Tag1 / TS2 / R40, `sink_core_temp_mc`)
    // SINK = NTC near exhaust/side wall (Tag2 / TS1 / R39, `sink_exhaust_temp_mc`)
    fn compute_status_lines(&self) -> [String<20>; 5] {
        let mut run = String::<20>::new();
        let _ = run.push_str("RUN ");
        let _ = run.push_str(self.run_time.as_str());

        let mut core = String::<20>::new();
        let _ = core.push_str("CORE ");
        append_temp_1dp(&mut core, self.sink_core_temp);
        let _ = core.push('C');

        let mut exhaust = String::<20>::new();
        let _ = exhaust.push_str("SINK ");
        append_temp_1dp(&mut exhaust, self.sink_exhaust_temp);
        let _ = exhaust.push('C');

        let mut mcu = String::<20>::new();
        let _ = mcu.push_str("MCU  ");
        append_temp_1dp(&mut mcu, self.mcu_temp);
        let _ = mcu.push('C');

        let mut ctl = String::<20>::new();
        // Dashboard reason line (frozen by docs):
        // show fault > "LNK" (latched link-drop-class) > trip ("OCP/OPP") > "UVLO" > "OFF"
        // when LOAD cannot be enabled / is forced OFF.
        if self.fault_flags != 0 {
            let _ = ctl.push_str(fault_flags_abbrev(self.fault_flags));
        } else if self.link_alarm_latched {
            let _ = ctl.push_str("LNK");
        } else if let Some(trip) = self.trip_alarm_abbrev {
            let _ = ctl.push_str(trip);
        } else if self.uv_latched {
            let _ = ctl.push_str("UVLO");
        } else if let Some(blocked) = self.blocked_enable_abbrev {
            let _ = ctl.push_str(blocked);
        } else if !self.link_up {
            if self.hello_seen {
                let _ = ctl.push_str("LNK");
            } else {
                let _ = ctl.push_str("OFF");
            }
        } else if self.calibration_mode != CalibrationUiMode::Off {
            match self.analog_state {
                AnalogState::Faulted => {
                    let _ = ctl.push_str("FLT");
                }
                AnalogState::Offline => {
                    let _ = ctl.push_str("OFF");
                }
                AnalogState::CalMissing | AnalogState::Ready => {
                    let _ = ctl.push_str(self.calibration_mode.status_text());
                }
            }
        } else {
            // Normal status line avoids debug-y bitfields like "P1 CC OUT0 UV0 ...",
            // which are easy to misread on SmallFont (0/O) and exceed the visible width.
            match self.analog_state {
                AnalogState::Offline => {
                    let _ = ctl.push_str("OFF");
                }
                AnalogState::CalMissing => {
                    let _ = ctl.push_str("CAL");
                }
                AnalogState::Ready => {
                    let _ = ctl.push_str("RDY");
                }
                AnalogState::Faulted => {
                    let _ = ctl.push_str("FLT");
                }
            }
        }

        [run, core, exhaust, mcu, ctl]
    }

    pub fn status_lines(&self) -> &[String<20>; 5] {
        &self.status_lines
    }
}

fn append_temp_1dp<const N: usize>(buf: &mut String<N>, value: f32) {
    // 简单 1 位小数格式化（不做宽度对齐），与 format_value 使用同样的缩放策略。
    let mut v = value;
    if v.is_nan() {
        let _ = buf.push_str("NaN");
        return;
    }
    if v.is_infinite() {
        if v.is_sign_negative() {
            let _ = buf.push_str("-Inf");
        } else {
            let _ = buf.push_str("Inf");
        }
        return;
    }
    if v < 0.0 {
        let _ = buf.push('-');
        v = -v;
    }
    let scaled = (v * 10.0 + 0.5) as u32;
    let int_part = scaled / 10;
    let frac_part = scaled % 10;
    append_u32(buf, int_part);
    let _ = buf.push('.');
    append_frac(buf, frac_part, 1);
}

#[derive(Copy, Clone)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibration_status_line_uses_channel_specific_copy_when_ready() {
        let mut snapshot = UiSnapshot::demo();
        snapshot.calibration_mode = CalibrationUiMode::CurrentCh1;
        snapshot.analog_state = AnalogState::Ready;
        snapshot.update_strings();

        assert_eq!(snapshot.status_lines()[4].as_str(), "CAL C1");

        snapshot.calibration_mode = CalibrationUiMode::CurrentCh2;
        snapshot.update_strings();
        assert_eq!(snapshot.status_lines()[4].as_str(), "CAL C2");
    }

    #[test]
    fn calibration_status_line_keeps_fault_priority() {
        let mut snapshot = UiSnapshot::demo();
        snapshot.calibration_mode = CalibrationUiMode::CurrentCh1;
        snapshot.fault_flags = FAULT_OVERCURRENT;
        snapshot.update_strings();

        assert_eq!(snapshot.status_lines()[4].as_str(), "OCF");
    }

    #[test]
    fn calibration_ui_mode_maps_from_protocol_kind() {
        assert_eq!(
            CalibrationUiMode::from_cal_kind(CalKind::CurrentCh1),
            CalibrationUiMode::CurrentCh1
        );
        assert_eq!(
            CalibrationUiMode::from_cal_kind(CalKind::CurrentCh2),
            CalibrationUiMode::CurrentCh2
        );
        assert_eq!(
            CalibrationUiMode::from_cal_kind(CalKind::Voltage),
            CalibrationUiMode::Voltage
        );
        assert_eq!(
            CalibrationUiMode::from_cal_kind(CalKind::Off),
            CalibrationUiMode::Off
        );
    }
}
