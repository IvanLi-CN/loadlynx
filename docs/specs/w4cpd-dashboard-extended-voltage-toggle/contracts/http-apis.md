# HTTP APIs

## GET /api/v1/pd

- Add `allow_extended_voltage: boolean` to the response body.
- `saved` keeps the persisted PD configuration payload shape.
- Fixed-mode `saved.target_mv` is the persisted PDO voltage for the saved selection; it is not normalized on read against the currently attached source.
- `allow_extended_voltage=false` means the effective runtime policy is Safe5V, regardless of the saved PD target.

## PUT /api/v1/pd

- Accept optional `allow_extended_voltage: boolean` in the request body.
- If omitted, keep the previous persisted value.
- Updating `saved` fields must not implicitly force `allow_extended_voltage=true`.
- Updating `saved` fields still validates against the currently attached PD source capabilities; if no PD status is available, only `allow_extended_voltage` may be updated.
- Runtime effect:
  - `allow_extended_voltage=true`: use the saved PD configuration as the effective policy.
  - `allow_extended_voltage=false`: persist the saved PD configuration, but keep/apply Safe5V as the effective policy.
