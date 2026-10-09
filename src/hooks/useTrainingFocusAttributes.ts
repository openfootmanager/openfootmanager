import { useEffect, useState } from "react";

import { getTrainingFocusAttributes } from "../services/trainingService";
import type { TrainingFocusAttributesData } from "../store/types";

export function useTrainingFocusAttributes(): Record<string, TrainingFocusAttributesData> {
  const [byFocus, setByFocus] = useState<Record<string, TrainingFocusAttributesData>>({});

  useEffect(() => {
    let cancelled = false;
    getTrainingFocusAttributes()
      .then((entries) => {
        if (!cancelled)
          setByFocus(Object.fromEntries(entries.map((entry) => [entry.focus, entry])));
      })
      .catch((error) => console.error("Failed to load training focus attributes:", error));
    return () => {
      cancelled = true;
    };
  }, []);

  return byFocus;
}
