# Gate-Compensation Tuning Log (NMOS CC Loop)

Context: Analog board NMOS linear load (IRFP4468, OPA2365 driver, INA193 20× sense). Issue: large current ripple / self-oscillation while targeting 1 A (DAC ≈59.7 mV → ≈0.5 V at INA193).

## Adjustable nodes in this round

- R11: gate series resistor (default 1 kΩ at start of log)
- C13: op-amp output to GND (dominant pole)
- C14 + R17: lead network (zero) from op-amp output to GND
- R9/R10: 100 Ω input resistors (kept constant)

## Iteration timeline (latest at bottom)

1. **Baseline after R11=100 Ω**: Ripple ~21 kHz, 36 mVpp on shunt.
2. **C14=6.8 nF, C13=10 nF**: Ripple worsened (~24 kHz).
3. **C14 DNP, C13=100 nF**: HF oscillation ~124 kHz appeared.
4. **R11↑ to 330 Ω**: Oscillation ~75 kHz.
5. **C14=2.2 nF (R17=1 kΩ)**: Moved to ~37 kHz, amplitude still large.
6. **C13=47 nF, C14=22 nF, R11=330 Ω**: Settled near 7 kHz but still oscillatory; shunt average low.
7. **R17=2.2 kΩ, C14=22 nF, C13=100 nF, R11=1 kΩ**: ~72 kHz oscillation.
8. **C14=1 nF, R17=10 kΩ, C13≈220 nF, R11=1 kΩ**: ~11 kHz oscillation; INA193 shows ~0.5 V avg (≈1 A) but shunt avg low due to limit-cycle.
9. **C14 stepped to 4.7 nF → 1 nF → 470 pF → 220 pF with R17=10 kΩ; C13 increased to 470 nF**: Oscillation migrated 34 kHz → 55 kHz → 80 kHz; average current improved slightly but still ~0.5 A; limit-cycle persists.
10. **Current state (2025-11-24)**  
    - R11 = 1 kΩ  
    - C13 ≈ 470 nF (single 470 nF)  
    - C14 = 220 pF  
    - R17 = 10 kΩ  
    - R9/R10 = 100 Ω  
    - Observed: ~80 kHz oscillation on shunt, ~35 mVpp, average ≈15 mV (≈0.6 A).

## Observations

- Limit-cycle behavior persists across wide compensation sweeps; moving zero/pole only shifts oscillation frequency.
- INA193 output shows average near target when loop is in relaxation oscillation, but shunt average is depressed because gate drive hits saturation and releases cyclically.
- Strong contributors: large MOSFET Ciss in linear mode, multiple high-frequency poles (op-amp output, gate RC, sense amplifier delay), and lack of extra damping elements.

## Next hypotheses (require confirming with user before changes)

- Further lower bandwidth: C13 → 1 µF, R11 → 330–470 Ω while keeping C14 small (≈220 pF) and R17=10 kΩ.
- Add gate snubber (not allowed in current constraint) or reduce INA193 gain (not allowed) if more degrees of freedom become permissible.
