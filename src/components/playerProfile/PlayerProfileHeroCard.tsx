import { useRatingTiers } from "../../hooks/useRatingTiers";
import { ratingTierClass } from "../../lib/ratingTier";
import { Shield } from "lucide-react";
import { countryName } from "../../lib/countries";
import type { PlayerData } from "../../store/gameStore";
import ContextMenu from "../ContextMenu";
import { buildViewTeamMenuItem } from "../playerActions/playerContextMenuItems";
import { translatePositionLabel } from "../squad/SquadTab.helpers";
import { visibleRatingText } from "../../lib/playerKnowledge";
import { formatPlayerMarketValue } from "./PlayerProfile.helpers";
import type { PlayerProfileScoutStatus, ScoutAvailability } from "./PlayerProfile.scouting";
import PlayerProfileScoutAction from "./PlayerProfileScoutAction";
import { TraitList } from "../TraitBadge";
import { Badge, Card, CountryFlag, JerseyIcon, PlayerAvatar } from "../ui";
import type { TeamData } from "../../store/types";
import PlayerPositionBadge from "../PlayerPositionBadge";

type TranslateFn = (key: string, options?: Record<string, string | number>) => string;

const NO_DATA = "–";

interface PlayerProfileHeroCardProps {
  player: PlayerData;
  /** Null when the club does not know his rating: the tile reads "–" and takes no tier colour. */
  ovr: number | null;
  primaryPosition: string;
  age: number;
  teamName: string;
  footednessLabel: string;
  weakFootValue: number;
  language: string;
  isOwnClub: boolean;
  scoutAvailability: ScoutAvailability;
  scoutStatus: PlayerProfileScoutStatus;
  scoutError: string | null;
  onScout: () => void;
  onSelectTeam?: (id: string) => void;
  team?: TeamData;
  /** The traits the club may see (`visibleTraits`). */
  traits: string[];
  /** Condition, morale and the exact wage are shown only for the club's own player. */
  showsPrivateState: boolean;
  /** His weekly wage as the club may see it, already written: exact, approximate or "–". */
  wageText: string;
  t: TranslateFn;
}

export default function PlayerProfileHeroCard({
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
  traits,
  showsPrivateState,
  wageText,
  t,
}: PlayerProfileHeroCardProps) {
  const tierOf = useRatingTiers();
  const teamContextItems =
    player.team_id && onSelectTeam
      ? [buildViewTeamMenuItem(t, () => onSelectTeam(player.team_id!))]
      : [];

  return (
    <Card accent="primary" className="mb-5">
      <div className="bg-linear-to-r from-navy-700 to-navy-800 p-8 rounded-t-xl">
        <div className="flex flex-wrap items-start gap-6">
          <PlayerAvatar
            player={player}
            className="w-24 h-24 shrink-0 rounded-2xl flex items-center justify-center font-heading font-bold text-3xl border-2 border-white/10 bg-white/5 text-gray-300 overflow-hidden"
          />
          <div
            className={`flex h-24 w-20 shrink-0 flex-col items-center justify-center rounded-2xl border-2 font-heading ${ovr === null ? "border-white/10 bg-white/5 text-chrome-fg-muted" : ratingTierClass(tierOf(ovr), "hero")}`}
          >
            <span className="text-xs font-bold uppercase tracking-wider">{t("common.ovr")}</span>
            <span className="text-4xl font-bold leading-none">{visibleRatingText(ovr)}</span>
          </div>
          {player.jersey_number != null && team != null && (
            <JerseyIcon
              primaryColor={team.colors.primary}
              secondaryColor={team.colors.secondary}
              pattern={team.kit_pattern ?? "Solid"}
              number={player.jersey_number}
              size="lg"
              className="flex-shrink-0 self-center"
            />
          )}
          <div className="min-w-0 flex-1 basis-64">
            <h2 className="text-3xl font-heading font-bold text-white uppercase tracking-wide break-words">
              {player.full_name}
            </h2>
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 mt-2">
              <PlayerPositionBadge position={primaryPosition} long />
              {player.alternate_positions?.map((alternatePosition) => (
                <Badge key={alternatePosition} variant="neutral">
                  {translatePositionLabel(t, alternatePosition)}
                </Badge>
              ))}
              <span className="whitespace-nowrap text-gray-400 text-sm">
                <CountryFlag
                  code={player.nationality}
                  locale={language}
                  className="mr-1 text-sm leading-none"
                />
                {countryName(player.nationality, language)}
              </span>
              <span className="text-gray-500">•</span>
              <span className="whitespace-nowrap text-gray-400 text-sm">
                {t("common.age")} {age}
              </span>
              <span className="text-gray-500">•</span>
              <span className="whitespace-nowrap text-gray-400 text-sm">
                {t("common.footednessLabel")}: {footednessLabel}
              </span>
              <span className="text-gray-500">•</span>
              <span className="whitespace-nowrap text-gray-400 text-sm">
                {t("common.weakFoot")}: {weakFootValue}/5
              </span>
            </div>
            <p className="text-gray-400 text-sm mt-2 flex items-center gap-1.5">
              <Shield className="w-4 h-4" />
              {player.team_id && onSelectTeam ? (
                <ContextMenu items={teamContextItems}>
                  <button
                    type="button"
                    data-testid="player-profile-team-link"
                    onClick={() => onSelectTeam(player.team_id!)}
                    className="hover:text-primary-400 transition-colors underline underline-offset-2"
                  >
                    {teamName}
                  </button>
                </ContextMenu>
              ) : (
                <span>{teamName}</span>
              )}
            </p>
            {traits.length > 0 ? (
              <div className="mt-3">
                <TraitList traits={traits} size="sm" />
              </div>
            ) : null}
          </div>

          {!isOwnClub ? (
            <div className="mt-3">
              <PlayerProfileScoutAction
                availability={scoutAvailability}
                scoutStatus={scoutStatus}
                scoutError={scoutError}
                onScout={onScout}
              />
            </div>
          ) : null}

          <div className="hidden md:grid grid-cols-2 gap-3 shrink-0">
            <QuickStat
              label={t("common.condition")}
              value={showsPrivateState ? `${player.condition}%` : NO_DATA}
              color={
                !showsPrivateState
                  ? "text-chrome-fg-muted"
                  : player.condition >= 70
                    ? "text-primary-400"
                    : "text-red-400"
              }
            />
            <QuickStat
              label={t("common.morale")}
              value={showsPrivateState ? `${player.morale}%` : NO_DATA}
              color={
                !showsPrivateState
                  ? "text-chrome-fg-muted"
                  : player.morale >= 70
                    ? "text-primary-400"
                    : "text-accent-400"
              }
            />
            <QuickStat
              label={t("common.value")}
              value={formatPlayerMarketValue(player.market_value)}
              color="text-white"
            />
            <QuickStat
              label={t("common.wage")}
              value={wageText}
              color={wageText === NO_DATA ? "text-chrome-fg-muted" : "text-white"}
            />
          </div>
        </div>
      </div>

      <div className="grid grid-cols-4 gap-px bg-gray-200 dark:bg-navy-600 md:hidden">
        <MobileQuickStat
          label={t("common.condition")}
          value={showsPrivateState ? `${player.condition}%` : NO_DATA}
          color={
            !showsPrivateState
              ? "text-fg-muted"
              : player.condition >= 70
                ? "text-primary-500"
                : "text-red-500"
          }
        />
        <MobileQuickStat
          label={t("common.morale")}
          value={showsPrivateState ? `${player.morale}%` : NO_DATA}
          color={
            !showsPrivateState
              ? "text-fg-muted"
              : player.morale >= 70
                ? "text-primary-500"
                : "text-accent-500"
          }
        />
        <MobileQuickStat
          label={t("common.value")}
          value={formatPlayerMarketValue(player.market_value)}
          color="text-gray-700 dark:text-gray-200"
        />
        <MobileQuickStat
          label={t("common.wage")}
          value={wageText}
          color={wageText === NO_DATA ? "text-fg-muted" : "text-gray-700 dark:text-gray-200"}
        />
      </div>
    </Card>
  );
}

function QuickStat({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="bg-white/5 rounded-xl px-5 py-3 text-center min-w-25">
      <p className="text-xs text-gray-400 font-heading uppercase tracking-wider">{label}</p>
      <p className={`font-heading font-bold text-xl mt-0.5 ${color}`}>{value}</p>
    </div>
  );
}

function MobileQuickStat({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="bg-white dark:bg-navy-800 p-3 text-center">
      <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
        {label}
      </p>
      <p className={`font-heading font-bold text-lg mt-0.5 ${color}`}>{value}</p>
    </div>
  );
}
