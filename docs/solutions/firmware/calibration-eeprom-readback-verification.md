# Calibration EEPROM Readback Verification

## Context

The digital board permits RAM-only calibration Apply before an operator decides to persist the four-curve profile. A commit response must therefore prove persistence rather than infer it from the active RAM points.

## Failure Mode

Comparing an incoming curve to the active RAM curve is not a persistence check. When Apply had already installed the same curve, that comparison let Commit return success without an EEPROM write. The next boot then loaded the factory fallback.

## Resolution

Build a complete candidate profile from the active profile plus the requested curve. Serialize and deserialize that candidate to validate format, hardware revision, CRC, and all curves. Write the serialized blob, then read EEPROM in bounded pages and compare every byte to the candidate blob. Publish the candidate to RAM only after all checks pass.

Expose the boot load result and the last write result through calibration profile and diagnostics. Keep `ram-only` distinct from `commit-verified` so an operator can determine whether a user profile survived reboot.

## Guardrails

- Never optimize Commit based solely on current RAM calibration points.
- Do not allocate a second full EEPROM blob on the HTTP task stack for readback verification; use page-sized reads.
- After every page write, use the EEPROM's address-only ACK polling rather than a random-read probe; write-cycle readiness must not depend on the internal address pointer.
- Treat any write, readback, format, hardware-revision, CRC, or profile mismatch as unavailable persistence and retain the existing active profile.
