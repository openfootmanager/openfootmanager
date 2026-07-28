/**
 * Resolution for artwork that arrives inside an installed `.ofm` package.
 *
 * Package files are extracted at world load into `<appData>/package-assets/`,
 * and every path is stamped with the package it came from, so a club badge
 * reads `brazil-1962/assets/images/santos.png`. Those live outside the app
 * bundle and can only be loaded through Tauri's asset protocol.
 *
 * Paths that ship *inside* the app are served from the frontend root instead and
 * must not be rewritten — they are told apart by their first segment.
 */

/** Directory under the app data dir holding every package's extracted assets. */
export const PACKAGE_ASSET_DIR = "package-assets";

const URI_SCHEME = /^[a-z][a-z0-9+.-]*:/i;

/**
 * Whether a media path points into an installed package rather than at an asset
 * bundled with the app. Bundled paths are absolute or begin `assets/`; a
 * package-qualified path always begins with its package id.
 */
export function isPackageQualifiedAsset(path: string | null | undefined): boolean {
  if (!path) return false;
  const trimmed = path.trim();
  if (!trimmed) return false;
  if (trimmed.startsWith("/") || trimmed.startsWith("\\") || trimmed.startsWith("//")) {
    return false;
  }
  if (URI_SCHEME.test(trimmed)) return false;
  return !trimmed.startsWith("assets/");
}

/** Absolute path of a package asset, given the app data directory. */
export function joinAssetRoot(appDataDir: string, relativePath: string): string {
  const base = appDataDir.replace(/[\\/]+$/, "");
  const relative = relativePath.replace(/^[\\/]+/, "");
  return `${base}/${PACKAGE_ASSET_DIR}/${relative}`;
}
