import { useMutation } from "@tanstack/react-query";
import type { Identity } from "../api/types.ts";
import { buildSubnetPlanFromSeedIp } from "./scan-subnet.ts";

function isStorybookRuntime(): boolean {
  return globalThis.__LOADLYNX_STORYBOOK__ === true;
}

export interface DiscoveredDevice {
  ip: string;
  hostname: string | null;
  short_id: string | null;
  identity: Identity;
}

export interface ScanOptions {
  seedIp: string;
  maxConcurrency?: number; // default 24
  perHostTimeoutMs?: number; // default 400
  signal?: AbortSignal;
}

export interface ScanProgress {
  scannedCount: number;
  totalCount: number;
  foundCount: number;
}

function parseDiscoveredIdentity(text: string): Identity | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text) as unknown;
  } catch {
    return null;
  }

  if (!parsed || typeof parsed !== "object") {
    return null;
  }

  const identity = parsed as Partial<Identity>;
  if (
    typeof identity.device_id !== "string" ||
    typeof identity.digital_fw_version !== "string" ||
    typeof identity.analog_fw_version !== "string" ||
    typeof identity.protocol_version !== "number" ||
    typeof identity.uptime_ms !== "number" ||
    !identity.network ||
    typeof identity.network.ip !== "string" ||
    typeof identity.network.mac !== "string" ||
    typeof identity.network.hostname !== "string" ||
    !identity.capabilities ||
    typeof identity.capabilities.api_version !== "string"
  ) {
    return null;
  }

  const isLoadLynx =
    identity.capabilities.api_version.length > 0 ||
    identity.device_id.startsWith("llx-");
  if (!isLoadLynx) {
    return null;
  }

  return identity as Identity;
}

export const __testParseDiscoveredIdentity = parseDiscoveredIdentity;

// Internal worker to scan a single IP
async function scanSingleHost(
  ip: string,
  timeoutMs: number,
  signal?: AbortSignal,
): Promise<DiscoveredDevice | null> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

  // Link the passed signal (cancel whole scan) to this request's controller
  const onExternalAbort = () => controller.abort();
  if (signal) {
    if (signal.aborted) {
      clearTimeout(timeoutId);
      return null;
    }
    signal.addEventListener("abort", onExternalAbort);
  }

  try {
    const response = await fetch(`http://${ip}/api/v1/identity`, {
      method: "GET",
      signal: controller.signal,
      // Assumption: Devices support CORS or this is a simple request that allows opaque response.
      // Ideally valid JSON is returned.
      // If we are in "no-cors" mode, we can't read the body, so we MUST rely on standard CORS.
      // LoadLynx firmware is assumed to be accessible.
    });

    clearTimeout(timeoutId);

    if (!response.ok) {
      return null; // 404 or 500 etc -> not a valid target or busy
    }

    const text = await response.text();
    const identity = parseDiscoveredIdentity(text);
    if (!identity) {
      return null;
    }

    return {
      ip,
      hostname: identity.hostname ?? null,
      short_id: identity.short_id ?? null,
      identity,
    };
  } catch (_error) {
    // Network error, timeout, or abort
    return null;
  } finally {
    clearTimeout(timeoutId);
    if (signal) {
      signal.removeEventListener("abort", onExternalAbort);
    }
  }
}

/**
 * Scans a /24 subnet based on the seed IP.
 * @param options Scan configuration
 * @param onProgress Callback for progress updates
 * @returns List of discovered devices
 */
export async function scanSubnet(
  options: ScanOptions,
  onProgress?: (progress: ScanProgress) => void,
): Promise<DiscoveredDevice[]> {
  if (isStorybookRuntime()) {
    throw new Error(
      "[LoadLynx] LAN subnet scanning is disabled in Storybook. This action would initiate real network traffic; use mock:// devices instead.",
    );
  }

  const {
    seedIp,
    maxConcurrency = 24,
    perHostTimeoutMs = 400,
    signal,
  } = options;

  const plan = buildSubnetPlanFromSeedIp(seedIp);
  const hosts = plan.hosts;
  const total = hosts.length;

  // Results
  const discovered: DiscoveredDevice[] = [];
  let scannedCount = 0;
  const scanQueue = [...hosts];

  const worker = async () => {
    while (scanQueue.length > 0) {
      if (signal?.aborted) {
        break;
      }
      const ip = scanQueue.shift();
      if (!ip) break;

      const result = await scanSingleHost(ip, perHostTimeoutMs, signal);
      if (result) {
        // De-dupe by IP (should be unique by def, but safety first)
        if (!discovered.some((d) => d.ip === result.ip)) {
          discovered.push(result);
        }
      }
      scannedCount++;
      onProgress?.({
        scannedCount,
        totalCount: total,
        foundCount: discoveryCount(),
      });
    }
  };

  const discoveryCount = () => discovered.length;

  const workers = Array.from({ length: maxConcurrency }).map(() => worker());
  await Promise.all(workers);

  // Sort by IP for consistency
  discovered.sort((a, b) => {
    const ipA = a.ip.split(".").map(Number);
    const ipB = b.ip.split(".").map(Number);
    for (let i = 0; i < 4; i++) {
      if (ipA[i] !== ipB[i]) return ipA[i] - ipB[i];
    }
    return 0;
  });

  return discovered;
}

export const __testScanSingleHost = scanSingleHost;
export const __testScanSubnet = scanSubnet;

export function useSubnetScanMutation() {
  return useMutation<
    DiscoveredDevice[],
    Error,
    { options: ScanOptions; onProgress?: (p: ScanProgress) => void }
  >({
    mutationFn: async (args) => {
      if (isStorybookRuntime()) {
        throw new Error(
          "[LoadLynx] LAN subnet scanning is disabled in Storybook. This action would initiate real network traffic; use mock:// devices instead.",
        );
      }
      return scanSubnet(args.options, args.onProgress);
    },
  });
}
