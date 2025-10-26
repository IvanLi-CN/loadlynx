Over‑Temperature Protection (OTP) — Hardware + Software
=======================================================

Scope

- Targets: [STM32G431](https://www.st.com/resource/en/datasheet/stm32g431c6.pdf) (analog board), [TPS22810](../other-datasheets/tps22810.md) load switch (OVCC), NTC sensors.
- Sensors: TS1 (PB0/ADC1_IN15) and TS2 (PB1/ADC1_IN12), both 10 kΩ/3950 K NTC with 5.11 kΩ pull‑up at 3.3 V.
- Goal: Ensure default‑off on power‑up or MCU failure; provide graded software derating/shutdown; guarantee hard cut‑off by hardware without CPU.

Signals & Resources

- TS1: PB0 → ADC1_IN15 (sampling) and COMP4_INP (non‑inverting input)
- TS2: PB1 → ADC1_IN12 (sampling)
- LOAD_EN_CTL: PB13 → MCU intent (GPIO push‑pull)
- COMP4_OUT: PB14 → AND gate input (COMP_OK)
- Final LOAD_EN → [TPS22810](../other-datasheets/tps22810.md) EN/UVLO (controls OVCC of op‑amp stage)
- Logic gate: [74LVC1G08GW](../other-datasheets/74lvc1g08-nexperia.md) (single 2‑input AND), VCC=3.3 V, decoupled by 100 nF close to pins

Protection States (default thresholds)

- Pre‑warn (SW): 70 °C (TSx), clear at 65 °C
- Derate (SW): 85 °C (TSx), clear at 80 °C
- Hard cut‑off (HW via COMP): ≈111.3 °C trip, clear at ≈108–110 °C (by COMP hysteresis)

Hardware Protection (COMP + AND)

- Comparator configuration ([STM32G431](https://www.st.com/resource/en/datasheet/stm32g431c6.pdf)):
  - COMP4_INP = PB0 (TS1 node)
  - COMP4_INM = VREFINT/4 ≈ 0.3 V (maps to ~111.3 °C with 10k/3950K + 5.11k divider)
  - Output polarity: normal = High when TS1 > 0.3 V (temperature below trip); Low when over‑temp
  - Hysteresis: medium (target ~1–3 °C equivalent), speed: medium
  - Route COMP4_OUT → PB14 (AF), net = LOAD_EN_TS (aka COMP_OK)
- External AND logic ([74LVC1G08GW](../other-datasheets/74lvc1g08-nexperia.md)):
  - IN_A = LOAD_EN_CTL (PB13, MCU intent)
  - IN_B = LOAD_EN_TS (PB14, COMP_OK)
  - OUT = LOAD_EN → TPS22810 EN/UVLO
  - Function: LOAD_EN = LOAD_EN_CTL AND COMP_OK
- Default‑off guarantees:
  - TPS22810 EN/UVLO has 100 kΩ pull‑down to GND (must be placed near EN pin)
  - AND inputs have high‑value pull‑downs (≥100 kΩ) to ensure 0 AND 0 → 0 during power‑up/reset
  - 74LVC1G08 VCC decoupled with 100 nF close to device; short returns to ground plane
- Behavior without CPU:
  - Once COMP is configured after boot, hard protection is fully hardware. If CPU later hangs, COMP→AND→EN cuts OVCC immediately upon over‑temp.
  - Prior to COMP init, default‑off is enforced by pull‑downs so the load remains off even if MCU never comes up.

Software Protection (ADC on TS1/TS2 + MCU internal sensor)

- Sampling
  - ADC1 channels: TS1=IN15, TS2=IN12; use long sample time to accommodate 100 nF at node (see ntc doc)
  - Oversampling/filtering: average ≥16 samples; optional small IIR to reduce noise
  - Use VREFINT ratio to desensitize VSUP drift (periodically sample VREFINT)
  - Convert counts → temperature via Beta model (R0=10 kΩ, B=3950 K, T0=25 °C)
- Threshold policy (per TS channel)
  - Pre‑warn 70 °C: raise warning, log, optionally reduce current limit by small step
  - Derate 85 °C: enforce power/current derating (e.g., reduce setpoints; fan to high)
  - Software emergency off 100 °C (optional guard): set LOAD_EN_CTL=Low and latch fault; requires operator or cold‑clear to re‑enable
  - Hard cut‑off: COMP trip (~111 °C) will force hardware off regardless of software state
- MCU internal temperature sensor
  - Monitor die temp; if MCU die exceeds safe temp (e.g., 90–100 °C), reduce activity and proactively pull LOAD_EN_CTL Low; log event
- Fault latching & recovery
  - SW latched states: require operator action or hysteresis + minimum cool‑down time before clearing (e.g., cool‑down ≥10 °C and ≥5 s)
  - After a COMP‑induced hard cut‑off, firmware should detect OVCC off (via LOAD_PG or derived metric) and keep LOAD_EN_CTL Low until TS1 has cooled below clear threshold

Boot/Reset Behavior

- Default state: LOAD_EN must be Low (off)
  - Enforced by EN pull‑down and AND input pull‑downs
- Early init order
  1) Configure GPIO defaults (PB13=Low, PB14=AF, low‑speed drive)
  2) Initialize COMP4 (INP=PB0, INM=VREFINT/4, hysteresis, output to PB14) and LOCK if used
  3) Initialize ADC1 (TS1/TS2), VREFINT path, and logging
  4) Only after thermal status is valid, allow LOAD_EN_CTL to go High

Electrical/EMC Notes

- Keep PB13/PB14/LOAD_EN traces short, referenced to continuous ground; avoid power switcher zones
- GPIO output speed: set PB13 to lowest speed to reduce edge rate; COMP_OUT pin uses AF—keep route short
- Optional (space‑permitting): EN to GND small capacitor (1–4.7 nF) near TPS22810 to suppress spikes; DNP if not needed

Calibration & Tolerances

- Sensor tolerances: NTC R25/B tolerance, divider tolerance, VREFINT tolerance, COMP offset
- Margins: keep 5–10 °C design margin between SW thresholds and absolute limits; rely on COMP for final safety cutoff
- Consider 2‑ or 3‑point thermal calibration during manufacturing for improved reporting accuracy; OTP thresholds should use conservative values

Test Plan (bring‑up & regression)

1) Cold/ambient/hot points: verify TS1/TS2 conversions match reference thermometer at 0 °C / ambient / ≥90 °C
2) COMP trip: heat TS1 to exceed ~111 °C; observe LOAD_EN drops Low immediately; verify MCU hang (halt core) still cuts OVCC
3) Hysteresis: cool down until COMP clears; confirm re‑enable policy follows spec (latched vs auto‑clear)
4) SW thresholds: sweep temp across 70 °C / 85 °C; verify warnings, derating, and (if enabled) SW emergency off at 100 °C
5) Power‑up/reset: cycle power with induced MCU boot failure; confirm default‑off persists
6) EMI: inject conducted/radiated noise typical to power stage; verify no false enabling of LOAD_EN

References

- Thermal sensing details and divider selection: thermal/ntc-temperature-sensing.md
- Load switch datasheet: other-datasheets/tps22810.md
- 74LVC1G08 datasheet: other-datasheets/74lvc1g08-nexperia.md
- STM32G431 datasheet (external): https://www.st.com/resource/en/datasheet/stm32g431c6.pdf
