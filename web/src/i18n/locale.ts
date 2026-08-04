export function resolveInitialLocale(
  isStorybook: boolean,
  readStoredLocale: () => string | null,
): string {
  return isStorybook ? "zh-CN" : (readStoredLocale() ?? "zh-CN");
}
