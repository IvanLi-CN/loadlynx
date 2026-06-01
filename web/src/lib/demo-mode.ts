export const DEMO_MODE_STORAGE_KEY = "loadlynx.demoMode";

type DemoModeLocation = Pick<Location, "href" | "pathname" | "search">;

export function parseDemoModeParam(search: string): boolean | null {
  const value = new URLSearchParams(search).get("demo");

  if (value === "true") return true;
  if (value === "false") return false;
  return null;
}

export function readStoredDemoMode(storage: Storage): boolean | null {
  try {
    const value = storage.getItem(DEMO_MODE_STORAGE_KEY);

    if (value === "true") return true;
    if (value === "false") return false;
    return null;
  } catch {
    return null;
  }
}

export function writeStoredDemoMode(storage: Storage, value: boolean) {
  try {
    storage.setItem(DEMO_MODE_STORAGE_KEY, value ? "true" : "false");
  } catch {
    // Best-effort only; URL parameters still control the current navigation.
  }
}

export function resolveDemoMode(
  location: DemoModeLocation,
  storage: Storage,
): boolean {
  const paramValue = parseDemoModeParam(location.search);

  if (paramValue !== null) {
    writeStoredDemoMode(storage, paramValue);
    return paramValue;
  }

  const storedValue = readStoredDemoMode(storage);
  if (storedValue !== null) return storedValue;

  return false;
}

export function stripDemoModeParam(location: DemoModeLocation): string | null {
  const nextUrl = new URL(location.href);

  if (!nextUrl.searchParams.has("demo")) return null;

  nextUrl.searchParams.delete("demo");
  return `${nextUrl.pathname}${nextUrl.search}${nextUrl.hash}`;
}
