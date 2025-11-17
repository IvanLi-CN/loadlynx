mod fonts;

use embedded_graphics::pixelcolor::{
    Rgb565,
    raw::{RawData, RawU16},
};
use heapless::String;
use lcd_async::raw_framebuf::RawFrameBuf;

use crate::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

use self::fonts::{SEVEN_SEG_FONT, SMALL_FONT};

const LOGICAL_WIDTH: i32 = 320;
const LOGICAL_HEIGHT: i32 = 240;
const DEBUG_OVERLAY: bool = false;

// 左侧三张主卡片的布局参数，便于在全帧/局部渲染中保持一致。
const CARD_TOPS: [i32; 3] = [0, 80, 160];
const CARD_BG_LEFT: i32 = 8;
const CARD_BG_RIGHT: i32 = 182;
// 背景在 Y 方向的偏移：与原设计保持上边距 6px，同时向下扩展到 +80，
// 以完全覆盖 32x50 的七段字体（area.top = top+28，高度 50 → bottom=top+78）。
const CARD_BG_TOP_OFFSET: i32 = 6;
const CARD_BG_BOTTOM_OFFSET: i32 = 80;

/// Bitmask describing which logical UI regions need to be updated for a frame.
#[derive(Copy, Clone, Default)]
pub struct UiChangeMask {
    pub main_metrics: bool,
    pub voltage_pair: bool,
    pub current_pair: bool,
    pub telemetry_lines: bool,
    pub bars: bool,
}

impl UiChangeMask {
    pub fn is_empty(&self) -> bool {
        !(self.main_metrics
            || self.voltage_pair
            || self.current_pair
            || self.telemetry_lines
            || self.bars)
    }
}

pub fn render_default(frame: &mut RawFrameBuf<Rgb565, &mut [u8]>) {
    render(frame, &UiSnapshot::demo());
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

    let card_colors = [rgb(0x171f33), rgb(0x141d2f), rgb(0x111828)];
    for (idx, &top) in CARD_TOPS.iter().enumerate() {
        canvas.fill_rect(
            Rect::new(
                CARD_BG_LEFT,
                top + CARD_BG_TOP_OFFSET,
                CARD_BG_RIGHT,
                top + CARD_BG_BOTTOM_OFFSET,
            ),
            card_colors[idx],
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
    draw_main_metric(
        &mut canvas,
        "POWER",
        data.main_power_text.as_str(),
        "W",
        160,
        rgb(0x6EF58C),
    );

    draw_voltage_pair(
        &mut canvas,
        data,
        data.remote_voltage_text.as_str(),
        data.local_voltage_text.as_str(),
    );

    draw_current_pair(
        &mut canvas,
        data,
        data.ch1_current_text.as_str(),
        data.ch2_current_text.as_str(),
    );
    draw_telemetry(&mut canvas, data);

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

    if mask.main_metrics {
        // 先恢复三张卡片的背景色，再重绘大数码管和标签，避免数字形态变化时残影堆叠。
        let card_colors = [rgb(0x171f33), rgb(0x141d2f), rgb(0x111828)];
        for (idx, &top) in CARD_TOPS.iter().enumerate() {
            canvas.fill_rect(
                Rect::new(
                    CARD_BG_LEFT,
                    top + CARD_BG_TOP_OFFSET,
                    CARD_BG_RIGHT,
                    top + CARD_BG_BOTTOM_OFFSET,
                ),
                card_colors[idx],
            );
        }

        // 左侧主数值区域：按需整体重绘各自区域（相比整屏已经大幅减载）。
        draw_main_metric(
            &mut canvas,
            "VOLTAGE",
            curr.main_voltage_text.as_str(),
            "V",
            0,
            rgb(0xFFB347),
        );
        draw_main_metric(
            &mut canvas,
            "CURRENT",
            curr.main_current_text.as_str(),
            "A",
            80,
            rgb(0xFF5252),
        );
        draw_main_metric(
            &mut canvas,
            "POWER",
            curr.main_power_text.as_str(),
            "W",
            160,
            rgb(0x6EF58C),
        );
    }

    if mask.voltage_pair {
        // 清理右侧电压对所占区域的背景，再重绘标题和条形图。
        canvas.fill_rect(
            Rect::new(190, 8, LOGICAL_WIDTH, 96),
            rgb(0x080f19),
        );
        let remote_text = curr.remote_voltage_text.as_str();
        let local_text = curr.local_voltage_text.as_str();
        draw_voltage_pair(&mut canvas, curr, remote_text, local_text);
    }

    if mask.current_pair {
        // 清理右侧电流对所占区域的背景，再重绘标题和条形图。
        canvas.fill_rect(
            Rect::new(190, 96, LOGICAL_WIDTH, 180),
            rgb(0x080f19),
        );
        let ch1_text = curr.ch1_current_text.as_str();
        let ch2_text = curr.ch2_current_text.as_str();
        draw_current_pair(&mut canvas, curr, ch1_text, ch2_text);
    }

    if mask.telemetry_lines {
        // 底部 5 行状态文本：先擦除对应背景行，再写新文本，避免字符串长度变化时残影。
        canvas.fill_rect(
            Rect::new(190, 180, LOGICAL_WIDTH, LOGICAL_HEIGHT),
            rgb(0x080f19),
        );
        draw_telemetry(&mut canvas, curr);
    }

    if mask.bars {
        // Bars are driven from remote/local voltage and currents; reuse the
        // existing helpers to redraw the bars over the existing background.
        draw_mirror_bar(
            &mut canvas,
            8 + 34,
            curr.remote_voltage / 40.0,
            curr.local_voltage / 40.0,
        );
        draw_mirror_bar(
            &mut canvas,
            96 + 34,
            curr.ch1_current / 5.0,
            curr.ch2_current / 5.0,
        );
    }
}

/// 在左上角叠加显示 FPS 信息。这里使用简单的整数 FPS 估计：
/// fps ≈ 1000 / dt_ms，当 dt_ms==0 时显示为 0。
pub fn render_fps_overlay(
    frame: &mut RawFrameBuf<Rgb565, &mut [u8]>,
    dt_ms: u32,
) {
    let fps = if dt_ms > 0 {
        // 四舍五入到最近的整数 FPS。
        (1000 + dt_ms / 2) / dt_ms
    } else {
        0
    };

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

fn draw_main_metric(
    canvas: &mut Canvas,
    label: &str,
    value: &str,
    unit: &str,
    top: i32,
    digit_color: Rgb565,
) {
    draw_small_text(canvas, label, 16, top + 10, rgb(0x9ab0d8), 1);
    let area = Rect::new(24, top + 28, 170, top + 72);
    draw_seven_seg_value(canvas, value, &area, digit_color);
    draw_small_text(canvas, unit, area.right, top + 56, rgb(0x9ab0d8), 1);
}

fn draw_voltage_pair(canvas: &mut Canvas, data: &UiSnapshot, left_value: &str, right_value: &str) {
    draw_pair_header(canvas, ("REMOTE", left_value), ("LOCAL", right_value), 8);
    draw_mirror_bar(
        canvas,
        8 + 34,
        data.remote_voltage / 40.0,
        data.local_voltage / 40.0,
    );
}

fn draw_current_pair(canvas: &mut Canvas, data: &UiSnapshot, left_value: &str, right_value: &str) {
    draw_pair_header(canvas, ("CH1", left_value), ("CH2", right_value), 96);
    draw_mirror_bar(
        canvas,
        96 + 34,
        data.ch1_current / 5.0,
        data.ch2_current / 5.0,
    );
}

fn draw_pair_header(canvas: &mut Canvas, left: (&str, &str), right: (&str, &str), top: i32) {
    draw_small_text(canvas, left.0, 198, top, rgb(0x6d7fa4), 0);
    draw_small_text(canvas, left.1, 198, top + 12, rgb(0xdfe7ff), 0);
    draw_small_text(canvas, right.0, 258, top, rgb(0x6d7fa4), 0);
    draw_small_text(canvas, right.1, 258, top + 12, rgb(0xdfe7ff), 0);
}

fn draw_mirror_bar(canvas: &mut Canvas, top: i32, left_ratio: f32, right_ratio: f32) {
    let bar_height = 8;
    let left = 198;
    let right = 314;
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

fn draw_telemetry(canvas: &mut Canvas, data: &UiSnapshot) {
    let lines = data.status_lines();
    let mut baseline = 180;
    for line in lines.iter() {
        draw_small_text(canvas, line.as_str(), 198, baseline, rgb(0xdfe7ff), 0);
        baseline += 12;
    }
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

fn format_value(value: f32, decimals: usize) -> String<8> {
    // 手写一个小型格式化器，避免依赖核心库的浮点 fmt 路径（会拉入大整数算法）。
    // 约定：总宽度最多 5 个字符（含符号和小数点），不足左侧不强制补空格。
    let mut s = String::<8>::new();

    // 处理符号
    let mut v = value;
    if v.is_nan() {
        let _ = s.push_str("NaN");
        return s;
    }
    if v.is_infinite() {
        if v.is_sign_negative() {
            let _ = s.push_str("-Inf");
        } else {
            let _ = s.push_str("Inf");
        }
        return s;
    }
    if v < 0.0 {
        let _ = s.push('-');
        v = -v;
    }

    let scale = match decimals {
        0 => 1.0,
        1 => 10.0,
        _ => 100.0,
    };
    let scaled = (v * scale + 0.5) as u32;
    let int_part = scaled / (scale as u32);
    let frac_part = scaled % (scale as u32);

    append_u32(&mut s, int_part);

    if decimals > 0 {
        let _ = s.push('.');
        let frac_digits = match decimals {
            0 => 0,
            1 => 1,
            _ => 2,
        };
        append_frac(&mut s, frac_part, frac_digits);
    }

    s
}

fn format_pair_value(value: f32, unit: char) -> String<6> {
    let digits = format_four_digits(value);
    let mut s = String::<6>::new();
    let _ = s.push_str(digits.as_str());
    let _ = s.push(unit);
    s
}

fn format_four_digits(value: f32) -> String<5> {
    // 生成恰好 4 个字符的“整数+小数”数字，不使用核心库浮点 fmt。
    let mut s = String::<5>::new();

    let mut v = value;
    if v.is_nan() {
        let _ = s.push_str("----");
        return s;
    }
    if v < 0.0 {
        v = -v;
    }

    let (scale, frac_digits) = if v >= 100.0 {
        (1.0, 0)
    } else if v >= 10.0 {
        (10.0, 1)
    } else {
        (100.0, 2)
    };

    let scaled = (v * scale + 0.5) as u32;
    let int_part = scaled / (scale as u32);
    let frac_part = scaled % (scale as u32);

    // 暂存在临时 buffer 中，再根据总宽度裁剪到 4 个字符。
    let mut tmp = String::<8>::new();
    append_u32(&mut tmp, int_part);
    if frac_digits > 0 {
        let _ = tmp.push('.');
        append_frac(&mut tmp, frac_part, frac_digits);
    }

    // 若超过 4 字符，截断右侧多余部分；不足则左侧不补空格（UI 仍使用等宽字体保障稳定）。
    if tmp.len() > 4 {
        s.push_str(&tmp.as_str()[0..4]).ok();
    } else {
        s.push_str(tmp.as_str()).ok();
    }

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
    pub run_time: String<16>,
    pub sink_core_temp: f32,
    pub sink_exhaust_temp: f32,
    pub mcu_temp: f32,
    pub energy_wh: f32,
    // Preformatted strings for on-demand, character-aware updates.
    pub main_voltage_text: String<8>,
    pub main_current_text: String<8>,
    pub main_power_text: String<8>,
    pub remote_voltage_text: String<6>,
    pub local_voltage_text: String<6>,
    pub ch1_current_text: String<6>,
    pub ch2_current_text: String<6>,
    pub status_lines: [String<20>; 5],
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
            run_time,
            sink_core_temp: 42.3,
            sink_exhaust_temp: 38.1,
            mcu_temp: 35.0,
            energy_wh: 125.4,
            main_voltage_text: String::new(),
            main_current_text: String::new(),
            main_power_text: String::new(),
            remote_voltage_text: String::new(),
            local_voltage_text: String::new(),
            ch1_current_text: String::new(),
            ch2_current_text: String::new(),
            status_lines: Default::default(),
        }
    }

    /// Recompute all preformatted strings from the current numeric snapshot.
    pub fn update_strings(&mut self) {
        self.main_voltage_text = format_value(self.main_voltage, 2);
        self.main_current_text = format_value(self.main_current, 2);
        self.main_power_text = format_value(self.main_power, 1);

        self.remote_voltage_text = format_pair_value(self.remote_voltage, 'V');
        self.local_voltage_text = format_pair_value(self.local_voltage, 'V');
        self.ch1_current_text = format_pair_value(self.ch1_current, 'A');
        self.ch2_current_text = format_pair_value(self.ch2_current, 'A');

        self.status_lines = self.compute_status_lines();
    }

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

        let mut energy = String::<20>::new();
        let _ = energy.push_str("ENERGY ");
        append_temp_1dp(&mut energy, self.energy_wh);
        let _ = energy.push_str("Wh");

        [run, core, exhaust, mcu, energy]
    }

    pub fn status_lines(&self) -> [String<20>; 5] {
        self.status_lines.clone()
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
