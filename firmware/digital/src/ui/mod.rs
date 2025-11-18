mod fonts;

use embedded_graphics::pixelcolor::{
    Rgb565,
    raw::{RawData, RawU16},
};
use heapless::String;
use lcd_async::raw_framebuf::RawFrameBuf;
use micromath::F32Ext;

use crate::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

use self::fonts::{SEVEN_SEG_FONT, SMALL_FONT};

const LOGICAL_WIDTH: i32 = 320;
const LOGICAL_HEIGHT: i32 = 240;
const DEBUG_OVERLAY: bool = false;

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
    for (idx, &top) in [0, 80, 160].iter().enumerate() {
        canvas.fill_rect(Rect::new(8, top + 6, 182, top + 74), card_colors[idx]);
    }

    let voltage_text = format_value(data.main_voltage, 2);
    let current_text = format_value(data.main_current, 2);
    let power_text = format_value(data.main_power, 1);
    // Digits colors per design palette
    draw_main_metric(
        &mut canvas,
        "VOLTAGE",
        voltage_text.as_str(),
        "V",
        0,
        rgb(0xFFB347),
    );
    draw_main_metric(
        &mut canvas,
        "CURRENT",
        current_text.as_str(),
        "A",
        80,
        rgb(0xFF5252),
    );
    draw_main_metric(
        &mut canvas,
        "POWER",
        power_text.as_str(),
        "W",
        160,
        rgb(0x6EF58C),
    );

    let remote_text = format_pair_value(data.remote_voltage, 'V');
    let local_text = format_pair_value(data.local_voltage, 'V');
    draw_voltage_pair(&mut canvas, data, remote_text.as_str(), local_text.as_str());

    let ch1_text = format_pair_value(data.ch1_current, 'A');
    let ch2_text = format_pair_value(data.ch2_current, 'A');
    draw_current_pair(&mut canvas, data, ch1_text.as_str(), ch2_text.as_str());
    draw_telemetry(&mut canvas, data);

    if DEBUG_OVERLAY {
        draw_debug_overlay(&mut canvas);
    }
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
    let mut baseline = 200;
    for line in [
        data.run_time.as_str(),
        data.temperature_display().as_str(),
        data.energy_display().as_str(),
    ] {
        draw_small_text(
            canvas,
            line,
            198,
            baseline - SMALL_FONT.height() as i32,
            rgb(0xdfe7ff),
            0,
        );
        baseline += 14;
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
    let mut s = String::<8>::new();
    let _ = match decimals {
        0 => core::fmt::write(&mut s, format_args!("{value:5.0}")),
        1 => core::fmt::write(&mut s, format_args!("{value:5.1}")),
        _ => core::fmt::write(&mut s, format_args!("{value:5.2}")),
    };
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
    let mut s = String::<5>::new();
    let abs = value.abs();
    if abs >= 100.0 {
        let _ = core::fmt::write(&mut s, format_args!("{value:04.0}"));
    } else if abs >= 10.0 {
        let _ = core::fmt::write(&mut s, format_args!("{value:04.1}"));
    } else {
        let _ = core::fmt::write(&mut s, format_args!("{value:04.2}"));
    }
    if s.len() > 4 {
        s.truncate(4);
    }
    s
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

#[derive(Debug)]
pub struct UiSnapshot {
    pub main_voltage: f32,
    pub main_current: f32,
    pub main_power: f32,
    pub remote_voltage: f32,
    pub local_voltage: f32,
    pub ch1_current: f32,
    pub ch2_current: f32,
    pub run_time: String<16>,
    pub temperature_c: f32,
    pub energy_wh: f32,
}

impl UiSnapshot {
    pub const fn demo_const() -> Self {
        Self {
            main_voltage: 5.0,
            main_current: 0.0,
            main_power: 0.0,
            remote_voltage: 0.0,
            local_voltage: 0.0,
            ch1_current: 0.0,
            ch2_current: 0.0,
            run_time: String::new(),
            temperature_c: 25.0,
            energy_wh: 0.0,
        }
    }

    pub fn demo() -> Self {
        Self::demo_const()
    }

    pub fn update_from_status(
        &mut self,
        ch1_i_mA: u32,
        ch2_i_mA: u32,
        vnr_sp_mV: u32,
        vrmt_sp_mV: u32,
        v5sns_mV: u32,
    ) {
        // 电流：mA → A
        self.ch1_current = ch1_i_mA as f32 / 1000.0;
        self.ch2_current = ch2_i_mA as f32 / 1000.0;

        // 近端/远端 ADC 节点电压：mV → V（不在此处还原分压，只显示 ADC 电压）
        self.local_voltage = vnr_sp_mV as f32 / 1000.0;
        self.remote_voltage = vrmt_sp_mV as f32 / 1000.0;

        // 主电压：这里简单用 5V 监测节点推导近似总线电压（分压：75k/10k）
        let v5 = v5sns_mV as f32 * (85.0 / 10.0) / 1000.0;
        self.main_voltage = v5;
        self.main_current = self.ch1_current; // 当前只启用 CH1
        self.main_power = self.main_voltage * self.main_current;
    }

    fn temperature_display(&self) -> String<16> {
        let mut s = String::<16>::new();
        if self.temperature_c.fract().abs() < 0.05 {
            let _ = core::fmt::write(&mut s, format_args!("TEMP {:02.0}C", self.temperature_c));
        } else {
            let _ = core::fmt::write(&mut s, format_args!("TEMP {:02.1}C", self.temperature_c));
        }
        s
    }

    fn energy_display(&self) -> String<16> {
        let mut s = String::<16>::new();
        let _ = core::fmt::write(&mut s, format_args!("ENERGY {:04.1}Wh", self.energy_wh));
        s
    }
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
