interface ImportMetaEnv {
  readonly DEV: boolean;
  readonly VITE_USE_HTTP_BACKEND?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
