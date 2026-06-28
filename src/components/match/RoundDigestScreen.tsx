import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { FixtureData, GameStateData } from "../../store/gameStore";
import type { CompactMatchEventData } from "../../store/types";
import { MatchSnapshot, MatchEvent, RoundSummary } from "./types";
import { getEventDisplay, makeTeamFallback } from "./helpers";
import { QuickStat } from "./PostMatchHelpers";
import { Badge, TeamLogo } from "../ui";
import {
  Trophy,
  TrendingDown,
  Minus,
  ChevronRight,
  ArrowUp,
  ArrowDown,
  Flame,
} from "lucide-react";

interface RoundDigestScreenProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  currentFixture: FixtureData | null;
  userSide: "Home" | "Away" | null;
  isLeagueFixture: boolean;
  roundSummary: RoundSummary | null;
  onPressConference: () => void;
  onFinish: () => void;
}
export default function RoundDigestScreen({
  snapshot,
  gameState,
  currentFixture,
  userSide,
  isLeagueFixture,
  roundSummary,
  onPressConference,
  onFinish,
}: RoundDigestScreenProps) {
  const { t } = useTranslation();
  const [selectedOtherFixtureId, setSelectedOtherFixtureId] = useState<
    string | null
  >(null);
  const modalCloseRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (!selectedOtherFixtureId) return;
    modalCloseRef.current?.focus();
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setSelectedOtherFixtureId(null);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [selectedOtherFixtureId]);

  const userTeamId = gameState.manager.team_id;

  const getTeamNameById = (teamId: string) =>
    gameState.teams.find((team) => team.id === teamId)?.name || teamId;

  const getTeamShortName = (teamId: string, fallbackName: string) =>
    gameState.teams.find((team) => team.id === teamId)?.short_name ||
    fallbackName.substring(0, 3).toUpperCase();

  const getPlayerDisplayName = (playerId: string | null | undefined) => {
    if (!playerId) return t("common.unknown");
    return (
      gameState.players.find((player) => player.id === playerId)?.match_name ||
      playerId
    );
  };

  const getFixtureReport = (fixture: FixtureData | null | undefined) =>
    fixture?.result?.report || null;

  const formatOtherMatchScorers = (fixture: FixtureData) => {
    if (!fixture.result) return null;
    const scorers = [
      ...fixture.result.home_scorers,
      ...fixture.result.away_scorers,
    ]
      .sort((a, b) => a.minute - b.minute)
      .map((s) => `${getPlayerDisplayName(s.player_id)} ${s.minute}'`);
    if (scorers.length === 0) return null;
    return scorers.slice(0, 3).join(" • ");
  };

  const formatOtherMatchStats = (fixture: FixtureData) => {
    const report = getFixtureReport(fixture);
    if (!report) return null;
    const totalYellow =
      report.home_stats.yellow_cards + report.away_stats.yellow_cards;
    return [
      `${report.home_stats.possession_pct}-${report.away_stats.possession_pct} ${t("match.possession")}`,
      `${report.home_stats.shots + report.away_stats.shots} ${t("match.shots")}`,
      `${totalYellow} ${t("match.yellowCards")}`,
    ].join(" • ");
  };

  const formatOtherMatchEvent = (event: CompactMatchEventData) => {
    const primary = getPlayerDisplayName(event.player_id);
    switch (event.event_type) {
      case "Goal":
        return event.secondary_player_id
          ? `${primary} (${t("match.assist", { name: getPlayerDisplayName(event.secondary_player_id) })})`
          : primary;
      case "PenaltyGoal":
        return `${primary} (P)`;
      case "PenaltyMiss":
        return `${primary} (PM)`;
      case "Substitution":
        return `${primary} ${t("match.subFor", { name: getPlayerDisplayName(event.secondary_player_id) })}`;
      default:
        return primary;
    }
  };

  const otherMatchEntries = isLeagueFixture
    ? (roundSummary?.completed_results || [])
        .filter((r) => r.fixture_id !== currentFixture?.id)
        .map((r) => {
          const fixture = gameState.league?.fixtures.find(
            (f) => f.id === r.fixture_id,
          );
          if (!fixture?.result) return null;
          return {
            fixture,
            homeTeamName: r.home_team_name,
            awayTeamName: r.away_team_name,
          };
        })
        .filter(
          (
            e,
          ): e is {
            fixture: FixtureData;
            homeTeamName: string;
            awayTeamName: string;
          } => e !== null,
        )
    : (gameState.league?.fixtures || [])
        .filter(
          (f) =>
            f.id !== currentFixture?.id &&
            f.status === "Completed" &&
            f.result &&
            f.date === currentFixture?.date &&
            f.competition === currentFixture?.competition,
        )
        .map((f) => ({
          fixture: f,
          homeTeamName: getTeamNameById(f.home_team_id),
          awayTeamName: getTeamNameById(f.away_team_id),
        }));

  const selectedOtherFixture = selectedOtherFixtureId
    ? otherMatchEntries.find((e) => e.fixture.id === selectedOtherFixtureId)
        ?.fixture || null
    : null;
  const selectedOtherFixtureReport = getFixtureReport(selectedOtherFixture);

  // User result
  const homeFullTeam = gameState.teams.find(
    (t) => t.id === snapshot.home_team.id,
  );
  const awayFullTeam = gameState.teams.find(
    (t) => t.id === snapshot.away_team.id,
  );
  const homeTeamColor = homeFullTeam?.colors?.primary || "#10b981";
  const awayTeamColor = awayFullTeam?.colors?.primary || "#6366f1";

  const resultType =
    userSide === "Home"
      ? snapshot.home_score > snapshot.away_score
        ? "win"
        : snapshot.home_score < snapshot.away_score
          ? "loss"
          : "draw"
      : userSide === "Away"
        ? snapshot.away_score > snapshot.home_score
          ? "win"
          : snapshot.away_score < snapshot.home_score
            ? "loss"
            : "draw"
        : "neutral";

  // Position context from standings delta
  const userStanding = roundSummary?.standings_delta.find(
    (s) => s.team_id === userTeamId,
  );
  const positionChange = userStanding
    ? userStanding.previous_position - userStanding.current_position
    : 0;

  const leagueName = gameState.league?.name;
  const matchdayLabel = roundSummary
    ? t("schedule.matchday", { number: roundSummary.matchday })
    : null;
  const headingParts = [matchdayLabel, leagueName].filter(Boolean).join(" — ");

  return (
    <div className="min-h-screen bg-gray-100 text-gray-900 dark:bg-navy-900 dark:text-white flex flex-col transition-colors duration-300">
      {/* Header */}
      <header className="bg-white dark:bg-navy-800 border-b border-gray-200 dark:border-navy-700 px-6 py-4 transition-colors duration-300">
        <div className="flex items-center justify-between">
          <div>
            {headingParts && (
              <p className="text-xs font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-0.5">
                {headingParts}
              </p>
            )}
            <h1 className="text-lg font-heading font-bold text-gray-900 dark:text-white">
              {isLeagueFixture
                ? t("match.roundSummary")
                : t("match.otherMatches")}
            </h1>
          </div>
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={onFinish}
              className="px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-navy-700 dark:hover:bg-navy-600 rounded-lg font-heading font-bold uppercase tracking-wider text-sm text-gray-700 dark:text-gray-300 transition-colors"
            >
              {t("match.skip")}
            </button>
            <button
              type="button"
              onClick={onPressConference}
              className="flex items-center gap-2 px-5 py-2 bg-linear-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 rounded-lg font-heading font-bold uppercase tracking-wider text-sm text-white shadow-md shadow-primary-500/20 transition-all"
            >
              {t("match.pressConference")}
              <ChevronRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      </header>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        <div className="px-6 py-6 flex flex-col gap-6">
          {/* Your Result Hero Card */}
          <div
            className={`rounded-2xl border p-6 transition-colors duration-300 ${
              resultType === "win"
                ? "bg-linear-to-r from-primary-50 to-white dark:from-primary-900/30 dark:to-navy-800 border-primary-200 dark:border-primary-700/50"
                : resultType === "loss"
                  ? "bg-linear-to-r from-red-50 to-white dark:from-red-900/20 dark:to-navy-800 border-red-200 dark:border-red-700/50"
                  : "bg-white dark:bg-navy-800 border-gray-200 dark:border-navy-700"
            }`}
          >
            <div className="flex items-center justify-between mb-4">
              <p className="text-xs font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
                {t("match.yourResult")}
              </p>
              <div className="flex items-center gap-1.5">
                {resultType === "win" && (
                  <div className="inline-flex items-center gap-1.5 px-3 py-1 bg-primary-100 dark:bg-primary-500/20 rounded-full">
                    <Trophy className="w-3.5 h-3.5 text-accent-700 dark:text-accent-400" />
                    <span className="font-heading font-bold text-xs uppercase tracking-widest text-primary-700 dark:text-primary-400">
                      {t("match.victory")}
                    </span>
                  </div>
                )}
                {resultType === "loss" && (
                  <div className="inline-flex items-center gap-1.5 px-3 py-1 bg-red-500/10 rounded-full">
                    <TrendingDown className="w-3.5 h-3.5 text-red-400" />
                    <span className="font-heading font-bold text-xs uppercase tracking-widest text-red-400">
                      {t("match.defeat")}
                    </span>
                  </div>
                )}
                {resultType === "draw" && (
                  <div className="inline-flex items-center gap-1.5 px-3 py-1 bg-gray-200 dark:bg-gray-700/50 rounded-full">
                    <Minus className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400" />
                    <span className="font-heading font-bold text-xs uppercase tracking-widest text-gray-500 dark:text-gray-400">
                      {t("match.draw")}
                    </span>
                  </div>
                )}
              </div>
            </div>

            <div className="flex items-center justify-center gap-8">
              <div className="flex items-center gap-3">
                <TeamLogo
                  team={
                    homeFullTeam ?? makeTeamFallback(snapshot.home_team.name)
                  }
                  className="w-12 h-12 rounded-xl flex items-center justify-center font-heading font-bold overflow-hidden"
                  imageClassName="h-9 w-9 object-contain drop-shadow"
                  style={{
                    backgroundColor: homeTeamColor + "30",
                    borderColor: homeTeamColor,
                    borderWidth: 2,
                  }}
                />
                <p className="font-heading font-bold text-gray-800 dark:text-gray-200">
                  {snapshot.home_team.name}
                </p>
              </div>
              <div className="flex items-center gap-4">
                <span className="text-5xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
                  {snapshot.home_score}
                </span>
                <span className="text-2xl font-heading text-gray-400">–</span>
                <span className="text-5xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
                  {snapshot.away_score}
                </span>
              </div>
              <div className="flex items-center gap-3">
                <p className="font-heading font-bold text-gray-800 dark:text-gray-200">
                  {snapshot.away_team.name}
                </p>
                <TeamLogo
                  team={
                    awayFullTeam ?? makeTeamFallback(snapshot.away_team.name)
                  }
                  className="w-12 h-12 rounded-xl flex items-center justify-center font-heading font-bold overflow-hidden"
                  imageClassName="h-9 w-9 object-contain drop-shadow"
                  style={{
                    backgroundColor: awayTeamColor + "30",
                    borderColor: awayTeamColor,
                    borderWidth: 2,
                  }}
                />
              </div>
            </div>

            {/* Position context */}
            {isLeagueFixture && userStanding && (
              <div className="mt-4 flex items-center justify-center gap-2">
                {positionChange > 0 ? (
                  <ArrowUp className="w-4 h-4 text-green-500" />
                ) : positionChange < 0 ? (
                  <ArrowDown className="w-4 h-4 text-red-400" />
                ) : null}
                <p className="text-sm font-heading font-bold text-gray-600 dark:text-gray-400">
                  #{userStanding.current_position} ·{" "}
                  <span className="text-accent-400">
                    {userStanding.points} {t("match.pts")}
                  </span>
                </p>
              </div>
            )}
          </div>

          {/* Main content grid */}
          <div className="grid grid-cols-3 gap-6">
            {/* Other Results — 2 cols for league (leaves room for table), full width for friendly */}
            <div className={`flex flex-col gap-3 ${isLeagueFixture ? "col-span-2" : "col-span-3"}`}>
              <h2 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                {isLeagueFixture
                  ? t("match.otherMatchesToday")
                  : t("match.otherMatches")}
              </h2>
              {otherMatchEntries.length > 0 ? (
                <div className="grid grid-cols-2 gap-3">
                  {otherMatchEntries.map((entry) => {
                    const scorerSummary = formatOtherMatchScorers(
                      entry.fixture,
                    );
                    const statSummary = formatOtherMatchStats(entry.fixture);
                    return (
                      <div
                        key={entry.fixture.id}
                        className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm px-4 py-3 transition-colors duration-300"
                      >
                        <div className="flex items-center justify-between gap-2 mb-1">
                          <span className="font-heading font-bold text-sm text-gray-800 dark:text-gray-200 truncate">
                            {entry.homeTeamName}{" "}
                            {entry.fixture.result?.home_goals} –{" "}
                            {entry.fixture.result?.away_goals}{" "}
                            {entry.awayTeamName}
                          </span>
                          {entry.fixture.result?.report && (
                            <button
                              type="button"
                              onClick={() =>
                                setSelectedOtherFixtureId(entry.fixture.id)
                              }
                              className="shrink-0 text-[10px] font-heading font-bold uppercase tracking-widest text-accent-400 hover:text-accent-300 transition-colors"
                            >
                              {t("match.viewDetails")}
                            </button>
                          )}
                        </div>
                        {scorerSummary && (
                          <p className="text-[11px] text-gray-500 dark:text-gray-400">
                            {scorerSummary}
                          </p>
                        )}
                        {statSummary && (
                          <p className="mt-0.5 text-[10px] uppercase tracking-wider text-gray-400 dark:text-gray-500">
                            {statSummary}
                          </p>
                        )}
                      </div>
                    );
                  })}
                </div>
              ) : (
                <p className="text-sm text-gray-500 dark:text-gray-400">
                  {isLeagueFixture
                    ? t("match.roundSummaryUnavailable")
                    : t("match.otherMatchesUnavailable")}
                </p>
              )}
            </div>

            {/* League Table + Top Scorers (1 col) — league only */}
            {isLeagueFixture && roundSummary && (
              <div className="flex flex-col gap-4">
                {/* Standings */}
                <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
                  <h3 className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
                    {t("tournaments.leagueTable")}
                  </h3>
                  <div className="flex flex-col gap-1.5">
                    {roundSummary.standings_delta.slice(0, 6).map((entry) => {
                      const change =
                        entry.previous_position - entry.current_position;
                      const isUserTeam = entry.team_id === userTeamId;
                      return (
                        <div
                          key={entry.team_id}
                          className={`flex items-center gap-2 text-xs rounded-md px-2 py-1 ${
                            isUserTeam
                              ? "bg-primary-50 dark:bg-primary-500/10 font-bold"
                              : ""
                          }`}
                        >
                          <span className="w-4 text-right tabular-nums text-gray-500 dark:text-gray-400 font-heading">
                            {entry.current_position}
                          </span>
                          {change > 0 ? (
                            <ArrowUp className="w-3 h-3 text-green-500 shrink-0" />
                          ) : change < 0 ? (
                            <ArrowDown className="w-3 h-3 text-red-400 shrink-0" />
                          ) : (
                            <span className="w-3 shrink-0" />
                          )}
                          <span
                            className={`truncate flex-1 ${isUserTeam ? "text-primary-600 dark:text-primary-400" : "text-gray-700 dark:text-gray-300"}`}
                          >
                            {entry.team_name}
                          </span>
                          <span className="font-heading font-bold tabular-nums text-gray-500 dark:text-gray-400">
                            {entry.points}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                </div>

                {/* Top Scorers */}
                {roundSummary.top_scorer_delta.length > 0 && (
                  <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
                    <h3 className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
                      {t("tournaments.topScorers")}
                    </h3>
                    <div className="flex flex-col gap-1.5">
                      {roundSummary.top_scorer_delta.slice(0, 5).map((entry) => {
                        const isUserTeamScorer = entry.team_id === userTeamId;
                        return (
                          <div
                            key={entry.player_id}
                            className="flex items-center gap-2 text-xs"
                          >
                            <span className="w-4 text-right tabular-nums text-gray-500 dark:text-gray-400 font-heading">
                              {entry.current_rank}.
                            </span>
                            <span
                              className={`truncate flex-1 ${isUserTeamScorer ? "font-bold text-primary-600 dark:text-primary-400" : "text-gray-700 dark:text-gray-300"}`}
                            >
                              {entry.player_name}
                            </span>
                            <span className="font-heading font-bold tabular-nums text-accent-400">
                              {entry.current_goals}
                            </span>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Notable Upset */}
          {isLeagueFixture && roundSummary?.notable_upset && (
            <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 flex items-center gap-4 transition-colors duration-300">
              <div className="w-8 h-8 rounded-lg bg-orange-100 dark:bg-orange-500/20 flex items-center justify-center shrink-0">
                <Flame className="w-4 h-4 text-orange-500 dark:text-orange-400" />
              </div>
              <div>
                <p className="text-[10px] font-heading font-bold uppercase tracking-widest text-orange-500 dark:text-orange-400 mb-0.5">
                  {t("match.notableUpset")}
                </p>
                <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-200">
                  {roundSummary.notable_upset.underdog_team_name}{" "}
                  {roundSummary.notable_upset.home_goals} –{" "}
                  {roundSummary.notable_upset.away_goals}{" "}
                  {roundSummary.notable_upset.favorite_team_name}
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Other Match Detail Modal */}
      {selectedOtherFixture && selectedOtherFixtureReport && (
        <div
          role="dialog"
          aria-modal="true"
          aria-label={t("match.matchDetails")}
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 px-4"
          onClick={() => setSelectedOtherFixtureId(null)}
        >
          <div
            className="w-full max-w-3xl rounded-2xl border border-gray-200 dark:border-navy-700 bg-white dark:bg-navy-900 shadow-2xl transition-colors duration-300"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between border-b border-gray-200 dark:border-navy-700 px-5 py-4">
              <div>
                <p className="text-xs font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
                  {t("match.matchDetails")}
                </p>
                <p className="text-lg font-heading font-bold text-gray-900 dark:text-white">
                  {getTeamNameById(selectedOtherFixture.home_team_id)}{" "}
                  {selectedOtherFixture.result?.home_goals} –{" "}
                  {selectedOtherFixture.result?.away_goals}{" "}
                  {getTeamNameById(selectedOtherFixture.away_team_id)}
                </p>
              </div>
              <button
                ref={modalCloseRef}
                type="button"
                onClick={() => setSelectedOtherFixtureId(null)}
                className="rounded-lg px-3 py-2 text-sm font-heading font-bold uppercase tracking-wider text-gray-500 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-400 dark:hover:bg-navy-800 dark:hover:text-white transition-colors"
              >
                {t("common.close")}
              </button>
            </div>

            <div className="grid gap-5 p-5 md:grid-cols-[1.15fr_0.85fr]">
              <div className="rounded-xl border border-gray-200 dark:border-navy-700 bg-gray-50 dark:bg-navy-800 p-4 transition-colors duration-300">
                <h4 className="mb-3 text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                  {t("match.matchEvents")}
                </h4>
                {selectedOtherFixtureReport.events.length > 0 ? (
                  <div className="flex max-h-96 flex-col gap-2 overflow-auto">
                    {selectedOtherFixtureReport.events.map((event, index) => {
                      const display = getEventDisplay({
                        ...event,
                        zone: "Midfield",
                      } as MatchEvent);
                      const sideTeamId =
                        event.side === "Home"
                          ? selectedOtherFixture.home_team_id
                          : selectedOtherFixture.away_team_id;
                      const sideFallbackName =
                        event.side === "Home"
                          ? getTeamNameById(selectedOtherFixture.home_team_id)
                          : getTeamNameById(selectedOtherFixture.away_team_id);
                      return (
                        <div
                          key={`${event.minute}-${event.event_type}-${index}`}
                          className="flex items-center gap-2 text-xs"
                        >
                          <span className="w-8 text-right font-heading tabular-nums text-gray-500 dark:text-gray-400">
                            {event.minute}'
                          </span>
                          <span>{display.icon}</span>
                          <span
                            className={`${display.color} flex-1 truncate font-medium`}
                          >
                            {formatOtherMatchEvent(event)}
                          </span>
                          <Badge
                            variant={event.side === "Home" ? "primary" : "accent"}
                            size="sm"
                          >
                            {getTeamShortName(sideTeamId, sideFallbackName)}
                          </Badge>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    {t("match.quietMatch")}
                  </p>
                )}
              </div>

              <div className="flex flex-col gap-4">
                <div className="rounded-xl border border-gray-200 dark:border-navy-700 bg-gray-50 dark:bg-navy-800 p-4 transition-colors duration-300">
                  <h4 className="mb-3 text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                    {t("match.quickStats")}
                  </h4>
                  <QuickStat
                    label={t("match.possession")}
                    home={`${selectedOtherFixtureReport.home_stats.possession_pct}%`}
                    away={`${selectedOtherFixtureReport.away_stats.possession_pct}%`}
                    homePct={selectedOtherFixtureReport.home_stats.possession_pct}
                  />
                  <QuickStat
                    label={t("match.shots")}
                    home={selectedOtherFixtureReport.home_stats.shots}
                    away={selectedOtherFixtureReport.away_stats.shots}
                  />
                  <QuickStat
                    label={t("match.shotsOnTarget")}
                    home={selectedOtherFixtureReport.home_stats.shots_on_target}
                    away={selectedOtherFixtureReport.away_stats.shots_on_target}
                  />
                  <QuickStat
                    label={t("match.fouls")}
                    home={selectedOtherFixtureReport.home_stats.fouls}
                    away={selectedOtherFixtureReport.away_stats.fouls}
                  />
                  <QuickStat
                    label={t("match.corners")}
                    home={selectedOtherFixtureReport.home_stats.corners}
                    away={selectedOtherFixtureReport.away_stats.corners}
                  />
                  <QuickStat
                    label={t("match.yellowCards")}
                    home={selectedOtherFixtureReport.home_stats.yellow_cards}
                    away={selectedOtherFixtureReport.away_stats.yellow_cards}
                  />
                </div>
                <div className="rounded-xl border border-gray-200 dark:border-navy-700 bg-gray-50 dark:bg-navy-800 p-4 transition-colors duration-300">
                  <h4 className="mb-3 text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                    {t("match.scorers")}
                  </h4>
                  {formatOtherMatchScorers(selectedOtherFixture) ? (
                    <p className="text-xs text-gray-700 dark:text-gray-300">
                      {formatOtherMatchScorers(selectedOtherFixture)}
                    </p>
                  ) : (
                    <p className="text-xs text-gray-500 dark:text-gray-400">
                      {t("match.noGoals")}
                    </p>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
