import {
  ArrowLeftRight,
  CalendarDays,
  Building2,
  TrendingUp,
  UserRound,
} from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import type {
  CompletedTransferData,
  GameStateData,
  PlayerData,
  TeamData,
  TransferRumourData,
} from "../../store/gameStore";
import { formatDateShort } from "../../lib/dateFormatting";
import { formatVal } from "../../lib/valueFormatting";
import { Badge, Card, CardBody, CardHeader } from "../ui";

interface TransferCentreWorldTabProps {
  gameState: GameStateData;
  onSelectPlayer?: (id: string) => void;
  onSelectTeam?: (id: string) => void;
}

function sortByDateDesc<T extends { date: string }>(entries: T[]): T[] {
  return [...entries].sort((left, right) => right.date.localeCompare(left.date));
}

function playerMap(players: PlayerData[]): Map<string, PlayerData> {
  return new Map(players.map((player) => [player.id, player]));
}

function teamMap(teams: TeamData[]): Map<string, TeamData> {
  return new Map(teams.map((team) => [team.id, team]));
}

export default function TransferCentreWorldTab({
  gameState,
  onSelectPlayer,
  onSelectTeam,
}: TransferCentreWorldTabProps) {
  const { t, i18n } = useTranslation();
  const league = gameState.league;
  const rumours = sortByDateDesc(league?.transfer_rumours ?? []);
  const completedDeals = sortByDateDesc(league?.transfer_log ?? []);
  const playersById = playerMap(gameState.players);
  const teamsById = teamMap(gameState.teams);

  return (
    <div className="mx-auto max-w-6xl space-y-5">
      <Card accent="primary">
        <CardBody className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <div>
            <p className="text-xs font-heading font-bold uppercase tracking-[0.25em] text-primary-500">
              {t("transferCentreWorld.title")}
            </p>
            <h2 className="text-2xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-gray-100">
              {t("transferCentreWorld.subtitle")}
            </h2>
          </div>
          <div className="flex flex-wrap gap-2 text-xs">
            <Badge variant="primary" size="md">
              {t("transferCentreWorld.rumourTag")}
            </Badge>
            <Badge variant="accent" size="md">
              {t("transferCentreWorld.completedTag")}
            </Badge>
          </div>
        </CardBody>
      </Card>

      <div className="grid grid-cols-1 gap-5 xl:grid-cols-2">
        <TransferRumoursSection
          rumours={rumours}
          playersById={playersById}
          locale={i18n.language}
          onSelectPlayer={onSelectPlayer}
          onSelectTeam={onSelectTeam}
          t={t}
        />
        <CompletedDealsSection
          deals={completedDeals}
          playersById={playersById}
          teamsById={teamsById}
          locale={i18n.language}
          onSelectPlayer={onSelectPlayer}
          onSelectTeam={onSelectTeam}
          t={t}
        />
      </div>
    </div>
  );
}

interface SectionProps {
  t: (key: string, params?: Record<string, string | number>) => string;
  locale: string;
  onSelectPlayer?: (id: string) => void;
  onSelectTeam?: (id: string) => void;
}

interface RumoursSectionProps extends SectionProps {
  rumours: TransferRumourData[];
  playersById: Map<string, PlayerData>;
}

function TransferRumoursSection({
  rumours,
  playersById,
  locale,
  onSelectPlayer,
  onSelectTeam,
  t,
}: RumoursSectionProps) {
  return (
    <Card>
      <CardHeader
        action={<Badge variant="primary">{rumours.length}</Badge>}
      >
        {t("transferCentreWorld.rumours")}
      </CardHeader>
      <CardBody className="space-y-4">
        {rumours.length === 0 ? (
          <p className="text-sm text-gray-500 dark:text-gray-400">
            {t("transferCentreWorld.noRumours")}
          </p>
        ) : (
          rumours.map((rumour) => {
            const player = playersById.get(rumour.player_id);
            return (
              <div
                key={rumour.id}
                className="rounded-xl border border-gray-100 bg-gray-50 p-4 dark:border-navy-600 dark:bg-navy-800/70"
              >
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <button
                      type="button"
                      aria-label={t("transferCentreWorld.playerButton", {
                        player: rumour.player_name,
                      })}
                      onClick={() => onSelectPlayer?.(rumour.player_id)}
                      className="text-left text-xl font-heading font-bold uppercase tracking-wide text-primary-500 transition-colors hover:text-primary-400"
                    >
                      {rumour.player_name}
                    </button>
                    <div className="mt-2 flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                      <span className="font-heading font-bold uppercase tracking-[0.2em]">
                        {t("transferCentreWorld.currentClub")}
                      </span>
                      <button
                        type="button"
                        onClick={() => onSelectTeam?.(rumour.team_id)}
                        className="text-left font-semibold text-gray-700 transition-colors hover:text-primary-500 dark:text-gray-200"
                      >
                        {rumour.team_name}
                      </button>
                    </div>
                  </div>
                  <div className="rounded-xl bg-primary-100 p-3 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300">
                    <TrendingUp className="h-6 w-6" />
                  </div>
                </div>

                <div className="mt-4 grid grid-cols-2 gap-3 text-sm">
                  <StatTile
                    icon={<CalendarDays className="h-4 w-4" />}
                    label={t("transferCentreWorld.reportedOn")}
                    value={formatDateShort(rumour.date, locale)}
                  />
                  <StatTile
                    icon={<UserRound className="h-4 w-4" />}
                    label={t("finances.marketValue")}
                    value={player ? formatVal(player.market_value) : "-"}
                  />
                </div>
              </div>
            );
          })
        )}
      </CardBody>
    </Card>
  );
}

interface CompletedDealsSectionProps extends SectionProps {
  deals: CompletedTransferData[];
  playersById: Map<string, PlayerData>;
  teamsById: Map<string, TeamData>;
}

function CompletedDealsSection({
  deals,
  playersById,
  teamsById,
  locale,
  onSelectPlayer,
  onSelectTeam,
  t,
}: CompletedDealsSectionProps) {
  return (
    <Card accent="accent">
      <CardHeader
        action={<Badge variant="accent">{deals.length}</Badge>}
      >
        {t("transferCentreWorld.completedDeals")}
      </CardHeader>
      <CardBody className="space-y-4">
        {deals.length === 0 ? (
          <p className="text-sm text-gray-500 dark:text-gray-400">
            {t("transferCentreWorld.noCompletedDeals")}
          </p>
        ) : (
          deals.map((deal) => {
            const player = playersById.get(deal.player_id);
            const fromTeam = teamsById.get(deal.from_team_id);
            const toTeam = teamsById.get(deal.to_team_id);
            const playerName = player?.match_name ?? deal.player_id;

            return (
              <div
                key={`${deal.player_id}-${deal.date}`}
                className="rounded-xl border border-gray-100 bg-gray-50 p-4 dark:border-navy-600 dark:bg-navy-800/70"
              >
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <button
                      type="button"
                      aria-label={t("transferCentreWorld.playerButton", {
                        player: playerName,
                      })}
                      onClick={() => onSelectPlayer?.(deal.player_id)}
                      className="text-left text-xl font-heading font-bold uppercase tracking-wide text-accent-500 transition-colors hover:text-accent-400"
                    >
                      {playerName}
                    </button>
                    <div className="mt-2 grid gap-2 text-sm text-gray-500 dark:text-gray-400">
                      <TeamLine
                        label={t("transferCentreWorld.sourceClub")}
                        teamId={deal.from_team_id}
                        teamName={fromTeam?.name ?? deal.from_team_id}
                        onSelectTeam={onSelectTeam}
                      />
                      <TeamLine
                        label={t("transferCentreWorld.destinationClub")}
                        teamId={deal.to_team_id}
                        teamName={toTeam?.name ?? deal.to_team_id}
                        onSelectTeam={onSelectTeam}
                      />
                    </div>
                  </div>
                  <div className="rounded-xl bg-accent-100 p-3 text-accent-700 dark:bg-accent-900/40 dark:text-accent-300">
                    <ArrowLeftRight className="h-6 w-6" />
                  </div>
                </div>

                <div className="mt-4 grid grid-cols-2 gap-3 text-sm">
                  <StatTile
                    icon={<CalendarDays className="h-4 w-4" />}
                    label={t("transferCentreWorld.agreedOn")}
                    value={formatDateShort(deal.date, locale)}
                  />
                  <StatTile
                    icon={<Building2 className="h-4 w-4" />}
                    label={t("transferCentreWorld.fee")}
                    value={formatVal(deal.fee)}
                  />
                </div>
              </div>
            );
          })
        )}
      </CardBody>
    </Card>
  );
}

interface TeamLineProps {
  label: string;
  teamId: string;
  teamName: string;
  onSelectTeam?: (id: string) => void;
}

function TeamLine({ label, teamId, teamName, onSelectTeam }: TeamLineProps) {
  return (
    <div className="flex items-center gap-2">
      <span className="font-heading font-bold uppercase tracking-[0.2em]">{label}</span>
      <button
        type="button"
        onClick={() => onSelectTeam?.(teamId)}
        className="text-left font-semibold text-gray-700 transition-colors hover:text-primary-500 dark:text-gray-200"
      >
        {teamName}
      </button>
    </div>
  );
}

interface StatTileProps {
  icon: ReactNode;
  label: string;
  value: string;
}

function StatTile({ icon, label, value }: StatTileProps) {
  return (
    <div className="rounded-lg bg-white p-3 dark:bg-navy-700/70">
      <div className="mb-2 flex items-center gap-2 text-gray-400 dark:text-gray-500">
        {icon}
        <span className="text-[11px] font-heading font-bold uppercase tracking-[0.18em]">
          {label}
        </span>
      </div>
      <p className="text-lg font-heading font-bold text-gray-800 dark:text-gray-100">
        {value}
      </p>
    </div>
  );
}
