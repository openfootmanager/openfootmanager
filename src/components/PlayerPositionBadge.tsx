import { useTranslation } from "react-i18next";

import { PositionBadge } from "./ui";
import type { PositionFamiliarity } from "./ui/PositionBadge";
import { translatePositionAbbreviation, translatePositionLabel } from "./squad/SquadTab.helpers";

interface PlayerPositionBadgeProps {
  /** The position to draw, natural or granular. */
  position: string;
  /** Show the position's name where there is room, instead of its short label. */
  long?: boolean;
  kind?: "natural" | "alternate";
  familiarity?: PositionFamiliarity;
}

/** A position badge with its translated labels, for the screens that show a player's position. */
export default function PlayerPositionBadge({
  position,
  long = false,
  kind,
  familiarity,
}: PlayerPositionBadgeProps) {
  const { t } = useTranslation();
  const name = translatePositionLabel(t, position);
  return (
    <PositionBadge
      position={position}
      label={long ? name : translatePositionAbbreviation(t, position)}
      name={name}
      kind={kind}
      familiarity={familiarity}
    />
  );
}
