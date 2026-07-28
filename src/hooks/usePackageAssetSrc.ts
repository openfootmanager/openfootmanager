import { convertFileSrc } from "@tauri-apps/api/core";
import { appDataDir } from "@tauri-apps/api/path";
import { useEffect, useState } from "react";

import { resolveLocalMediaPath } from "../lib/mediaAssets";
import { isPackageQualifiedAsset, joinAssetRoot } from "../lib/packageAssets";

/**
 * The app data directory, resolved once per session.
 *
 * Every club badge and player face on a screen asks for this, so the lookup is
 * shared rather than repeated per image.
 */
let appDataDirPromise: Promise<string> | null = null;

function resolveAppDataDir(): Promise<string> {
  appDataDirPromise ??= appDataDir();
  return appDataDirPromise;
}

/** Test seam: drop the cached directory so each test starts clean. */
export function resetPackageAssetRoot(): void {
  appDataDirPromise = null;
}

/**
 * Turn a stored media path into something an `<img>` can load.
 *
 * Assets bundled with the app resolve synchronously against the frontend root.
 * Assets extracted from an installed package live outside the bundle and have
 * to go through Tauri's asset protocol, which needs the app data directory —
 * so those resolve on a later tick and the hook re-renders.
 */
export function usePackageAssetSrc(path: string | null | undefined): string | null {
  const bundled = isPackageQualifiedAsset(path) ? null : resolveLocalMediaPath(path);
  const [packageSrc, setPackageSrc] = useState<string | null>(null);

  useEffect(() => {
    if (!isPackageQualifiedAsset(path)) {
      setPackageSrc(null);
      return;
    }

    let active = true;
    resolveAppDataDir()
      .then((dir) => {
        if (!active) return;
        setPackageSrc(convertFileSrc(joinAssetRoot(dir, path!.trim())));
      })
      .catch(() => {
        // No app data dir means no package assets; the caller's fallback
        // (a generated crest or portrait) is the right outcome, not an error.
        if (active) setPackageSrc(null);
      });

    return () => {
      active = false;
    };
  }, [path]);

  return bundled ?? packageSrc;
}
