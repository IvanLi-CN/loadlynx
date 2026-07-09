# USB CDC JSONL bridge

LoadLynx digital firmware and `loadlynx-devd` use LF-delimited JSON frames on the ESP32-S3 USB CDC channel.

## Framing

- Encoding: UTF-8 JSON object followed by `\n`.
- Protocol identifier: `loadlynx.cdc.v1`.
- Each host request includes `request_id`; every `response` or `error` echoes it.
- `psk` and equivalent secret fields are redacted before traces or diagnostics leave devd.

## Frames

### `hello`

```json
{
  "type": "hello",
  "protocol": "loadlynx.cdc.v1",
  "identity": {
    "device_id": "loadlynx-aabbcc",
    "firmware": {
      "target": "digital_esp32s3",
      "features": ["net_http", "mdns_dns_sd", "usb_cdc_jsonl"]
    }
  },
  "capabilities": [
    "get_identity",
    "get_status",
    "get_pd",
    "set_pd_policy",
    "set_output_enabled",
    "set_cc_target",
    "get_control",
    "set_control",
    "get_presets",
    "set_preset",
    "apply_preset",
    "get_calibration_profile",
    "calibration_apply",
    "calibration_commit",
    "calibration_reset",
    "calibration_mode",
    "get_wifi_status",
    "get_wifi_credentials",
    "set_wifi_config",
    "clear_wifi_config",
    "soft_reset",
    "get_diagnostics"
  ]
}
```

### `request`

Supported `op` values are `get_identity`, `get_status`, `get_pd`, `set_pd_policy`, `set_output_enabled`, `set_cc_target`, `get_control`, `set_control`, `get_presets`, `set_preset`, `apply_preset`, `get_calibration_profile`, `calibration_apply`, `calibration_commit`, `calibration_reset`, `calibration_mode`, `get_wifi_status`, `get_wifi_credentials`, `set_wifi_config`, `clear_wifi_config`, `soft_reset` and `get_diagnostics`.

```json
{
  "type": "request",
  "request_id": "req-1",
  "op": "get_status"
}
```

`set_output_enabled` accepts `enable`. `set_cc_target` is an alias for the same firmware handler. When `target_i_ma` is present, firmware applies the active preset as CC mode with that current target before enabling or disabling output:

```json
{
  "type": "request",
  "request_id": "req-2",
  "op": "set_output_enabled",
  "enable": true,
  "target_i_ma": 2000
}
```

Control, preset and calibration ops reuse the HTTP/Web JSON payload shapes from `docs/interfaces/network-http-api.md`, but remain compact JSONL requests rather than full HTTP requests over USB. `get_status` and `get_control` are part of the saved-device USB compat read path and should stay on compact USB response bodies: `get_status` returns the current `status`, `link_up`, `hello_seen`, `analog_state` and compact control summary, while `get_control` returns `active_preset_id`, `output_enabled`, `uv_latched` and the active `preset`. The `get_calibration_profile` firmware response may use the compact `cal_profile_v1` data shape (`a`, `c1`, `c2`, `vl`, `vr` arrays); devd expands it back to the HTTP/Web profile shape before serving CLI or Web callers. WiFi config ops use `ssid`, `psk` and optional `wait`, but Web/devd writes use `wait=false` and treat the firmware response as storage acknowledgement; LAN association/disconnection is observed through later `get_wifi_status` polling. Status responses and diagnostics must never echo PSK. `get_wifi_credentials` is the explicit backup-export exception and returns `{ ssid, psk, source }` as plaintext to the caller.

### `response`

```json
{
  "type": "response",
  "request_id": "req-1",
  "ok": true,
  "data": {}
}
```

### `error`

```json
{
  "type": "error",
  "request_id": "req-1",
  "error": {
    "code": "LINK_DOWN",
    "message": "UART link is down",
    "retryable": true
  }
}
```

### `status` and `log`

`status` frames carry the same snapshot shape as `GET /api/v1/status`. `log` frames are structured firmware logs with `level`, `target`, `message` and optional `fields`.

## Ownership

The browser and user-facing CLI never write directly to this channel. Web and CLI operations use devd's internal lease protocol, and `loadlynx-devd` is the single owner of the USB CDC port inside one daemon process.

For each physical USB port, devd runs one serial owner while any lease for that port is active. JSONL commands from multiple clients are queued through that owner, devd assigns a unique `request_id`, and a command succeeds only when a matching response frame is received. Other response IDs are recorded as trace evidence and do not satisfy the request.

For the Web `/cc` owner-facing fast-status path, the serial owner may keep one lease-scoped background `get_status` refresh at about `200 ms` and publish those matched results into the USB compat status cache. Web callers that explicitly opt into `GET /api/v1/status?cache=true` then consume that cache instead of opening a fresh USB request on every page tick. This caps USB status pressure per physical port rather than per tab.

ESP32-S3 USB Serial/JTAG may interleave binary log bytes with JSONL response text. devd prefers a complete matching `request_id` response. For the saved-device USB compat read path (`get_status` and `get_control`) and output-control operations, it may recover a response only from frames observed after the matching transmit frame and only when the recovered payload has the expected operation shape; unrelated or mismatched `request_id` frames still do not satisfy the command. `serial_response_timeout`, `serial_response_mismatch`, `serial_response_missing` and `serial_response_invalid` are bounded retryable serial response gap failures for these reads, and each retry window must remain traceable in devd session evidence.

Monitor/log/event reads consume devd's bounded in-memory session state and do not open the serial port. The serial owner is also the only reader while active, so unsolicited `hello`, `status` and `log` frames are recorded and broadcast from one place.

Flash/reset flows that invoke tools such as `espflash --port` are exclusive. devd closes the serial owner before running the tool and returns a clear busy/in-progress error to same-port JSONL commands until the exclusive operation is finished.

`loadlynx-devd` owns the USB CDC session for Web/CLI control-plane verification. This path does not use external MCU daemon selector state; its CLI default digital USB port memory reuses `.esp32-port`.

When validating against hardware, set the default ESP32-S3 digital USB CDC port through the CLI, such as `loadlynx usb-port set digital /dev/cu.usbmodemXXXX`, when the owner has identified the intended device. CLI/devd operations then use that project-local memory as the hardware target, reading only the port path line if `.esp32-port` also contains selector metadata such as `mac=...`. A passing hardware validation must include protocol-level evidence from that port: a decoded `hello`, a successful `get_identity`, a successful `get_status`, or equivalent JSONL request/response frames.

Serial-open-only checks, candidate discovery, Web lease creation, mock identity/status data, and firmware dry-run target evidence are diagnostic signals only. They are not sufficient proof that the Web/devd USB CDC control plane works on the real device.
