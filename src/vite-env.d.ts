/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly TAURI_ENV_DEBUG?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
