#![allow(dead_code)]

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::RgbColor as _;
use lcd_async::raw_framebuf::RawFrameBuf;

use crate::speaker::{self, SpeakerSound};
use crate::{DISPLAY_HEIGHT as PHYS_HEIGHT, DISPLAY_WIDTH as PHYS_WIDTH};

use super::fonts::SMALL_FONT;
use super::{Canvas, Rect, rgb};

const TOP_BAR_H: i32 = 24;
const PAD_X: i32 = 12;
const LIST_TOP: i32 = 34;
const ROW_H: i32 = 22;
const ROW_GAP: i32 = 2;
const ROW_RADIUS: i32 = 6;
const COL_GAP: i32 = 8;

const COLOR_TOP_BG: u32 = 0x1c2638;
const COLOR_BG: u32 = 0x0b111e;
const COLOR_ROW_0: u32 = 0x101829;
const COLOR_ROW_1: u32 = 0x0e1526;
const COLOR_ROW_BORDER: u32 = 0x2a3a4f;
const COLOR_TEXT: Rgb565 = Rgb565::WHITE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioMenuHit {
    Back,
    Item(usize),
}

fn draw_text(canvas: &mut Canvas<'_>, x: i32, y: i32, text: &str, color: Rgb565) {
    let mut cx = x;
    let cw = SMALL_FONT.width() as i32;
    for ch in text.chars() {
        SMALL_FONT.draw_char(ch, |px, py| canvas.set_pixel(px, py, color), cx, y);
        cx += cw;
    }
}

fn hit_in_rect(x: i32, y: i32, rect: Rect) -> bool {
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
}

fn list_columns() -> (i32, i32, i32, i32) {
    let list_left = PAD_X;
    let list_right = super::LOGICAL_WIDTH - PAD_X;
    let total_w = list_right - list_left;
    // Keep the right column slightly wider if rounding forces it.
    let col_w = (total_w - COL_GAP) / 2;
    let col0_left = list_left;
    let col0_right = col0_left + col_w;
    let col1_left = col0_right + COL_GAP;
    let col1_right = list_right;
    (col0_left, col0_right, col1_left, col1_right)
}

pub fn render_audio_menu(frame: &mut RawFrameBuf<Rgb565, &mut [u8]>) {
    let bytes = frame.as_mut_bytes();
    let mut canvas = Canvas::new(bytes, PHYS_WIDTH, PHYS_HEIGHT);

    // Background.
    canvas.fill_rect(
        Rect::new(0, 0, super::LOGICAL_WIDTH, super::LOGICAL_HEIGHT),
        rgb(COLOR_BG),
    );

    // Top bar.
    canvas.fill_rect(
        Rect::new(0, 0, super::LOGICAL_WIDTH, TOP_BAR_H),
        rgb(COLOR_TOP_BG),
    );
    draw_text(&mut canvas, PAD_X, 7, "Back", COLOR_TEXT);
    draw_text(
        &mut canvas,
        super::LOGICAL_WIDTH - 12 - 5 * (SMALL_FONT.width() as i32),
        7,
        "Audio",
        COLOR_TEXT,
    );

    // Simple list: fixed items, no scrolling.
    let items: &[SpeakerSound] = speaker::AUDIO_MENU_SOUNDS;
    let (col0_left, col0_right, col1_left, col1_right) = list_columns();

    for (idx, sound) in items.iter().copied().enumerate() {
        let row = idx / 2;
        let col = idx & 1;
        let (left, right) = if col == 0 {
            (col0_left, col0_right)
        } else {
            (col1_left, col1_right)
        };

        let row_top = LIST_TOP + row as i32 * (ROW_H + ROW_GAP);
        let row_bottom = row_top + ROW_H;
        if row_bottom > super::LOGICAL_HEIGHT {
            break;
        }

        let bg = if (idx & 1) == 0 {
            COLOR_ROW_0
        } else {
            COLOR_ROW_1
        };
        let row = Rect::new(left, row_top, right, row_bottom);
        canvas.fill_round_rect(row, ROW_RADIUS, rgb(bg));

        // Border (1px).
        canvas.fill_round_rect(
            Rect::new(row.left, row.top, row.right, row.top + 1),
            ROW_RADIUS,
            rgb(COLOR_ROW_BORDER),
        );
        canvas.fill_round_rect(
            Rect::new(row.left, row.bottom - 1, row.right, row.bottom),
            ROW_RADIUS,
            rgb(COLOR_ROW_BORDER),
        );

        let label = speaker::sound_label(sound);
        draw_text(&mut canvas, row.left + 8, row.top + 6, label, COLOR_TEXT);
    }

    // Footer hint.
    draw_text(
        &mut canvas,
        PAD_X,
        super::LOGICAL_HEIGHT - 14,
        "Tap: play. Hold POWER: exit.",
        COLOR_TEXT,
    );
}

pub fn hit_test_audio_menu(x: i32, y: i32) -> Option<AudioMenuHit> {
    // Back region (top-left).
    let back_rect = Rect::new(0, 0, 80, TOP_BAR_H);
    if hit_in_rect(x, y, back_rect) {
        return Some(AudioMenuHit::Back);
    }

    if y < LIST_TOP {
        return None;
    }

    let stride = ROW_H + ROW_GAP;
    let row = ((y - LIST_TOP) / stride) as usize;
    let row_top = LIST_TOP + row as i32 * stride;
    if y >= row_top + ROW_H {
        return None;
    }

    let (col0_left, col0_right, col1_left, col1_right) = list_columns();
    let col = if x >= col0_left && x < col0_right {
        0usize
    } else if x >= col1_left && x < col1_right {
        1usize
    } else {
        return None;
    };

    let idx = row * 2 + col;
    if idx >= speaker::AUDIO_MENU_SOUNDS.len() {
        return None;
    }
    Some(AudioMenuHit::Item(idx))
}
