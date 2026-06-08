export function getResetDraftConfirmConfig(draftEmpty: boolean) {
  return {
    title: "Reset Draft (Web only)",
    body: "This clears the local draft (user calibration points). The device is unchanged.",
    details: [
      "Affects: v_local, v_remote, current_ch1, current_ch2 (local draft only).",
      "Writes device: No.",
      "This clears all local draft points (export first if needed).",
    ],
    confirmLabel: "Reset Draft",
    destructive: false,
    confirmDisabled: draftEmpty,
  };
}

export function getCopyCh1ToCh2ConfirmConfig(sourceLabel: string) {
  return {
    title: "Copy CH1 → CH2 (Draft)",
    body: "This overwrites CH2 draft points with CH1 calibration points. The device is unchanged.",
    details: [
      "Affects: current_ch2 (local draft only).",
      `Source: current_ch1 (${sourceLabel}).`,
      "Writes device: No.",
      "Irreversible locally: Yes (export draft first if needed).",
    ],
    confirmLabel: "Copy",
    destructive: false,
  };
}

export function getResetCurrentDeviceConfirmConfig(
  curve: "current_ch1" | "current_ch2",
  channelLabel: string,
) {
  return {
    title: `Reset Device Calibration (Current ${channelLabel})`,
    body: "This resets current calibration on the device.",
    details: [
      `Affects: ${curve}.`,
      "Writes device: Yes.",
      "Irreversible: Yes (re-calibrate + commit to recover).",
    ],
    confirmLabel: "Reset",
    destructive: true,
  };
}

export function getResetVoltageDeviceConfirmConfig() {
  return {
    title: "Reset Device Calibration (Voltage)",
    body: "This resets voltage calibration on the device.",
    details: [
      "Affects: v_local + v_remote.",
      "Writes device: Yes.",
      "Irreversible: Yes (re-calibrate + commit to recover).",
      "Does not affect: current_ch1/current_ch2.",
    ],
    confirmLabel: "Reset",
    destructive: true,
  };
}
