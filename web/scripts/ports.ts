import * as path from "node:path";
import { fileURLToPath } from "node:url";

export const LOADLYNX_PORT_REGISTRY = {
  webDev: { envVar: "LOADLYNX_WEB_DEV_PORT", defaultPort: 25219 },
  webPreview: { envVar: "LOADLYNX_WEB_PREVIEW_PORT", defaultPort: 22848 },
  storybook: { envVar: "LOADLYNX_STORYBOOK_PORT", defaultPort: 32931 },
  storybookTest: { envVar: "LOADLYNX_STORYBOOK_TEST_PORT", defaultPort: 34033 },
} as const;

export type LoadLynxPortKey = keyof typeof LOADLYNX_PORT_REGISTRY;

export type ResolvedPort = {
  envVar: string;
  port: number;
  fromEnv: boolean;
};

const PORT_MIN = 1024;
const PORT_MAX = 65535;

function parsePort(rawValue: string, envVar: string): number {
  const trimmed = rawValue.trim();
  if (trimmed.length === 0) {
    throw new Error(`[ports] Invalid ${envVar}="${rawValue}": expected integer ${PORT_MIN}..${PORT_MAX}`);
  }

  if (!/^\d+$/.test(trimmed)) {
    throw new Error(`[ports] Invalid ${envVar}="${rawValue}": expected integer ${PORT_MIN}..${PORT_MAX}`);
  }

  const parsed = Number.parseInt(trimmed, 10);
  if (!Number.isSafeInteger(parsed) || parsed < PORT_MIN || parsed > PORT_MAX) {
    throw new Error(`[ports] Invalid ${envVar}="${rawValue}": expected integer ${PORT_MIN}..${PORT_MAX}`);
  }

  return parsed;
}

export function resolvePortByEnvVar(
  envVar: string,
  defaultPort: number,
  env: NodeJS.ProcessEnv = process.env,
): ResolvedPort {
  const rawValue = env[envVar];
  if (rawValue === undefined) {
    return { envVar, port: defaultPort, fromEnv: false };
  }

  return { envVar, port: parsePort(rawValue, envVar), fromEnv: true };
}

export function resolvePort(
  key: LoadLynxPortKey,
  env: NodeJS.ProcessEnv = process.env,
): ResolvedPort {
  const spec = LOADLYNX_PORT_REGISTRY[key];
  return resolvePortByEnvVar(spec.envVar, spec.defaultPort, env);
}

export function localhostUrl(port: number): string {
  return `http://localhost:${port}`;
}

export function loopbackUrl(port: number): string {
  return `http://127.0.0.1:${port}`;
}

function usage(): string {
  const keys = Object.keys(LOADLYNX_PORT_REGISTRY).join("|");
  return [
    "Usage:",
    `  bun scripts/ports.ts print <${keys}>`,
    `  bun scripts/ports.ts check <${keys}>`,
  ].join("\n");
}

const isExecutedDirectly =
  process.argv[1] !== undefined &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (isExecutedDirectly) {
  const cmd = process.argv[2];
  const key = process.argv[3];

  if (cmd !== "print" && cmd !== "check") {
    console.error(usage());
    process.exit(2);
  }

  if (!key) {
    console.error(usage());
    process.exit(2);
  }

  if (!Object.hasOwn(LOADLYNX_PORT_REGISTRY, key)) {
    console.error(`[ports] Unknown key "${key}"`);
    console.error(usage());
    process.exit(2);
  }

  const resolved = resolvePort(key as LoadLynxPortKey);
  if (resolved.fromEnv) {
    console.error(`[ports] Using ${resolved.envVar}=${resolved.port}`);
  }

  if (cmd === "print") {
    console.log(resolved.port);
  }
}
