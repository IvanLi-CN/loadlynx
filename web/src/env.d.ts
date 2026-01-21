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
  readonly VITE_APP_VERSION?: string;
  readonly VITE_APP_GIT_SHA?: string;
  readonly VITE_APP_GIT_TAG?: string;
  readonly VITE_GITHUB_REPO?: string;
}

// biome-ignore lint/correctness/noUnusedVariables: Used by Vite/TypeScript global typings.
interface ImportMeta {
  readonly env: ImportMetaEnv;
}

export {};
