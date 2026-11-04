import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";

/** A result as the backend keeps it in a club's form. */
export type FormResult = "W" | "D" | "L";

/**
 * Each result's letter (the locale's own one-letter mark, not the standings table's column header,
 * which is two letters in Spanish), its word for one match, and its tone from the role tokens. The
 * keys are written out in `t(...)` calls, so the key-coverage test sees them.
 */
const RESULT: Record<
  FormResult,
  { letter: (t: TFunction) => string; word: (t: TFunction) => string; tone: string }
> = {
  W: {
    letter: (t) => t("common.formResult.W"),
    word: (t) => t("match.victory"),
    tone: "bg-positive-tint text-positive",
  },
  // The inset line, as Badge's neutral has it: sunken alone is 1.02:1 on a white panel.
  D: {
    letter: (t) => t("common.formResult.D"),
    word: (t) => t("match.draw"),
    tone: "bg-sunken text-fg-muted ring-1 ring-inset ring-line",
  },
  L: {
    letter: (t) => t("common.formResult.L"),
    word: (t) => t("match.defeat"),
    tone: "bg-danger-tint text-danger",
  },
};

/** The three results, by name: a list, so a key every object inherits ("constructor") isn't one. */
const FORM_RESULTS: readonly string[] = ["W", "D", "L"] satisfies FormResult[];

export function isFormResult(value: string): value is FormResult {
  return FORM_RESULTS.includes(value);
}

/** A match the club played, as the recap names it, in the form's letters. */
export function formResultOf(outcome: "win" | "draw" | "loss"): FormResult {
  return { win: "W", draw: "D", loss: "L" }[outcome] satisfies FormResult;
}

/**
 * One result as a small mark: the locale's letter, read out as the result's word; the tone repeats
 * what the letter says, so colour never carries the result alone. The caller sizes it.
 */
export function FormResultMark({
  result,
  className = "",
}: {
  result: FormResult;
  className?: string;
}) {
  const { t } = useTranslation();
  const { letter, word, tone } = RESULT[result];
  return (
    <span
      className={`relative grid place-items-center rounded font-heading text-caption font-bold ${tone} ${className}`}
    >
      <span aria-hidden="true">{letter(t)}</span>
      <span className="sr-only">{word(t)}</span>
    </span>
  );
}
