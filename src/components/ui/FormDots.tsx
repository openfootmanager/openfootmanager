import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";

/** A result as the backend keeps it in a club's form. */
type FormResult = "W" | "D" | "L";

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
    // biome-ignore lint/a11y/noRedundantRoles: WebKit (Tauri's webview on macOS and Linux) drops a list's semantics under list-style: none, so the role is restated.
    <ol role="list" aria-label={t("common.recentForm")} className="flex list-none gap-1 p-0">
      {known.map((result, index) => {
        const { letter, word, tone } = RESULT[result];
        return (
          <li
            // A form is five results in order; the same result can repeat, so its place is its key.
            key={index}
            className={`relative grid size-5 place-items-center rounded font-heading text-caption font-bold ${tone}`}
          >
            <span aria-hidden="true">{letter(t)}</span>
            <span className="sr-only">{word(t)}</span>
          </li>
        );
      })}
    </ol>
  );
}
