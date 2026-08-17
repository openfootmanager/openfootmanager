/// <reference types="vite/client" />

// Vite's own ImportMetaEnv is indexed `[key: string]: any`, so without this a typo'd variable
// name type-checks and silently reads undefined forever.
interface ImportMetaEnv {
  /** Dev only: label for the rendering benchmark in src/dev/benchUi.ts. */
  readonly VITE_OFM_BENCH?: string;
}

// Injected by the `define` block in vite.config.ts.
declare const __APP_VERSION__: string;
declare const __APP_CHANNEL__: string;
declare const __APP_COMMIT__: string;
declare const __APP_BUILD_DATE__: string;
