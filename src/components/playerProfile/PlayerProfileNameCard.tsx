import { useId, type ReactNode } from "react";

import { useRatingTiers } from "../../hooks/useRatingTiers";
import { countryName } from "../../lib/countries";
import { kitStyle } from "../../lib/kitStyle";
import { visibleRatingText } from "../../lib/playerKnowledge";
import type { PlayerData } from "../../store/gameStore";
import type { TeamData } from "../../store/types";
import ContextMenu from "../ContextMenu";
import PlayerPositionBadge from "../PlayerPositionBadge";
import { buildViewTeamMenuItem } from "../playerActions/playerContextMenuItems";
import { CountryFlag, PlayerAvatar, RatingChip } from "../ui";
import type { PlayerProfileScoutStatus, ScoutAvailability } from "./PlayerProfile.scouting";
import PlayerProfileScoutAction from "./PlayerProfileScoutAction";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

interface PlayerProfileNameCardProps {
  player: PlayerData;
  /** Null when the club does not know his rating: the chip then reads "–" and takes no tier tone. */
  ovr: number | null;
  primaryPosition: string;
  age: number;
  teamName: string;
  footednessLabel: string;
  weakFootValue: number;
  language: string;
  /** His own club's player (or a screen that cannot scout): no scouting button. */
  isOwnClub: boolean;
  scoutAvailability: ScoutAvailability;
  scoutStatus: PlayerProfileScoutStatus;
  scoutError: string | null;
  onScout: () => void;
  onSelectTeam?: (id: string) => void;
  team?: TeamData;
  /** The Actions menu, drawn beside the name. */
  actions?: ReactNode;
  t: TranslateFn;
}

/**
 * The profile's name card (round 2, approved 10 Nov): the club's kit as a stripe down the left and
 * a wash behind the identity, the avatar, the name with his positions, one meta line, the Actions
 * menu and the rating at display size. Nothing else: his condition, morale, value and wage are on
 * the tabs, once each, not in a band under the card.
 */
export default function PlayerProfileNameCard({
  player,
  ovr,
  primaryPosition,
  age,
  teamName,
  footednessLabel,
  weakFootValue,
  language,
  isOwnClub,
  scoutAvailability,
  scoutStatus,
  scoutError,
  onScout,
  onSelectTeam,
  team,
  actions,
  t,
}: PlayerProfileNameCardProps) {
  const nameId = useId();
  const tierOf = useRatingTiers();
  const clubId = player.team_id;
  const openClub = clubId && onSelectTeam ? () => onSelectTeam(clubId) : null;
  const teamContextItems = openClub ? [buildViewTeamMenuItem(t, openClub)] : [];

  return (
    <section
      aria-labelledby={nameId}
      style={kitStyle(team?.colors)}
      className="mb-4 overflow-hidden rounded-xl border border-l-4 border-line border-l-kit bg-panel text-fg shadow-card"
    >
      <div className="flex flex-wrap items-center gap-4 bg-kit-wash px-4 py-4">
        <div className="flex min-w-0 flex-1 basis-80 items-center gap-4">
          <PlayerAvatar
            player={player}
            className="flex size-16 shrink-0 items-center justify-center overflow-hidden rounded-full border border-line bg-sunken font-heading text-lead font-bold text-fg-muted"
          />
          <div className="flex min-w-0 flex-1 flex-col gap-1.5">
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
              <h2
                id={nameId}
                className="font-heading text-title font-bold uppercase tracking-label break-words"
              >
                {player.full_name}
              </h2>
              <PlayerPositionBadge position={primaryPosition} />
              {player.alternate_positions?.map((alternate) => (
                <PlayerPositionBadge key={alternate} position={alternate} kind="alternate" />
              ))}
            </div>
            <p className="flex flex-wrap items-center gap-x-2 gap-y-1 text-body text-fg-muted">
              <span className="whitespace-nowrap">
                <CountryFlag
                  code={player.nationality}
                  locale={language}
                  className="mr-1 text-body leading-none"
                />
                {countryName(player.nationality, language)}
              </span>
              <span aria-hidden="true">·</span>
              {openClub ? (
                <ContextMenu items={teamContextItems}>
                  <button
                    type="button"
                    data-testid="player-profile-team-link"
                    onClick={openClub}
                    className="rounded-sm text-selected underline underline-offset-2 hover:text-fg focus:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-page"
                  >
                    {teamName}
                  </button>
                </ContextMenu>
              ) : (
                <span>{teamName}</span>
              )}
              <span aria-hidden="true">·</span>
              <span className="whitespace-nowrap">
                {t("common.age")} {age}
              </span>
              <span aria-hidden="true">·</span>
              <span className="whitespace-nowrap">
                {t("common.footednessLabel")}: {footednessLabel}
              </span>
              <span aria-hidden="true">·</span>
              <span className="whitespace-nowrap">
                {t("common.weakFoot")}: {weakFootValue}/5
              </span>
            </p>
          </div>
        </div>
        {/* One group, so when the card is too narrow for a single row the rating wraps with the
            buttons, on the right, rather than dropping alone under the avatar. */}
        <div className="ml-auto flex items-center gap-3">
          {!isOwnClub ? (
            <PlayerProfileScoutAction
              availability={scoutAvailability}
              scoutStatus={scoutStatus}
              scoutError={scoutError}
              onScout={onScout}
            />
          ) : null}
          {actions}
          <div className="flex flex-col items-center gap-0.5">
            {ovr === null ? (
              <span
                title={t("playerProfile.ratingUnknown")}
                className="inline-flex h-16 min-w-18 items-center justify-center rounded-xl bg-sunken px-3 font-heading text-display font-bold text-fg-subtle"
              >
                <span aria-hidden="true">{visibleRatingText(ovr)}</span>
                <span className="sr-only">{t("playerProfile.ratingUnknown")}</span>
              </span>
            ) : (
              <RatingChip value={ovr} tier={tierOf(ovr)} size="display" />
            )}
            <span className="text-caption text-fg-subtle">{t("playerProfile.overall")}</span>
          </div>
        </div>
      </div>
    </section>
  );
}
