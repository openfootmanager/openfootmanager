import type { ReactNode } from "react";

import ProfileLoadFailed from "./ProfileLoadFailed";
import type { SheetAnswer } from "./useContractSheetData";

export type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

/** The key prefix of the Contract tab's own strings. */
export const SECTION = "playerProfile.contractSheet";

/** A figure and the words for it, divided from the row before it. */
export function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-4 py-2 first:pt-0 last:pb-0">
      <dt className="shrink-0 text-body text-fg-muted">{label}</dt>
      <dd className="text-right text-body font-semibold text-fg tabular-nums">{children}</dd>
    </div>
  );
}

/** What a decision costs, written before the button that takes it. */
export function Panel({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="rounded-lg border border-line bg-panel p-2">
      <p className="text-caption text-fg-muted">{label}</p>
      <div className="text-body font-semibold text-fg">{children}</div>
    </div>
  );
}

/**
 * One decision: its title, what it does, the panel of what it costs and its button. At a width
 * where the three sit side by side the four parts share rows with the others (a subgrid), so the
 * panels sit at one height at the bottom of each column; the columns after the first are divided.
 */
export function Decision({
  title,
  what,
  panel,
  children,
}: {
  title: string;
  what: string;
  panel: ReactNode;
  children: ReactNode;
}) {
  return (
    // biome-ignore lint/a11y/useSemanticElements: a group of related controls under one heading, named by its title
    <div
      role="group"
      aria-label={title}
      className="grid min-w-0 gap-2 @[34rem]:row-span-4 @[34rem]:grid-rows-subgrid @[34rem]:border-l @[34rem]:border-line @[34rem]:pl-4 @[34rem]:first:border-l-0 @[34rem]:first:pl-0"
    >
      <h4 className="font-heading text-lead font-bold uppercase tracking-label text-fg">{title}</h4>
      <p className="text-body text-fg-muted">{what}</p>
      <div className="flex flex-col justify-end gap-2">{panel}</div>
      <div>{children}</div>
    </div>
  );
}

/** A preview that is still loading or failed in place of its figures; null once it is answered. */
export function Pending<T>({
  answer,
  retry,
  t,
}: {
  answer: SheetAnswer<T>;
  retry: () => void;
  t: TranslateFn;
}) {
  if (answer.status === "loading") {
    return <p className="text-body text-fg-subtle">{t("common.loading")}</p>;
  }
  if (answer.status === "failed") return <ProfileLoadFailed t={t} onRetry={retry} />;
  return null;
}

/** `value` of `whole` as the language writes a percentage: a ratio of two figures the backend sent. */
export function percentOf(value: number, whole: number, language: string, digits: number): string {
  return new Intl.NumberFormat(language, {
    style: "percent",
    maximumFractionDigits: digits,
  }).format(value / whole);
}
