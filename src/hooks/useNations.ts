import { useEffect, useState } from "react";
import { getNations, type NationInfo } from "../services/nationsService";

/**
 * The backend's nation catalog, or `null` while it is still loading.
 *
 * `ofm_core::nations::all_nations()` is the single source of truth for which
 * nations are built in, and it carries each nation's canonical name — the name
 * a package should *store*, as distinct from the locale-translated name a
 * picker *shows*.
 *
 * A catalog that cannot be read resolves to an empty list rather than staying
 * `null` forever. Nothing can be called built-in without it, so callers fall
 * back to authoring by hand — which shows the author their own data untouched,
 * the safe direction to fail in.
 */
export function useNations(): NationInfo[] | null {
  const [nations, setNations] = useState<NationInfo[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    getNations()
      .then((list) => {
        if (!cancelled) setNations(list);
      })
      .catch((error: unknown) => {
        // Falling back is deliberate, but a failed call and a genuinely empty
        // catalog are indistinguishable downstream — every country quietly
        // drops to manual authoring either way. Leave a trace of which it was.
        console.error("Could not load the nation catalog", error);
        if (!cancelled) setNations([]);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return nations;
}
