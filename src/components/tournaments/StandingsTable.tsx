import { useTranslation } from "react-i18next";

import ContextMenu from "../ContextMenu";
import { buildViewTeamMenuItem } from "../playerActions/playerContextMenuItems";
import type { TournamentsTeamLookup } from "./teamLookup";
import type { StandingData } from "../../store/gameStore";

interface StandingsTableProps {
  standings: StandingData[];
  /**
   * `full` is the standings screen's own table: roomier, with goals for and
   * against, and promotion/relegation marks down the left. `compact` is the
   * summary the overview shows alongside everything else.
   */
  variant: "compact" | "full";
  teams: TournamentsTeamLookup;
  /** Rows get `${testIdPrefix}-${team_id}`, so the two tables stay tellable apart. */
  testIdPrefix: string;
  zones?: { promotionSlots: number; relegationSlots: number };
}

/** Already sorted by the caller — this only decides how a table looks. */
export default function StandingsTable({
  standings,
  variant,
  teams,
  testIdPrefix,
  zones,
}: StandingsTableProps) {
  const { t } = useTranslation();
  const { userTeamId, isClubTeam, resolveTeamName, onSelectTeam } = teams;
  const pad = variant === "full" ? "py-3 px-4" : "py-2 px-3";
  const headPad = variant === "full" ? "py-3 px-4" : "py-2 px-3";

  const statColumns: {
    key: string;
    label: string;
    value: (entry: StandingData) => number;
  }[] = [
    { key: "played", label: t("common.played"), value: (e) => e.played },
    { key: "won", label: t("common.won"), value: (e) => e.won },
    { key: "drawn", label: t("common.drawn"), value: (e) => e.drawn },
    { key: "lost", label: t("common.lost"), value: (e) => e.lost },
    ...(variant === "full"
      ? [
          { key: "gf", label: t("common.gf"), value: (e: StandingData) => e.goals_for },
          {
            key: "ga",
            label: t("common.ga"),
            value: (e: StandingData) => e.goals_against,
          },
        ]
      : []),
  ];

  const headClass = `${headPad} font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400`;
  const statClass = `${pad} text-center text-sm text-gray-600 dark:text-gray-400 tabular-nums`;

  return (
    <table className="w-full text-left border-collapse">
      <thead>
        <tr className="bg-gray-50 dark:bg-navy-800 border-b border-gray-200 dark:border-navy-600 text-xs">
          <th className={`${headClass} w-8`}>#</th>
          <th className={headClass}>{t("common.team")}</th>
          {statColumns.map((column) => (
            <th key={column.key} className={`${headClass} text-center`}>
              {column.label}
            </th>
          ))}
          <th className={`${headClass} text-center`}>{t("common.gd")}</th>
          <th className={`${headClass} text-center`}>{t("common.pts")}</th>
        </tr>
      </thead>
      <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
        {standings.map((entry, idx) => {
          const isUser = entry.team_id === userTeamId;
          const gd = entry.goals_for - entry.goals_against;
          const inPromotionZone = idx < (zones?.promotionSlots ?? 0);
          const inRelegationZone =
            (zones?.relegationSlots ?? 0) > 0 &&
            idx >= standings.length - (zones?.relegationSlots ?? 0);

          // A qualifying group with no groups defined lands national teams in
          // this table, and those have no team page to open.
          const clickable = isClubTeam(entry.team_id);

          return (
            <ContextMenu
              items={
                clickable
                  ? [buildViewTeamMenuItem(t, () => onSelectTeam(entry.team_id))]
                  : []
              }
              key={entry.team_id}
            >
              <tr
                onClick={clickable ? () => onSelectTeam(entry.team_id) : undefined}
                className={`transition-colors ${clickable ? "cursor-pointer" : ""} ${isUser ? "bg-primary-50 dark:bg-primary-500/10" : "hover:bg-gray-50 dark:hover:bg-navy-700/50"}`}
                data-testid={`${testIdPrefix}-${entry.team_id}`}
              >
                <td
                  className={`${pad} font-heading font-bold text-sm ${
                    inPromotionZone
                      ? "border-l-2 border-primary-500 text-primary-500"
                      : inRelegationZone
                        ? "border-l-2 border-red-500 text-red-500"
                        : "text-gray-400"
                  }`}
                  data-testid={
                    inPromotionZone
                      ? `tournaments-promotion-${entry.team_id}`
                      : inRelegationZone
                        ? `tournaments-relegation-${entry.team_id}`
                        : undefined
                  }
                >
                  {idx + 1}
                </td>
                <td
                  className={`${pad} font-semibold text-sm ${isUser ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                >
                  {clickable ? (
                    <button
                      type="button"
                      onClick={(event) => {
                        // The row is clickable too; without this the team opens twice.
                        event.stopPropagation();
                        onSelectTeam(entry.team_id);
                      }}
                      className="text-left hover:underline focus:outline-none focus:ring-2 focus:ring-primary-500 rounded"
                    >
                      {resolveTeamName(entry.team_id)}
                    </button>
                  ) : (
                    resolveTeamName(entry.team_id)
                  )}
                </td>
                {statColumns.map((column) => (
                  <td key={column.key} className={statClass}>
                    {column.value(entry)}
                  </td>
                ))}
                <td
                  className={`${pad} text-center text-sm font-semibold tabular-nums ${gd > 0 ? "text-primary-500" : gd < 0 ? "text-red-500" : "text-gray-500"}`}
                >
                  {gd > 0 ? `+${gd}` : gd}
                </td>
                <td
                  className={`${pad} text-center font-heading font-bold text-sm text-gray-800 dark:text-gray-100 tabular-nums`}
                >
                  {entry.points}
                </td>
              </tr>
            </ContextMenu>
          );
        })}
      </tbody>
    </table>
  );
}
