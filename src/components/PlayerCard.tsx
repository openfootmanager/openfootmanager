import type { ComponentProps, ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { visibleRatingText } from "../lib/playerKnowledge";
import type { RatingTier } from "../services/squadService";
import type { TeamColors } from "../store/types";
import { CountryFlag, EntityCard, PlayerAvatar, RatingChip } from "./ui";
import PlayerPositionBadge from "./PlayerPositionBadge";

interface PlayerCardProps {
  /** What the portrait is drawn from: the player's identity and face. */
  portrait: ComponentProps<typeof PlayerAvatar>["player"];
  name: string;
  /** The position to show, natural or granular as the list knows it. */
  position: string;
  nationality: string;
  age: number;
  /** His club's name; null for a free agent, which draws no club. */
  teamName: string | null;
  onSelectTeam?: () => void;
  /** His rating as the club may see it; null when hidden. */
  ovr: number | null;
  /** The backend's tier for the rating (`useRatingTiers`); null when the rating is hidden. */
  tier: RatingTier | null;
  /** His club's kit; null for a free agent, which gets the neutral card. */
  kit?: TeamColors | null;
  /** Listing, injury and contract chips, if the list has any to show. */
  status?: ReactNode;
  /** The two facts the list chooses: its own figures, or the club's view of another's. */
  facts: ComponentProps<typeof EntityCard>["facts"];
  onOpen?: () => void;
}

/**
 * A player's card for the Players, Transfer market and Scouting lists: the shared card shell with
 * his face, position, country, age and club in the identity row, his rating as the hero, and the
 * two facts the list passes. The list decides what it may show; the card only draws it.
 */
export default function PlayerCard({
  portrait,
  name,
  position,
  nationality,
  age,
  teamName,
  onSelectTeam,
  ovr,
  tier,
  kit,
  status,
  facts,
  onOpen,
}: PlayerCardProps) {
  const { t, i18n } = useTranslation();
  return (
    <EntityCard
      kit={kit}
      media={
        <PlayerAvatar
          player={portrait}
          className="size-12 shrink-0 overflow-hidden rounded-full bg-sunken flex items-center justify-center text-caption font-heading font-bold text-fg-muted"
        />
      }
      name={name}
      meta={
        <>
          <PlayerPositionBadge position={position} />
          <CountryFlag code={nationality} locale={i18n.language} />
          <span>
            <span className="sr-only">{t("common.age")} </span>
            {age}
          </span>
          {teamName ? (
            onSelectTeam ? (
              <button
                type="button"
                onClick={onSelectTeam}
                className="rounded-sm hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-panel"
              >
                {teamName}
              </button>
            ) : (
              <span>{teamName}</span>
            )
          ) : null}
        </>
      }
      hero={
        ovr !== null && tier !== null ? (
          <RatingChip value={ovr} tier={tier} size="md" />
        ) : (
          <span className="text-fg-muted">{visibleRatingText(null)}</span>
        )
      }
      status={status}
      facts={facts}
      onOpen={onOpen}
    />
  );
}
