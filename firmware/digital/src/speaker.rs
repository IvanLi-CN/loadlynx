use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use defmt::*;
use embassy_futures::yield_now;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::Timer;
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    i2s::master::{Channels, Config as I2sConfig, DataFormat, I2s},
    time::Rate,
};

#[derive(Clone, Copy, Format)]
pub enum SpeakerSound {
    BootChirp,
    AlarmPrimary,
    AlarmSecondary,
    AlarmTrip,
    UiOk,
    LoadOnOk,
    UiOkOff,
    UiFail,
    UiWarn,
    UiTouch,
    UiTick,
    Test,
}

static SPEAKER_QUEUE: Channel<CriticalSectionRawMutex, SpeakerSound, 4> = Channel::new();

pub static SPEAKER_ENQUEUE_DROPS: AtomicU32 = AtomicU32::new(0);
pub static SPEAKER_PLAY_TOTAL: AtomicU32 = AtomicU32::new(0);

static PROMPT_TONE_DUTY_PCT: AtomicU32 = AtomicU32::new(0);
static PROMPT_TONE_ALARM_ACTIVE: AtomicBool = AtomicBool::new(false);
static SPEAKER_HARD_MUTE: AtomicBool = AtomicBool::new(false);

pub fn enqueue(sound: SpeakerSound) {
    // Continuous alarms suppress voice output. Drop queued clips while the alarm is active
    // to avoid a delayed burst after the alarm clears.
    if PROMPT_TONE_ALARM_ACTIVE.load(Ordering::Relaxed) || SPEAKER_HARD_MUTE.load(Ordering::Relaxed)
    {
        SPEAKER_ENQUEUE_DROPS.fetch_add(1, Ordering::Relaxed);
        return;
    }
    if SPEAKER_QUEUE.try_send(sound).is_err() {
        SPEAKER_ENQUEUE_DROPS.fetch_add(1, Ordering::Relaxed);
    }
}

/// The fixed list of audio clips exposed in the on-device audio menu.
///
/// Keep the list small and the clips short: this is intended for HIL diagnostics.
pub const AUDIO_MENU_SOUNDS: &[SpeakerSound] = &[
    SpeakerSound::BootChirp,
    SpeakerSound::AlarmPrimary,
    SpeakerSound::AlarmSecondary,
    SpeakerSound::AlarmTrip,
    SpeakerSound::UiOk,
    SpeakerSound::LoadOnOk,
    SpeakerSound::UiOkOff,
    SpeakerSound::UiFail,
    SpeakerSound::UiWarn,
    SpeakerSound::UiTouch,
    SpeakerSound::UiTick,
    SpeakerSound::Test,
];

pub fn sound_label(sound: SpeakerSound) -> &'static str {
    match sound {
        SpeakerSound::BootChirp => "Boot",
        SpeakerSound::AlarmPrimary => "FATAL Alarm",
        SpeakerSound::AlarmSecondary => "LINK Alarm",
        SpeakerSound::AlarmTrip => "TRIP Alarm",
        SpeakerSound::UiOk => "UI OK",
        SpeakerSound::LoadOnOk => "LOAD On OK",
        SpeakerSound::UiOkOff => "LOAD Off OK",
        SpeakerSound::UiFail => "UI Fail",
        SpeakerSound::UiWarn => "Warning",
        SpeakerSound::UiTouch => "UI Touch",
        SpeakerSound::UiTick => "UI Tick",
        SpeakerSound::Test => "Test Melody",
    }
}

/// Set the currently requested prompt tone "loudness" as a duty-like percent.
///
/// This is driven by `prompt_tone` and rendered via the MAX98357A/I2S speaker.
pub fn set_prompt_tone_duty_pct(duty_pct: u8) {
    PROMPT_TONE_DUTY_PCT.store(duty_pct as u32, Ordering::Relaxed);
}

/// When `active`, the speaker output must suppress other sounds (voice clips, etc.).
///
/// Note: `prompt_tone` keeps its own policy; this is only the audio backend.
pub fn set_prompt_tone_alarm_active(active: bool) {
    PROMPT_TONE_ALARM_ACTIVE.store(active, Ordering::Relaxed);
}

/// Force the amplifier into a hard mute/shutdown state.
///
/// This is a safety valve to ensure we don't get stuck in an audible alarm.
pub fn set_hard_mute(mute: bool) {
    SPEAKER_HARD_MUTE.store(mute, Ordering::Relaxed);
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

// Use circular DMA to avoid I2S stop/start glitches ("pop"/"click").
//
// IMPORTANT: prompt-tone ticks are short (on the order of 10–20ms).
// With circular DMA, audio is generated *ahead* of playback: a too-large ring
// makes short pulses feel "randomly missing" because they can start/end between
// two buffer refills. Keep the ring small to keep latency low.
//
// Note: TX_CHUNK_BYTES controls both:
// - descriptor size (DMA interrupt cadence), and
// - the time resolution at which prompt-tone state changes can affect output.
// NOTE: The ring must be large enough to survive worst-case scheduling hiccups,
// otherwise the MAX98357A output devolves into harsh distortion/noise.
// Keep it modest to avoid excessive UI latency.
const TX_RING_BYTES: usize = 2048; // 128ms @ 8kHz mono PCM16LE
const TX_CHUNK_BYTES: usize = 256; // 16ms chunks; must be < TX_RING_BYTES/2 for esp-hal circular DMA

// Prompt tone synthesis:
// - Keep it intentionally simple (square wave) for low CPU.
// - The duty_pct values in `prompt_tone` were originally PWM duty; here we map
//   them into a conservative PCM amplitude scale.
const PROMPT_TONE_FREQ_HZ: u32 = 2_200;
const PROMPT_TONE_REF_DUTY_PCT: i32 = 6; // 6% ~ "full" for alarms today
const PROMPT_TONE_MAX_AMP: i32 = 12_000; // < 32767, keeps headroom for mixing
const PROMPT_TONE_RAMP_MS: u32 = 2; // reduce clicks on on/off edges

const SPEAKER_WAV_BOOT_CHIRP: &[u8] = include_bytes!("../assets/audio/speaker-boot-chirp-8k.wav");
const SPEAKER_WAV_ALARM_PRIMARY: &[u8] =
    include_bytes!("../assets/audio/speaker-alarm-primary-8k.wav");
const SPEAKER_WAV_ALARM_SECONDARY: &[u8] =
    include_bytes!("../assets/audio/speaker-alarm-secondary-8k.wav");
const SPEAKER_WAV_ALARM_TRIP: &[u8] = include_bytes!("../assets/audio/speaker-alarm-trip-8k.wav");
const SPEAKER_WAV_UI_OK: &[u8] = include_bytes!("../assets/audio/speaker-ui-ok-new-8k.wav");
const SPEAKER_WAV_LOAD_ON_OK: &[u8] = include_bytes!("../assets/audio/speaker-ui-ok-8k.wav");
const SPEAKER_WAV_UI_OK_OFF: &[u8] = include_bytes!("../assets/audio/speaker-ui-ok-off-8k.wav");
const SPEAKER_WAV_UI_FAIL: &[u8] = include_bytes!("../assets/audio/speaker-ui-fail-8k.wav");
const SPEAKER_WAV_UI_WARN: &[u8] = include_bytes!("../assets/audio/speaker-ui-warn-8k.wav");
const SPEAKER_WAV_UI_TOUCH: &[u8] = include_bytes!("../assets/audio/speaker-ui-touch-8k.wav");
const SPEAKER_WAV_UI_TICK: &[u8] = include_bytes!("../assets/audio/speaker-ui-tick-8k.wav");
const SPEAKER_WAV_DIAG_440: &[u8] = include_bytes!("../assets/audio/speaker-diag-440-8k.wav");
const SPEAKER_WAV_DIAG_554: &[u8] = include_bytes!("../assets/audio/speaker-diag-554-8k.wav");
const SPEAKER_WAV_DIAG_659: &[u8] = include_bytes!("../assets/audio/speaker-diag-659-8k.wav");
const SPEAKER_WAV_DIAG_880: &[u8] = include_bytes!("../assets/audio/speaker-diag-880-8k.wav");
const SPEAKER_WAV_TEST: &[u8] = include_bytes!("../assets/audio/speaker-test-8k.wav");

// Boot audio policy:
// - The legacy long self-test playlist (Plan #0021) is disabled by default because it's noisy and
//   can be perceived as "stuttering" under heavy load.
// - We keep a short "boot chirp" so the user can confirm the speaker path is alive.
const ENABLE_BOOT_SELFTEST: bool = false;
const ENABLE_BOOT_CHIRP: bool = true;

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

fn prompt_tone_target_amp(duty_pct: u8) -> i32 {
    let duty = (duty_pct as i32).clamp(0, 100);
    if duty == 0 {
        return 0;
    }

    // Map the small duty values used by prompt_tone (3/6%) into a sensible
    // PCM amplitude range. Clamp to keep it safe if higher duty is introduced.
    let scaled = duty.saturating_mul(PROMPT_TONE_MAX_AMP) / PROMPT_TONE_REF_DUTY_PCT.max(1);
    scaled.clamp(0, PROMPT_TONE_MAX_AMP)
}

#[derive(Clone, Copy)]
struct ToneSynth {
    phase: u32, // 0..SAMPLE_RATE_HZ-1
    amp_cur: i32,
}

impl ToneSynth {
    fn new() -> Self {
        Self {
            phase: 0,
            amp_cur: 0,
        }
    }

    fn next_sample(&mut self, target_amp: i32) -> i16 {
        let target_amp = target_amp.clamp(0, PROMPT_TONE_MAX_AMP);

        // Short linear ramp to reduce clicks when toggling tone/silence.
        let ramp_samples = core::cmp::max(
            1u32,
            ((SAMPLE_RATE_HZ as u64) * (PROMPT_TONE_RAMP_MS as u64) / 1000) as u32,
        ) as i32;
        let step = core::cmp::max(1, PROMPT_TONE_MAX_AMP / ramp_samples);

        if self.amp_cur < target_amp {
            self.amp_cur = (self.amp_cur + step).min(target_amp);
        } else if self.amp_cur > target_amp {
            self.amp_cur = (self.amp_cur - step).max(target_amp);
        }

        // Square wave via phase accumulator.
        let sign = if self.phase < (SAMPLE_RATE_HZ / 2) {
            1
        } else {
            -1
        };
        let out = sign * self.amp_cur;
        self.phase = self.phase.wrapping_add(PROMPT_TONE_FREQ_HZ);
        if self.phase >= SAMPLE_RATE_HZ {
            self.phase = self.phase.wrapping_sub(SAMPLE_RATE_HZ);
        }

        out.clamp(i16::MIN as i32, i16::MAX as i32) as i16
    }
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
                // High-frequency UI sounds (encoder ticks) should not spam logs; it can
                // amplify stutter by spending too much time in the logger path.
                debug!("speaker: playlist item {} ({=usize}B)", label, pcm.len());
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

#[derive(Clone, Copy)]
struct VoicePlayer {
    kind: PlaylistKind,
    state: StreamState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaylistKind {
    BootChirp,
    Boot,
    AlarmPrimary,
    AlarmSecondary,
    AlarmTrip,
    UiOk,
    LoadOnOk,
    UiOkOff,
    UiFail,
    UiWarn,
    UiTouch,
    UiTick,
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
    boot_chirp: &'static [u8],
    alarm_primary: &'static [u8],
    alarm_secondary: &'static [u8],
    alarm_trip: &'static [u8],
    ui_ok: &'static [u8],
    load_on_ok: &'static [u8],
    ui_ok_off: &'static [u8],
    ui_fail: &'static [u8],
    ui_warn: &'static [u8],
    ui_touch: &'static [u8],
    ui_tick: &'static [u8],
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
            boot_chirp: load_one("boot_chirp", SPEAKER_WAV_BOOT_CHIRP),
            alarm_primary: load_one("alarm_primary", SPEAKER_WAV_ALARM_PRIMARY),
            alarm_secondary: load_one("alarm_secondary", SPEAKER_WAV_ALARM_SECONDARY),
            alarm_trip: load_one("alarm_trip", SPEAKER_WAV_ALARM_TRIP),
            ui_ok: load_one("ui_ok", SPEAKER_WAV_UI_OK),
            load_on_ok: load_one("load_on_ok", SPEAKER_WAV_LOAD_ON_OK),
            ui_ok_off: load_one("ui_ok_off", SPEAKER_WAV_UI_OK_OFF),
            ui_fail: load_one("ui_fail", SPEAKER_WAV_UI_FAIL),
            ui_warn: load_one("ui_warn", SPEAKER_WAV_UI_WARN),
            ui_touch: load_one("ui_touch", SPEAKER_WAV_UI_TOUCH),
            ui_tick: load_one("ui_tick", SPEAKER_WAV_UI_TICK),
            test: load_one("test", SPEAKER_WAV_TEST),
        }
    }

    fn item(&self, kind: PlaylistKind, idx: usize) -> Option<PlaylistItem> {
        match kind {
            PlaylistKind::BootChirp => match idx {
                0 => Some(PlaylistItem::SilenceMs(80)),
                1 => Some(PlaylistItem::Audio {
                    label: "boot chirp",
                    pcm: self.boot_chirp,
                }),
                2 => Some(PlaylistItem::SilenceMs(120)),
                _ => None,
            },
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
            PlaylistKind::AlarmPrimary => match idx {
                0 => Some(PlaylistItem::Audio {
                    label: "alarm primary",
                    pcm: self.alarm_primary,
                }),
                1 => Some(PlaylistItem::SilenceMs(40)),
                _ => None,
            },
            PlaylistKind::AlarmSecondary => match idx {
                0 => Some(PlaylistItem::Audio {
                    label: "alarm secondary",
                    pcm: self.alarm_secondary,
                }),
                1 => Some(PlaylistItem::SilenceMs(40)),
                _ => None,
            },
            PlaylistKind::AlarmTrip => match idx {
                0 => Some(PlaylistItem::Audio {
                    label: "alarm trip",
                    pcm: self.alarm_trip,
                }),
                1 => Some(PlaylistItem::SilenceMs(40)),
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
            PlaylistKind::LoadOnOk => match idx {
                0 => Some(PlaylistItem::SilenceMs(20)),
                1 => Some(PlaylistItem::Audio {
                    label: "load on ok",
                    pcm: self.load_on_ok,
                }),
                2 => Some(PlaylistItem::SilenceMs(40)),
                _ => None,
            },
            PlaylistKind::UiOkOff => match idx {
                0 => Some(PlaylistItem::SilenceMs(20)),
                1 => Some(PlaylistItem::Audio {
                    label: "load off ok",
                    pcm: self.ui_ok_off,
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
            PlaylistKind::UiWarn => match idx {
                0 => Some(PlaylistItem::SilenceMs(10)),
                1 => Some(PlaylistItem::Audio {
                    label: "ui warn",
                    pcm: self.ui_warn,
                }),
                2 => Some(PlaylistItem::SilenceMs(40)),
                _ => None,
            },
            PlaylistKind::UiTouch => match idx {
                0 => Some(PlaylistItem::SilenceMs(0)),
                1 => Some(PlaylistItem::Audio {
                    label: "ui touch",
                    pcm: self.ui_touch,
                }),
                2 => Some(PlaylistItem::SilenceMs(0)),
                _ => None,
            },
            PlaylistKind::UiTick => match idx {
                0 => Some(PlaylistItem::SilenceMs(0)),
                1 => Some(PlaylistItem::Audio {
                    label: "ui tick",
                    pcm: self.ui_tick,
                }),
                2 => Some(PlaylistItem::SilenceMs(0)),
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

fn fill_mixed_bytes(
    assets: &AudioAssets,
    voice: &mut Option<VoicePlayer>,
    tone: &mut ToneSynth,
    suppress_voice: bool,
    hard_mute: bool,
    out_buf: &mut [u8],
) -> usize {
    // IMPORTANT:
    // `esp-hal` circular DMA uses fixed-size descriptors. If we return a `written` length smaller
    // than the descriptor length, the remaining bytes of that descriptor will still be sent and
    // will contain stale data => harsh distortion/noise. Therefore we always fill the whole slice.
    let want = out_buf.len() & !0x3;
    if want == 0 {
        return 0;
    }
    let out_buf = &mut out_buf[..want];

    if hard_mute {
        out_buf.fill(0);
        return want;
    }

    // Base layer: voice/clip playback (optional).
    if suppress_voice {
        out_buf.fill(0);
        *voice = None;
    } else if let Some(v) = voice.as_mut() {
        fill_stream_bytes(assets, v.kind, &mut v.state, out_buf);
        if v.state.done && v.state.cur.is_none() {
            *voice = None;
        }
    } else {
        out_buf.fill(0);
    }

    // Overlay: prompt tone square wave.
    for s in out_buf.chunks_exact_mut(2) {
        // Read the current duty *per sample* so short pulses (e.g. 16ms detent ticks)
        // are not lost when TX_CHUNK_BYTES is larger than the pulse width.
        let duty = PROMPT_TONE_DUTY_PCT.load(Ordering::Relaxed) as u8;
        let target_amp = prompt_tone_target_amp(duty);

        let base = i16::from_le_bytes([s[0], s[1]]) as i32;
        let t = tone.next_sample(target_amp) as i32;
        let mixed = base
            .saturating_add(t)
            .clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let [a, b] = mixed.to_le_bytes();
        s[0] = a;
        s[1] = b;
    }

    want
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
        "speaker: task starting (MAX98357A, AMP_SD_MODE=GPIO34, I2S_BCLK=GPIO35, I2S_LRCLK=GPIO36, I2S_DIN=GPIO37, {}Hz, wav=pcm16le/mono, prompt_tone={}Hz, ring={=usize}B, chunk={=usize}B)",
        SAMPLE_RATE_HZ, PROMPT_TONE_FREQ_HZ, TX_RING_BYTES, TX_CHUNK_BYTES
    );

    let mut amp = Output::new(amp_sd_mode, Level::Low, OutputConfig::default());
    let assets = AudioAssets::load();

    let (_, _, tx_ring, tx_desc) =
        esp_hal::dma_circular_buffers_chunk_size!(0, TX_RING_BYTES, TX_CHUNK_BYTES);
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
    // We also support a hard mute by pulling SD_MODE LOW.
    let _ = amp.set_high();

    // Start with a boot self-test playlist queued once (Plan #0021). Prompt tones
    // can overlay on top; continuous alarms will suppress voice output.
    let mut voice: Option<VoicePlayer> = if ENABLE_BOOT_SELFTEST {
        Some(VoicePlayer {
            kind: PlaylistKind::Boot,
            state: StreamState {
                idx: 0,
                cur: None,
                done: false,
            },
        })
    } else if ENABLE_BOOT_CHIRP {
        Some(VoicePlayer {
            kind: PlaylistKind::BootChirp,
            state: StreamState {
                idx: 0,
                cur: None,
                done: false,
            },
        })
    } else {
        None
    };
    if voice.is_some() {
        let boot_play_id = SPEAKER_PLAY_TOTAL
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1);
        if ENABLE_BOOT_SELFTEST {
            info!("speaker: boot selftest start #{} (playlist)", boot_play_id);
        } else {
            info!("speaker: boot chirp start #{} (playlist)", boot_play_id);
        }
    }

    let mut tone = ToneSynth::new();

    // Prefill the ring so the boot playlist starts smoothly (avoid initial "silence gap"
    // while waiting for the first DMA EOF interrupts).
    tx_ring.fill(0);
    for chunk in tx_ring.chunks_exact_mut(TX_CHUNK_BYTES) {
        let _ = fill_mixed_bytes(&assets, &mut voice, &mut tone, false, false, chunk);
    }

    let mut transfer = match tx.write_dma_circular(&tx_ring) {
        Ok(t) => t,
        Err(err) => {
            warn!("speaker: start dma_circular failed ({=?})", err);
            return;
        }
    };

    let mut last_alarm_active = false;
    let mut last_hard_mute = false;

    'run: loop {
        let hard_mute = SPEAKER_HARD_MUTE.load(Ordering::Relaxed);
        if hard_mute != last_hard_mute {
            last_hard_mute = hard_mute;
            if hard_mute {
                info!("speaker: hard mute ON (AMP_SD_MODE=LOW)");
                let _ = amp.set_low();
                // Drop pending clips immediately.
                voice = None;
                while SPEAKER_QUEUE.try_receive().is_ok() {}
            } else {
                info!("speaker: hard mute OFF (AMP_SD_MODE=HIGH)");
                let _ = amp.set_high();
            }
        }

        let alarm_active = PROMPT_TONE_ALARM_ACTIVE.load(Ordering::Relaxed);
        if alarm_active != last_alarm_active {
            last_alarm_active = alarm_active;
            if alarm_active {
                // Immediate suppression: stop any in-flight voice clips and drop queued ones.
                voice = None;
                while SPEAKER_QUEUE.try_receive().is_ok() {}
            }
        }

        // Start next queued clip when idle.
        if !hard_mute && !alarm_active && voice.is_none() {
            if let Ok(sound) = SPEAKER_QUEUE.try_receive() {
                let kind = match sound {
                    SpeakerSound::BootChirp => PlaylistKind::BootChirp,
                    SpeakerSound::AlarmPrimary => PlaylistKind::AlarmPrimary,
                    SpeakerSound::AlarmSecondary => PlaylistKind::AlarmSecondary,
                    SpeakerSound::AlarmTrip => PlaylistKind::AlarmTrip,
                    SpeakerSound::UiOk => PlaylistKind::UiOk,
                    SpeakerSound::LoadOnOk => PlaylistKind::LoadOnOk,
                    SpeakerSound::UiOkOff => PlaylistKind::UiOkOff,
                    SpeakerSound::UiFail => PlaylistKind::UiFail,
                    SpeakerSound::UiWarn => PlaylistKind::UiWarn,
                    SpeakerSound::UiTouch => PlaylistKind::UiTouch,
                    SpeakerSound::UiTick => PlaylistKind::UiTick,
                    SpeakerSound::Test => PlaylistKind::Test,
                };
                let play_id = SPEAKER_PLAY_TOTAL
                    .fetch_add(1, Ordering::Relaxed)
                    .wrapping_add(1);
                debug!("speaker: play #{} ({=?})", play_id, sound);
                voice = Some(VoicePlayer {
                    kind,
                    state: StreamState {
                        idx: 0,
                        cur: None,
                        done: false,
                    },
                });
            }
        }

        let avail = match transfer.available() {
            Ok(v) => v,
            Err(err) => {
                warn!("speaker: dma available failed ({=?})", err);
                break;
            }
        };

        if avail == 0 {
            // Don't busy-spin: when running on a SW-interrupt executor this would starve the
            // thread-mode executor (UI) and can actually make audio stutter worse.
            Timer::after_millis(1).await;
            continue;
        }

        // IMPORTANT: one push may cover multiple descriptors depending on wrap-around; the
        // closure must fill the entire provided slice to avoid stale bytes being transmitted.
        let wrote = match transfer.push_with(|buf| {
            fill_mixed_bytes(&assets, &mut voice, &mut tone, alarm_active, hard_mute, buf)
        }) {
            Ok(n) => n,
            Err(err) => {
                warn!("speaker: dma push_with failed ({=?})", err);
                break 'run;
            }
        };
        if wrote == 0 {
            Timer::after_millis(1).await;
            continue;
        }

        // Give other tasks a chance even if we were catching up.
        yield_now().await;
    }

    // Safety: ensure we exit with the amp disabled.
    let _ = amp.set_low();
}
