import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { PlayerData } from "../../store/gameStore";
import { Badge, Card, CardBody, CardHeader, PlayerAvatar } from "../ui";

interface HomeUnavailablePlayersCardProps {
  players: PlayerData[];
  resolveInjuryName: (injuryName: string) => string;
  onNavigate?: (tab: string) => void;
}

export default function HomeUnavailablePlayersCard({
  players,
  resolveInjuryName,
  onNavigate,
}: HomeUnavailablePlayersCardProps) {
  const { t } = useTranslation();

  if (players.length === 0) {
    return null;
  }

  return (
    <Card>
      <CardHeader
        action={
          <button
            onClick={() => onNavigate?.("Squad")}
            className="text-primary-500 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-primary-600 dark:hover:text-primary-300 transition-colors"
          >
            {t("dashboard.squad")}
          </button>
        }
      >
        <div className="flex items-center gap-2">
          <AlertTriangle className="w-4 h-4 text-red-500" />
          {t("home.unavailablePlayers")}
        </div>
      </CardHeader>
      <CardBody>
        <div className="flex flex-col gap-2.5">
          {players.map((player) => (
            <div
              key={player.id}
              className="flex flex-col gap-2 rounded-lg border border-gray-100 px-3 py-2.5 dark:border-navy-700 sm:flex-row sm:items-center sm:justify-between"
            >
              <div className="min-w-0 flex items-center gap-3">
                <PlayerAvatar player={player} />
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="truncate text-sm font-heading font-bold text-gray-800 dark:text-gray-200">
                      {player.full_name}
                    </span>
                    <Badge variant="danger" size="sm">
                      {t("common.injured")}
                    </Badge>
                  </div>
                  <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                    {player.injury ? `${resolveInjuryName(player.injury.name)} - ` : ""}
                    {t("home.daysUnavailable", {
                      count: player.injury?.days_remaining ?? 0,
                    })}
                  </p>
                </div>
              </div>
              <div className="text-xs font-heading font-bold uppercase tracking-wider text-gray-400 dark:text-gray-500">
                {t(`common.positions.${player.position}`, {
                  defaultValue: player.position,
                })}
              </div>
            </div>
          ))}
        </div>
      </CardBody>
    </Card>
  );
}