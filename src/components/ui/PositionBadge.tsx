import { useTranslation } from "react-i18next";

import { usePositionGroups } from "../../hooks/usePositionGroups";
import { Badge } from "./Badge";

/** How the position a player is deployed in relates to the ones he knows. */
export type PositionFamiliarity = "natural" | "adapted" | "out";

const FAMILIARITY_KEY: Record<PositionFamiliarity, string> = {
  natural: "squad.naturalFit",
  adapted: "squad.adaptedFit",
  out: "squad.outOfPosition",
};

/** The tone of each group the backend names; a group it does not name reads as neutral. */
const GROUP_VARIANT: Record<string, "accent" | "primary" | "success" | "danger"> = {
  Goalkeeper: "accent",
  Defender: "primary",
  Midfielder: "success",
  Forward: "danger",
};

/** The wrapper's outline: quiet for a natural slot, dashed for an adapted one, solid warning out. */
const OUTLINE_CLASS: Record<PositionFamiliarity, string> = {
  natural: "",
  adapted: "outline outline-1 outline-dashed outline-fg-subtle",
  out: "outline outline-1 outline-warning-solid",
};

/**
 * The corner dot that says a player is not in his natural position: hollow when he can play there,
 * filled warning when he cannot. Natural has no marker, so it is the quiet default. Hidden from
 * assistive tech; the badge's text carries the same fact in words.
 */
export function FamiliarityDot({ familiarity }: { familiarity: PositionFamiliarity }) {
  if (familiarity === "natural") return null;
  const tone =
    familiarity === "out" ? "bg-warning-solid border-warning-solid" : "bg-panel border-fg-subtle";
  return (
    <span
      data-testid="familiarity-dot"
      aria-hidden="true"
      className={`absolute -right-1 -top-1 size-2.5 rounded-full border ${tone}`}
    />
  );
}

interface PositionBadgeProps {
  /** Raw position enum (e.g. "CenterBack"); picks the group colour. */
  position: string;
  /** The short, translated label drawn on the badge (e.g. "CB"). */
  label: string;
  /** The translated full position name, read out and shown as the tooltip. */
  name: string;
  /** `natural` is filled with the group colour; `alternate` is outlined. */
  kind?: "natural" | "alternate";
  /** Set for the position the player is deployed in; leave out when no slot is being judged. */
  familiarity?: PositionFamiliarity;
}

export function PositionBadge({
  position,
  label,
  name,
  kind = "natural",
  familiarity,
}: PositionBadgeProps) {
  const { t } = useTranslation();
  const groupOf = usePositionGroups();
  const variant = GROUP_VARIANT[groupOf(position) ?? ""] ?? "neutral";
  const words = familiarity ? `${name}, ${t(FAMILIARITY_KEY[familiarity])}` : name;
  return (
    <span
      title={words}
      className={`relative inline-flex rounded-md ${familiarity ? OUTLINE_CLASS[familiarity] : ""}`}
    >
      <Badge variant={kind === "alternate" ? "outline" : variant}>
        {label === words ? (
          // The label already is the position's name: say it once, to everyone.
          <span>{label}</span>
        ) : (
          <>
            <span aria-hidden="true">{label}</span>
            <span className="sr-only">{words}</span>
          </>
        )}
      </Badge>
      {familiarity ? <FamiliarityDot familiarity={familiarity} /> : null}
    </span>
  );
}

interface PositionBadgesProps {
  /** The player's natural position. */
  natural: string;
  /** The other positions he can play. */
  alternates: string[];
  /** The position of the slot he is deployed in, when he is in one. */
  deployed?: string;
  /**
   * How that slot relates to his positions. The caller states it, because the squad already
   * decides it (a deployed "Defender" slot is a centre-back's natural group, not a string match).
   */
  familiarity?: PositionFamiliarity;
  /** How many badges to draw before the rest collapse into a "+n" count. */
  max?: number;
  /** The translated label and full name of a position. */
  describe: (position: string) => { label: string; name: string };
}

interface BadgeEntry {
  position: string;
  kind: "natural" | "alternate";
  familiarity?: PositionFamiliarity;
}

function badgeEntries({
  natural,
  alternates,
  deployed,
  familiarity,
}: PositionBadgesProps): BadgeEntry[] {
  const entries: BadgeEntry[] = [
    { position: natural, kind: "natural" },
    ...alternates.map((position): BadgeEntry => ({ position, kind: "alternate" })),
  ];
  if (!deployed || !familiarity) return entries;
  const slot = entries.find((entry) => entry.position === deployed);
  if (slot) {
    slot.familiarity = familiarity;
    return entries;
  }
  return [...entries, { position: deployed, kind: "alternate", familiarity }];
}

/** Keeps the slot he is deployed in on screen when it would otherwise fall into the overflow. */
function visibleEntries(entries: BadgeEntry[], max: number): BadgeEntry[] {
  const shown = entries.slice(0, max);
  const deployed = entries.find((entry) => entry.familiarity);
  if (deployed && !shown.includes(deployed)) shown[max - 1] = deployed;
  return shown;
}

/** A player's positions as badges: natural filled, the others outlined, the deployed slot marked. */
export function PositionBadges(props: PositionBadgesProps) {
  const { max, describe } = props;
  const entries = badgeEntries(props);
  const shown = max === undefined ? entries : visibleEntries(entries, max);
  const hidden = entries.length - shown.length;
  return (
    <span className="inline-flex flex-wrap items-center gap-1">
      {shown.map((entry) => (
        <span key={entry.position} data-testid="position-badge-item" className="inline-flex">
          <PositionBadge
            position={entry.position}
            kind={entry.kind}
            familiarity={entry.familiarity}
            {...describe(entry.position)}
          />
        </span>
      ))}
      {hidden > 0 ? (
        <span className="text-caption text-fg-muted tabular-nums">+{hidden}</span>
      ) : null}
    </span>
  );
}
