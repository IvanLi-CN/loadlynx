/// <reference types="vite/client" />
interface GlobalThis {
  __LOADLYNX_STORYBOOK__?: boolean;
}

interface ImportMetaEnv {
  readonly DEV: boolean;
  readonly VITE_ENABLE_MOCK_BACKEND?: string;
  readonly VITE_USE_HTTP_BACKEND?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
