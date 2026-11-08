import { useTranslation } from "react-i18next";
import { FormResultMark, isFormResult } from "./FormResultMark";

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
      {known.map((result, index) => (
        // A form is five results in order; the same result can repeat, so its place is its key.
        <li key={index} className="flex">
          <FormResultMark result={result} className="size-5" />
        </li>
      ))}
    </ol>
  );
}
