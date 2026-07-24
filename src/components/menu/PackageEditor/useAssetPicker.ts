import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useAssetDataUrl, evictAssetDataUrl } from "../../../hooks/useAssetDataUrl";

const IMAGE_FILTERS = [
  { name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg"] },
];

interface UseAssetPickerOptions {
  /** The entity's current asset path, relative to the package root, or null. */
  relPath: string | null | undefined;
  projectDir: string | undefined;
  /**
   * Namespaces the copied file so two entities picking sources with the same
   * name can't overwrite each other. Read lazily at pick time so a not-yet-named
   * entity still gets a distinct temporary name.
   */
  entityId: () => string;
  /** Writes the resolved (or cleared) path onto the entity. */
  commit: (relPath: string | null) => void;
  /**
   * Reports a failed copy/read. Required, not optional: a swallowed failure is
   * the exact regression the surrounding change fixes, so every caller must
   * choose how it surfaces.
   */
  onError: (err: unknown) => void;
}

/**
 * The pick → copy-into-package → evict-cache → refresh-preview → commit flow,
 * shared by every asset picker in the package editor (team logo, player photo,
 * competition badge, package logo). Centralised so the commit-on-pick and
 * error-surfacing behaviour can't drift or regress per form.
 */
export function useAssetPicker(options: UseAssetPickerOptions) {
  const { relPath, projectDir, entityId, commit, onError } = options;
  const [refresh, setRefresh] = useState(0);
  const dataUrl = useAssetDataUrl(relPath, projectDir, refresh);

  async function pick() {
    if (!projectDir) return;
    // The dialog is inside the try too: callers invoke this as `void pick()`,
    // so a rejection from `open()` would otherwise escape as an unhandled
    // rejection instead of reaching the required `onError`.
    try {
      const selected = await open({ multiple: false, filters: IMAGE_FILTERS });
      if (!selected || Array.isArray(selected)) return;
      const copied = await invoke<string>("copy_package_asset", {
        dir: projectDir,
        entityId: entityId(),
        srcPath: selected,
      });
      // The path is reused for an entity, so drop any cached data URL before
      // pointing the preview at the freshly written file, then bump the refresh
      // key so it re-reads even when the path is unchanged.
      evictAssetDataUrl(projectDir, copied);
      setRefresh((k) => k + 1);
      commit(copied);
    } catch (err) {
      onError(err);
    }
  }

  function clear() {
    commit(null);
  }

  return { dataUrl, pick, clear };
}
