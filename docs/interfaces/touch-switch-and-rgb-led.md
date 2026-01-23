# Touch Switch and RGB LED (Plan #0021)

This document describes the **touch spring switch** and the **RGB indicator LED** used on the digital board, including intended behavior, electrical constraints, and layout guidance.

For the authoritative pin assignment, see:
- `docs/interfaces/pinmaps/esp32-s3.md` (ESP32‑S3 pinmap)
- `docs/plan/0021:touch-spring-load-switch-rgb-led/PLAN.md` (scope and acceptance)

## Goals

- Add a **touch switch** (through an acrylic panel) that toggles the load switch state.
- Add a **3‑channel RGB indicator LED** driven by PWM, visible through the same panel area.
- Keep the solution low‑cost and simple: **no external touch controller**, and RGB is **direct‑driven from the MCU** (no transistor/MOSFET driver).

## Touch Switch

### Functional behavior

- Touch input is treated as a **momentary button**; firmware detects `touch-down` edges.
- A single touch toggles `load_enabled` **through the same CC model** used by other control paths (rotary encoder push / HTTP API), so arbitration stays **last-writer-wins**.
- Safety rule remains strict: if `set_* == 0`, firmware must **force `load_enabled=false`** (touch must not override this).

### Electrical recommendations (minimal, robust)

- **Series resistor (recommended)**: place `100–470Ω` in series between `TOUCH_SPRING` and the MCU pin (default `220Ω`).
  - Limits injected current during ESD events and reduces ringing.
- **Optional ESD clamp (recommended as footprint; DNP acceptable)**:
  - Touch electrodes can still receive ESD via **capacitive coupling** through acrylic; a low‑cap ESD diode reduces resets/false triggers.
  - **Capacitance is the key constraint**: prefer **ultra‑low capacitance** ESD parts (`<= 1pF` class).
  - Example candidates:
    - `TPD1E01B04DPYR` (very low IO capacitance; bidirectional; Vrwm around 3.6 V)
    - `TPD1E05U06DPYR` (low IO capacitance; unidirectional; Vrwm around 5.5 V)
  - Avoid “general TVS” parts with tens of pF or higher on the touch line (they usually make touch calibration harder or unstable).

### Layout guidance

- Treat the touch net as a **high‑impedance analog signal**:
  - Keep the `TOUCH_SPRING` trace **short**, with **few vias**, and away from high‑slew nets (notably `I2S_BCLK` and RGB PWM traces).
  - Do not route other signals **under** the electrode.
- **Electrode area**:
  - A **copper keepout on all layers** under the electrode projection is usually beneficial for sensitivity (especially when sensing through acrylic).
    - Keep the actual spring connection pad outside the keepout rule (i.e., keepout around the pad, not eliminating the pad itself).
  - Avoid large GND pours directly under the electrode projection (both sides) unless you intentionally trade sensitivity for noise immunity.
  - If you add a guard/ground ring, keep it outside the effective electrode area and tie it to a quiet ground.
- If the RGB LED must sit inside the touch area and routing is unavoidable:
  - Prefer a **pour‑keepout only** (block copper pours) rather than a strict “no copper at all” keepout, so you can route the required LED nets while still avoiding large reference planes that increase parasitic capacitance.
  - Keep LED traces **as short as possible**, avoid long parallel runs with `TOUCH_SPRING`, and cross at **90°** if they must cross.
  - Place the RGB series resistors closer to the MCU side when possible (slows edges before the long trace segment), and prefer small current targets for green/blue under `3V3` direct drive.
- If PWM coupling still disturbs touch:
  - Firmware may temporarily **freeze RGB PWM** during a short touch sampling window (sub‑ms), which is visually imperceptible but improves stability.

## RGB Indicator LED (Direct Drive, Common Anode)

### Electrical contract

- RGB LED is **common‑anode**, and **COM must connect to `3V3` (VDDIO)**.
  - Do **not** connect COM to `5V` when direct‑driving the cathodes from GPIO, otherwise current can back‑feed into `3V3` via MCU clamp paths.
- Each color channel must have its own **series current‑limit resistor**.
- Since COM is `3V3` and GPIO sinks current, PWM polarity is **active‑low**:
  - `GPIO=LOW` → LED on
  - `GPIO=HIGH` → LED off

### Current targets and starting resistor values

Direct drive is constrained by GPIO current capability and by limited headroom for green/blue at `3V3`.

Frozen starting point (tune by measurement later):
- **R**: `180Ω` (≈ 6–8 mA typical for a red LED around `Vf≈2.0V`)
- **G**: `100Ω` (≈ 1–3 mA typical; depends strongly on green `Vf`)
- **B**: `100Ω` (≈ 1–3 mA typical; depends strongly on blue `Vf`)

Note:
- Under this constraint set, green/blue cannot be expected to reach “20 mA per color” in a controlled way.
- Prefer optical improvements (light pipe, diffusion, LED selection) over pushing current.

### Supply decoupling

- Do not add large “PWM smoothing” capacitors to COM→GND (they increase pulsed supply currents and may worsen touch coupling).
- If the COM feed trace is long or narrow, a small local decoupling footprint near the LED (`0.1µF` + optional `1µF`) is acceptable.

### Preventing visible flash at boot/reset

Even if PAD‑JTAG is not used, LEDs can flash briefly due to GPIO default states and PWM initialization.

Firmware recommendation:
- Configure RGB GPIOs **as push‑pull outputs and drive HIGH (OFF)** as early as possible, then enable LEDC/PWM.

## Cross‑coupling notes (Touch + RGB + Audio)

- The touch electrode area is sensitive to PWM edge noise and to `I2S_BCLK` coupling.
- Keep the touch trace far from:
  - `I2S_BCLK` / `I2S_LRCLK` / `I2S_DIN`
  - RGB PWM traces and their return currents
  - switching power nodes (buck SW, inductor, etc.)
