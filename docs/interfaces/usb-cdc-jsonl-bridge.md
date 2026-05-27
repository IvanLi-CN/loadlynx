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
  "capabilities": ["get_identity", "get_status", "get_pd", "set_pd_policy"]
}
```

### `request`

Supported `op` values are `get_identity`, `get_status`, `get_pd`, `set_pd_policy`, and `set_output_enabled`.

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

`loadlynx-devd` owns the USB CDC session for Web control-plane verification. This path does not use `mcu-agentd` or `mcu-agentd selector` state; its CLI default digital USB port memory reuses `.esp32-port`.

When validating against hardware, set the default ESP32-S3 digital USB CDC port through the CLI, such as `loadlynx usb-port set digital /dev/cu.usbmodemXXXX`, when the owner has identified the intended device. CLI/devd operations then use that project-local memory as the hardware target, reading only the port path line if `.esp32-port` also contains selector metadata such as `mac=...`. A passing hardware validation must include protocol-level evidence from that port: a decoded `hello`, a successful `get_identity`, a successful `get_status`, or equivalent JSONL request/response frames.

Serial-open-only checks, candidate discovery, Web lease creation, mock identity/status data, and firmware dry-run target evidence are diagnostic signals only. They are not sufficient proof that the Web/devd USB CDC control plane works on the real device.
