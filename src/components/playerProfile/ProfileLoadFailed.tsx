type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

const RETRY_CLASSES =
  "rounded font-semibold text-fg underline underline-offset-2 hover:text-fg-muted focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page";

/**
 * What the profile shows where an answer about the player failed to load: "Couldn't load" and a
 * retry. Never the empty state that means the club knows nothing (`useKnowledgeAnswer`).
 */
export default function ProfileLoadFailed({ t, onRetry }: { t: TranslateFn; onRetry: () => void }) {
  return (
    <p className="mb-3 flex flex-wrap items-center gap-2 text-caption text-fg-muted">
      <span>{t("playerProfile.loadFailed")}</span>
      <button type="button" onClick={onRetry} className={RETRY_CLASSES}>
        {t("common.retry")}
      </button>
    </p>
  );
}

/**
 * "Couldn't load" as its own retry button, for a slot with room only for the value (the hero's
 * wage). `label` names what is retried, for a screen reader.
 */
export function LoadFailedRetryText({
  t,
  onRetry,
  label,
}: {
  t: TranslateFn;
  onRetry: () => void;
  label: string;
}) {
  return (
    <button type="button" onClick={onRetry} aria-label={label} className={RETRY_CLASSES}>
      {t("playerProfile.loadFailed")}
    </button>
  );
}
