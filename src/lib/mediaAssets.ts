import { convertFileSrc, isTauri } from "@tauri-apps/api/core";

const URI_SCHEME = /^[a-z][a-z0-9+.-]*:/i;

export function resolveLocalMediaPath(path: string | null | undefined): string | null {
  if (!path) return null;
  const trimmed = path.trim();
  if (!trimmed || URI_SCHEME.test(trimmed) || trimmed.startsWith("//")) {
    return null;
  }
  const absolute = trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
  // Paths extracted from OFM packages are stored as absolute filesystem paths
  // (e.g. /home/user/.local/share/.../package-assets/...). Bundle assets live
  // under /assets/ and are served directly by the webview.
  if (!absolute.startsWith("/assets/") && isTauri()) {
    return convertFileSrc(absolute);
  }
  return absolute;
}
