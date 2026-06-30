import { useTranslation } from "react-i18next";

import type {
  PlayerData,
  ScoutingAssignment,
  StaffData,
} from "../../store/gameStore";
import { countryName } from "../../lib/countries";
import { Badge, Card, CardBody, CardHeader, CountryFlag, ProgressBar } from "../ui";
import { Eye } from "lucide-react";
import { scoutAssignmentCount, scoutMaxSlots } from "./ScoutingTab.helpers";

interface ScoutingScoutDetailsCardProps {
  scouts: StaffData[];
  assignments: ScoutingAssignment[];
  players: PlayerData[];
}

export default function ScoutingScoutDetailsCard({
  scouts,
  assignments,
  players,
}: ScoutingScoutDetailsCardProps) {
  const { t, i18n } = useTranslation();

  if (scouts.length === 0) {
    return null;
  }

  return (
    <Card>
      <CardHeader>{t("scouting.yourScouts")}</CardHeader>
      <CardBody>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {scouts.map((scout) => {
            const count = scoutAssignmentCount(assignments, scout.id);
            const maxSlots = scoutMaxSlots(scout.attributes.judgingAbility);
            const isFull = count >= maxSlots;
            const scoutAssignments = assignments.filter(
              (assignment) => assignment.scout_id === scout.id,
            );

            return (
              <div
                key={scout.id}
                className="p-3 rounded-lg border border-gray-200 dark:border-navy-600"
              >
                <div className="flex items-center gap-3">
                  <div className="w-9 h-9 rounded-lg bg-accent-500/10 flex items-center justify-center">
                    <Eye className="w-4 h-4 text-accent-500" />
                  </div>
                  <div className="flex-1">
                    <p className="font-heading font-bold text-sm text-gray-800 dark:text-gray-100">
                      {scout.first_name} {scout.last_name}
                    </p>
                    <div className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5 flex items-center gap-1">
                      <CountryFlag
                        code={scout.nationality}
                        locale={i18n.language}
                        className="text-xs leading-none"
                      />
                      <span>{countryName(scout.nationality, i18n.language)}</span>
                    </div>
                  </div>
                  <Badge variant={isFull ? "accent" : "success"} size="sm">
                    {count}/{maxSlots} {t("scouting.slots")}
                  </Badge>
                </div>
                <div className="mt-2 grid grid-cols-2 gap-2">
                  <div>
                    <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase">
                      {t("scouting.judgingAbility")}
                    </p>
                    <ProgressBar
                      value={scout.attributes.judgingAbility}
                      variant="auto"
                      size="sm"
                    />
                  </div>
                  <div>
                    <p className="text-[10px] text-gray-400 dark:text-gray-500 font-heading uppercase">
                      {t("scouting.judgingPotential")}
                    </p>
                    <ProgressBar
                      value={scout.attributes.judgingPotential}
                      variant="auto"
                      size="sm"
                    />
                  </div>
                </div>
                {scoutAssignments.length > 0 && (
                  <div className="mt-2 flex flex-col gap-1">
                    {scoutAssignments.map((assignment) => {
                      const player = players.find(
                        (currentPlayer) => currentPlayer.id === assignment.player_id,
                      );

                      return player ? (
                        <p
                          key={assignment.id}
                          className="text-xs text-gray-500 dark:text-gray-400"
                        >
                          {t("scouting.scoutLabel", { name: "" })}
                          <span className="font-heading font-bold text-gray-700 dark:text-gray-300">
                            {player.full_name}
                          </span>{" "}
                          - {assignment.days_remaining}d
                        </p>
                      ) : null;
                    })}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </CardBody>
    </Card>
  );
}