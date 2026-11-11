import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { useRatingTiers } from "../../../hooks/useRatingTiers";
import { formatDateFull, getContractRiskBadgeVariant } from "../../../lib/helpers";
import { getPlayerSquadRole } from "../../../lib/playerSquad";
import type { PositionRatings } from "../../../store/generated/PositionRatings";
import type { PlayerData } from "../../../store/gameStore";
import { Badge, ConditionMeter, FormDots, FormResultMark, RatingChip, Select } from "../../ui";
import { TraitList } from "../../TraitBadge";
import { formatPlayerMarketValue } from "../PlayerProfile.helpers";
import type { PlayerRecentMatchEntry } from "../PlayerProfile.helpers";
import { RetryButton } from "../ProfileLoadFailed";
import { OverviewCard, OverviewRow } from "./OverviewCard";
import { formOf, resultOf, shortLabels } from "./overviewModel";
import { isGoalkeeper } from "../PlayerProfile.attributes";
import { PlayerProfileRadar, type RadarAxis } from "./PlayerProfileRadar";

type PlayerRole = string;

export interface PlayerProfileOverviewProps {
  player: PlayerData;
  language: string;
  /** False for a rival, a scouted player or a stranger: his condition and morale are not the club's to see. */
  showsPrivateState: boolean;
  /** Latest first, as the backend returns them. */
  recentMatches: PlayerRecentMatchEntry[];
  seasonRatingRated: boolean;
  /** The own player's tactical role and how to change it; null for anyone else. */
  tacticalRole: {
    value: PlayerRole;
    options: PlayerRole[];
    onChange: (role: PlayerRole) => void;
  } | null;
  contractRiskLevel: "critical" | "warning" | "stable";
  contractRiskLabel: string;
  /** His weekly wage as the club may see it, already written: exact, approximate or "–". */
  wageText: string;
  /** Set when the wage failed to load: the row reads "Couldn't load" with a retry. */
  wageRetry?: () => void;
  /** The club's opinion of his potential, or its load-failed state, drawn at the foot of Details. */
  potentialOpinion: ReactNode;
  positionRatings: PositionRatings | null;
  /** The attributes the club may read (a scout's report for a scouted player); null when none. */
  attributes: PlayerData["attributes"] | null;
  traits: string[];
  onOpenTab: (tab: "career") => void;
}

/**
 * The profile's Overview, laid out as the approved board (round 2): Now and Details on the left,
 * his previous matches and season in the middle, his attributes and traits on the right. Below the
 * container's wide width it stacks. Each card is a labelled region, and nothing here is drawn twice
 * on the profile.
 */
export default function PlayerProfileOverview(props: PlayerProfileOverviewProps) {
  return (
    <div className="@container">
      <div className="grid grid-cols-1 gap-3 @[40rem]:grid-cols-2 @[58rem]:grid-cols-[18.75rem_minmax(0,1fr)_minmax(0,1fr)]">
        <div className="flex min-w-0 flex-col gap-3">
          <NowCard {...props} />
          <DetailsCard {...props} />
        </div>
        <div className="flex min-w-0 flex-col gap-3">
          <PreviousMatchesCard {...props} />
          <SeasonCard {...props} />
        </div>
        <div className="flex min-w-0 flex-col gap-3 @[40rem]:col-span-2 @[58rem]:col-span-1">
          <AttributesCard {...props} />
          <TraitsCard {...props} />
        </div>
      </div>
    </div>
  );
}

const NO_VALUE = "–";

/** The radar's eight axes, the same for every outfield player so two radars compare at a glance. */
const OUTFIELD_AXES = [
  "pace",
  "shooting",
  "dribbling",
  "passing",
  "vision",
  "composure",
  "strength",
  "aerial",
] as const;

/** A goalkeeper's eight, in place of the outfield ones. */
const GOALKEEPER_AXES = [
  "handling",
  "reflexes",
  "aerial",
  "composure",
  "vision",
  "agility",
  "stamina",
  "strength",
] as const;

function NowCard({
  player,
  showsPrivateState,
  recentMatches,
  tacticalRole,
}: PlayerProfileOverviewProps) {
  const { t } = useTranslation();
  const form = formOf(recentMatches);
  return (
    <OverviewCard title={t("playerProfile.overview.now")}>
      <OverviewRow label={t("common.condition")}>
        {showsPrivateState ? (
          <ConditionMeter value={player.condition} status={player.condition_status} />
        ) : (
          NO_VALUE
        )}
      </OverviewRow>
      <OverviewRow label={t("common.morale")}>
        {showsPrivateState ? `${player.morale}%` : NO_VALUE}
      </OverviewRow>
      {form.length > 0 ? (
        <OverviewRow label={t("common.recentForm")}>
          <FormDots results={form} />
        </OverviewRow>
      ) : null}
      {tacticalRole ? (
        <OverviewRow label={t("tactics.playerRoleLabel")}>
          <Select
            selectSize="sm"
            value={tacticalRole.value}
            onChange={(event) => tacticalRole.onChange(event.target.value)}
            aria-label={t("tactics.playerRoleLabel")}
          >
            {tacticalRole.options.map((role) => (
              <option key={role} value={role}>
                {t(`tactics.playerRoles.${role}`, role)}
              </option>
            ))}
          </Select>
        </OverviewRow>
      ) : null}
    </OverviewCard>
  );
}

function DetailsCard(props: PlayerProfileOverviewProps) {
  const { player, language, contractRiskLevel, contractRiskLabel, wageText, wageRetry } = props;
  const { t } = useTranslation();
  const squadRole = getPlayerSquadRole(player);
  return (
    <OverviewCard title={t("playerProfile.overview.details")} className="flex-1">
      <OverviewRow label={t("playerProfile.overview.born")}>
        {formatDateFull(player.date_of_birth, language)}
      </OverviewRow>
      <OverviewRow label={t("playerProfile.overview.squadRole")}>
        {squadRole === "Youth"
          ? t("playerProfile.overview.squadYouth")
          : t("playerProfile.overview.squadSenior")}
      </OverviewRow>
      <OverviewRow label={t("common.contract")}>
        {player.contract_end ? (
          <>
            {t("playerProfile.overview.contractUntil", {
              date: formatDateFull(player.contract_end, language),
            })}
            <Badge variant={getContractRiskBadgeVariant(contractRiskLevel)}>
              {contractRiskLabel}
            </Badge>
          </>
        ) : (
          t("playerProfile.noContract")
        )}
      </OverviewRow>
      <OverviewRow label={t("common.value")}>
        {formatPlayerMarketValue(player.market_value)}
      </OverviewRow>
      <OverviewRow label={t("common.wage")}>
        {wageRetry ? (
          <>
            {t("playerProfile.loadFailed")}
            <RetryButton onRetry={wageRetry} description={t("playerProfile.retryWage")}>
              {t("common.retry")}
            </RetryButton>
          </>
        ) : (
          wageText
        )}
      </OverviewRow>
      {props.potentialOpinion ? (
        <div className="mt-auto border-t border-line bg-sunken px-4 py-2.5">
          {props.potentialOpinion}
        </div>
      ) : null}
    </OverviewCard>
  );
}

function PreviousMatchesCard({ recentMatches, onOpenTab }: PlayerProfileOverviewProps) {
  const { t } = useTranslation();
  const shown = recentMatches.slice(0, 5);
  return (
    <OverviewCard
      title={t("playerProfile.overview.previousMatches")}
      link={{ label: t("playerProfile.overview.allMatches"), onPress: () => onOpenTab("career") }}
    >
      {shown.length === 0 ? (
        <p className="px-4 py-3 text-body text-fg-muted">{t("playerProfile.noRecentMatches")}</p>
      ) : (
        // biome-ignore lint/a11y/noRedundantRoles: WebKit drops a list's semantics under list-style: none, so the role is restated.
        <ul role="list" className="m-0 list-none p-0">
          {shown.map((match) => (
            <li
              key={match.fixture_id}
              className="flex items-center gap-3 border-b border-line px-4 py-2 text-body last:border-b-0"
            >
              <FormResultMark result={resultOf(match)} className="size-5" />
              <span className="min-w-0 flex-1 truncate">
                {t("playerProfile.vs")} {match.opponent_name}
              </span>
              <span className="font-semibold tabular-nums">
                {match.team_goals}–{match.opponent_goals}
              </span>
              <span className="text-caption text-fg-muted">{contribution(match, t)}</span>
              <span className="w-8 text-right font-semibold tabular-nums">
                {match.rated ? match.rating.toFixed(1) : NO_VALUE}
              </span>
            </li>
          ))}
        </ul>
      )}
    </OverviewCard>
  );
}

/** His part in the match, in words: his goals, else his assists, else nothing to say. */
function contribution(
  match: PlayerRecentMatchEntry,
  t: (key: string, options?: Record<string, string | number>) => string,
): string {
  if (match.goals > 0) return t("playerProfile.nGoals", { count: match.goals });
  if (match.assists > 0) return t("playerProfile.nAssists", { count: match.assists });
  return NO_VALUE;
}

function SeasonCard({
  player,
  language,
  seasonRatingRated,
  onOpenTab,
}: PlayerProfileOverviewProps) {
  const { t } = useTranslation();
  const stats = player.stats;
  const tiles: [string, string][] = [
    [t("playerProfile.apps"), String(stats.appearances)],
    [t("playerProfile.goals"), String(stats.goals)],
    [t("playerProfile.assists"), String(stats.assists)],
    [t("playerProfile.avgRating"), seasonRatingRated ? stats.avg_rating.toFixed(1) : NO_VALUE],
    [t("playerProfile.mins"), stats.minutes_played.toLocaleString(language)],
  ];
  return (
    <OverviewCard
      title={t("playerProfile.seasonStats")}
      link={{ label: t("playerProfile.tabs.career"), onPress: () => onOpenTab("career") }}
    >
      <div className="grid grid-cols-5 gap-2 p-3">
        {tiles.map(([label, value]) => (
          <div key={label} className="flex min-w-0 flex-col gap-0.5">
            <span className="text-caption text-fg-muted">{label}</span>
            <span className="font-heading text-title font-bold tabular-nums text-fg">{value}</span>
          </div>
        ))}
      </div>
    </OverviewCard>
  );
}

function AttributesCard({ player, positionRatings, attributes }: PlayerProfileOverviewProps) {
  const { t } = useTranslation();
  const tierOf = useRatingTiers();
  const natural = positionRatings?.rows[0];
  // The club can read his attributes but the position's ratings have not arrived: no axes to draw yet.
  if (attributes && !natural) return null;
  if (!attributes || !natural) {
    return (
      <OverviewCard title={t("playerProfile.attributes")}>
        <div className="px-4 py-3">
          <p className="font-semibold text-fg">{t("playerProfile.attributesHidden")}</p>
          <p className="text-body text-fg-muted">{t("playerProfile.scoutToView")}</p>
        </div>
      </OverviewCard>
    );
  }
  const keyAttributes = new Set<string>(natural.key_attributes.map((key) => key.attribute));
  // The axes are fixed; the position's key attributes (what the game weighs most there) only mark
  // which of them are drawn heavier.
  const shown = isGoalkeeper(player) ? GOALKEEPER_AXES : OUTFIELD_AXES;
  const names = shown.map((attribute) => t(`common.attributes.${attribute}`));
  const short = shortLabels(names);
  const axes: RadarAxis[] = shown.map((attribute, index) => ({
    label: names[index],
    short: short[index],
    value: attributes[attribute],
    key: keyAttributes.has(attribute),
  }));
  return (
    <OverviewCard title={t("playerProfile.attributes")}>
      <div className="px-2 pt-2">
        <PlayerProfileRadar axes={axes} />
      </div>
      <div className="flex flex-col gap-1.5 border-t border-line px-4 py-2.5">
        <span className="text-caption text-fg-muted">
          {t("playerProfile.overview.keyFor", {
            position: t(`common.positions.${natural.position}`, natural.position),
          })}
        </span>
        <ul className="m-0 flex list-none flex-wrap gap-x-4 gap-y-1 p-0">
          {natural.key_attributes.map(({ attribute, value }) => (
            <li key={attribute} className="inline-flex items-center gap-1.5 font-semibold">
              <span>{t(`common.attributes.${attribute}`)}</span>
              <RatingChip
                value={value}
                tier={tierOf(value)}
                name={t(`common.attributes.${attribute}`)}
              />
            </li>
          ))}
        </ul>
      </div>
    </OverviewCard>
  );
}

function TraitsCard({ traits }: PlayerProfileOverviewProps) {
  const { t } = useTranslation();
  if (traits.length === 0) return null;
  return (
    <OverviewCard title={t("playerProfile.overview.traits")} className="flex-1">
      <div className="p-3">
        <TraitList traits={traits} size="sm" />
      </div>
    </OverviewCard>
  );
}
