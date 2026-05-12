import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useGameStore, GameStateData, PlayerData } from "../store/gameStore";
import { formatVal } from "../lib/helpers";
import { Card, CardBody, Badge, TeamLocation, ThemeToggle } from "../components/ui";
import { ArrowLeft, Users, Trophy, Landmark, ChevronRight, Star, Loader2 } from "lucide-react";
import { resolveBackendError } from "../utils/backendI18n";

export default function TeamSelection() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { gameState, setGameState, setGameActive } = useGameStore();
  const [selectedTeamId, setSelectedTeamId] = useState<string | null>(null);
  const [isConfirming, setIsConfirming] = useState(false);

  if (!gameState) {
    navigate("/");
    return null;
  }

  const teams = gameState.teams;

  const getTeamPlayers = (teamId: string): PlayerData[] =>
    gameState.players.filter((p) => p.team_id === teamId);

  const getTeamAvgOvr = (teamId: string): number => {
    const players = getTeamPlayers(teamId);
    if (players.length === 0) return 0;
    const total = players.reduce((sum, p) => {
      const a = p.attributes;
      return sum + Math.round(
        (a.pace + a.stamina + a.strength + a.passing + a.shooting +
          a.tackling + a.dribbling + a.defending + a.positioning +
          a.vision + a.decisions) / 11
      );
    }, 0);
    return Math.round(total / players.length);
  };

  const getReputationLabel = (rep: number): { label: string; variant: "primary" | "accent" | "success" | "danger" | "neutral" } => {
    if (rep >= 750) return { label: t('teamSelect.repWorldClass'), variant: "accent" };
    if (rep >= 600) return { label: t('teamSelect.repStrong'), variant: "success" };
    if (rep >= 400) return { label: t('teamSelect.repAverage'), variant: "neutral" };
    return { label: t('teamSelect.repDeveloping'), variant: "danger" };
  };

  const handleConfirm = async () => {
    if (!selectedTeamId || isConfirming) return;
    setIsConfirming(true);
    try {
      const updatedGame = await invoke<GameStateData>("select_team", { teamId: selectedTeamId });
      setGameState(updatedGame);
      const mgr = updatedGame.manager;
      setGameActive(true, `${mgr.first_name} ${mgr.last_name}`);
      navigate("/dashboard");
    } catch (error) {
      console.error("Failed to select team:", error);
      alert(
        t("teamSelect.failedToSelectTeam", {
          error: resolveBackendError(error),
        }),
      );
    } finally {
      setIsConfirming(false);
    }
  };

  const selectedTeam = teams.find((t) => t.id === selectedTeamId);

  return (
    <div className="min-h-screen bg-gray-100 dark:bg-navy-900 transition-colors duration-300">
      {/* Header */}
      <header className="bg-white dark:bg-navy-800 border-b border-gray-200 dark:border-navy-700 px-6 py-4 flex justify-between items-center shadow-sm">
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate("/")}
            className="p-2 rounded-lg text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-navy-700 transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <div>
            <h1 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-800 dark:text-gray-100">
              {t('teamSelect.title')}
            </h1>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              {t('teamSelect.subtitle')}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <ThemeToggle />
          {selectedTeam && (
            <button
              onClick={handleConfirm}
              disabled={isConfirming}
              className={`bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white px-6 py-2.5 rounded-lg font-heading font-bold uppercase tracking-wider text-sm shadow-md hover:shadow-lg hover:shadow-primary-500/20 transition-all flex items-center gap-2 ${isConfirming ? "opacity-70 cursor-wait" : ""}`}
            >
              <span>{isConfirming ? t('teamSelect.confirming') : t('teamSelect.manage', { name: selectedTeam.short_name })}</span>
              {isConfirming ? <Loader2 className="w-4 h-4 animate-spin" /> : <ChevronRight className="w-4 h-4" />}
            </button>
          )}
        </div>
      </header>

      <div className="max-w-7xl mx-auto p-6">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {teams.map((team) => {
            const isSelected = selectedTeamId === team.id;
            const avgOvr = getTeamAvgOvr(team.id);
            const repInfo = getReputationLabel(team.reputation);
            const playerCount = getTeamPlayers(team.id).length;

            return (
              <button
                key={team.id}
                onClick={() => setSelectedTeamId(team.id)}
                className={`text-left transition-all duration-200 rounded-xl ${isSelected
                    ? "ring-2 ring-primary-500 ring-offset-2 dark:ring-offset-navy-900 scale-[1.02]"
                    : "hover:scale-[1.01]"
                  }`}
              >
                <Card
                  accent={isSelected ? "primary" : "none"}
                  className="h-full"
                >
                  {/* Team header with gradient */}
                  <div className={`p-4 rounded-t-xl ${isSelected
                      ? "bg-gradient-to-r from-primary-600 to-primary-700"
                      : "bg-gradient-to-r from-navy-700 to-navy-800"
                    }`}>
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div className={`w-12 h-12 rounded-lg flex items-center justify-center font-heading font-bold text-lg ${isSelected
                            ? "bg-white/20 text-white"
                            : "bg-white/10 text-gray-300"
                          }`}>
                          {team.short_name}
                        </div>
                        <div>
                          <h3 className="font-heading font-bold text-white uppercase tracking-wide text-sm">
                            {team.name}
                          </h3>
                          <TeamLocation
                            city={team.city}
                            countryCode={team.country}
                            locale={i18n.language}
                            className="mt-0.5 text-xs text-gray-300"
                            iconClassName="w-3 h-3"
                            flagClassName="text-xs leading-none"
                          />
                        </div>
                      </div>
                      {isSelected && (
                        <Star className="w-5 h-5 text-accent-400 fill-current" />
                      )}
                    </div>
                  </div>

                  <CardBody className="p-4">
                    <div className="grid grid-cols-2 gap-3">
                      <StatItem
                        icon={<Trophy className="w-3.5 h-3.5" />}
                        label={t('teamSelect.reputation')}
                        value={<Badge variant={repInfo.variant} size="sm">{repInfo.label}</Badge>}
                      />
                      <StatItem
                        icon={<Users className="w-3.5 h-3.5" />}
                        label={t('teamSelect.squad')}
                        value={<span className="font-heading font-bold text-gray-800 dark:text-gray-200">{playerCount}</span>}
                      />
                      <StatItem
                        icon={<Landmark className="w-3.5 h-3.5" />}
                        label={t('teamSelect.finances')}
                        value={<span className="font-heading font-bold text-gray-800 dark:text-gray-200">{formatVal(team.finance)}</span>}
                      />
                      <StatItem
                        icon={<Star className="w-3.5 h-3.5" />}
                        label={t('teamSelect.avgOvr')}
                        value={
                          <span className={`font-heading font-bold text-lg ${avgOvr >= 70 ? "text-primary-500" :
                              avgOvr >= 55 ? "text-accent-600 dark:text-accent-400" :
                                "text-gray-500"
                            }`}>{avgOvr}</span>
                        }
                      />
                    </div>

                    {/* Stadium */}
                    <div className="mt-3 pt-3 border-t border-gray-100 dark:border-navy-600">
                      <p className="text-xs text-gray-400 dark:text-gray-500">
                        {t('teamSelect.seats', { name: team.stadium_name, capacity: team.stadium_capacity.toLocaleString() })}
                      </p>
                    </div>
                  </CardBody>
                </Card>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function StatItem({ icon, label, value }: { icon: React.ReactNode; label: string; value: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs text-gray-400 dark:text-gray-500 flex items-center gap-1">
        {icon} {label}
      </span>
      {value}
    </div>
  );
}
