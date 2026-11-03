import { type ReactNode, useId } from "react";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

/** The surface a retry sits on: a page-coloured card, or the hero's always-dark chrome. */
type RetryTone = "page" | "chrome";

const TONE_CLASSES: Record<RetryTone, string> = {
  page: "text-fg hover:text-fg-muted focus-visible:ring-offset-page",
  chrome: "text-chrome-fg hover:text-chrome-fg-muted focus-visible:ring-offset-chrome",
};

/**
 * A retry for an answer that failed to load: one look on every surface, its colours the surface's.
 * Its accessible name is its visible text, so a voice-control user can say what they see (WCAG
 * 2.5.3); `description` says what is retried, as the button's description.
 */
export function RetryButton({
  onRetry,
  description,
  tone = "page",
  children,
}: {
  onRetry: () => void;
  /** What is retried, when the visible text does not say: read as the button's description. */
  description?: string;
  tone?: RetryTone;
  children: ReactNode;
}) {
  const descriptionId = useId();
  return (
    <>
      <button
        type="button"
        onClick={onRetry}
        aria-describedby={description ? descriptionId : undefined}
        className={`rounded font-semibold underline underline-offset-2 focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 ${TONE_CLASSES[tone]}`}
      >
        {children}
      </button>
      {description ? (
        <span id={descriptionId} className="sr-only">
          {description}
        </span>
      ) : null}
    </>
  );
}

/**
 * What the profile shows where an answer about the player failed to load: "Couldn't load" and a
 * retry. Never the empty state that means the club knows nothing (`useKnowledgeAnswer`).
 */
export default function ProfileLoadFailed({ t, onRetry }: { t: TranslateFn; onRetry: () => void }) {
  return (
    <p className="mb-3 flex flex-wrap items-center gap-2 text-caption text-fg-muted">
      <span>{t("playerProfile.loadFailed")}</span>
      <RetryButton onRetry={onRetry}>{t("common.retry")}</RetryButton>
    </p>
  );
}

/**
 * "Couldn't load" as its own retry button, for a slot with room only for the value (the hero's
 * wage). `description` says what is retried; `tone` is the surface's.
 */
export function LoadFailedRetryText({
  t,
  onRetry,
  description,
  tone = "page",
}: {
  t: TranslateFn;
  onRetry: () => void;
  description: string;
  tone?: RetryTone;
}) {
  return (
    <RetryButton onRetry={onRetry} description={description} tone={tone}>
      {t("playerProfile.loadFailed")}
    </RetryButton>
  );
}
