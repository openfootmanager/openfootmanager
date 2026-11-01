import { type ReactNode, useId } from "react";

import { kitStyle } from "../../lib/kitStyle";
import type { TeamColors } from "../../store/types";
import { isOnControl } from "./clickTarget";

interface EntityFact {
  /** What the fact is, translated: "Value", "Morale". */
  label: string;
  value: ReactNode;
}

interface EntityCardProps {
  /** `full` in a grid of cards; `compact` is the identity row alone, for a message or side panel. */
  size?: "full" | "compact";
  /** The club's kit colours, from the backend: the stripe, the header's wash and its hairline. */
  kit?: TeamColors | null;
  /** The face: a player's portrait, a club's crest, a manager's or staff member's badge. */
  media: ReactNode;
  name: string;
  /** One line under the name: the country, then the entity's own facts. */
  meta: ReactNode;
  /** The one big number on the right: a rating, a league position, a reputation. */
  hero?: ReactNode;
  /** Full only: the status rows (condition, contract; form, manager). */
  status?: ReactNode;
  /** Full only: the two facts at the foot. */
  facts?: readonly [EntityFact, EntityFact];
  /** Opens the entity's profile: from the name's button, or a click anywhere else on the card. */
  onOpen?: () => void;
}

/**
 * The card shell of every list's card view (round 4, approved v4): the club's kit as a stripe down
 * the left and a wash behind the identity row, then the status band and two facts. The name is the
 * card's one button; everything else stays outside it so a screen reader reads it, and a click on
 * the card's body opens it too, as a pointer convenience. It draws no data of its own.
 */
export function EntityCard({
  size = "full",
  kit,
  media,
  name,
  meta,
  hero,
  status,
  facts,
  onOpen,
}: EntityCardProps) {
  const nameId = useId();
  const compact = size === "compact";
  return (
    // A pointer convenience only: the keyboard opens the card from the name's button.
    // biome-ignore lint/a11y/useKeyWithClickEvents: the name's button is the keyboard's way in.
    <article
      aria-labelledby={nameId}
      style={kitStyle(kit)}
      onClick={onOpen ? (event) => !isOnControl(event) && onOpen() : undefined}
      className={`flex min-w-0 overflow-hidden rounded-xl border border-line border-l-4 border-l-kit bg-panel text-fg shadow-card ${
        compact ? "flex-row" : "flex-col"
      } ${onOpen ? "cursor-pointer" : ""}`}
    >
      <header
        className={`flex flex-1 items-center gap-3 border-b-2 border-b-kit bg-kit-wash kit-hairline ${
          compact ? "px-3 py-2" : "px-4 py-3"
        }`}
      >
        <div className={`grid shrink-0 place-items-center ${compact ? "size-10" : "size-12"}`}>
          {media}
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <h3
            id={nameId}
            // Uppercase only at heading size: a compact card's name is lead-sized text.
            className={`font-heading font-extrabold leading-tight ${
              compact ? "text-lead" : "text-heading uppercase tracking-label"
            }`}
          >
            {onOpen ? (
              <button
                type="button"
                onClick={onOpen}
                // The offset keeps the ring clear of the kit-washed header behind the name.
                className="rounded-sm text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel"
              >
                {name}
              </button>
            ) : (
              name
            )}
          </h3>
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-caption text-fg-muted">
            {meta}
          </div>
        </div>
        {hero ? (
          <div
            className={`shrink-0 text-right font-heading font-extrabold tabular-nums leading-none ${
              compact ? "text-title" : "text-display"
            }`}
          >
            {hero}
          </div>
        ) : null}
      </header>
      {!compact && status ? (
        <div className="flex flex-col gap-2 border-t border-line bg-card-band px-4 py-3 text-body">
          {status}
        </div>
      ) : null}
      {!compact && facts ? (
        <dl className="grid grid-cols-2 gap-2 border-t border-line px-4 py-3 tabular-nums">
          {facts.map((fact, index) => (
            <div key={fact.label} className={index === 1 ? "text-right" : ""}>
              <dt className="text-caption text-fg-muted">{fact.label}</dt>
              <dd className="text-lead font-semibold">{fact.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
    </article>
  );
}
