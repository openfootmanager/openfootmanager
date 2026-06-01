import { ChevronLeft, ChevronRight, ScanSearch, Search } from "lucide-react";
import { useTranslation } from "react-i18next";

import { countryName } from "../../lib/countries";
import { calcAge, formatVal, getTeamName } from "../../lib/helpers";
import type { PlayerData, TeamData } from "../../store/gameStore";
import ContextMenu from "../ContextMenu";
import {
  buildDividerMenuItem,
  buildOfferFreeAgentContractMenuItem,
  buildMakeTransferBidMenuItem,
  buildScoutPlayerMenuItem,
  buildViewProfileMenuItem,
  buildViewTeamMenuItem,
} from "../playerActions/playerContextMenuItems";
import { Badge, Card, CardBody, CardHeader, CountryFlag } from "../ui";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";

const POSITION_FILTERS = [
  "All",
  "Goalkeeper",
  "Defender",
  "Midfielder",
  "Forward",
];

interface ScoutingPlayerSearchCardProps {
  players: PlayerData[];
  teams: TeamData[];
  posFilter: string;
  searchQuery: string;
  errorMessage?: string | null;
  alreadyScoutingIds: Set<string>;
  availableScoutCount: number;
  sendingPlayerId: string | null;
  safePage: number;
  totalPages: number;
  totalPlayers: number;
  pageSize: number;
  onPositionFilterChange: (position: string) => void;
  onSearchQueryChange: (query: string) => void;
  onBidPlayer?: (player: PlayerData) => void;
  onOfferFreeAgent?: (player: PlayerData) => void;
  onSelectPlayer?: (id: string) => void;
  onSelectTeam?: (id: string) => void;
  onSendScout: (playerId: string) => void;
  onPreviousPage: () => void;
  onNextPage: () => void;
}

export default function ScoutingPlayerSearchCard({
  players,
  teams,
  posFilter,
  searchQuery,
  errorMessage,
  alreadyScoutingIds,
  availableScoutCount,
  sendingPlayerId,
  safePage,
  totalPages,
  totalPlayers,
  pageSize,
  onPositionFilterChange,
  onSearchQueryChange,
  onBidPlayer,
  onOfferFreeAgent,
  onSelectPlayer,
  onSelectTeam,
  onSendScout,
  onPreviousPage,
  onNextPage,
}: ScoutingPlayerSearchCardProps) {
  const { t, i18n } = useTranslation();

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-3 w-full">
          <span>{t("scouting.findPlayers")}</span>
          <div className="ml-auto flex items-center gap-2">
            {POSITION_FILTERS.map((position) => (
              <button
                key={position}
                onClick={() => onPositionFilterChange(position)}
                className={`px-2.5 py-1 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-colors ${posFilter === position
                  ? "bg-primary-500 text-white"
                  : "bg-gray-100 dark:bg-navy-700 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-navy-600"
                  }`}
              >
                {position === "All"
                  ? t("common.all")
                  : position.slice(0, 3)}
              </button>
            ))}
          </div>
        </div>
      </CardHeader>
      <CardBody>
        <div className="relative mb-3">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
          <input
            type="text"
            placeholder={t("scouting.searchPlaceholder")}
            value={searchQuery}
            onChange={(event) => onSearchQueryChange(event.target.value)}
            className="w-full pl-9 pr-4 py-2 text-sm bg-gray-50 dark:bg-navy-700 border border-gray-200 dark:border-navy-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500/50 text-gray-800 dark:text-gray-100 placeholder:text-gray-400"
          />
        </div>

        {errorMessage ? (
          <p
            role="alert"
            className="mb-3 text-xs font-heading font-bold uppercase tracking-wider text-red-500"
          >
            {errorMessage}
          </p>
        ) : null}

        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-xs text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider border-b border-gray-100 dark:border-navy-700">
                <th className="text-left py-2 px-2">{t("scouting.player")}</th>
                <th className="text-left py-2 px-1">{t("scouting.pos")}</th>
                <th className="text-center py-2 px-1">{t("scouting.age")}</th>
                <th className="text-left py-2 px-1">{t("scouting.team")}</th>
                <th className="text-center py-2 px-1">{t("scouting.value")}</th>
                <th className="text-right py-2 px-2">{t("scouting.action")}</th>
              </tr>
            </thead>
            <tbody>
              {players.map((player) => {
                const isScouting = alreadyScoutingIds.has(player.id);
                const team = player.team_id
                  ? getTeamName(teams, player.team_id)
                  : t("common.freeAgent");
                const scoutState = isScouting
                  ? "already-assigned"
                  : sendingPlayerId === player.id
                    ? "busy"
                    : availableScoutCount === 0
                      ? "unavailable"
                      : "ready";
                const contextItems = [
                  ...(onSelectPlayer
                    ? [
                      buildViewProfileMenuItem(t, () => {
                        onSelectPlayer(player.id);
                      }),
                    ]
                    : []),
                  ...(player.team_id && onSelectTeam
                    ? [
                      buildViewTeamMenuItem(t, () => {
                        onSelectTeam(player.team_id!);
                      }),
                    ]
                    : []),
                  buildDividerMenuItem(),
                  ...(player.team_id && onBidPlayer
                    ? [
                      buildMakeTransferBidMenuItem(t, () => {
                        onBidPlayer(player);
                      }),
                    ]
                    : !player.team_id && onOfferFreeAgent
                      ? [
                        buildOfferFreeAgentContractMenuItem(t, () => {
                          onOfferFreeAgent(player);
                        }),
                      ]
                    : []),
                  buildScoutPlayerMenuItem(t, scoutState, () => {
                    onSendScout(player.id);
                  }),
                ];

                const row = (
                  <tr
                    key={player.id}
                    className="border-b border-gray-50 dark:border-navy-700/50 hover:bg-gray-50 dark:hover:bg-navy-700/30 transition-colors"
                  >
                    <td className="py-2 px-2">
                      <button
                        onClick={() => onSelectPlayer?.(player.id)}
                        className="font-heading font-bold text-gray-800 dark:text-gray-100 hover:text-primary-500 transition-colors text-left"
                      >
                        {player.full_name}
                      </button>
                      <div className="text-[10px] text-gray-400 mt-0.5 flex items-center gap-1">
                        <CountryFlag
                          code={player.nationality}
                          locale={i18n.language}
                          className="text-xs leading-none"
                        />
                        <span>{countryName(player.nationality, i18n.language)}</span>
                      </div>
                    </td>
                    <td className="py-2 px-1">
                      <Badge
                        variant={
                          player.position === "Goalkeeper"
                            ? "accent"
                            : player.position === "Defender"
                              ? "primary"
                              : player.position === "Midfielder"
                                ? "success"
                                : "danger"
                        }
                        size="sm"
                      >
                        {translatePositionAbbreviation(t, player.position)}
                      </Badge>
                    </td>
                    <td className="text-center py-2 px-1 text-gray-600 dark:text-gray-400">
                      {calcAge(player.date_of_birth)}
                    </td>
                    <td className="py-2 px-1 text-gray-600 dark:text-gray-400 text-xs truncate max-w-[120px]">
                      {team}
                    </td>
                    <td className="text-center py-2 px-1 text-gray-600 dark:text-gray-400 text-xs">
                      {formatVal(player.market_value)}
                    </td>
                    <td className="text-right py-2 px-2">
                      {isScouting ? (
                        <span className="text-xs text-primary-400 font-heading font-bold">
                          {t("scouting.scoutingInProgress")}
                        </span>
                      ) : availableScoutCount === 0 ? (
                        <span className="text-xs text-gray-400">
                          {t("scouting.noScoutsFree")}
                        </span>
                      ) : (
                        <button
                          disabled={sendingPlayerId === player.id}
                          onClick={() => onSendScout(player.id)}
                          className="flex items-center gap-1 ml-auto px-2.5 py-1 rounded-lg bg-primary-500/10 text-primary-500 hover:bg-primary-500/20 transition-colors text-xs font-heading font-bold uppercase tracking-wider disabled:opacity-50"
                        >
                          <ScanSearch className="w-3 h-3" />
                          {sendingPlayerId === player.id ? "..." : t("scouting.scoutBtn")}
                        </button>
                      )}
                    </td>
                  </tr>
                );

                return (
                  <ContextMenu items={contextItems} key={player.id}>
                    {row}
                  </ContextMenu>
                );
              })}
            </tbody>
          </table>
          {players.length === 0 && (
            <p className="text-center text-sm text-gray-400 py-4">
              {t("scouting.noPlayersFound")}
            </p>
          )}
        </div>

        {totalPages > 1 && (
          <div className="flex items-center justify-between pt-3 border-t border-gray-100 dark:border-navy-700 mt-3">
            <span className="text-xs text-gray-400 dark:text-gray-500">
              {t("scouting.showingRange", {
                from: safePage * pageSize + 1,
                to: Math.min((safePage + 1) * pageSize, totalPlayers),
                total: totalPlayers,
              })}
            </span>
            <div className="flex items-center gap-2">
              <button
                aria-label={t("scouting.previousPage")}
                disabled={safePage === 0}
                onClick={onPreviousPage}
                className="p-1.5 rounded-lg bg-gray-100 dark:bg-navy-700 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-navy-600 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
              >
                <ChevronLeft className="w-4 h-4" />
              </button>
              <span className="text-xs font-heading font-bold text-gray-500 dark:text-gray-400 tabular-nums">
                {safePage + 1} / {totalPages}
              </span>
              <button
                aria-label={t("scouting.nextPage")}
                disabled={safePage >= totalPages - 1}
                onClick={onNextPage}
                className="p-1.5 rounded-lg bg-gray-100 dark:bg-navy-700 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-navy-600 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
              >
                <ChevronRight className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}
      </CardBody>
    </Card>
  );
}
