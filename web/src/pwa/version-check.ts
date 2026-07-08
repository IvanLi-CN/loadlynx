export interface AppVersionPayload {
  version?: string | null;
  builtAt?: string | null;
}

function normalize(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

export function hasRemoteAppUpdate(
  currentVersion: string | null | undefined,
  remote: AppVersionPayload | null | undefined,
): boolean {
  const current = normalize(currentVersion);
  const remoteVersion = normalize(remote?.version);

  if (!current || !remoteVersion) {
    return false;
  }

  return current !== remoteVersion;
}
