import { invoke } from "@tauri-apps/api/core";

/**
 * A selectable football nation, mirrored from the backend `NATION_CATALOG`.
 * This catalog is the single source of truth for which nationalities are
 * selectable and importable (see #270), so the UI reads it rather than offering
 * every ISO country.
 */
export interface NationInfo {
  code: string;
  name: string;
  region: string;
}

let cache: Promise<NationInfo[]> | null = null;

/** Fetch the nation catalog from the backend, cached for the session. */
export function getNations(): Promise<NationInfo[]> {
  cache ??= invoke<NationInfo[]>("get_nations");
  return cache;
}

/** The catalog's nation codes, for filtering selectable nationalities. */
export async function getNationCodes(): Promise<string[]> {
  const nations = await getNations();
  return nations.map((nation) => nation.code);
}

/** Testing hook: drop the cached catalog so the next call refetches. */
export function resetNationsCache(): void {
  cache = null;
}
