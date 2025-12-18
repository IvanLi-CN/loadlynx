# CC Loop Debug Log (CH1)

This log captures the current-stability troubleshooting for the analog board **CH1** constant-current loop, with scope captures taken using a **ground spring**.

## Hardware context (from netlist)

- Driver / error amp: `U18 = OPA2365` (U18_OUT = `$1N356`)
- MOSFET: `Q1 = IRFP4468PBF` (Gate = `$1N395`, Source / shunt-top = `$1N151181`)
- Shunt: `R42 = 50mΩ` between `VLOAD_N` and `$1N151181`
- Current sense filter: `R9 = 100Ω` (`CUR1_SNS` ↔ `I_SENSE_CH1`), `C48 = 10nF` (`CUR1_SNS` ↔ `GND`)
- Compensation network:
  - `C14`: `$1N356` ↔ `CUR1_SNS`
  - `R17`: `CUR1_SNS` ↔ `$1N91324`
  - `C13`: `$1N91324` ↔ `$1N356`

## Firmware guardrail (single-channel validation)

To avoid cross-coupling between channels during debugging, the analog firmware has a temporary override:

- `firmware/analog/src/main.rs`: set exactly one of:
  - `FORCE_CH1_ONLY = true` → forces `CH2 target = 0`
  - `FORCE_CH2_ONLY = true` → forces `CH1 target = 0`

## Measurement notes / artifacts

- Scope captures are stored locally outside this repo:
  - `pastepreset-batch-20251217-022025/` (files named by current setpoint: `3A`, `4A`, `4.2`, `4.2A`, `4.3`).
- Naming correction (2025-12-17): some screenshots are saved with filenames containing `R24` (e.g. `R24_AC.png`), but the actual probe point was the **CH1 shunt `R42` (50 mΩ)** (shunt-top node vs `GND/VLOAD_N`).

## Starting point (capture set `pastepreset-batch-20251217-022025/`)

- `R11 = 330Ω`
- `R9 = 100Ω`
- `C48 = 10nF`
- `C14 = 10nF` (changed from 47nF before this capture set)
- `C13 = 100nF`
- `R17 = 220Ω`

## Installed values (during `pastepreset-batch-20251217-155530/`)

- `R11 = 680Ω`
- `R9 = 100Ω`
- `C48 = 10nF`
- `C14 = 10nF`
- `C13 = 220nF`
- `R17 = 220Ω`

## Current installed values (as of 5 A retest with `C13 = 100nF`)

- `R11 = 680Ω`
- `R9 = 100Ω`
- `C48 = 10nF`
- `C14 = 10nF`
- `C13 = 100nF`
- `R17 = 220Ω`

## Observations (2025-12-17)

Assuming these screenshots are taken at the shunt-top node (`$1N151181`) to ground:

- Up to ~`4.2 A`: ripple is small (a few mVpp), no dominant sinusoid.
- At `≥4.2 A`: a clean sinusoidal oscillation appears and persists up to higher currents with similar shape (limit-cycle / self-oscillation):
  - `4.2A`: ~`103 kHz`, ~`57.8 mVpp`
  - `4.3A`: ~`101 kHz`, ~`92.5 mVpp`
- This behavior does not match PSU current limit (PSU confirmed not in current-limit).

## Working hypothesis

- The CC loop loses phase margin at a higher-current operating point and locks into a ~`100 kHz` oscillation.
- The `R9/C48` pole (`~1/(2π·100Ω·10nF) ≈ 159 kHz`) likely contributes meaningful phase lag near 100 kHz, together with power-stage poles (MOSFET in linear region, gate dynamics, layout parasitics).

## Next experiment (highest value)

Change **one part only** and re-test the same current sweep:

- `C48: 10nF → 2.2nF` (or `4.7nF` if 2.2nF is unavailable), keep everything else constant.
  - Expected outcome: the ~100 kHz sinusoid should reduce significantly or shift in frequency; either result helps confirm whether `R9/C48` is the dominant contributor.

## Experiment result: C48 reduced (2025-12-17)

Test condition: `C48 = 2.2nF` with the same remaining values.

- Observed: oscillation remains near `~101 kHz` and amplitude increases (example capture shows `~102 mVpp @ 101 kHz` with `Vavg ~210 mV`).
- Interpretation: `R9/C48` is not setting the oscillation frequency; decreasing `C48` removes damping / increases HF loop gain, making the limit-cycle worse.
- Action: revert `C48` back to `10nF` before further loop-compensation work; next knobs to isolate are `R11` (gate damping) and the `C13/C14/R17` compensation network.

## Experiment result: C48 restored + R11 increased (2025-12-17)

Test condition:

- `C48 = 10nF` (restored)
- `R11 = 680Ω` (gate series resistor)
- `C14 = 10nF`, `C13 = 100nF`, `R17 = 220Ω`, `R9 = 100Ω` (unchanged)

Scope captures (local):

- `pastepreset-batch-20251217-140351/` (filenames include testpoint + current)
  - `U18_OUT_*A.png` (U18 output, AC-coupled)
  - `Q1_*A.png` (Q1 gate, AC-coupled)
  - `R12_*A.png` / `R12_AC耦合.png` (shunt-related node; verify exact point on next pass)

Observed behavior:

- Previous `~100 kHz` high-current limit-cycle seen on the shunt node is no longer dominant in this capture set (shunt ripple stays in the single-digit mVpp range at 3–5 A).
- However, U18 output shows a low-frequency oscillation that grows with current:
  - `U18_OUT_3A`: `~79 mVpp @ ~6.82 kHz`
  - `U18_OUT_5A`: `~139 mVpp @ ~6.20 kHz`
- This `6–7 kHz` band is close to `1/(2π·R17·C13) ≈ 7.2 kHz` for `R17=220Ω`, `C13=100nF`, suggesting the compensation network is near the stability margin at higher current.

## Experiment result: R17 reduced to 100 Ω (2025-12-17)

Test condition (incremental change from the previous section):

- `R17 = 100Ω` (was 220Ω)
- `R11 = 680Ω`, `C48 = 10nF`, `C14 = 10nF`, `C13 = 100nF`, `R9 = 100Ω` unchanged

Scope captures (local):

- `pastepreset-batch-20251217-143011/`
  - `Q1.png` (gate, AC): `~134 mVpp @ ~6.24 kHz`
  - `R24 DC.png` (R42 shunt-top, DC): `Vavg ~250 mV`, `~8.5 mVpp` ripple
  - `R24 AC.png` (R42 shunt-top, AC): `~7.43 mVpp` ripple (frequency readout varies)

Observed / interpretation:

- Reducing `R17` does **not** push the observed low-frequency oscillation upward as a simple `1/(R17·C13)` finger-test would predict.
- It makes the output→sense feed-forward network more capacitive around `6–7 kHz` (because `f_z = 1/(2π·R17·C13)` moves up to ~`15.9 kHz`), which can reduce phase margin and is consistent with the larger coherent oscillation seen at the gate.
- Net: `R17=100Ω` is not a useful direction for damping the `6–7 kHz` behavior; revert to `R17=220Ω` (or move `f_z` downward via larger `C13` instead).

## Experiment result: R17 increased to 1 kΩ (2025-12-17)

Test condition (user report):

- `R17 = 1kΩ`
- Other values assumed unchanged from the previous round (`R11=680Ω`, `C48=10nF`, `C14=10nF`, `C13=100nF`, `R9=100Ω`).

Observed (user report):

- At `3 A`, node `R42` (shunt-top; saved as `R24*` in screenshots) shows a strong `~70 kHz` sinusoid with `Vpp ≥ 170 mV`.

Interpretation / action:

- This indicates a return of a high-frequency oscillation mode (likely tied to the power-stage/gate dynamics) when `R17` is made too large.
- A large `R17` weakens the resistive damping path in the output↔sense feed-forward network at high frequency (leaving mostly capacitive coupling via `C14`), which can reduce stability margin.
- Revert `R17` back to `220Ω` before further tuning, then adjust `C13` upward (instead of increasing `R17`) if the `6–7 kHz` behavior remains.

## Measurement: R17 reverted to 220 Ω @ 5 A (2025-12-17)

Test condition (user report):

- `R17 = 220Ω` (reverted)
- `R11 = 680Ω`, `C48 = 10nF`, `C14 = 10nF`, `C13 = 100nF`, `R9 = 100Ω`
- Current: `5 A`

Scope captures (local):

- `pastepreset-batch-20251217-152515/`
  - `U18_OUT.png` (AC): `~141 mVpp @ ~6.18 kHz`
  - `Q1.png` (AC): `~14.4 mVpp @ ~9.91 kHz`
  - `R24.png` (R42 shunt-top, DC): `Vavg ~248 mV`, `~6.73 mVpp` ripple

Observed / interpretation:

- High-frequency (`~70–100 kHz`) shunt oscillation is not dominant in this configuration.
- Driver output still exhibits a low-frequency (~`6 kHz`) limit-cycle that grows with current, while the gate and shunt node show much smaller ripple.

## Experiment result: C13 increased to 220 nF (2025-12-17)

Test condition (incremental change from the previous section):

- `C13 = 220nF` (was 100nF)
- `R17 = 220Ω`, `R11 = 680Ω`, `C48 = 10nF`, `C14 = 10nF`, `R9 = 100Ω` unchanged

Scope captures (local):

- `pastepreset-batch-20251217-155530/` (likely at `5 A`)
  - `U18_OUT.png` (AC): `~7.7 mVpp @ ~6.15 kHz`
  - `Q1.png` (AC): `~142 mVpp @ ~6.18 kHz`
  - `R24_AC.png` (R42 shunt-top, AC): `~14.0 mVpp @ ~6.25 kHz`

Observed / interpretation:

- The dominant oscillation remains in the `~6 kHz` band, but shunt ripple roughly doubles compared to the prior `C13=100nF` capture set.
- Increasing `C13` strengthens the AC coupling between U18 output and the sense node in this topology and appears to reduce damping around the `~6 kHz` limit-cycle.
- Net: `C13=220nF` is worse for stability in this configuration; revert to `C13=100nF`.

## Experiment result: C13 reverted to 100 nF (2025-12-17)

Test condition:

- `C13 = 100nF` (reverted)
- `R17 = 220Ω`, `R11 = 680Ω`, `C48 = 10nF`, `C14 = 10nF`, `R9 = 100Ω` unchanged

Scope capture (user-provided screenshot):

- `R42_AC` (saved as `R24_AC`): `~7.25 mVpp @ ~32.8 kHz` (AC-coupled, timebase 100 µs/div)
- `U18_OUT`: `~136 mVpp @ ~6.23 kHz` (AC-coupled, timebase 100 µs/div)

Interpretation:

- The `~6.2 kHz` oscillation still exists at the driver/error amp output (`U18_OUT`) with large amplitude, but it is not clearly present at `R42_AC` in this capture (which is dominated by a smaller `~33 kHz` ripple).
- This points to either (a) measurement-point mismatch vs earlier runs, or (b) the loop oscillation being mostly confined to the driver/gate control path at this configuration.

## Input rail ripple / PSU interaction (2025-12-17 → 2025-12-18)

Additional scope sets show that the dominant “problem waveform” is often **on the load input rail** (`LOAD`), and its amplitude is only weakly affected by the CC-loop compensation tweaks, but **strongly affected** by adding **bulk capacitance at the PSU output**.

### Repeatable symptom: ~6.2 kHz on `LOAD`

Across multiple hardware settings, the `LOAD` node shows a clean sinusoid around **6.18–6.48 kHz** with **~0.54–0.65 Vpp**:

- `pastepreset-batch-20251217-181802/LOAD.png`: `~6.48 kHz`, `~539 mVpp`
- `pastepreset-batch-20251217-190812/LOAD.png`: `~6.20 kHz`, `~637 mVpp`
- `pastepreset-batch-20251217-215356/LOAD.png`: `~6.22 kHz`, `~639 mVpp`
- `pastepreset-batch-20251217-225235/LOAD.png`: `~6.18 kHz`, `~650 mVpp`
- `pastepreset-batch-20251217-233157/LOAD.png`: `~6.21 kHz`, `~644 mVpp`
- `pastepreset-batch-20251217-234717/LOAD.png`: `~6.21 kHz`, `~645 mVpp`

The shunt-top node (`R42`, measured to the local ground spring point) often contains the same frequency but at much smaller amplitude:

- `pastepreset-batch-20251217-181802/R42.png`: `~6.42 kHz`, `~7.25 mVpp` (~`0.145 App` if measured across `50 mΩ`)
- `pastepreset-batch-20251217-190812/R42.png`: `~6.20 kHz`, `~8.41 mVpp` (~`0.168 App`)
- `pastepreset-batch-20251217-215356/R42.png`: `~6.21 kHz`, `~7.97 mVpp` (~`0.159 App`)
- `pastepreset-batch-20251217-233157/R42.png`: `~6.19 kHz`, `~10.2 mVpp` (~`0.204 App`)
- `pastepreset-batch-20251217-234717/R42.png`: `~6.20 kHz`, `~9.04 mVpp` (~`0.181 App`)

### Attempted on-board damping: `R64/C58` (no clear improvement with ≤10 µF)

`R64/C58` are a **series RC** branch from `VLOAD_P1` to `VLOAD_N` (a snubber/damper, not the main current path).

- With `R64 = 2.2 Ω`, `C58 = 10 µF`: `pastepreset-batch-20251217-190812/` still shows `LOAD ~637 mVpp @ 6.20 kHz`.
- With `R64 removed` (open): `pastepreset-batch-20251217-215356/` still shows `LOAD ~639 mVpp @ 6.22 kHz`.
- With `R64 = 0 Ω` (C58 directly across rails): user reports no meaningful improvement.

Conclusion: with the available capacitance (≤`10 µF`), the snubber branch cannot significantly change the system at ~`6 kHz`.

### Strong evidence the source side dominates: bulk cap at PSU output reduces the ripple

User added an external electrolytic (`4700 µF`) at the PSU-side parallel connector:

- `pastepreset-batch-20251218-002543/LOAD.png`: `~6.04 kHz`, `~255 mVpp` (ripple reduced by ~2.5× vs ~0.64 Vpp baseline)
- `pastepreset-batch-20251218-002543/R42.png`: `~5.82 mVpp` with the scope’s freq readout locking to a higher-frequency component (`~50 kHz`)

Interpretation:

- The large reduction of `LOAD` ripple with an external bulk capacitor indicates a **PSU/cable/output-network interaction** is a primary contributor.
- The CC loop can still be improved, but expecting `C13/C14/R17/R11` tweaks alone to remove a **~0.6–0.8 Vpp rail oscillation** is unrealistic without adding meaningful input energy storage / damping.

Action item for next PCB revision:

- Add a proper bulk-cap footprint (electrolytic/polymer) close to the VLOAD connector, and provide an intentional damping path (ESR or series R) so the load behaves well across common bench PSUs and cable inductances.

### Follow-up: swapping PSU eliminates the ripple (2025-12-18)

User report: after switching to a different bench PSU, the `LOAD` ripple/oscillation is no longer observable on the scope.

Implication:

- The ~`6 kHz` rail sinusoid is not an intrinsic CC-loop oscillation of the load; it is primarily a **source-side phenomenon** (PSU + cables + output network) that the load excites.
- CC-loop compensation tuning should therefore focus on preventing **internal** HF oscillation modes (70–100 kHz class), while recognizing that “PSU compatibility” may still require input bulk/damping that this PCB currently cannot accommodate.

## System-coupling confirmation: VLOAD input oscillates at same frequency (2025-12-17)

New capture set (local): `pastepreset-batch-20251217-181802/` (5 A)

- `LOAD.png` (VLOAD_P–VLOAD_N, AC): `~539 mVpp @ ~6.48 kHz`
- `R42.png` (shunt-top, AC): `~7.25 mVpp @ ~6.42 kHz` → `~145 mApp` (@ 50 mΩ)

Interpretation:

- The same ~6–7 kHz tone appears on both the **input voltage** and the **shunt current**, which strongly suggests a **PSU / wiring / input impedance ↔ CC loop interaction**, not a purely local small-signal compensation issue.

## Experiment: add input snubber on VLOAD_P1 (2025-12-17)

Hardware change: install series RC across the DC-jack input net:

- `R64 = 2.2Ω` (series)
- `C58 = 10µF` (to VLOAD_N)
- Topology in netlist: `VLOAD_P1 — R64 — $1N220151 — C58 — VLOAD_N` (series RC across VLOAD_P1↔VLOAD_N)

Scope captures (local): `pastepreset-batch-20251217-190812/` (5 A)

- `LOAD.png` (VLOAD_P–VLOAD_N, AC): `~637 mVpp @ ~6.20 kHz`
- `R42.png` (shunt-top, AC): `~8.41 mVpp @ ~6.20 kHz` → `~168 mApp` (@ 50 mΩ)

Result / interpretation:

- This RC does **not** damp the ~6.2 kHz oscillation (amplitude slightly increases and frequency shifts down).
- Critical placement check: the snubber is on `VLOAD_P1` (DC jack). If the PSU is connected to the screw terminal `VLOAD_P` (U8), the snubber only takes effect when `R44` (VLOAD_P ↔ VLOAD_P1 jumper pad) is shorted/bridged.

## Experiment: remove R64 (disable snubber) (2025-12-17)

Hardware change:

- Remove `R64` (open-circuit). (`C58` left in place but isolated.)

Scope captures (local): `pastepreset-batch-20251217-215356/` (5 A)

- `LOAD.png` (VLOAD_P–VLOAD_N, AC): `~639 mVpp @ ~6.22 kHz`
- `R42.png` (shunt-top, AC): `~7.97 mVpp @ ~6.21 kHz` → `~159 mApp` (@ 50 mΩ)

Result / interpretation:

- Removing the snubber does **not** materially change the ~6.2 kHz oscillation (input ripple stays ~0.64 Vpp; shunt ripple changes only slightly).
- Conclusion: the `R64/C58` snubber is not the main control knob for the 6 kHz mode; proceed to tuning the CC loop compensation (U18 network) rather than input damping.

## Next single-knob experiment (proposed)

Goal: add phase lead / reduce loop gain around the ~6 kHz mode without re-triggering the 70–100 kHz oscillation modes.

- Change `R17: 220Ω → 330Ω` (keep `C13=100nF`, `C14=47nF`, `R11=680Ω`, `C48=10nF`, `R9=100Ω` unchanged)
  - Expected effect: move the `R17·C13` zero from ~7.2 kHz down to ~4.8 kHz and increase the effective feedback impedance near 6 kHz, which should improve phase margin and/or reduce the 6 kHz limit-cycle amplitude.

## Experiment: R17 increased to 330 Ω (2025-12-17)

Hardware change:

- `R17 = 330Ω` (was 220Ω)

Scope captures (local): `pastepreset-batch-20251217-225235/` (5 A)

- `LOAD.png` (VLOAD_P–VLOAD_N, AC): `~650 mVpp @ ~6.18 kHz`
- `R42.png` (shunt-top, AC): `~9.58 mVpp` with scope frequency readout `~29.2 kHz` (waveform no longer a clean 6 kHz sinusoid)

Result / interpretation:

- The dominant **input** ripple at ~6.2 kHz remains essentially unchanged (still ~0.65 Vpp).
- The **shunt** waveform loses the clean 6 kHz sinusoid, but total ripple magnitude is still ~10 mVpp (~0.2 App if interpreted as sinusoidal).
- Net: `R17=330Ω` alone is not sufficient to suppress the 6 kHz system oscillation; next knob should reduce CC loop gain/bandwidth around a few kHz (e.g. increase `C14`) while keeping the 70–100 kHz modes suppressed.

## Experiment: C14 increased to 100 nF (2025-12-17)

Hardware change:

- `C14 = 100nF` (was 47nF)
- Keep `R17 = 330Ω`, `C13 = 100nF`, `R11 = 680Ω`, `C48 = 10nF`, `R9 = 100Ω`
- Input snubber remains disabled: `R64` removed (open)

Scope captures (local): `pastepreset-batch-20251217-233157/` (5 A)

- `LOAD.png` (VLOAD_P–VLOAD_N, AC): `~644 mVpp @ ~6.21 kHz`
- `R42.png` (shunt-top, AC): `~10.2 mVpp @ ~6.19 kHz` → `~204 mApp` (@ 50 mΩ)

Result / interpretation:

- The dominant ~6.2 kHz oscillation remains on the input voltage at essentially the same amplitude.
- Shunt current ripple is still ~0.16–0.20 App order; increasing `C14` does not damp the 6 kHz mode.

## Experiment: add C56 (gate-source) = 47 nF (2025-12-17)

Hardware change:

- Install `C56 = 47nF` between `Q1_G ($1N395)` and shunt-top / source (`$1N151181`).

Scope captures (local): `pastepreset-batch-20251217-234717/` (5 A)

- `LOAD.png` (VLOAD_P–VLOAD_N, AC): `~645 mVpp @ ~6.21 kHz`
- `R42.png` (shunt-top, AC): `~9.04 mVpp @ ~6.20 kHz` → `~181 mApp` (@ 50 mΩ)

Result / interpretation:

- Adding `C56` does not reduce the dominant ~6.2 kHz oscillation on the input voltage.
- Shunt ripple improves only slightly; the 6 kHz mode remains a system-level issue.

## Experiment: short R64 (make C58 a direct input capacitor) (2025-12-17)

Hardware change:

- Set `R64 = 0Ω` (short), so `C58 = 10µF` becomes a direct capacitor across `VLOAD_P1`↔`VLOAD_N`.

Observed (user report):

- No improvement: `VLOAD_P–VLOAD_N` ripple remains ~`600 mVpp` (dominant tone unchanged).
- PSU output measured at the bench supply terminals shows even larger ripple: ~`800 mVpp` (same regime as the ~6 kHz oscillation).

Interpretation:

- The oscillation is already present at the supply output and is not materially changed by a small 10 µF local capacitor.
- This supports the hypothesis that the ~6 kHz issue is a **PSU control-loop / cable impedance ↔ electronic load interaction**, and that solving it likely needs either (a) a different PSU, or (b) significantly more local bulk capacitance with appropriate damping/ESR.

## Experiment: add 4700 µF electrolytic at PSU output (2025-12-18)

Hardware change (external):

- Add a `4700µF` electrolytic capacitor in parallel at the bench PSU side (another parallel output connector).

Scope captures (local): `pastepreset-batch-20251218-002543/` (5 A)

- `LOAD.png` (VLOAD_P–VLOAD_N, AC): `~255 mVpp @ ~6.04 kHz` (previously ~600–650 mVpp @ ~6.2 kHz)
- `R42.png` (shunt-top, AC): `~5.82 mVpp` with scope frequency readout `~50.4 kHz`

Result / interpretation:

- The ~6 kHz input ripple drops by ~2–3× with added bulk electrolytic at the PSU side.
- This is strong evidence that the dominant 6 kHz oscillation is primarily a **bench PSU / cable impedance ↔ electronic-load interaction** (source impedance shaping matters a lot).
- It does not fully absolve the load loop (it participates), but it means “fixing only U18 compensation” is unlikely to solve it reliably across different PSUs.

## Recommendation (stop-the-bleed, based on observed evidence)

Given that:

- The dominant ~6.2 kHz tone is measurable at the PSU output terminals, and
- Adding large **electrolytic bulk** at the PSU side reduces the ripple by ~2–3×, while on-board small MLCC/comp tweaks do not,

the fastest practical mitigation is to **shape the source impedance** seen by the load.

Recommended “final” component setup for this bench PSU + 5 A CC use-case:

- **External (required for stability with the tested PSU):**
  - Add `≥4700µF` electrolytic directly across PSU output (or as close as possible to the load along the cable).
  - If you need more margin: go to `2×4700µF` (total ~9400µF).
- **On-board CC loop (keep in the safe region we already mapped):**
  - `R11 = 680Ω`
  - `R9 = 100Ω`, `C48 = 10nF`
  - `C13 = 100nF`
  - `R17 = 220Ω`
  - `C14 = 47nF` (this is the last value before the unsuccessful R17/C14/C56 sweeps, and it yielded the lowest pre-electrolytic VLOAD ripple in our captures)
  - `C56` (gate-source): DNP (47nF did not reduce the 6 kHz VLOAD oscillation and unnecessarily slows the power stage)

Notes:

- This does not claim the load loop is “perfect”; it claims the 6 kHz issue is dominated by **PSU interaction** and that the above is the only knob proven to move it significantly in this debug session.
- Next: tune the compensation around `U18` (likely via `C14`) to reduce the `~6 kHz` oscillation at `U18_OUT` without reintroducing the old `~70–100 kHz` mode.

## Experiment result: C14 increased to 47 nF (2025-12-17)

Test condition (incremental change from the previous section):

- `C14 = 47nF` (was 10nF)
- `R11 = 680Ω`, `R17 = 220Ω`, `C13 = 100nF`, `C48 = 10nF`, `R9 = 100Ω` unchanged

Scope captures (local):

- `pastepreset-batch-20251217-173650/`
  - `U18_OUT.png` (AC): `~125 mVpp @ ~6.20 kHz`
  - `R42.png` (AC, shunt): `~7.79 mVpp @ ~6.21 kHz`

Interpretation:

- The dominant `~6.2 kHz` oscillation persists at `U18_OUT` with only a small amplitude change, and it now clearly appears at the shunt (`R42`) as well.
- This suggests `C14` alone is not the right knob to eliminate the `~6 kHz` mode; the limiting phase likely sits in the power stage / gate dynamics (e.g., `R11` + MOSFET capacitances) and/or supply interaction.

## Evidence: PSU / cabling interaction at ~6 kHz (2025-12-17)

User observation: the same dominant frequency is visible at the **load input voltage** as well as at the **shunt**.

Scope captures (local):

- `pastepreset-batch-20251217-181802/`
  - `LOAD.png` (VLOAD_P vs VLOAD_N, AC): `~539 mVpp @ ~6.48 kHz`
  - `R42.png` (shunt, AC): `~7.25 mVpp @ ~6.42 kHz` (≈ `145 mApp` through `R42=50mΩ`)

Interpretation:

- This is strong evidence that the instability is a **combined system oscillation** (PSU output impedance + cabling inductance + load input + CC loop), not purely an internal CC-loop pole/zero issue.
- In this case, tweaking only `C13/C14/R17/R11` may change the symptom but not eliminate the root cause unless the load's input impedance is shaped to be stable with the source.

## Next experiment: add input damping without electrolytic (RC damper) (pending)

Constraint: no large electrolytic available.

Rationale:

- The `~6.4 kHz` oscillation is visible on both the input voltage (`VLOAD_P-VLOAD_N`) and shunt current (`R42`), indicating a source↔load interaction.
- A small MLCC placed directly across the input has very low ESR and limited effective capacitance (DC-bias), so it often **shifts** a resonance upward rather than damping it.
- A **series RC branch** across the input provides a resistive loss term at the oscillation frequency without drawing DC current (capacitor blocks DC).

Recommended starting values (one change):

- Solder a series `R + C` branch across `VLOAD_P` and `VLOAD_N`, as close to the input terminals as practical:
  - `C = 10 µF` (X7R/X5R, prefer `≥25 V`, ideally `≥50 V` if your bus can be high)
  - `R = 2.2 Ω` (any small resistor `≥0.25 W` is fine; DC current is blocked, dissipation should be small)

Validation (one capture):

- At `5 A`, re-capture `LOAD` (VLOAD_P vs VLOAD_N, AC-coupled) and note the new `~6 kHz` Vpp.

Note (netlist updated 2025-12-17):

- After replacing the analog netlist with `Netlist_Power_rev4_1_2025-12-17.enet`, the reserved damper is present:
  - `R64` + `C58` form a **series RC** from `VLOAD_P1` → `VLOAD_N` (`VLOAD_P1` — `R64` — `$1N220151` — `C58` — `VLOAD_N`).
  - `R44` links `VLOAD_P` ↔ `VLOAD_P1`, so if `R44` is populated/shorted, the RC damper also effectively sits across `VLOAD_P` ↔ `VLOAD_N`.
