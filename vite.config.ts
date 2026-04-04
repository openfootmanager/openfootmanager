import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

function manualChunks(id: string): string | undefined {
  const normalizedId = id.replace(/\\/g, "/");

  if (normalizedId.indexOf("node_modules") === -1) {
    if (normalizedId.indexOf("/src/store/") !== -1) {
      return "state";
    }

    if (normalizedId.indexOf("/src/components/ui/") !== -1) {
      return "ui-kit";
    }

    if (normalizedId.indexOf("/src/lib/") !== -1) {
      return "domain-lib";
    }

    return undefined;
  }

  if (
    normalizedId.indexOf("react-router-dom") !== -1 ||
    normalizedId.indexOf("react-router") !== -1 ||
    normalizedId.indexOf("@remix-run/router") !== -1
  ) {
    return "router";
  }

  if (normalizedId.indexOf("react-dom") !== -1) {
    return "react-dom-vendor";
  }

  if (
    normalizedId.indexOf("react") !== -1 ||
    normalizedId.indexOf("scheduler") !== -1
  ) {
    return "react-vendor";
  }

  if (normalizedId.indexOf("@tauri-apps") !== -1) {
    return "tauri";
  }

  if (normalizedId.indexOf("i18next") !== -1) {
    return "i18n";
  }

  if (normalizedId.indexOf("lucide-react") !== -1) {
    return "icons";
  }

  return undefined;
}

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  test: {
    environment: "jsdom",
    globals: true,
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["src/test-setup.ts"],
    coverage: {
      exclude: ["src/i18n/locales/**", "src/**/*.test.{ts,tsx}", "src/test-setup.ts"],
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  build: {
    rollupOptions: {
      output: {
        manualChunks,
      },
    },
  },
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
        protocol: "ws",
        host,
        port: 1421,
      }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
