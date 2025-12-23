/// <reference types="vite/client" />

declare global {
  // Global marker used to enable Storybook-specific guardrails (no real network, no LAN scan, etc.)
  // eslint-disable-next-line no-var
  var __LOADLYNX_STORYBOOK__: boolean | undefined;
}

interface ImportMetaEnv {
  readonly DEV: boolean;
  readonly VITE_ENABLE_MOCK_BACKEND?: string;
  readonly VITE_USE_HTTP_BACKEND?: string;
}

// biome-ignore lint/correctness/noUnusedVariables: Used by Vite/TypeScript global typings.
interface ImportMeta {
  readonly env: ImportMetaEnv;
}

export {};
