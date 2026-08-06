import { useTranslation } from "react-i18next";

import { byTablePosition } from "./TournamentsTab.helpers";
import type { TournamentsTeamLookup } from "./teamLookup";
import type { LeagueData } from "../../store/gameStore";

type Group = NonNullable<LeagueData["groups"]>[number];

interface TournamentsGroupTableProps {
  group: Group;
  teams: TournamentsTeamLookup;
}

/**
 * A group's mini table: place, team, played, points.
 *
 * Deliberately not the shared StandingsTable. A group has no header row, no
 * context menu, and rows that only open when the team is a club — a World Cup
 * group is national teams, which have no page to go to.
 */
export default function TournamentsGroupTable({
  group,
  teams,
}: TournamentsGroupTableProps) {
  const { t } = useTranslation();
  const { userTeamId, isClubTeam, resolveTeamName, onSelectTeam } = teams;
  const groupStandings = [...group.standings].sort(byTablePosition);

  return (
    <div data-testid={`tournaments-group-${group.id}`}>
      <div className="px-4 py-2 border-b border-gray-100 dark:border-navy-600 bg-gray-50 dark:bg-navy-800">
        <h5 className="font-heading font-bold text-xs uppercase tracking-wider text-gray-600 dark:text-gray-300">
          {t("tournaments.group", { name: group.name })}
        </h5>
      </div>
      <table className="w-full text-left border-collapse">
        <tbody className="divide-y divide-gray-100 dark:divide-navy-600">
          {groupStandings.map((entry, idx) => {
            const isUser = entry.team_id === userTeamId;
            const clickable = isClubTeam(entry.team_id);
            return (
              <tr
                key={entry.team_id}
                onClick={clickable ? () => onSelectTeam(entry.team_id) : undefined}
                className={`${clickable ? "cursor-pointer" : ""} transition-colors ${isUser ? "bg-primary-50 dark:bg-primary-500/10" : "hover:bg-gray-50 dark:hover:bg-navy-700/50"}`}
                data-testid={`tournaments-group-standing-${entry.team_id}`}
              >
                <td className="py-1.5 px-3 font-heading font-bold text-xs text-gray-400 w-6">
                  {idx + 1}
                </td>
                <td
                  className={`py-1.5 px-3 font-semibold text-sm ${isUser ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
                >
                  {resolveTeamName(entry.team_id)}
                </td>
                <td className="py-1.5 px-3 text-center text-xs text-gray-600 dark:text-gray-400 tabular-nums">
                  {entry.played}
                </td>
                <td className="py-1.5 px-3 text-center font-heading font-bold text-sm text-gray-800 dark:text-gray-100 tabular-nums">
                  {entry.points}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
