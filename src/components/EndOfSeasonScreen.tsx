import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { GameStateData, SeasonAwardsData } from "../store/gameStore";
import { useGameStore } from "../store/gameStore";
import { invokeForCareer } from "../services/careerInvoke";
import { useReviewDivision } from "./season/useReviewDivision";
import { Button } from "./ui";
import { competitionDisplayName } from "../lib/competitionName";
import AwardsCeremonyScreen from "./season/AwardsCeremonyScreen";
import SeasonReviewTables from "./season/SeasonReviewTables";
import SeasonReviewClub from "./season/SeasonReviewClub";
import { Trophy, Star, ArrowRight, Crown } from "lucide-react";

export interface EndOfSeasonSummary {
  season: number;
  league_name: string;
  competition_id?: string;
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

/**
 * Where the player is once the next season has been started: the awards of the one just played,
 * then the new season's announcement. Held by the dashboard, not here: the same game update that
 * carries the new season says the old one is no longer complete, which unmounts this screen, and
 * what it kept in its own state went with it.
 */
export interface SeasonTransition {
  summary: EndOfSeasonSummary;
  step: "ceremony" | "done";
}

interface EndOfSeasonScreenProps {
  gameState: GameStateData;
  onGameUpdate: (g: GameStateData) => void;
  transition: SeasonTransition | null;
  onTransitionChange: (transition: SeasonTransition | null) => void;
}

export default function EndOfSeasonScreen({
  gameState,
  onGameUpdate,
  transition,
  onTransitionChange,
}: EndOfSeasonScreenProps) {
  const { t } = useTranslation();
  const setShowFiredModal = useGameStore((s) => s.setShowFiredModal);
  const [loading, setLoading] = useState(false);
  const summary = transition?.summary ?? null;
  const step = transition?.step ?? "review";

  const review = useReviewDivision(gameState, step === "review");
  const league = review.league;
  const userTeamId = gameState.manager.team_id;

  // Compute standings for display
  const standings = league
    ? [...league.standings].sort(
        (a, b) =>
          b.points - a.points ||
          b.goals_for - b.goals_against - (a.goals_for - a.goals_against) ||
          b.goals_for - a.goals_for,
      )
    : [];

  const champion = standings[0];
  const isChampion = champion?.team_id === userTeamId;

  const handleAdvance = async () => {
    if (loading) return;
    setLoading(true);
    try {
      const result = await invokeForCareer<{
        action?: string;
        game: GameStateData;
        summary: EndOfSeasonSummary;
      }>("advance_to_next_season");
      if (result.action === "fired") {
        onGameUpdate(result.game);
        setShowFiredModal(true);
        return;
      }
      // The transition first: the game update below is what ends the season this screen reviews.
      onTransitionChange({
        summary: result.summary,
        step: result.summary.league_name ? "ceremony" : "done",
      });
      onGameUpdate(result.game);
    } catch (err) {
      console.error("Failed to advance season:", err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="py-8 px-4">
      {step === "review" && (
        <>
          {review.loading && <p role="status">{t("common.loading")}</p>}
          {review.failed && (
            <div>
              <p role="alert">{t("errors.title")}</p>
              <Button size="sm" variant="outline" onClick={review.retry}>
                {t("common.retry")}
              </Button>
            </div>
          )}
          {/* Hero */}
          <div className="text-center mb-8">
            <div
              className={`w-20 h-20 mx-auto rounded-2xl flex items-center justify-center mb-4 ${
                isChampion
                  ? "bg-gradient-to-br from-accent-400 to-accent-600 shadow-lg shadow-accent-500/30"
                  : "bg-gradient-to-br from-navy-700 to-navy-800"
              }`}
            >
              {isChampion ? (
                <Crown className="w-10 h-10 text-white" />
              ) : (
                <Trophy className="w-10 h-10 text-gray-300" />
              )}
            </div>
            <h1 className="text-3xl font-heading font-bold text-gray-900 dark:text-gray-100 uppercase tracking-wide">
              {t("endOfSeason.seasonComplete")}
            </h1>
            {league && (
              <p className="text-lg text-gray-500 dark:text-gray-400 mt-1">
                {t("endOfSeason.seasonLine", {
                  league: league ? competitionDisplayName(league, t) : "",
                  season: league?.season ?? "",
                })}
              </p>
            )}
            {isChampion && (
              <p className="text-xl font-heading font-bold text-accent-500 mt-2 uppercase tracking-wider animate-pulse">
                {t("endOfSeason.champions")}
              </p>
            )}
          </div>

          <SeasonReviewClub gameState={gameState} standings={standings} isChampion={isChampion} />

          <SeasonReviewTables
            gameState={gameState}
            league={league}
            standings={standings}
            isChampion={isChampion}
          />

          {/* Action */}
          <div className="text-center">
            <button
              type="button"
              onClick={handleAdvance}
              disabled={loading}
              className="px-8 py-4 bg-primary-500 text-white rounded-xl font-heading font-bold text-lg uppercase tracking-wider hover:bg-primary-600 transition-all shadow-lg shadow-primary-500/20 hover:shadow-xl hover:shadow-primary-500/30 disabled:opacity-50 flex items-center gap-3 mx-auto"
            >
              {loading ? t("endOfSeason.processing") : t("endOfSeason.startNextSeason")}
              <ArrowRight className="w-5 h-5" />
            </button>
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-3">
              {t("endOfSeason.statsArchived")}
            </p>
          </div>
        </>
      )}

      {step === "ceremony" && summary && (
        <AwardsCeremonyScreen
          season={summary.season}
          leagueName={summary.league_name}
          competitionId={summary.competition_id}
          gameState={gameState}
          awards={summary.season_awards}
          onContinue={() => onTransitionChange({ summary, step: "done" })}
        />
      )}

      {step === "done" && summary && (
        <div className="text-center">
          <div className="w-20 h-20 mx-auto rounded-2xl bg-gradient-to-br from-primary-500 to-primary-600 flex items-center justify-center mb-4 shadow-lg shadow-primary-500/30">
            <Star className="w-10 h-10 text-white" />
          </div>
          <h1 className="text-3xl font-heading font-bold text-gray-900 dark:text-gray-100 uppercase tracking-wide mb-2">
            {t("endOfSeason.newSeason", { n: summary.season + 1 })}
          </h1>
          {summary.league_name && (
            <p className="text-gray-500 dark:text-gray-400 mb-8">
              {t("endOfSeason.newScheduleReleased")}
            </p>
          )}

          <button
            type="button"
            onClick={() => onTransitionChange(null)}
            className="px-8 py-3 bg-primary-500 text-white rounded-xl font-heading font-bold uppercase tracking-wider hover:bg-primary-600 transition-all shadow-lg shadow-primary-500/20"
          >
            {t("endOfSeason.continueDashboard")}
          </button>
        </div>
      )}
    </div>
  );
}
