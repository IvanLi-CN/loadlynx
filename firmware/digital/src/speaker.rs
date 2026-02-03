use core::sync::atomic::{AtomicU32, Ordering};

use defmt::*;
use embassy_futures::yield_now;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Instant};
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

// Proven reference implementation: `mains-aegis/firmware/src/audio_demo.rs`.
// - WAV(PCM16LE), mono, 8kHz.
// - I2S Philips-like (TDM), 16-bit samples.
// - MAX98357A is configured as "Left" via SD_MODE=HIGH; we use `Channels::MONO`
//   which duplicates the same DMA data to both L/R in hardware (see esp-hal `tx_dma_equal`).
const SAMPLE_RATE_HZ: u32 = 8_000;
const BYTES_PER_SAMPLE: usize = 2; // s16le mono

// Digital gain (Q8): 256 = 1.0x, 512 = 2.0x (+6dB).
// Current assets peak at about -7 dBFS, so 2.0x stays below clipping.
const PCM_GAIN_Q8: i32 = 512;

// Use circular DMA to avoid I2S stop/start glitches ("pop"/"click") between DMA blocks.
// Keep the ring moderate: large rings can overflow DRAM for the S3 build.
// We rely on tight refill (`yield_now` instead of millisecond sleeps) to avoid underflow.
const TX_RING_BYTES: usize = 12 * 4092;

// Keep a small post-playback margin so we stop on silence even if
// the actual DMA start time differs slightly from our timestamp.
const STOP_MARGIN_MS: u64 = 20;

const SPEAKER_WAV_UI_OK: &[u8] = include_bytes!("../assets/audio/speaker-ui-ok-8k.wav");
const SPEAKER_WAV_UI_FAIL: &[u8] = include_bytes!("../assets/audio/speaker-ui-fail-8k.wav");
const SPEAKER_WAV_DIAG_440: &[u8] = include_bytes!("../assets/audio/speaker-diag-440-8k.wav");
const SPEAKER_WAV_DIAG_554: &[u8] = include_bytes!("../assets/audio/speaker-diag-554-8k.wav");
const SPEAKER_WAV_DIAG_659: &[u8] = include_bytes!("../assets/audio/speaker-diag-659-8k.wav");
const SPEAKER_WAV_DIAG_880: &[u8] = include_bytes!("../assets/audio/speaker-diag-880-8k.wav");
const SPEAKER_WAV_TEST: &[u8] = include_bytes!("../assets/audio/speaker-test-8k.wav");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Format)]
enum WavError {
    Invalid,
    Unsupported,
}

#[derive(Clone, Copy)]
struct WavView<'a> {
    data: &'a [u8],
}

fn parse_wav_pcm16le_mono_8khz(bytes: &[u8]) -> Result<WavView<'_>, WavError> {
    if bytes.len() < 44 {
        return Err(WavError::Invalid);
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(WavError::Invalid);
    }

    let mut fmt: Option<(u16, u16, u32, u16)> = None; // (audio_format, channels, sample_rate, bits_per_sample)
    let mut data: Option<&[u8]> = None;

    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        offset += 8;

        if offset + size > bytes.len() {
            return Err(WavError::Invalid);
        }

        match id {
            b"fmt " => {
                if size < 16 {
                    return Err(WavError::Invalid);
                }
                let audio_format = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                let channels = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
                let sample_rate = u32::from_le_bytes([
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ]);
                let bits_per_sample = u16::from_le_bytes([bytes[offset + 14], bytes[offset + 15]]);
                fmt = Some((audio_format, channels, sample_rate, bits_per_sample));
            }
            b"data" => data = Some(&bytes[offset..offset + size]),
            _ => {}
        }

        // Chunks are padded to 16-bit alignment.
        offset += size + (size % 2);
        if fmt.is_some() && data.is_some() {
            break;
        }
    }

    let (audio_format, channels, sample_rate, bits_per_sample) = fmt.ok_or(WavError::Invalid)?;
    let data = data.ok_or(WavError::Invalid)?;

    // PCM = 1.
    if audio_format != 1 {
        return Err(WavError::Unsupported);
    }
    if channels != 1 {
        return Err(WavError::Unsupported);
    }
    if sample_rate != SAMPLE_RATE_HZ {
        return Err(WavError::Unsupported);
    }
    if bits_per_sample != 16 {
        return Err(WavError::Unsupported);
    }
    if (data.len() % 2) != 0 {
        return Err(WavError::Invalid);
    }

    Ok(WavView { data })
}

#[derive(Clone, Copy)]
enum StreamOp {
    Silence {
        remaining_bytes: usize,
    },
    Audio {
        label: &'static str,
        pcm_mono_s16le: &'static [u8],
        offset_bytes: usize,
    },
}

fn ms_to_samples(ms: u32) -> usize {
    ((SAMPLE_RATE_HZ as u64) * (ms as u64) / 1000) as usize
}

#[derive(Clone, Copy)]
struct StreamState {
    idx: usize,
    cur: Option<StreamOp>,
    done: bool,
}

fn ensure_next_op(assets: &AudioAssets, kind: PlaylistKind, state: &mut StreamState) {
    if state.done || state.cur.is_some() {
        return;
    }

    // Note: we must never leave `state.cur == None` when more playlist items exist,
    // because `fill_stream_bytes()` treats `None` as "fill rest with silence and return".
    loop {
        match assets.item(kind, state.idx) {
            Some(PlaylistItem::SilenceMs(ms)) => {
                state.idx = state.idx.saturating_add(1);
                let samples = ms_to_samples(ms);
                // Keep DMA writes 32-bit aligned.
                let bytes = (samples * BYTES_PER_SAMPLE + 3) & !0x3;
                if bytes == 0 {
                    continue;
                }
                state.cur = Some(StreamOp::Silence {
                    remaining_bytes: bytes,
                });
                break;
            }
            Some(PlaylistItem::Audio { label, pcm }) => {
                state.idx = state.idx.saturating_add(1);
                if pcm.is_empty() {
                    warn!("speaker: playlist item {} empty; skipping", label);
                    continue;
                }
                info!("speaker: playlist item {} ({=usize}B)", label, pcm.len());
                state.cur = Some(StreamOp::Audio {
                    label,
                    pcm_mono_s16le: pcm,
                    offset_bytes: 0,
                });
                break;
            }
            None => {
                state.done = true;
                break;
            }
        }
    }
}

fn fill_stream_bytes(
    assets: &AudioAssets,
    kind: PlaylistKind,
    state: &mut StreamState,
    out_buf: &mut [u8],
) -> usize {
    let want = out_buf.len() & !0x3;
    if want == 0 {
        return 0;
    }

    let mut out = 0usize;
    while out < want {
        ensure_next_op(assets, kind, state);

        let Some(op) = state.cur else {
            out_buf[out..want].fill(0);
            out = want;
            break;
        };

        match op {
            StreamOp::Silence {
                mut remaining_bytes,
            } => {
                if remaining_bytes == 0 {
                    state.cur = None;
                    continue;
                }
                let mut take = core::cmp::min(want - out, remaining_bytes);
                take &= !0x3;
                let take_bytes = take;
                if take_bytes == 0 {
                    // Avoid returning 0 bytes to DMA (can corrupt descriptor ownership state).
                    state.cur = None;
                    continue;
                }

                out_buf[out..out + take_bytes].fill(0);
                out += take_bytes;
                remaining_bytes = remaining_bytes.saturating_sub(take_bytes);
                if remaining_bytes == 0 {
                    state.cur = None;
                } else {
                    state.cur = Some(StreamOp::Silence { remaining_bytes });
                }
            }
            StreamOp::Audio {
                label,
                pcm_mono_s16le,
                mut offset_bytes,
            } => {
                let remaining_bytes = pcm_mono_s16le.len().saturating_sub(offset_bytes);
                if remaining_bytes == 0 {
                    state.cur = None;
                    continue;
                }
                let mut take = core::cmp::min(want - out, remaining_bytes);
                if take < 4 && remaining_bytes != 0 {
                    // Pad the final odd bytes (rare) to keep DMA writes aligned.
                    let mut tmp = [0u8; 4];
                    let copy = remaining_bytes.min(4);
                    tmp[..copy].copy_from_slice(&pcm_mono_s16le[offset_bytes..offset_bytes + copy]);
                    // Apply gain to the final (possibly single) i16 sample.
                    if copy >= 2 {
                        let s = i16::from_le_bytes([tmp[0], tmp[1]]) as i32;
                        let s = (s * PCM_GAIN_Q8) >> 8;
                        let s = s.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                        let [a, b] = s.to_le_bytes();
                        tmp[0] = a;
                        tmp[1] = b;
                    }
                    if copy >= 4 {
                        let s = i16::from_le_bytes([tmp[2], tmp[3]]) as i32;
                        let s = (s * PCM_GAIN_Q8) >> 8;
                        let s = s.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                        let [a, b] = s.to_le_bytes();
                        tmp[2] = a;
                        tmp[3] = b;
                    }
                    out_buf[out..out + 4].copy_from_slice(&tmp);
                    out += 4;
                    offset_bytes = offset_bytes.saturating_add(copy);
                    state.cur = None;
                    continue;
                }
                take &= !0x3;
                if take == 0 {
                    // Avoid returning 0 bytes to DMA (can corrupt descriptor ownership state).
                    state.cur = None;
                    continue;
                }

                // Copy + apply gain.
                let src = &pcm_mono_s16le[offset_bytes..offset_bytes + take];
                let dst = &mut out_buf[out..out + take];
                for (d, s) in dst.chunks_exact_mut(2).zip(src.chunks_exact(2)) {
                    let sample = i16::from_le_bytes([s[0], s[1]]) as i32;
                    let scaled = (sample * PCM_GAIN_Q8) >> 8;
                    let scaled = scaled.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                    let [a, b] = scaled.to_le_bytes();
                    d[0] = a;
                    d[1] = b;
                }

                out += take;
                offset_bytes = offset_bytes.saturating_add(take);

                if offset_bytes >= pcm_mono_s16le.len() {
                    let _ = label; // keep for debugging context if needed
                    state.cur = None;
                } else {
                    state.cur = Some(StreamOp::Audio {
                        label,
                        pcm_mono_s16le,
                        offset_bytes,
                    });
                }
            }
        }
    }

    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaylistKind {
    Boot,
    UiOk,
    UiFail,
    Test,
}

#[derive(Clone, Copy)]
enum PlaylistItem {
    SilenceMs(u32),
    Audio {
        label: &'static str,
        pcm: &'static [u8],
    },
}

#[derive(Clone, Copy)]
struct AudioAssets {
    diag_440: &'static [u8],
    diag_554: &'static [u8],
    diag_659: &'static [u8],
    diag_880: &'static [u8],
    ui_ok: &'static [u8],
    ui_fail: &'static [u8],
    test: &'static [u8],
}

impl AudioAssets {
    fn load() -> Self {
        fn load_one(name: &'static str, wav: &'static [u8]) -> &'static [u8] {
            match parse_wav_pcm16le_mono_8khz(wav) {
                Ok(view) => view.data,
                Err(err) => {
                    warn!("speaker: wav parse failed (name={}, err={:?})", name, err);
                    &[]
                }
            }
        }

        Self {
            diag_440: load_one("diag_440", SPEAKER_WAV_DIAG_440),
            diag_554: load_one("diag_554", SPEAKER_WAV_DIAG_554),
            diag_659: load_one("diag_659", SPEAKER_WAV_DIAG_659),
            diag_880: load_one("diag_880", SPEAKER_WAV_DIAG_880),
            ui_ok: load_one("ui_ok", SPEAKER_WAV_UI_OK),
            ui_fail: load_one("ui_fail", SPEAKER_WAV_UI_FAIL),
            test: load_one("test", SPEAKER_WAV_TEST),
        }
    }

    fn item(&self, kind: PlaylistKind, idx: usize) -> Option<PlaylistItem> {
        match kind {
            PlaylistKind::Boot => match idx {
                0 => Some(PlaylistItem::SilenceMs(60)),
                1 => Some(PlaylistItem::Audio {
                    label: "boot diag 440Hz",
                    pcm: self.diag_440,
                }),
                2 => Some(PlaylistItem::SilenceMs(2000)),
                3 => Some(PlaylistItem::Audio {
                    label: "boot diag 554Hz",
                    pcm: self.diag_554,
                }),
                4 => Some(PlaylistItem::SilenceMs(2000)),
                5 => Some(PlaylistItem::Audio {
                    label: "boot diag 659Hz",
                    pcm: self.diag_659,
                }),
                6 => Some(PlaylistItem::SilenceMs(2000)),
                7 => Some(PlaylistItem::Audio {
                    label: "boot diag 880Hz",
                    pcm: self.diag_880,
                }),
                8 => Some(PlaylistItem::SilenceMs(2000)),
                9 => Some(PlaylistItem::Audio {
                    label: "boot test melody",
                    pcm: self.test,
                }),
                10 => Some(PlaylistItem::SilenceMs(120)),
                _ => None,
            },
            PlaylistKind::UiOk => match idx {
                0 => Some(PlaylistItem::SilenceMs(20)),
                1 => Some(PlaylistItem::Audio {
                    label: "ui ok",
                    pcm: self.ui_ok,
                }),
                2 => Some(PlaylistItem::SilenceMs(40)),
                _ => None,
            },
            PlaylistKind::UiFail => match idx {
                0 => Some(PlaylistItem::SilenceMs(20)),
                1 => Some(PlaylistItem::Audio {
                    label: "ui fail",
                    pcm: self.ui_fail,
                }),
                2 => Some(PlaylistItem::SilenceMs(60)),
                _ => None,
            },
            PlaylistKind::Test => match idx {
                0 => Some(PlaylistItem::SilenceMs(40)),
                1 => Some(PlaylistItem::Audio {
                    label: "test melody",
                    pcm: self.test,
                }),
                2 => Some(PlaylistItem::SilenceMs(120)),
                _ => None,
            },
        }
    }
}

fn bytes_per_second() -> u64 {
    core::cmp::max(1u64, (SAMPLE_RATE_HZ as u64) * (BYTES_PER_SAMPLE as u64))
}

fn duration_ms_for_playlist(assets: &AudioAssets, kind: PlaylistKind) -> u64 {
    let mut total_bytes: u64 = 0;
    let mut idx = 0usize;
    loop {
        let Some(item) = assets.item(kind, idx) else {
            break;
        };
        match item {
            PlaylistItem::SilenceMs(ms) => {
                let samples = ms_to_samples(ms);
                // Keep DMA writes 32-bit aligned.
                let bytes = (samples * BYTES_PER_SAMPLE + 3) & !0x3;
                total_bytes = total_bytes.saturating_add(bytes as u64);
            }
            PlaylistItem::Audio { pcm, .. } => {
                // Align the final block just like `fill_stream_bytes()` does.
                let bytes = (pcm.len() + 3) & !0x3;
                total_bytes = total_bytes.saturating_add(bytes as u64);
            }
        }
        idx = idx.saturating_add(1);
    }

    let bps = bytes_per_second();
    (total_bytes
        .saturating_mul(1000)
        .saturating_add(bps.saturating_sub(1)))
        / bps
}

async fn play_playlist(
    tx: &mut esp_hal::i2s::master::I2sTx<'_, esp_hal::Blocking>,
    tx_ring: &mut [u8],
    assets: &AudioAssets,
    kind: PlaylistKind,
) {
    let play_ms = duration_ms_for_playlist(assets, kind);

    // Pre-fill the ring with the first chunk of the stream so playback starts immediately.
    let mut state = StreamState {
        idx: 0,
        cur: None,
        done: false,
    };
    let _ = fill_stream_bytes(assets, kind, &mut state, tx_ring);

    // Circular DMA transfer (stable, avoids buffer-boundary clicks).
    let mut transfer = match tx.write_dma_circular(&tx_ring) {
        Ok(t) => t,
        Err(err) => {
            warn!("speaker: start dma_circular failed ({=?})", err);
            return;
        }
    };
    let started_at = Instant::now();
    let deadline = started_at + Duration::from_millis(play_ms.saturating_add(STOP_MARGIN_MS));

    // Keep pushing until all playlist items are queued.
    while !state.done {
        // Poll for space to push.
        let avail = match transfer.available() {
            Ok(v) => v,
            Err(err) => {
                warn!("speaker: dma available failed ({=?})", err);
                break;
            }
        };

        if avail == 0 {
            yield_now().await;
            continue;
        }

        let wrote = match transfer.push_with(|buf| fill_stream_bytes(assets, kind, &mut state, buf))
        {
            Ok(n) => n,
            Err(err) => {
                warn!("speaker: dma push_with failed ({=?})", err);
                break;
            }
        };

        if wrote == 0 {
            yield_now().await;
        }
    }

    // Tail: keep the DMA fed with silence until the playlist duration elapses.
    // This prevents `Late` while avoiding multi-second UI sound latency.
    while Instant::now() < deadline {
        let wrote = match transfer.push_with(|buf| {
            let want = buf.len() & !0x3;
            if want == 0 {
                return 0;
            }
            buf[..want].fill(0);
            want
        }) {
            Ok(n) => n,
            Err(err) => {
                warn!("speaker: dma push_with failed ({=?})", err);
                break;
            }
        };

        if wrote == 0 {
            yield_now().await;
        }
    }

    if let Err(err) = transfer.stop() {
        warn!("speaker: dma stop failed ({=?})", err);
    }
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
        "speaker: task starting (MAX98357A, AMP_SD_MODE=GPIO34, I2S_BCLK=GPIO35, I2S_LRCLK=GPIO36, I2S_DIN=GPIO37, {}Hz, wav=pcm16le/mono)",
        SAMPLE_RATE_HZ
    );

    let mut amp = Output::new(amp_sd_mode, Level::Low, OutputConfig::default());
    let assets = AudioAssets::load();

    let (_, _, tx_ring, tx_desc) = esp_hal::dma_circular_buffers!(0, TX_RING_BYTES);
    let i2s = I2s::new(
        i2s,
        dma,
        I2sConfig::new_tdm_philips()
            .with_sample_rate(Rate::from_hz(SAMPLE_RATE_HZ))
            .with_data_format(DataFormat::Data16Channel16)
            .with_channels(Channels::MONO),
    )
    .expect("i2s init");

    let mut tx = i2s
        .i2s_tx
        .with_bclk(bclk)
        .with_ws(lrclk)
        .with_dout(din)
        .build(tx_desc);

    // Keep SD_MODE asserted (HIGH = "Left") and send silence when needed.
    let _ = amp.set_high();

    // Auto-run the boot self-test playlist once.
    let boot_play_id = SPEAKER_PLAY_TOTAL
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1);
    info!("speaker: boot selftest start #{} (playlist)", boot_play_id);
    play_playlist(&mut tx, tx_ring, &assets, PlaylistKind::Boot).await;

    loop {
        let sound = SPEAKER_QUEUE.receive().await;
        let play_id = SPEAKER_PLAY_TOTAL
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1);
        info!("speaker: play #{} ({=?})", play_id, sound);

        let kind = match sound {
            SpeakerSound::UiOk => PlaylistKind::UiOk,
            SpeakerSound::UiFail => PlaylistKind::UiFail,
            SpeakerSound::Test => PlaylistKind::Test,
        };
        play_playlist(&mut tx, tx_ring, &assets, kind).await;
    }
}
