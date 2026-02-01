use core::sync::atomic::{AtomicU32, Ordering};

use defmt::*;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    i2s::master::{Channels, Config as I2sConfig, DataFormat, I2s},
    time::Rate,
};

#[derive(Clone, Copy, Format)]
pub enum SpeakerSound {
    UiOk,
    UiFail,
    Test,
}

static SPEAKER_QUEUE: Channel<CriticalSectionRawMutex, SpeakerSound, 4> = Channel::new();

pub static SPEAKER_ENQUEUE_DROPS: AtomicU32 = AtomicU32::new(0);
pub static SPEAKER_PLAY_TOTAL: AtomicU32 = AtomicU32::new(0);

pub fn enqueue(sound: SpeakerSound) {
    if SPEAKER_QUEUE.try_send(sound).is_err() {
        SPEAKER_ENQUEUE_DROPS.fetch_add(1, Ordering::Relaxed);
    }
}

const SAMPLE_RATE_HZ: u32 = 16_000;
const BYTES_PER_FRAME: usize = 4; // stereo, 16-bit per channel
const TONE_DURATION_MS: u32 = 90;
const TONE_FRAMES: usize = (SAMPLE_RATE_HZ as usize * TONE_DURATION_MS as usize) / 1000;
const TONE_BUF_BYTES: usize = TONE_FRAMES * BYTES_PER_FRAME;

fn fill_square_tone(buf: &mut [u8], freq_hz: u32, amp: i16) -> usize {
    let frames = (buf.len() / BYTES_PER_FRAME).min(TONE_FRAMES);
    let period = (SAMPLE_RATE_HZ / freq_hz).max(2) as usize;
    let half = (period / 2).max(1);

    for i in 0..frames {
        let sample = if (i % period) < half { amp } else { -amp };
        let le = sample.to_le_bytes();
        let off = i * BYTES_PER_FRAME;
        buf[off] = le[0];
        buf[off + 1] = le[1];
        buf[off + 2] = le[0];
        buf[off + 3] = le[1];
    }

    frames * BYTES_PER_FRAME
}

#[embassy_executor::task]
pub async fn speaker_task(
    i2s: esp_hal::peripherals::I2S0<'static>,
    dma: esp_hal::peripherals::DMA_CH2<'static>,
    amp_sd_mode: esp_hal::peripherals::GPIO34<'static>,
    bclk: esp_hal::peripherals::GPIO35<'static>,
    lrclk: esp_hal::peripherals::GPIO36<'static>,
    din: esp_hal::peripherals::GPIO37<'static>,
) {
    info!(
        "speaker: task starting (MAX98357A, AMP_SD_MODE=GPIO34, I2S_BCLK=GPIO35, I2S_LRCLK=GPIO36, I2S_DIN=GPIO37, {}Hz)",
        SAMPLE_RATE_HZ
    );

    let mut amp = Output::new(amp_sd_mode, Level::Low, OutputConfig::default());

    let (_, _, tx_buf, tx_desc) = esp_hal::dma_buffers!(0, TONE_BUF_BYTES);
    let i2s = I2s::new(
        i2s,
        dma,
        I2sConfig::new_tdm_philips()
            .with_sample_rate(Rate::from_hz(SAMPLE_RATE_HZ))
            .with_data_format(DataFormat::Data16Channel16)
            .with_channels(Channels::STEREO),
    )
    .expect("i2s init")
    .into_async();

    let mut tx = i2s
        .i2s_tx
        .with_bclk(bclk)
        .with_ws(lrclk)
        .with_dout(din)
        .build(tx_desc);

    {
        let freq = 440;
        let amp_i16 = 9_000;
        let used = fill_square_tone(tx_buf, freq, amp_i16);
        let play_id = SPEAKER_PLAY_TOTAL
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1);
        info!(
            "speaker: play #{} ({=?}, {}Hz, {}B)",
            play_id,
            SpeakerSound::Test,
            freq,
            used
        );
        let _ = amp.set_high();
        if let Err(err) = tx.write_dma_async(&mut tx_buf[..used]).await {
            warn!("speaker: write_dma_async failed: {:?}", err);
        }
        let _ = amp.set_low();
    }

    loop {
        let sound = SPEAKER_QUEUE.receive().await;
        let (freq, amp_i16) = match sound {
            SpeakerSound::UiOk => (880, 9_000),
            SpeakerSound::UiFail => (220, 10_000),
            SpeakerSound::Test => (440, 9_000),
        };

        let _ = amp.set_high();
        let used = fill_square_tone(tx_buf, freq, amp_i16);
        let play_id = SPEAKER_PLAY_TOTAL
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1);
        info!(
            "speaker: play #{} ({=?}, {}Hz, {}B)",
            play_id, sound, freq, used
        );

        if let Err(err) = tx.write_dma_async(&mut tx_buf[..used]).await {
            warn!("speaker: write_dma_async failed: {:?}", err);
        }

        let _ = amp.set_low();
    }
}
