# UI Components

## DashboardExtendedVoltageToggle

- Scope: internal
- Owner: digital firmware UI
- Bounds: keep the existing left dashboard button bounds `(198,118)-(277,145)`.
- States:
  - `safe5v_only`: gray, means `allow_extended_voltage=false`
  - `extended_allowed`: blue, means `allow_extended_voltage=true` and no current failure latch
  - `extended_failed`: red, means `allow_extended_voltage=true` and the latest non-Safe5V request hit the existing failure criteria
- Interaction: tap toggles `allow_extended_voltage`; no long-press or secondary gesture.
- Contract note: the approved copy is fixed to `PD` on the first line and `5V` / saved target voltage on the second line.

## DashboardPdSettingsEntry

- Scope: internal
- Owner: digital firmware UI
- Bounds: keep the existing right circular button bounds `(287,118)-(314,145)`.
- Appearance: settings-entry affordance only; do not encode PD success/failure colors here.
- Interaction: tap enters `UiView::PdSettings`.
- Contract note: the on-screen LOAD toggle is removed from the main dashboard; this entry does not toggle load output.
