# USB CDC JSONL bridge

LoadLynx digital firmware and `loadlynx-devd` use LF-delimited JSON frames on the ESP32-S3 USB CDC channel.

## Framing

- Encoding: UTF-8 JSON object followed by `\n`.
- Protocol identifier: `loadlynx.cdc.v1`.
- Each host request includes `request_id`; every `response` or `error` echoes it.
- `wifi_config.psk` and equivalent secret fields are redacted before traces leave devd.

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
  "capabilities": ["get_identity", "get_status", "wifi_config"]
}
```

### `request`

Supported `op` values are `get_identity`, `get_status`, `set_log_level`, `set_output_enable`, `set_setpoint`, `set_pd_policy`, `set_calibration_mode` and `wifi_config`.

```json
{
  "type": "request",
  "request_id": "req-1",
  "op": "get_status"
}
```

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

The browser never writes directly to this channel. Web operations acquire a devd per-device lease first; CLI operations use an exclusive devd session or direct LAN API where available.
