import { useEffect, useState } from "react";

import { validateManagerDob } from "../services/careerService";

/**
 * The translation key of what is wrong with the manager's date of birth for the chosen start
 * year and phase, or `null` while it is fine, not asked about yet (no date, or no valid start),
 * or the question could not be answered. The age rule and the day the career opens are the
 * backend's; the last answer stays while the next is on its way.
 */
export function useManagerDobError(
  dob: string,
  startYear: number | null,
  startPhase: string | null,
): string | null {
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const asked = dob !== "" && startYear !== null && startPhase !== null;

  useEffect(() => {
    if (!asked) {
      setErrorKey(null);
      return;
    }
    let cancelled = false;
    validateManagerDob(dob, startYear, startPhase)
      .then((answer) => {
        if (!cancelled) setErrorKey(answer);
      })
      .catch((error: unknown) => {
        console.error("Failed to check the manager's date of birth:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [asked, dob, startYear, startPhase]);

  return asked ? errorKey : null;
}
