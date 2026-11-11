import type { ReactNode } from "react";

interface OverviewCardProps {
  /** The card's name: its heading and the region's accessible name. */
  title: string;
  /** A link in the header that opens another tab, drawn as a button. */
  link?: { label: string; onPress: () => void };
  children: ReactNode;
  className?: string;
}

/** One card of the Overview (the profile board's "ch" header over its rows). */
export function OverviewCard({ title, link, children, className = "" }: OverviewCardProps) {
  return (
    <section
      aria-label={title}
      className={`relative flex min-w-0 flex-col overflow-hidden rounded-xl border border-line bg-panel shadow-card ${className}`}
    >
      <div className="flex items-center justify-between gap-3 border-b border-line px-4 py-2.5">
        <h3 className="font-heading text-body font-bold uppercase tracking-label text-fg">
          {title}
        </h3>
        {link ? (
          <button
            type="button"
            onClick={link.onPress}
            className="rounded-sm text-caption font-semibold text-selected underline underline-offset-2 hover:text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page"
          >
            {link.label}
          </button>
        ) : null}
      </div>
      {children}
    </section>
  );
}

/** A label and its value on one row of a card. */
export function OverviewRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-3 border-b border-line px-4 py-2 text-body last:border-b-0">
      <span className="text-fg-muted">{label}</span>
      <span className="flex min-w-0 items-center gap-2 text-right text-fg">{children}</span>
    </div>
  );
}
