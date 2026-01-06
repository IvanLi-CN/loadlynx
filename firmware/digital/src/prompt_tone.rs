//! Prompt tone manager (buzzer feedback + alarm policy).
//!
//! This module owns the policy for when to emit:
//! - low-volume UI feedback tones (touch / encoder detents / button actions)
//! - continuous alarm tones:
//!   - Primary: protection-class (UV / analog fault flags)
//!   - Secondary: link-drop-class (latched by `latch_link_alarm()`)
//!
//! Important semantics (frozen by requirements):
//! - Primary overrides Secondary.
//! - While any continuous alarm is active: it MUST keep playing and suppress other tones.
//! - After an underlying alarm condition clears: that alarm MUST keep playing until
//!   the *next* local interaction (touch / detent / button). Remote actions do not count.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use defmt::{info, warn};
use embassy_futures::select::{Either, select};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, signal::Signal,
};
use embassy_time::Timer;
use esp_hal::ledc::channel::{self as ledc_channel, ChannelIFace as _};

// --- Public event API -------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiSound {
    Ok,
    Fail,
}

static UI_SOUNDS: Channel<CriticalSectionRawMutex, UiSound, 8> = Channel::new();
static WAKE: Signal<CriticalSectionRawMutex, ()> = Signal::new();

static FAULT_FLAGS: AtomicU32 = AtomicU32::new(0);
static UV_LATCHED_ACTIVE: AtomicBool = AtomicBool::new(false);

static LINK_UP: AtomicBool = AtomicBool::new(true);
static LINK_ALARM_LATCHED: AtomicBool = AtomicBool::new(false);

static LOCAL_ACTIVITY: AtomicU32 = AtomicU32::new(0);
static PENDING_TICKS: AtomicU32 = AtomicU32::new(0);

/// Update the latest analog-side `fault_flags` snapshot.
///
/// The prompt tone task will detect edges and enforce the "fault cleared needs
/// local ack" policy.
pub fn set_fault_flags(flags: u32) {
    FAULT_FLAGS.store(flags, Ordering::Relaxed);
    WAKE.signal(());
}

/// Update the UV-latched state (protection-class).
///
/// Semantics:
/// - false -> true: start Primary alarm immediately.
/// - true -> false: keep Primary alarm until local ack, then stop (unless `fault_flags != 0`).
pub fn set_uv_latched(active: bool) {
    UV_LATCHED_ACTIVE.store(active, Ordering::Relaxed);
    WAKE.signal(());
}

/// Update the local view of link-up state.
///
/// Used to decide when a latched Secondary alarm has "cleared".
pub fn set_link_up(up: bool) {
    LINK_UP.store(up, Ordering::Relaxed);
    WAKE.signal(());
}

/// Latch the Secondary (link-drop-class) alarm.
///
/// Does not require link to be down at call time, but will only stop after
/// link is up + local ack.
pub fn latch_link_alarm() {
    LINK_ALARM_LATCHED.store(true, Ordering::Relaxed);
    WAKE.signal(());
}

/// Return whether the Secondary (link-drop-class) alarm is currently latched.
///
/// Note: this stays `true` while the alarm is actively sounding *or* while it
/// is waiting for the next local ack after the link has recovered.
pub fn is_link_alarm_latched() -> bool {
    LINK_ALARM_LATCHED.load(Ordering::Relaxed)
}

/// Enqueue a single "UI ok" feedback sound (low volume).
pub fn enqueue_ui_ok() {
    let _ = UI_SOUNDS.try_send(UiSound::Ok);
    WAKE.signal(());
}

/// Enqueue a single "UI fail" feedback sound (low volume).
///
/// This MUST represent a "business reject" (not link/ACK/timeout failures).
pub fn enqueue_ui_fail() {
    let _ = UI_SOUNDS.try_send(UiSound::Fail);
    WAKE.signal(());
}

/// Notify a local user interaction that should count for "fault cleared needs
/// local ack", without necessarily producing a sound.
///
/// Example: touch-down on a non-interactive area of the screen.
pub fn notify_local_activity() {
    // Saturate to keep it bounded; we only care about "non-zero".
    let _ = LOCAL_ACTIVITY.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
        Some(v.saturating_add(1).min(1_000_000))
    });
    WAKE.signal(());
}

/// Account for encoder detents that need a tick sound.
///
/// This uses an atomic counter so that high-frequency detents do not stall the
/// encoder task. The prompt tone task will drain this counter into actual
/// tick beeps when allowed.
pub fn enqueue_ticks(count: u32) {
    if count == 0 {
        return;
    }

    // Saturate to keep the counter bounded under pathological input.
    const MAX_PENDING_TICKS: u32 = 10_000;

    let mut cur = PENDING_TICKS.load(Ordering::Relaxed);
    loop {
        let next = cur.saturating_add(count).min(MAX_PENDING_TICKS);
        match PENDING_TICKS.compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => cur = observed,
        }
    }

    // Wake the prompt tone task so it can start draining ticks promptly.
    WAKE.signal(());
}

fn try_take_one_tick() -> bool {
    let mut cur = PENDING_TICKS.load(Ordering::Relaxed);
    while cur > 0 {
        match PENDING_TICKS.compare_exchange_weak(
            cur,
            cur - 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return true,
            Err(observed) => cur = observed,
        }
    }
    false
}

// --- Playback engine --------------------------------------------------------

pub const BUZZER_FREQ_HZ: u32 = 2_200;

// Low-volume UI feedback (duty controls loudness).
const UI_DUTY_PCT: u8 = 3;
// Stronger duty for "fail" so it feels clearly negative/distinct.
const UI_FAIL_DUTY_PCT: u8 = 6;
const PRIMARY_ALARM_DUTY_PCT: u8 = 6;
const SECONDARY_ALARM_DUTY_PCT: u8 = 6;

// UI tick: keep short so normal rotation does not backlog.
const UI_TICK_TONE_MS: u32 = 12;
const UI_TICK_GAP_MS: u32 = 8;

// UI ok/fail patterns.
const UI_OK_MS: u32 = 25;
// Make "fail" clearly distinct *and* emotionally negative:
// a short "pip" followed by a longer "buuu" and a pause.
const UI_FAIL_ON1_MS: u32 = 30;
const UI_FAIL_GAP_MS: u32 = 45;
const UI_FAIL_ON2_MS: u32 = 160;
const UI_FAIL_TAIL_MS: u32 = 120;

// Primary alarm cadence (protection-class).
const PRIMARY_ALARM_ON_MS: u32 = 300;
const PRIMARY_ALARM_OFF_MS: u32 = 700;

// Secondary alarm cadence (link-drop-class): double-pip, distinct from Primary.
const SECONDARY_ALARM_ON1_MS: u32 = 120;
const SECONDARY_ALARM_GAP_MS: u32 = 80;
const SECONDARY_ALARM_ON2_MS: u32 = 120;
const SECONDARY_ALARM_TAIL_MS: u32 = 680;

#[derive(Clone, Copy, Debug)]
struct Step {
    duty_pct: u8, // 0 == silence
    duration_ms: u32,
}

const STEPS_UI_TICK: &[Step] = &[
    Step {
        duty_pct: UI_DUTY_PCT,
        duration_ms: UI_TICK_TONE_MS,
    },
    Step {
        duty_pct: 0,
        duration_ms: UI_TICK_GAP_MS,
    },
];

const STEPS_UI_OK: &[Step] = &[Step {
    duty_pct: UI_DUTY_PCT,
    duration_ms: UI_OK_MS,
}];

const STEPS_UI_FAIL: &[Step] = &[
    Step {
        duty_pct: UI_FAIL_DUTY_PCT,
        duration_ms: UI_FAIL_ON1_MS,
    },
    Step {
        duty_pct: 0,
        duration_ms: UI_FAIL_GAP_MS,
    },
    Step {
        duty_pct: UI_FAIL_DUTY_PCT,
        duration_ms: UI_FAIL_ON2_MS,
    },
    Step {
        duty_pct: 0,
        duration_ms: UI_FAIL_TAIL_MS,
    },
];

const STEPS_PRIMARY_ALARM: &[Step] = &[
    Step {
        duty_pct: PRIMARY_ALARM_DUTY_PCT,
        duration_ms: PRIMARY_ALARM_ON_MS,
    },
    Step {
        duty_pct: 0,
        duration_ms: PRIMARY_ALARM_OFF_MS,
    },
];

const STEPS_SECONDARY_ALARM: &[Step] = &[
    Step {
        duty_pct: SECONDARY_ALARM_DUTY_PCT,
        duration_ms: SECONDARY_ALARM_ON1_MS,
    },
    Step {
        duty_pct: 0,
        duration_ms: SECONDARY_ALARM_GAP_MS,
    },
    Step {
        duty_pct: SECONDARY_ALARM_DUTY_PCT,
        duration_ms: SECONDARY_ALARM_ON2_MS,
    },
    Step {
        duty_pct: 0,
        duration_ms: SECONDARY_ALARM_TAIL_MS,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveSound {
    UiOk,
    UiFail,
    UiTick,
    PrimaryAlarm,
    SecondaryAlarm,
}

#[derive(Clone, Copy, Debug)]
struct Player {
    sound: ActiveSound,
    step_index: usize,
    step_deadline_ms: u32,
}

impl Player {
    fn steps(self) -> &'static [Step] {
        match self.sound {
            ActiveSound::UiOk => STEPS_UI_OK,
            ActiveSound::UiFail => STEPS_UI_FAIL,
            ActiveSound::UiTick => STEPS_UI_TICK,
            ActiveSound::PrimaryAlarm => STEPS_PRIMARY_ALARM,
            ActiveSound::SecondaryAlarm => STEPS_SECONDARY_ALARM,
        }
    }
}

#[inline]
fn now_ms32() -> u32 {
    crate::now_ms32()
}

#[inline]
fn buzzer_apply(
    channel: &'static ledc_channel::Channel<'static, esp_hal::ledc::LowSpeed>,
    duty_pct: u8,
) {
    if let Err(err) = channel.set_duty(duty_pct) {
        warn!("buzzer set_duty failed: {:?}", err);
    }
}

fn drain_ui_sounds(pending_ok: &mut u8, pending_fail: &mut u8, suppress: bool) {
    loop {
        let ev = UI_SOUNDS.try_receive();
        let Ok(sound) = ev else { break };
        if suppress {
            continue;
        }
        match sound {
            UiSound::Ok => {
                *pending_ok = pending_ok.saturating_add(1).min(32);
            }
            UiSound::Fail => {
                *pending_fail = pending_fail.saturating_add(1).min(32);
            }
        }
    }
}

fn start_player(
    sound: ActiveSound,
    channel: &'static ledc_channel::Channel<'static, esp_hal::ledc::LowSpeed>,
) -> Player {
    let now = now_ms32();
    let steps = match sound {
        ActiveSound::UiOk => STEPS_UI_OK,
        ActiveSound::UiFail => STEPS_UI_FAIL,
        ActiveSound::UiTick => STEPS_UI_TICK,
        ActiveSound::PrimaryAlarm => STEPS_PRIMARY_ALARM,
        ActiveSound::SecondaryAlarm => STEPS_SECONDARY_ALARM,
    };
    let step0 = steps[0];
    buzzer_apply(channel, step0.duty_pct);
    Player {
        sound,
        step_index: 0,
        step_deadline_ms: now.wrapping_add(step0.duration_ms),
    }
}

fn advance_player(
    mut player: Player,
    channel: &'static ledc_channel::Channel<'static, esp_hal::ledc::LowSpeed>,
    repeat: bool,
) -> Option<Player> {
    let now = now_ms32();
    if (now.wrapping_sub(player.step_deadline_ms) as i32) < 0 {
        return Some(player);
    }

    let steps = player.steps();
    player.step_index += 1;
    if player.step_index >= steps.len() {
        if repeat {
            player.step_index = 0;
        } else {
            buzzer_apply(channel, 0);
            return None;
        }
    }

    let step = steps[player.step_index];
    buzzer_apply(channel, step.duty_pct);
    player.step_deadline_ms = now.wrapping_add(step.duration_ms);
    Some(player)
}

// --- Main task --------------------------------------------------------------

#[embassy_executor::task]
pub async fn prompt_tone_task(
    buzzer_channel: &'static ledc_channel::Channel<'static, esp_hal::ledc::LowSpeed>,
) {
    info!(
        "prompt_tone: starting (GPIO21=BUZZER, freq={}Hz, ui_duty={}%, primary_duty={}%, secondary_duty={}%)",
        BUZZER_FREQ_HZ, UI_DUTY_PCT, PRIMARY_ALARM_DUTY_PCT, SECONDARY_ALARM_DUTY_PCT
    );

    // Ensure we start silent.
    buzzer_apply(buzzer_channel, 0);

    let mut last_fault_flags: u32 = 0;
    let mut last_uv_latched: bool = false;

    let mut primary_cleared_wait_ack: bool = false;

    let mut last_link_latched: bool = false;
    let mut secondary_cleared_wait_ack: bool = false;

    let mut pending_ok: u8 = 0;
    let mut pending_fail: u8 = 0;

    let mut player: Option<Player> = None;

    loop {
        let fault_flags = FAULT_FLAGS.load(Ordering::Relaxed);
        let uv_latched = UV_LATCHED_ACTIVE.load(Ordering::Relaxed);
        let link_latched = LINK_ALARM_LATCHED.load(Ordering::Relaxed);
        let link_up = LINK_UP.load(Ordering::Relaxed);

        let primary_condition_active = fault_flags != 0 || uv_latched;

        // Edge bookkeeping for logging + ack gating.
        if fault_flags != last_fault_flags || uv_latched != last_uv_latched {
            let primary_was_active = last_fault_flags != 0 || last_uv_latched;

            // Rising edge into Primary condition.
            if !primary_was_active && primary_condition_active {
                info!(
                    "prompt_tone: primary alarm entered (fault_flags=0x{:08x}, uv_latched={})",
                    fault_flags, uv_latched
                );
                primary_cleared_wait_ack = false;
                pending_ok = 0;
                pending_fail = 0;
                LOCAL_ACTIVITY.store(0, Ordering::Relaxed);
                PENDING_TICKS.store(0, Ordering::Relaxed); // discard suppressed ticks
                player = Some(start_player(ActiveSound::PrimaryAlarm, buzzer_channel));
            }

            // Falling edge out of Primary condition.
            if primary_was_active && !primary_condition_active {
                info!("prompt_tone: primary alarm cleared; waiting for local ack");
                primary_cleared_wait_ack = true;
                LOCAL_ACTIVITY.store(0, Ordering::Relaxed);
                if player.is_none() || player.is_some_and(|p| p.sound != ActiveSound::PrimaryAlarm)
                {
                    player = Some(start_player(ActiveSound::PrimaryAlarm, buzzer_channel));
                }
            }

            last_fault_flags = fault_flags;
            last_uv_latched = uv_latched;
        }

        // Secondary latch edge (for logging).
        if link_latched != last_link_latched {
            if !last_link_latched && link_latched {
                info!("prompt_tone: secondary alarm latched (link_up={})", link_up);
                secondary_cleared_wait_ack = false;
            }
            last_link_latched = link_latched;
        }

        // --- Primary alarm (highest priority) ---------------------------------

        if primary_condition_active {
            primary_cleared_wait_ack = false;

            // While Primary is active, suppress UI sounds and discard detent ticks.
            drain_ui_sounds(&mut pending_ok, &mut pending_fail, true);
            PENDING_TICKS.store(0, Ordering::Relaxed);

            if player.is_none() || player.is_some_and(|p| p.sound != ActiveSound::PrimaryAlarm) {
                player = Some(start_player(ActiveSound::PrimaryAlarm, buzzer_channel));
            }
            if let Some(p) = player {
                player = advance_player(p, buzzer_channel, true);
            }
        } else if primary_cleared_wait_ack {
            // Primary is cleared; keep playing alarm until the first local interaction happens.
            drain_ui_sounds(&mut pending_ok, &mut pending_fail, false);
            let has_activity = LOCAL_ACTIVITY.load(Ordering::Relaxed) > 0;
            let has_detent = PENDING_TICKS.load(Ordering::Relaxed) > 0;
            let has_sound = pending_ok > 0 || pending_fail > 0;
            if has_activity || has_detent || has_sound {
                info!("prompt_tone: local ack observed; stopping primary alarm");
                primary_cleared_wait_ack = false;
                LOCAL_ACTIVITY.store(0, Ordering::Relaxed);
                buzzer_apply(buzzer_channel, 0);
                player = None;
            } else {
                if player.is_none() || player.is_some_and(|p| p.sound != ActiveSound::PrimaryAlarm)
                {
                    player = Some(start_player(ActiveSound::PrimaryAlarm, buzzer_channel));
                }
                if let Some(p) = player {
                    player = advance_player(p, buzzer_channel, true);
                }
            }
        } else {
            // --- Secondary alarm ------------------------------------------------
            if link_latched {
                if link_up {
                    if !secondary_cleared_wait_ack {
                        info!("prompt_tone: secondary alarm cleared; waiting for local ack");
                        secondary_cleared_wait_ack = true;
                        LOCAL_ACTIVITY.store(0, Ordering::Relaxed);
                    }

                    drain_ui_sounds(&mut pending_ok, &mut pending_fail, false);
                    let has_activity = LOCAL_ACTIVITY.load(Ordering::Relaxed) > 0;
                    let has_detent = PENDING_TICKS.load(Ordering::Relaxed) > 0;
                    let has_sound = pending_ok > 0 || pending_fail > 0;
                    if has_activity || has_detent || has_sound {
                        info!("prompt_tone: local ack observed; stopping secondary alarm");
                        secondary_cleared_wait_ack = false;
                        LINK_ALARM_LATCHED.store(false, Ordering::Relaxed);
                        LOCAL_ACTIVITY.store(0, Ordering::Relaxed);
                        buzzer_apply(buzzer_channel, 0);
                        player = None;
                    } else {
                        if player.is_none()
                            || player.is_some_and(|p| p.sound != ActiveSound::SecondaryAlarm)
                        {
                            player =
                                Some(start_player(ActiveSound::SecondaryAlarm, buzzer_channel));
                        }
                        if let Some(p) = player {
                            player = advance_player(p, buzzer_channel, true);
                        }
                    }
                } else {
                    secondary_cleared_wait_ack = false;

                    // While Secondary is active, suppress UI sounds and discard detent ticks.
                    drain_ui_sounds(&mut pending_ok, &mut pending_fail, true);
                    PENDING_TICKS.store(0, Ordering::Relaxed);

                    if player.is_none()
                        || player.is_some_and(|p| p.sound != ActiveSound::SecondaryAlarm)
                    {
                        player = Some(start_player(ActiveSound::SecondaryAlarm, buzzer_channel));
                    }
                    if let Some(p) = player {
                        player = advance_player(p, buzzer_channel, true);
                    }
                }
            } else {
                // --- Normal UI mode ---------------------------------------------

                secondary_cleared_wait_ack = false;

                drain_ui_sounds(&mut pending_ok, &mut pending_fail, false);

                // Preempt: if an alarm player somehow remained, stop it.
                if player.is_some_and(|p| {
                    p.sound == ActiveSound::PrimaryAlarm || p.sound == ActiveSound::SecondaryAlarm
                }) {
                    buzzer_apply(buzzer_channel, 0);
                    player = None;
                }

                // Advance current player (no repeat for UI sounds).
                if let Some(p) = player {
                    player = advance_player(p, buzzer_channel, false);
                }

                // If idle, start the next sound by priority.
                if player.is_none() {
                    if pending_fail > 0 {
                        pending_fail = pending_fail.saturating_sub(1);
                        player = Some(start_player(ActiveSound::UiFail, buzzer_channel));
                    } else if pending_ok > 0 {
                        pending_ok = pending_ok.saturating_sub(1);
                        player = Some(start_player(ActiveSound::UiOk, buzzer_channel));
                    } else if try_take_one_tick() {
                        player = Some(start_player(ActiveSound::UiTick, buzzer_channel));
                    }
                }
            }
        }

        // Sleep until either:
        // - the next playback edge is due; or
        // - a new event arrives (fault/ticks/UI sounds).
        if let Some(p) = player {
            let now = now_ms32();
            if (now.wrapping_sub(p.step_deadline_ms) as i32) < 0 {
                let wait_ms = p.step_deadline_ms.wrapping_sub(now) as u64;
                match select(WAKE.wait(), Timer::after_millis(wait_ms)).await {
                    Either::First(_) => {}
                    Either::Second(_) => {}
                }
            } else {
                // Deadline already passed; loop immediately to advance.
                continue;
            }
        } else {
            WAKE.wait().await;
        }
    }
}
