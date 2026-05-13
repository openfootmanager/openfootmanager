import { TrendingDown, TrendingUp } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { PlayerData } from "../../store/gameStore";
import { Badge, Card, CardBody, CardHeader } from "../ui";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";

interface HomePlayerMomentumCardProps {
  hotPlayers: PlayerData[];
  coldPlayers: PlayerData[];
  onNavigate?: (tab: string) => void;
}

export default function HomePlayerMomentumCard({
  hotPlayers,
  coldPlayers,
  onNavigate,
}: HomePlayerMomentumCardProps) {
  const { t } = useTranslation();

  if (hotPlayers.length === 0 && coldPlayers.length === 0) {
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
        {t("home.playerMomentum")}
      </CardHeader>
      <CardBody>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {hotPlayers.length > 0 && (
            <div>
              <div className="flex items-center gap-1.5 mb-2">
                <TrendingUp className="w-3.5 h-3.5 text-green-500" />
                <span className="text-[10px] font-heading font-bold uppercase tracking-widest text-green-500">
                  {t("home.inForm")}
                </span>
              </div>
              <div className="flex flex-col gap-1.5">
                {hotPlayers.map((player) => (
                  <div
                    key={player.id}
                    className="flex items-center gap-2 px-2 py-1.5 rounded-lg bg-green-500/5 dark:bg-green-500/10"
                  >
                    <span className="text-xs font-medium text-gray-800 dark:text-gray-200 flex-1 truncate">
                      {player.full_name}
                    </span>
                    <Badge variant="success" size="sm">
                      {translatePositionAbbreviation(t, player.position)}
                    </Badge>
                    <span className="text-xs font-heading font-bold text-green-500 tabular-nums w-8 text-right">
                      {player.morale}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
          {coldPlayers.length > 0 && (
            <div>
              <div className="flex items-center gap-1.5 mb-2">
                <TrendingDown className="w-3.5 h-3.5 text-red-500" />
                <span className="text-[10px] font-heading font-bold uppercase tracking-widest text-red-500">
                  {t("home.lowMorale")}
                </span>
              </div>
              <div className="flex flex-col gap-1.5">
                {coldPlayers.map((player) => (
                  <div
                    key={player.id}
                    className="flex items-center gap-2 px-2 py-1.5 rounded-lg bg-red-500/5 dark:bg-red-500/10"
                  >
                    <span className="text-xs font-medium text-gray-800 dark:text-gray-200 flex-1 truncate">
                      {player.full_name}
                    </span>
                    <Badge variant="danger" size="sm">
                      {translatePositionAbbreviation(t, player.position)}
                    </Badge>
                    <span className="text-xs font-heading font-bold text-red-500 tabular-nums w-8 text-right">
                      {player.morale}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </CardBody>
    </Card>
  );
}
