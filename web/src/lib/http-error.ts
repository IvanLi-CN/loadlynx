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
