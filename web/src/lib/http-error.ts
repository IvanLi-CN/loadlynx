import type { HttpApiError } from "../api/client.ts";

export function formatHttpApiErrorSummary(error: HttpApiError): string {
  const code = error.code ?? "HTTP_ERROR";
  return `${code} — ${error.message}`;
}

export function isUnsupportedOperationError(error: HttpApiError): boolean {
  return error.status === 404 && error.code === "UNSUPPORTED_OPERATION";
}

export function isLinkUnavailableError(error: HttpApiError): boolean {
  return error.code === "LINK_DOWN" || error.code === "UNAVAILABLE";
}

export function isUsbSerialUnavailableError(error: HttpApiError): boolean {
  return (
    error.code === "web_session_expired" ||
    error.code === "web_session_required" ||
    error.code === "device_busy" ||
    error.code === "operation_in_progress" ||
    error.code === "QUEUE_WAIT_TIMEOUT" ||
    error.code === "REQUEST_TIMEOUT" ||
    error.code === "serial_open_failed" ||
    error.code === "serial_operation_timeout" ||
    error.code === "serial_response_timeout" ||
    error.code === "serial_response_mismatch" ||
    error.code === "serial_request_failed" ||
    error.code === "serial_owner_stopped"
  );
}

export function isAnalogNotReadyError(error: HttpApiError): boolean {
  return error.code === "ANALOG_NOT_READY" || error.code === "NOT_ATTACHED";
}

export function getNetworkErrorHint(baseUrl?: string): string {
  return (
    "无法连接设备" +
    (baseUrl ? `（baseUrl=${baseUrl}）` : "") +
    "，请检查网络与 IP 设置。"
  );
}

export function getUsbSerialErrorHint(baseUrl?: string): string {
  return (
    "USB/devd 管理通道不可用：lease 可能已过期、串口正忙，或设备没有返回匹配响应" +
    (baseUrl ? `（baseUrl=${baseUrl}）` : "") +
    "。请等待当前请求收口，必要时重新绑定 USB 连接。"
  );
}
