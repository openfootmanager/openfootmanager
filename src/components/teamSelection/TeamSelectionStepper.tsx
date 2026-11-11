import { Check } from "lucide-react";
import { useTranslation } from "react-i18next";

/** The new career's steps, in order: the player arrives at the last, choosing a club. */
const STEPS = [
  { id: "manager", labelKey: "teamSelect.steps.manager" },
  { id: "world", labelKey: "teamSelect.steps.world" },
  { id: "generation", labelKey: "teamSelect.steps.generation" },
  { id: "club", labelKey: "teamSelect.steps.club" },
] as const;

const CURRENT_STEP = "club";

/**
 * Where the player is in starting a career. A finished step carries a tick and says "completed" to a
 * screen reader; the current one is marked `aria-current="step"`, so no step is told by colour alone.
 */
export default function TeamSelectionStepper() {
  const { t } = useTranslation();
  return (
    <ol aria-label={t("teamSelect.steps.label")} className="flex flex-wrap gap-x-4 gap-y-1">
      {STEPS.map((step, index) => {
        const current = step.id === CURRENT_STEP;
        return (
          <li
            key={step.id}
            aria-current={current ? "step" : undefined}
            className={`flex items-center gap-1.5 text-caption ${
              current ? "font-semibold text-fg" : "text-fg-subtle"
            }`}
          >
            <span
              aria-hidden="true"
              className={`grid size-5 place-items-center rounded-full font-bold ${
                current ? "bg-selected-fill text-on-brand" : "bg-positive-tint text-positive"
              }`}
            >
              {current ? index + 1 : <Check className="size-3" />}
            </span>
            {t(step.labelKey)}
            {current ? null : <span className="sr-only">{t("teamSelect.steps.done")}</span>}
          </li>
        );
      })}
    </ol>
  );
}
