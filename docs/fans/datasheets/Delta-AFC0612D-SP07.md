# Delta AFC0612D-SP07 — Specification (Markdown)

Source PDF: https://www.delta-fan.com/Download/Spec/AFC0612D-SP07.pdf

Customer  S T D  
Description  D C  F A N  
Delta Model No.  AFC0612D-SP07  
REV. 00  
Sample Issue Date  DEC.31.2007

## 1. Scope
This specification defines the electrical and mechanical characteristics of the DC brushless axial flow fan. The fan motor is two-phase, four-pole.

## 2. Characters
- Rated voltage: 12 VDC
- Operation voltage: 10.8 – 13.8 VDC
- Input current (avg.): 0.13 A (0.60 A max.)
- Input power (avg.): 1.56 W (7.20 W max.)
- Speed: 4250 RPM ±10%
- Max. air flow (at zero static pressure): 0.662 m³/min (min. 0.596) / 23.38 CFM (min. 21.052)
- Max. air pressure (at zero airflow): 5.35 mmH₂O (min. 4.33) / 0.211 inchH₂O (min. 0.170)
- Acoustical noise (avg.): 34.0 dB-A (max. 38.0 dB-A)
- Insulation type: UL Class A

Additional electrical specs:
- Insulation resistance: 10 MΩ min. at 500 VDC (between frame and (+) terminal)
- Dielectric strength: 5 mA max. at 500 VAC 50/60 Hz for 1 minute (between frame and (+) terminal)
- External cover: Open type
- Life expectancy (L10): 70,000 hours continuous operation at 40°C with 15–65% RH
- Rotation: Clockwise, viewed from nameplate side
- Over-current shutdown: Current will shut down when rotor is locked
- Lead wires (UL1061 AWG #26):
  - Black: Negative (-)
  - Red: Positive (+)
  - Blue: Frequency (FG) [for -F00]
  - Yellow: PWM speed control

Notes: Measurements after 10 minutes warm-up. Values in parentheses are limit specs. Acoustic noise measured in free air at rated voltage, 1 m from intake, anechoic chamber.

## 3. Mechanical
- Dimensions: See drawing (60×60×25.4 mm class)
- Frame: Plastic UL 94V-0
- Impeller: Plastic UL 94V-0
- Bearing system: Two ball bearings
- Weight: 80 g (ref.)

## 4. Environmental
- Operating temperature: −10 to +70 °C
- Storage temperature: −40 to +75 °C
- Operating humidity: 5 to 90 % RH
- Storage humidity: 5 to 95 % RH

## 5. Protection
- Locked rotor protection: Motor winding impedance protects motor under 96 hours locked-rotor at rated voltage
- Polarity protection: Withstands reverse connection of (+) and (−) leads

## 6. Ozone Depleting Substances
No containing PBBs, PBBOs, CFCs, PBBEs, PBDPEs, HCFCs. RoHS compliant.

## 7. Production Location
China or Thailand or Taiwan

## 8. P–Q Curve
Refer to source PDF for chart (12 V @ 4250 RPM typical).

## 9. Dimension Drawing
Refer to source PDF. Label and mounting per standard 60 mm square fan (60.0 ±0.5 mm, 8×Ø4.5 mm holes, 25.4 mm thickness).

## 10. Frequency Generator (FG) Signal
- Output: Open collector
- Max VFG: 13.8 V
- IC max: 5 mA
- VCE(sat): 0.5 V max
- Waveform: See PDF; 4-pole motor reference

## 11. PWM Control Signal
- Acceptable PWM frequency: 30 Hz to 300 kHz
- Preferred PWM frequency: 25 kHz
- Behavior: 100% = max speed; 0% = min speed; if PWM lead disconnected, fan runs at max speed
- Start capability: At 25 kHz, 30% duty, the fan can start from a dead stop

## 12. Speed vs. PWM (12 V, ~25 kHz)
- 100% duty: 4250 ±10% RPM; current typ. 0.13 A
- 0% duty: 750 ±250 RPM; current typ. 0.03 A

## 13. PWM Control Lead Input Impedance
See PDF schematic; default to max speed if PWM input is left unconnected.

## Application Notices (Excerpt)
1) Performance not guaranteed outside specified conditions.  
2) Submit written request for deviations.  
3) Handle with care; avoid force on impeller, pulling leads, or drops.  
4) Not guaranteed against ingress of powder/water/insects unless specified.  
5) Conditions are representative examples.  
6) Ensure correct polarity before powering.  
7) Not suitable for corrosive environments unless specified.  
8) Follow storage limits; re-test if stored >6 months.  
9) Not all fans have lock-rotor protection.  
10) Mount correctly to avoid resonance/vibration/noise.  
11) Use suitable fan guard during testing.  
12) Tests at 25°C, 65% RH unless stated; values are for fan performance itself.  
13) Use ≥4.7 µF external capacitor when using multiple fans in parallel.

