export function requireDeviceBaseUrl(baseUrl: string | undefined): string {
  if (!baseUrl) {
    throw new Error("Device base URL is not available");
  }
  return baseUrl;
}
