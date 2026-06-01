import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { GameStateData, SeasonAwardsData } from "../store/gameStore";
import { useGameStore } from "../store/gameStore";
import { Card, CardBody } from "./ui";
import AwardsCeremonyScreen from "./season/AwardsCeremonyScreen";
import { Trophy, Star, ArrowRight, Crown } from "lucide-react";

interface EndOfSeasonSummary {
  season: number;
  league_name: string;
  champion_id: string;
  champion_name: string;
  user_position: number;
  user_points: number;
  user_won: number;
  user_drawn: number;
  user_lost: number;
  user_goals_for: number;
  user_goals_against: number;
  golden_boot_player: string;
  golden_boot_goals: number;
  poty_player: string;
  poty_rating: number;
  total_teams: number;
  season_awards: SeasonAwardsData;
}

interface EndOfSeasonScreenProps {
  gameState: GameStateData;
  onGameUpdate: (g: GameStateData) => void;
}

export default function EndOfSeasonScreen({ gameState, onGameUpdate }: EndOfSeasonScreenProps) {
  const { t } = useTranslation();
  const setShowFiredModal = useGameStore((s) => s.setShowFiredModal);
  const [loading, setLoading] = useState(false);
  const [summary, setSummary] = useState<EndOfSeasonSummary | null>(null);
  const [step, setStep] = useState<"review" | "ceremony" | "done">("review");

  const league = gameState.league;
  const userTeamId = gameState.manager.team_id;
  const userTeam = gameState.teams.find(t => t.id === userTeamId);

  // Compute standings for display
  const standings = league
    ? [...league.standings].sort((a, b) =>
      b.points - a.points || (b.goals_for - b.goals_against) - (a.goals_for - a.goals_against) || b.goals_for - a.goals_for
    )
    : [];

  const userStandingIdx = standings.findIndex(s => s.team_id === userTeamId);
  const userStanding = standings[userStandingIdx];
  const userPosition = userStandingIdx + 1;
  const champion = standings[0];
  const championName = gameState.teams.find(t => t.id === champion?.team_id)?.name || "";
  const isChampion = champion?.team_id === userTeamId;

  const handleAdvance = async () => {
    if (loading) return;
    setLoading(true);
    try {
      const result = await invoke<{ action?: string; game: GameStateData; summary: EndOfSeasonSummary }>("advance_to_next_season");
      if (result.action === "fired") {
        onGameUpdate(result.game);
        setShowFiredModal(true);
        return;
      }
      setSummary(result.summary);
      onGameUpdate(result.game);
      setStep("ceremony");
    } catch (err) {
      console.error("Failed to advance season:", err);
    } finally {
      setLoading(false);
    }
  };

  const posLabel = (pos: number) => {
    if (pos === 1) return t("common.place.1");
    if (pos === 2) return t("common.place.2");
    if (pos === 3) return t("common.place.3");
    return t("common.place.other", { n: pos });
  };

  return (
    <div className="max-w-4xl mx-auto py-8 px-4">
      {step === "review" && (
        <>
          {/* Hero */}
          <div className="text-center mb-8">
            <div className={`w-20 h-20 mx-auto rounded-2xl flex items-center justify-center mb-4 ${isChampion
                ? "bg-gradient-to-br from-accent-400 to-accent-600 shadow-lg shadow-accent-500/30"
                : "bg-gradient-to-br from-navy-700 to-navy-800"
              }`}>
              {isChampion ? <Crown className="w-10 h-10 text-white" /> : <Trophy className="w-10 h-10 text-gray-300" />}
            </div>
            <h1 className="text-3xl font-heading font-bold text-gray-900 dark:text-gray-100 uppercase tracking-wide">
              {t('endOfSeason.seasonComplete')}
            </h1>
            <p className="text-lg text-gray-500 dark:text-gray-400 mt-1">
              {t("endOfSeason.seasonLine", {
                league: league?.name ?? "",
                season: league?.season ?? "",
              })}
            </p>
            {isChampion && (
              <p className="text-xl font-heading font-bold text-accent-500 mt-2 uppercase tracking-wider animate-pulse">
                {t('endOfSeason.champions')}
              </p>
            )}
          </div>

          {/* User team summary */}
          <Card accent={isChampion ? "accent" : "primary"} className="mb-6">
            <CardBody>
              <div className="text-center">
                <p className="text-xs font-heading font-bold uppercase tracking-widest text-gray-400 dark:text-gray-500 mb-1">
                  {userTeam?.name}
                </p>
                <div className="flex items-center justify-center gap-6 mb-4">
                  <div>
                    <p className="text-4xl font-heading font-bold text-gray-900 dark:text-gray-100">{posLabel(userPosition)}</p>
                    <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase">{t('endOfSeason.position')}</p>
                  </div>
                  <div className="w-px h-12 bg-gray-200 dark:bg-navy-600" />
                  <div>
                    <p className="text-4xl font-heading font-bold text-primary-500">{userStanding?.points || 0}</p>
                    <p className="text-xs text-gray-400 dark:text-gray-500 font-heading uppercase">{t('endOfSeason.points')}</p>
                  </div>
                </div>
                <div className="flex items-center justify-center gap-8 text-sm">
                  <span className="text-green-500 font-heading font-bold">{userStanding?.won || 0}{t('common.won')}</span>
                  <span className="text-gray-500 font-heading font-bold">{userStanding?.drawn || 0}{t('common.drawn')}</span>
                  <span className="text-red-500 font-heading font-bold">{userStanding?.lost || 0}{t('common.lost')}</span>
                  <span className="text-gray-400">
                    {userStanding?.goals_for || 0} {t('common.gf')} — {userStanding?.goals_against || 0} {t('common.ga')}
                  </span>
                </div>
              </div>
            </CardBody>
          </Card>

          {/* Final top 5 standings */}
          <Card className="mb-6">
            <CardBody>
              <h3 className="font-heading font-bold text-sm uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3 flex items-center gap-2">
                <Trophy className="w-4 h-4 text-accent-500" /> {t('endOfSeason.finalStandings')}
              </h3>
              <div className="divide-y divide-gray-100 dark:divide-navy-600">
                {standings.slice(0, 5).map((entry, idx) => {
                  const teamName = gameState.teams.find(t => t.id === entry.team_id)?.name || "";
                  const isUser = entry.team_id === userTeamId;
                  const gd = entry.goals_for - entry.goals_against;
                  return (
                    <div key={entry.team_id} className={`flex items-center py-2.5 gap-3 ${isUser ? "bg-primary-50/50 dark:bg-primary-500/5 -mx-2 px-2 rounded-lg" : ""}`}>
                      <span className={`font-heading font-bold text-sm w-6 text-center ${idx === 0 ? "text-accent-500" : "text-gray-400"}`}>{idx + 1}</span>
                      <span className={`flex-1 text-sm font-semibold ${isUser ? "text-primary-600 dark:text-primary-400" : "text-gray-800 dark:text-gray-200"}`}>{teamName}</span>
                      <span className="text-xs text-gray-500 dark:text-gray-400 tabular-nums w-16 text-center">{entry.won}W {entry.drawn}D {entry.lost}L</span>
                      <span className={`text-xs font-semibold tabular-nums w-8 text-center ${gd > 0 ? "text-primary-500" : gd < 0 ? "text-red-500" : "text-gray-500"}`}>{gd > 0 ? `+${gd}` : gd}</span>
                      <span className="font-heading font-bold text-sm text-gray-800 dark:text-gray-100 tabular-nums w-8 text-right">{entry.points}</span>
                    </div>
                  );
                })}
              </div>
            </CardBody>
          </Card>

          {/* Champion + awards */}
          {!isChampion && (
            <Card className="mb-6">
              <CardBody>
                <div className="flex items-center gap-3">
                  <Crown className="w-6 h-6 text-accent-500" />
                  <div>
                    <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-200 uppercase tracking-wider">{t('endOfSeason.leagueChampions')}</p>
                    <p className="text-lg font-heading font-bold text-accent-500">{championName}</p>
                  </div>
                </div>
              </CardBody>
            </Card>
          )}

          {/* Action */}
          <div className="text-center">
            <button
              onClick={handleAdvance}
              disabled={loading}
              className="px-8 py-4 bg-primary-500 text-white rounded-xl font-heading font-bold text-lg uppercase tracking-wider hover:bg-primary-600 transition-all shadow-lg shadow-primary-500/20 hover:shadow-xl hover:shadow-primary-500/30 disabled:opacity-50 flex items-center gap-3 mx-auto"
            >
              {loading ? t('endOfSeason.processing') : t('endOfSeason.startNextSeason')}
              <ArrowRight className="w-5 h-5" />
            </button>
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-3">
              {t('endOfSeason.statsArchived')}
            </p>
          </div>
        </>
      )}

      {step === "ceremony" && summary && (
        <AwardsCeremonyScreen
          season={summary.season}
          leagueName={summary.league_name}
          gameState={gameState}
          awards={summary.season_awards}
          onContinue={() => setStep("done")}
        />
      )}

      {step === "done" && summary && (
        <div className="text-center">
          <div className="w-20 h-20 mx-auto rounded-2xl bg-gradient-to-br from-primary-500 to-primary-600 flex items-center justify-center mb-4 shadow-lg shadow-primary-500/30">
            <Star className="w-10 h-10 text-white" />
          </div>
          <h1 className="text-3xl font-heading font-bold text-gray-900 dark:text-gray-100 uppercase tracking-wide mb-2">
            {t('endOfSeason.newSeason', { n: summary.season + 1 })}
          </h1>
          <p className="text-gray-500 dark:text-gray-400 mb-8">
            {t('endOfSeason.newScheduleReleased')}
          </p>

          <button
            onClick={() => {
              // Game state is already updated via onGameUpdate, just force re-render
              // by calling onGameUpdate again with the current state
              if (gameState) onGameUpdate(gameState);
            }}
            className="px-8 py-3 bg-primary-500 text-white rounded-xl font-heading font-bold uppercase tracking-wider hover:bg-primary-600 transition-all shadow-lg shadow-primary-500/20"
          >
            {t('endOfSeason.continueDashboard')}
          </button>
        </div>
      )}
    </div>
  );
}
