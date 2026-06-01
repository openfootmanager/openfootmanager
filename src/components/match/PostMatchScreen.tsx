import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { FixtureData, GameStateData } from "../../store/gameStore";
import type { CompactMatchEventData } from "../../store/types";
import {
  MatchSnapshot,
  MatchEvent,
  getTeamTalkOptions,
  RoundSummary,
  TeamTalkTone,
} from "./types";
import { getEventDisplay, getPlayerName } from "./helpers";
import { getTalkIcon } from "./TeamTalkIcons";
import { Badge, ThemeToggle } from "../ui";
import {
  QuickStat,
  renderScorers,
  PlayerRatingsPanel,
} from "./PostMatchHelpers";
import {
  Trophy,
  TrendingDown,
  Minus,
  Star,
  MessageCircle,
  ChevronRight,
  BarChart3,
} from "lucide-react";

interface PostMatchScreenProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  currentFixture?: FixtureData | null;
  userSide: "Home" | "Away" | null;
  isSpectator: boolean;
  importantEvents: MatchEvent[];
  roundSummary: RoundSummary | null;
  onPressConference: () => void;
  onFinish: () => void;
}

export default function PostMatchScreen({
  snapshot,
  gameState,
  currentFixture,
  userSide,
  isSpectator,
  importantEvents,
  roundSummary,
  onPressConference,
  onFinish,
}: PostMatchScreenProps) {
  const { t } = useTranslation();
  const teamTalkOptions = getTeamTalkOptions(t);
  const [selectedTalk, setSelectedTalk] = useState<TeamTalkTone | null>(null);
  const [talkDelivered, setTalkDelivered] = useState(false);
  const [selectedOtherFixtureId, setSelectedOtherFixtureId] = useState<
    string | null
  >(null);
  const [talkResults, setTalkResults] = useState<
    {
      player_id: string;
      player_name: string;
      old_morale: number;
      new_morale: number;
      delta: number;
    }[]
  >([]);

  const homeTeamColor =
    gameState.teams.find((t) => t.id === snapshot.home_team.id)?.colors
      ?.primary || "#10b981";
  const awayTeamColor =
    gameState.teams.find((t) => t.id === snapshot.away_team.id)?.colors
      ?.primary || "#6366f1";

  const userScore =
    userSide === "Home" ? snapshot.home_score : snapshot.away_score;
  const oppScore =
    userSide === "Home" ? snapshot.away_score : snapshot.home_score;

  const resultType =
    userScore > oppScore ? "win" : userScore < oppScore ? "loss" : "draw";
  const isLeagueFixture =
    currentFixture?.competition !== "Friendly" &&
    currentFixture?.competition !== "PreseasonTournament";
  const summaryTitle = isLeagueFixture
    ? t("match.roundSummary")
    : t("match.otherMatches");
  const summaryContextLabel = isLeagueFixture
    ? roundSummary
      ? t("schedule.matchday", {
        number: roundSummary.matchday,
      })
      : null
    : currentFixture
      ? t("match.otherMatchesToday")
      : null;
  const summaryEmptyState = isLeagueFixture
    ? t("match.roundSummaryUnavailable")
    : t("match.otherMatchesUnavailable");
  const shouldRenderSummaryBody = isLeagueFixture
    ? roundSummary !== null
    : currentFixture !== null;
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
    if (!fixture.result) {
      return null;
    }

    const scorers = [
      ...fixture.result.home_scorers,
      ...fixture.result.away_scorers,
    ]
      .sort((left, right) => left.minute - right.minute)
      .map(
        (scorer) =>
          `${getPlayerDisplayName(scorer.player_id)} ${scorer.minute}'`,
      );

    if (scorers.length === 0) {
      return null;
    }

    return scorers.slice(0, 3).join(" • ");
  };
  const formatOtherMatchStats = (fixture: FixtureData) => {
    const report = getFixtureReport(fixture);
    if (!report) {
      return null;
    }

    const totalYellowCards =
      report.home_stats.yellow_cards + report.away_stats.yellow_cards;

    return [
      `${report.home_stats.possession_pct}-${report.away_stats.possession_pct} ${t("match.possession")}`,
      `${report.home_stats.shots + report.away_stats.shots} ${t("match.shots")}`,
      `${totalYellowCards} ${t("match.yellowCards")}`,
    ].join(" • ");
  };
  const formatOtherMatchEvent = (event: CompactMatchEventData) => {
    const primaryPlayer = getPlayerDisplayName(event.player_id);

    switch (event.event_type) {
      case "Goal":
        return event.secondary_player_id
          ? `${primaryPlayer} (${t("match.assist", {
            name: getPlayerDisplayName(event.secondary_player_id),
          })})`
          : primaryPlayer;
      case "PenaltyGoal":
        return `${primaryPlayer} (P)`;
      case "PenaltyMiss":
        return `${primaryPlayer} (P)`;
      case "Substitution":
        return `${primaryPlayer} ${t("match.subFor", {
          name: getPlayerDisplayName(event.secondary_player_id),
        })}`;
      default:
        return primaryPlayer;
    }
  };
  const otherMatchEntries = isLeagueFixture
    ? (roundSummary?.completed_results || [])
      .filter((result) => result.fixture_id !== currentFixture?.id)
      .map((result) => {
        const fixture = gameState.league?.fixtures.find(
          (candidate) => candidate.id === result.fixture_id,
        );

        if (!fixture || !fixture.result) {
          return null;
        }

        return {
          fixture,
          homeTeamName: result.home_team_name,
          awayTeamName: result.away_team_name,
        };
      })
      .filter(
        (
          entry,
        ): entry is {
          fixture: FixtureData;
          homeTeamName: string;
          awayTeamName: string;
        } => entry !== null,
      )
    : (gameState.league?.fixtures || [])
      .filter(
        (fixture) =>
          fixture.id !== currentFixture?.id &&
          fixture.status === "Completed" &&
          fixture.result &&
          fixture.date === currentFixture?.date &&
          fixture.competition === currentFixture?.competition,
      )
      .map((fixture) => ({
        fixture,
        homeTeamName: getTeamNameById(fixture.home_team_id),
        awayTeamName: getTeamNameById(fixture.away_team_id),
      }));
  const selectedOtherFixture = selectedOtherFixtureId
    ? otherMatchEntries.find(
      (entry) => entry.fixture.id === selectedOtherFixtureId,
    )?.fixture || null
    : null;
  const selectedOtherFixtureReport = getFixtureReport(selectedOtherFixture);

  // Key events (goals, cards, subs)
  const keyEvents = importantEvents.filter((e) =>
    [
      "Goal",
      "PenaltyGoal",
      "YellowCard",
      "RedCard",
      "SecondYellow",
      "PenaltyMiss",
      "Injury",
    ].includes(e.event_type),
  );

  // Count stats from snapshot events
  const homeEvents = snapshot.events.filter((e) => e.side === "Home");
  const awayEvents = snapshot.events.filter((e) => e.side === "Away");
  const countType = (events: MatchEvent[], type: string) =>
    events.filter((e) => e.event_type === type).length;

  const homeShots =
    countType(homeEvents, "Goal") +
    countType(homeEvents, "PenaltyGoal") +
    countType(homeEvents, "ShotSaved") +
    countType(homeEvents, "ShotOffTarget") +
    countType(homeEvents, "ShotBlocked");
  const awayShots =
    countType(awayEvents, "Goal") +
    countType(awayEvents, "PenaltyGoal") +
    countType(awayEvents, "ShotSaved") +
    countType(awayEvents, "ShotOffTarget") +
    countType(awayEvents, "ShotBlocked");

  // Suggested team talk based on result
  const suggestedTalks: TeamTalkTone[] =
    resultType === "win"
      ? ["praise", "calm", "motivational"]
      : resultType === "loss"
        ? ["motivational", "assertive", "disappointed"]
        : ["calm", "motivational", "assertive"];

  const handleDeliverTalk = async () => {
    if (!selectedTalk) return;
    const context =
      resultType === "win"
        ? "winning"
        : resultType === "loss"
          ? "losing"
          : "drawing";
    try {
      const results = await invoke<
        {
          player_id: string;
          player_name: string;
          old_morale: number;
          new_morale: number;
          delta: number;
        }[]
      >("apply_team_talk", { tone: selectedTalk, context });
      setTalkResults(results);
    } catch (err) {
      console.error("Team talk failed:", err);
    }
    setTalkDelivered(true);
  };

  return (
    <div className="min-h-screen bg-gray-100 text-gray-900 dark:bg-navy-900 dark:text-white flex flex-col transition-colors duration-300">
      {/* Result Header */}
      <header
        className={`border-b border-gray-200 dark:border-navy-700 px-4 py-8 transition-colors duration-300 ${resultType === "win"
            ? "bg-linear-to-r from-primary-100 via-white to-primary-100 dark:from-primary-900/50 dark:via-navy-900 dark:to-primary-900/50"
            : resultType === "loss"
              ? "bg-linear-to-r from-red-100 via-white to-red-100 dark:from-red-900/30 dark:via-navy-900 dark:to-red-900/30"
              : "bg-linear-to-r from-gray-200 via-white to-gray-200 dark:from-navy-800 dark:via-navy-900 dark:to-navy-800"
          }`}
      >
        <div className="max-w-5xl mx-auto text-center relative">
          <ThemeToggle className="absolute right-0 top-0" />
          {/* Result badge */}
          {!isSpectator && userSide && (
            <div className="mb-4">
              {resultType === "win" && (
                <div className="inline-flex items-center gap-2 px-4 py-1.5 bg-primary-100 dark:bg-primary-500/20 rounded-full transition-colors duration-300">
                  <Trophy className="w-4 h-4 text-accent-700 dark:text-accent-400" />
                  <span className="font-heading font-bold text-sm uppercase tracking-widest text-primary-700 dark:text-primary-400">
                    {t("match.victory")}
                  </span>
                </div>
              )}
              {resultType === "loss" && (
                <div className="inline-flex items-center gap-2 px-4 py-1.5 bg-red-500/20 rounded-full">
                  <TrendingDown className="w-4 h-4 text-red-400" />
                  <span className="font-heading font-bold text-sm uppercase tracking-widest text-red-400">
                    {t("match.defeat")}
                  </span>
                </div>
              )}
              {resultType === "draw" && (
                <div className="inline-flex items-center gap-2 px-4 py-1.5 bg-gray-500/20 rounded-full">
                  <Minus className="w-4 h-4 text-gray-400" />
                  <span className="font-heading font-bold text-sm uppercase tracking-widest text-gray-400">
                    {t("match.draw")}
                  </span>
                </div>
              )}
            </div>
          )}

          {/* Scoreboard */}
          <div className="flex items-center justify-center gap-10">
            <div className="flex items-center gap-4">
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center font-heading font-bold text-lg"
                style={{
                  backgroundColor: homeTeamColor + "30",
                  borderColor: homeTeamColor,
                  borderWidth: 2,
                }}
              >
                {snapshot.home_team.name.substring(0, 3).toUpperCase()}
              </div>
              <p className="font-heading font-bold text-lg text-gray-800 dark:text-gray-200">
                {snapshot.home_team.name}
              </p>
            </div>

            <div className="flex items-center gap-5">
              <span className="text-6xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
                {snapshot.home_score}
              </span>
              <div className="text-center">
                <p className="text-xs font-heading uppercase tracking-widest text-accent-700 dark:text-accent-400">
                  {t("match.fullTime")}
                </p>
                <p className="text-lg font-heading font-bold text-gray-500 dark:text-gray-500">
                  {t("match.ft")}
                </p>
              </div>
              <span className="text-6xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
                {snapshot.away_score}
              </span>
            </div>

            <div className="flex items-center gap-4">
              <p className="font-heading font-bold text-lg text-gray-800 dark:text-gray-200">
                {snapshot.away_team.name}
              </p>
              <div
                className="w-16 h-16 rounded-xl flex items-center justify-center font-heading font-bold text-lg"
                style={{
                  backgroundColor: awayTeamColor + "30",
                  borderColor: awayTeamColor,
                  borderWidth: 2,
                }}
              >
                {snapshot.away_team.name.substring(0, 3).toUpperCase()}
              </div>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex-1 overflow-auto">
        <div className="max-w-5xl mx-auto px-6 py-6 grid grid-cols-3 gap-6">
          {/* Left: Match Events */}
          <div className="flex flex-col gap-4">
            <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
              <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
                {t("match.matchEvents")}
              </h3>
              {keyEvents.length === 0 ? (
                <p className="text-xs text-gray-600 dark:text-gray-500">{t("match.quietMatch")}</p>
              ) : (
                <div className="flex flex-col gap-2">
                  {keyEvents.map((evt, i) => {
                    const display = getEventDisplay(evt);
                    return (
                      <div key={i} className="flex items-center gap-2 text-xs">
                        <span className="text-gray-600 dark:text-gray-500 tabular-nums w-6 text-right font-heading">
                          {evt.minute}'
                        </span>
                        <span>{display.icon}</span>
                        <span
                          className={`${display.color} font-medium truncate flex-1`}
                        >
                          {getPlayerName(snapshot, evt.player_id)}
                        </span>
                        <Badge
                          variant={evt.side === "Home" ? "primary" : "accent"}
                          size="sm"
                        >
                          {evt.side === "Home"
                            ? snapshot.home_team.name.substring(0, 3)
                            : snapshot.away_team.name.substring(0, 3)}
                        </Badge>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

            {/* Quick Stats */}
            <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
              <div className="flex items-center gap-2 mb-3">
                <BarChart3 className="w-4 h-4 text-gray-500 dark:text-gray-400" />
                <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                  {t("match.quickStats")}
                </h3>
              </div>
              <QuickStat
                label={t("match.possession")}
                home={`${snapshot.home_possession_pct.toFixed(0)}%`}
                away={`${snapshot.away_possession_pct.toFixed(0)}%`}
                homePct={snapshot.home_possession_pct}
              />
              <QuickStat
                label={t("match.shots")}
                home={homeShots}
                away={awayShots}
              />
              <QuickStat
                label={t("match.fouls")}
                home={countType(homeEvents, "Foul")}
                away={countType(awayEvents, "Foul")}
              />
              <QuickStat
                label={t("match.corners")}
                home={countType(homeEvents, "Corner")}
                away={countType(awayEvents, "Corner")}
              />
            </div>

            <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
              <div className="flex items-center gap-2 mb-3">
                <BarChart3 className="w-4 h-4 text-accent-400" />
                <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                  {summaryTitle}
                </h3>
              </div>

              {shouldRenderSummaryBody ? (
                <div className="flex flex-col gap-4">
                  <div>
                    {summaryContextLabel && (
                      <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-200 mb-2">
                        {summaryContextLabel}
                      </p>
                    )}
                    <div className="flex flex-col gap-2 text-xs text-gray-700 dark:text-gray-300">
                      {otherMatchEntries.length > 0 ? (
                        otherMatchEntries.map((entry) => {
                          const scorerSummary = formatOtherMatchScorers(
                            entry.fixture,
                          );
                          const statSummary = formatOtherMatchStats(
                            entry.fixture,
                          );

                          return (
                            <div
                              key={entry.fixture.id}
                              className="rounded-lg bg-gray-100 dark:bg-navy-700/40 px-3 py-2 transition-colors duration-300"
                            >
                              <div className="flex items-center justify-between gap-3">
                                <span className="truncate font-medium text-gray-800 dark:text-gray-200">
                                  {entry.homeTeamName}{" "}
                                  {entry.fixture.result?.home_goals} -{" "}
                                  {entry.fixture.result?.away_goals}{" "}
                                  {entry.awayTeamName}
                                </span>
                                {entry.fixture.result?.report && (
                                  <button
                                    onClick={() =>
                                      setSelectedOtherFixtureId(
                                        entry.fixture.id,
                                      )
                                    }
                                    className="shrink-0 rounded-md px-2 py-1 text-[10px] font-heading font-bold uppercase tracking-widest text-accent-400 hover:bg-gray-200 dark:hover:bg-navy-600/60 transition-colors"
                                  >
                                    {t("match.viewDetails")}
                                  </button>
                                )}
                              </div>
                              {scorerSummary && (
                                <p className="mt-1 text-[11px] text-gray-500 dark:text-gray-400">
                                  {scorerSummary}
                                </p>
                              )}
                              {statSummary && (
                                <p className="mt-1 text-[10px] uppercase tracking-wider text-gray-500 dark:text-gray-400">
                                  {statSummary}
                                </p>
                              )}
                            </div>
                          );
                        })
                      ) : (
                        <p className="text-gray-500 dark:text-gray-400">{summaryEmptyState}</p>
                      )}
                    </div>
                  </div>

                  {isLeagueFixture && roundSummary && (
                    <>
                      <div>
                        <p className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-2">
                          {t("tournaments.leagueTable")}
                        </p>
                        <div className="flex flex-col gap-1 text-xs">
                          {roundSummary.standings_delta
                            .slice(0, 5)
                            .map((entry) => (
                              <div
                                key={entry.team_id}
                                className="flex items-center justify-between gap-3 text-gray-700 dark:text-gray-300"
                              >
                                <span>
                                  {entry.current_position}. {entry.team_name}
                                </span>
                                <span className="font-heading font-bold tabular-nums text-gray-500 dark:text-gray-400">
                                  {entry.points}
                                </span>
                              </div>
                            ))}
                        </div>
                      </div>

                      <div>
                        <p className="text-[10px] font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-2">
                          {t("tournaments.topScorers")}
                        </p>
                        <div className="flex flex-col gap-1 text-xs">
                          {roundSummary.top_scorer_delta.length > 0 ? (
                            roundSummary.top_scorer_delta
                              .slice(0, 5)
                              .map((entry) => (
                                <div
                                  key={entry.player_id}
                                  className="flex items-center justify-between gap-3 text-gray-700 dark:text-gray-300"
                                >
                                  <span className="truncate">
                                    {entry.current_rank}. {entry.player_name}
                                  </span>
                                  <span className="font-heading font-bold tabular-nums text-accent-400">
                                    {entry.current_goals}
                                  </span>
                                </div>
                              ))
                          ) : (
                            <p className="text-gray-500 dark:text-gray-400">{t("home.noGoals")}</p>
                          )}
                        </div>
                      </div>

                      {roundSummary.notable_upset && (
                        <div className="rounded-lg bg-gray-100 dark:bg-navy-700/50 px-3 py-2 text-xs text-gray-700 dark:text-gray-300 transition-colors duration-300">
                          {roundSummary.notable_upset.underdog_team_name}{" "}
                          {roundSummary.notable_upset.home_goals} -{" "}
                          {roundSummary.notable_upset.away_goals}{" "}
                          {roundSummary.notable_upset.favorite_team_name}
                        </div>
                      )}
                    </>
                  )}
                </div>
              ) : (
                <p className="text-xs text-gray-500 dark:text-gray-400">{summaryEmptyState}</p>
              )}
            </div>
          </div>

          <div className="flex flex-col gap-4">
            {!isSpectator && userSide ? (
              <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
                <div className="flex items-center gap-2 mb-4">
                  <MessageCircle className="w-4 h-4 text-accent-400" />
                  <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                    {t("match.postMatchTeamTalk")}
                  </h3>
                </div>

                {!talkDelivered ? (
                  <>
                    <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
                      {t("match.addressPlayers")}
                    </p>
                    <div className="flex flex-col gap-2">
                      {teamTalkOptions.map((opt) => {
                        const isSuggested = suggestedTalks.includes(opt.id);
                        return (
                          <button
                            key={opt.id}
                            onClick={() => setSelectedTalk(opt.id)}
                            className={`flex items-center gap-3 p-3 rounded-lg text-left transition-all ${selectedTalk === opt.id
                                ? "bg-primary-500/20 ring-2 ring-primary-500/50"
                                : "bg-gray-100 hover:bg-gray-200 dark:bg-navy-700/50 dark:hover:bg-navy-700"
                              }`}
                          >
                            <span className="text-xl">
                              {getTalkIcon(opt.icon)}
                            </span>
                            <div className="flex-1">
                              <div className="flex items-center gap-2">
                                <p
                                  className={`text-sm font-heading font-bold ${selectedTalk === opt.id
                                      ? "text-primary-400"
                                      : "text-gray-800 dark:text-gray-200"
                                    }`}
                                >
                                  {opt.label}
                                </p>
                                {isSuggested && (
                                  <Star className="w-3 h-3 text-accent-400" />
                                )}
                              </div>
                              <p className="text-[11px] text-gray-500 dark:text-gray-400">
                                {opt.description}
                              </p>
                            </div>
                          </button>
                        );
                      })}
                    </div>
                    {selectedTalk && (
                      <button
                        onClick={handleDeliverTalk}
                        className="w-full mt-3 py-2.5 bg-primary-500/20 hover:bg-primary-500/30 text-primary-400 rounded-lg font-heading font-bold text-sm uppercase tracking-wider transition-colors"
                      >
                        {t("match.deliverTeamTalk")}
                      </button>
                    )}
                  </>
                ) : (
                  <div className="flex flex-col gap-2">
                    <div className="flex items-center gap-2 mb-1">
                      {getTalkIcon(selectedTalk || "")}
                      <p className="text-sm font-heading font-bold text-primary-400">
                        {
                          teamTalkOptions.find((o) => o.id === selectedTalk)
                            ?.label
                        }
                      </p>
                      <Badge variant="success" size="sm">
                        {t("match.delivered")}
                      </Badge>
                    </div>
                    {talkResults.length > 0 && (
                      <div className="flex flex-col gap-0.5 max-h-48 overflow-auto">
                        {talkResults.map((r) => (
                          <div
                            key={r.player_id}
                            className="flex items-center gap-2 px-2 py-1 text-xs"
                          >
                            <span className="text-gray-500 dark:text-gray-400 flex-1 truncate">
                              {r.player_name}
                            </span>
                            <span
                              className={`font-heading font-bold tabular-nums ${r.delta > 0 ? "text-green-400" : r.delta < 0 ? "text-red-400" : "text-gray-500 dark:text-gray-400"}`}
                            >
                              {r.delta > 0 ? "+" : ""}
                              {r.delta}
                            </span>
                            <div className="w-12 h-1.5 bg-gray-300 dark:bg-navy-600 rounded-full overflow-hidden transition-colors duration-300">
                              <div
                                className={`h-full rounded-full ${r.new_morale >= 70 ? "bg-green-500" : r.new_morale >= 40 ? "bg-yellow-500" : "bg-red-500"}`}
                                style={{ width: `${r.new_morale}%` }}
                              />
                            </div>
                            <span className="text-gray-500 dark:text-gray-400 tabular-nums w-6 text-right">
                              {r.new_morale}
                            </span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>
            ) : (
              <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 flex flex-col items-center justify-center py-12 transition-colors duration-300">
                <p className="text-lg font-heading font-bold text-gray-500 dark:text-gray-400 mb-2">
                  {t("match.matchOver")}
                </p>
                <p className="text-sm text-gray-500 dark:text-gray-400 text-center">
                  {snapshot.home_score} - {snapshot.away_score}
                </p>
              </div>
            )}
          </div>

          {/* Right: Player Ratings + Scorers */}
          <div className="flex flex-col gap-4">
            {/* Player Ratings — show for both teams */}
            {(["Home", "Away"] as const).map((side) => (
              <PlayerRatingsPanel
                key={side}
                snapshot={snapshot}
                side={side}
                teamColor={side === "Home" ? homeTeamColor : awayTeamColor}
                userSide={userSide}
              />
            ))}

            <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
              <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
                {t("match.scorers")}
              </h3>
              {renderScorers(snapshot, importantEvents, "Home")}
              {renderScorers(snapshot, importantEvents, "Away")}
              {keyEvents.filter(
                (e) =>
                  e.event_type === "Goal" || e.event_type === "PenaltyGoal",
              ).length === 0 && (
                  <p className="text-xs text-gray-600 dark:text-gray-500">{t("match.noGoals")}</p>
                )}
            </div>

            {/* Substitutions made */}
            {snapshot.substitutions.length > 0 && (
              <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
                <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
                  {t("match.substitutions")}
                </h3>
                <div className="flex flex-col gap-2">
                  {snapshot.substitutions.map((sub, i) => (
                    <div key={i} className="flex items-center gap-2 text-xs">
                      <span className="text-gray-600 dark:text-gray-500 tabular-nums w-6 text-right font-heading">
                        {sub.minute}'
                      </span>
                      <span className="text-green-400">↑</span>
                      <span className="text-gray-700 dark:text-gray-300 truncate flex-1">
                        {getPlayerName(snapshot, sub.player_on_id)}
                      </span>
                      <span className="text-red-400">↓</span>
                      <span className="text-gray-500 dark:text-gray-400 truncate">
                        {getPlayerName(snapshot, sub.player_off_id)}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {selectedOtherFixture && selectedOtherFixtureReport && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 px-4">
          <div className="w-full max-w-3xl rounded-2xl border border-gray-200 dark:border-navy-700 bg-white dark:bg-navy-900 shadow-2xl transition-colors duration-300">
            <div className="flex items-center justify-between border-b border-gray-200 dark:border-navy-700 px-5 py-4">
              <div>
                <p className="text-xs font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
                  {t("match.matchDetails")}
                </p>
                <p className="text-lg font-heading font-bold text-gray-900 dark:text-white">
                  {getTeamNameById(selectedOtherFixture.home_team_id)}{" "}
                  {selectedOtherFixture.result?.home_goals} -{" "}
                  {selectedOtherFixture.result?.away_goals}{" "}
                  {getTeamNameById(selectedOtherFixture.away_team_id)}
                </p>
              </div>
              <button
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
                            variant={
                              event.side === "Home" ? "primary" : "accent"
                            }
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
                    homePct={
                      selectedOtherFixtureReport.home_stats.possession_pct
                    }
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

      {/* Footer */}
      <footer className="bg-white dark:bg-navy-800 border-t border-gray-200 dark:border-navy-700 px-6 py-4 transition-colors duration-300">
        <div className="max-w-5xl mx-auto flex justify-between items-center">
          <p className="text-xs text-gray-600 dark:text-gray-500 font-heading uppercase tracking-wider">
            {isSpectator ? t("match.matchComplete") : t("match.addressPress")}
          </p>
          <div className="flex items-center gap-3">
            <button
              onClick={onFinish}
              className="flex items-center gap-2 px-6 py-3 bg-gray-200 hover:bg-gray-300 dark:bg-navy-700 dark:hover:bg-navy-600 rounded-xl font-heading font-bold uppercase tracking-wider text-sm text-gray-700 dark:text-gray-300 transition-colors"
            >
              {t("match.skip")}
              <ChevronRight className="w-4 h-4" />
            </button>
            {!isSpectator && (
              <button
                onClick={onPressConference}
                className="flex items-center gap-2 px-8 py-3 bg-linear-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 rounded-xl font-heading font-bold uppercase tracking-wider text-sm text-white shadow-lg shadow-primary-500/20 transition-all"
              >
                {t("match.pressConference")}
                <ChevronRight className="w-4 h-4" />
              </button>
            )}
          </div>
        </div>
      </footer>
    </div>
  );
}
