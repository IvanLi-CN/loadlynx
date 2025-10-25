# CAIZHU-FAN 6025 — DC Axial Fan (12 V Variant)

Source references:

- Vendor specification snapshot (“产品参数单”) captured in project documentation
- [Devicemart listing CNT-OFM078](https://www.devicemart.co.kr/goods/view?no=14944374) — 60×60×25 mm, 12 V, 5500 RPM fan data

## Key Parameters (12 V version)

| Item | Spec |
| --- | --- |
| Frame size | 60 mm × 60 mm × 25 mm |
| Rated voltage | 12 V DC |
| Operating voltage | 10.8 – 13.2 V DC (typical vendor range) |
| Rated current | 0.21 – 0.28 A (vendor listings vary; project batch measured 0.283 A) |
| Input power | ≈ 2.5 – 3.4 W |
| Rated speed | 5 500 RPM ± 10 % |
| Air flow | ≈ 26.3 CFM (0.0125 m³/s) |
| Static pressure | ≈ 61 Pa (6.22 mmH₂O) |
| Acoustic noise | ≈ 38 dB(A) |
| Weight | ~55 g |
| Connector | XH2.54 2-pin (as shipped) |

## Construction

- Motor: Brushless DC, fluid/sleeve bearing (vendor dependent)
- Materials: Plastic frame and impeller
- Intended use: General-purpose electronics cooling, 3D printer accessories

## Notes

- Values consolidate multiple vendor listings; verify against incoming batch labels when selecting operating limits.
- Noise and current can vary with supply voltage and PWM drive — recharacterise if using below 12 V.
- Ensure adequate inlet sealing; low static pressure makes the fan sensitive to bypass leakage.

