import { useTranslation } from "react-i18next";

import ContextMenu from "../ContextMenu";
import { buildViewTeamMenuItem } from "../playerActions/playerContextMenuItems";
import { Badge } from "../ui";
import type { TournamentsTeamLookup } from "./teamLookup";
import type { FixtureData } from "../../store/gameStore";

interface TournamentsFixtureRowProps {
  fixture: FixtureData;
  testId: string;
  teams: TournamentsTeamLookup;
}

/** One fixture: home, the score or a "vs", away. */
export default function TournamentsFixtureRow({
  fixture,
  testId,
  teams,
}: TournamentsFixtureRowProps) {
  const { t } = useTranslation();
  const { userTeamId, isClubTeam, resolveTeamName, onSelectTeam } = teams;

  const isUserMatch =
    fixture.home_team_id === userTeamId || fixture.away_team_id === userTeamId;
  const completed = fixture.status === "Completed";

  const menuItems = [fixture.home_team_id, fixture.away_team_id]
    .filter((teamId) => isClubTeam(teamId))
    .map((teamId) => ({
      ...buildViewTeamMenuItem(t, () => onSelectTeam(teamId)),
      label: `${t("common.viewTeam")}: ${resolveTeamName(teamId)}`,
    }));

  const teamName = (teamId: string, align: "text-right" | "text-left") => (
    <span
      onClick={isClubTeam(teamId) ? () => onSelectTeam(teamId) : undefined}
      className={`flex-1 ${align} font-semibold text-sm ${isClubTeam(teamId) ? "cursor-pointer hover:underline" : ""} ${teamId === userTeamId ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}
    >
      {resolveTeamName(teamId)}
    </span>
  );

  return (
    <ContextMenu items={menuItems}>
      <div
        className={`flex items-center px-5 py-3 transition-colors ${isUserMatch ? "bg-primary-50/50 dark:bg-primary-500/5" : ""}`}
        data-testid={testId}
      >
        {teamName(fixture.home_team_id, "text-right")}
        <div className="w-24 text-center mx-3">
          {completed && fixture.result ? (
            <span className="font-heading font-bold text-lg text-gray-800 dark:text-gray-100">
              {fixture.result.home_goals} - {fixture.result.away_goals}
            </span>
          ) : (
            <Badge variant="neutral" size="sm">
              vs
            </Badge>
          )}
        </div>
        {teamName(fixture.away_team_id, "text-left")}
      </div>
    </ContextMenu>
  );
}
