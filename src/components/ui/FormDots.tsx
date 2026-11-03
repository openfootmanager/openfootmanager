import { useTranslation } from "react-i18next";

/** A result as the backend keeps it in a club's form. */
type FormResult = "W" | "D" | "L";

/** Each result's letter, its word, and its tone from the role tokens. */
const RESULT: Record<FormResult, { letterKey: string; wordKey: string; tone: string }> = {
  W: {
    letterKey: "common.won",
    wordKey: "common.wonFull",
    tone: "bg-positive-tint text-positive",
  },
  D: { letterKey: "common.drawn", wordKey: "common.drawnFull", tone: "bg-sunken text-fg-muted" },
  L: {
    letterKey: "common.lost",
    wordKey: "common.lostFull",
    tone: "bg-danger-tint text-danger",
  },
};

/** The three results, by name: a list, so a key every object inherits ("constructor") isn't one. */
const FORM_RESULTS: readonly string[] = ["W", "D", "L"] satisfies FormResult[];

function isFormResult(value: string): value is FormResult {
  return FORM_RESULTS.includes(value);
}

/**
 * A club's recent results, oldest first, as the backend keeps them (`Team::form`). Each shows the
 * locale's letter (the backend's are English) and is read as its word; the tone repeats what the
 * letter says, so colour never carries the result alone. Nothing is drawn before the first result.
 */
export function FormDots({ results }: { results: readonly string[] }) {
  const { t } = useTranslation();
  const known = results.filter(isFormResult);
  if (known.length === 0) return null;
  return (
    <ol aria-label={t("common.recentForm")} className="flex list-none gap-1 p-0">
      {known.map((result, index) => {
        const { letterKey, wordKey, tone } = RESULT[result];
        return (
          <li
            // A form is five results in order; the same result can repeat, so its place is its key.
            key={index}
            className={`relative grid size-5 place-items-center rounded font-heading text-caption font-bold ${tone}`}
          >
            <span aria-hidden="true">{t(letterKey)}</span>
            <span className="sr-only">{t(wordKey)}</span>
          </li>
        );
      })}
    </ol>
  );
}
