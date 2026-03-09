# HTTP APIs

## GET /api/v1/pd

- Add `allow_extended_voltage: boolean` to the response body.
- `saved` keeps the persisted PD configuration payload shape.
- `allow_extended_voltage=false` means the effective runtime policy is Safe5V, regardless of the saved PD target.

## PUT /api/v1/pd

- Accept optional `allow_extended_voltage: boolean` in the request body.
- If omitted, keep the previous persisted value.
- Updating `saved` fields must not implicitly force `allow_extended_voltage=true`.
- Runtime effect:
  - `allow_extended_voltage=true`: use the saved PD configuration as the effective policy.
  - `allow_extended_voltage=false`: persist the saved PD configuration, but keep/apply Safe5V as the effective policy.
