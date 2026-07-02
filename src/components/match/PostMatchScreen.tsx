import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { GameStateData } from "../../store/gameStore";
import {
  MatchSnapshot,
  MatchEvent,
  getTeamTalkOptions,
  TeamTalkTone,
} from "./types";
import { getEventDisplay, getPlayerName, makeTeamFallback } from "./helpers";
import { getTalkIcon } from "./TeamTalkIcons";
import { Badge, TeamLogo, ThemeToggle } from "../ui";
import {
  QuickStat,
  renderScorers,
  PlayerRatingsPanel,
} from "./PostMatchHelpers";
import { PossessionDonut } from "./PostMatchCharts";
import {
  Trophy,
  TrendingDown,
  Minus,
  Star,
  MessageCircle,
  ChevronRight,
  BarChart3,
  Users,
  FileText,
  Shield,
} from "lucide-react";

interface PostMatchScreenProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  userSide: "Home" | "Away" | null;
  isSpectator: boolean;
  importantEvents: MatchEvent[];
  onContinue: () => void;
  onFinish: () => void;
}

type PostMatchTab = "teamTalk" | "matchReport" | "playerRatings" | "tactics";

const SET_PIECE_CLEAR_EVENTS = new Set([
  "ShotOffTarget", "ShotBlocked", "ShotSaved", "PenaltyMiss",
  "Clearance", "Interception", "PassIntercepted", "GoalKick",
]);

export function computeGoalSources(
  events: MatchEvent[],
  side: "Home" | "Away",
): { openPlay: number; corners: number; freekicks: number; penalties: number } {
  const sources = { openPlay: 0, corners: 0, freekicks: 0, penalties: 0 };
  // Track (type, side) so a set piece by one team doesn't attribute the other's goal
  let lastSetPiece: { type: "Corner" | "FreeKick"; side: "Home" | "Away" } | null = null;
  for (const evt of events) {
    if (evt.event_type === "Corner") {
      lastSetPiece = { type: "Corner", side: evt.side };
    } else if (evt.event_type === "FreeKick") {
      lastSetPiece = null;
      // Only dangerous FKs: taking side must be in their attacking third
      const dangerousZone = evt.side === "Home" ? "AwayDefense" : "HomeDefense";
      if (evt.zone === dangerousZone) {
        lastSetPiece = { type: "FreeKick", side: evt.side };
      }
    } else if (SET_PIECE_CLEAR_EVENTS.has(evt.event_type)) {
      lastSetPiece = null;
    } else if (evt.event_type === "Goal") {
      if (evt.side === side) {
        if (lastSetPiece?.type === "Corner" && lastSetPiece.side === side) sources.corners++;
        else if (lastSetPiece?.type === "FreeKick" && lastSetPiece.side === side) sources.freekicks++;
        else sources.openPlay++;
      }
      lastSetPiece = null;
    } else if (evt.event_type === "PenaltyGoal") {
      if (evt.side === side) sources.penalties++;
      lastSetPiece = null;
    }
  }
  return sources;
}

export default function PostMatchScreen({
  snapshot,
  gameState,
  userSide,
  isSpectator,
  importantEvents,
  onContinue,
  onFinish,
}: PostMatchScreenProps) {
  const { t } = useTranslation();
  const teamTalkOptions = getTeamTalkOptions(t);
  const [activeTab, setActiveTab] = useState<PostMatchTab>(
    !isSpectator && userSide ? "teamTalk" : "matchReport",
  );
  const [selectedTalk, setSelectedTalk] = useState<TeamTalkTone | null>(null);
  const [talkDelivered, setTalkDelivered] = useState(false);
  const [talkPending, setTalkPending] = useState(false);
  const [talkError, setTalkError] = useState<string | null>(null);
  const [talkResults, setTalkResults] = useState<
    {
      player_id: string;
      player_name: string;
      old_morale: number;
      new_morale: number;
      delta: number;
    }[]
  >([]);

  const homeFullTeam = gameState.teams.find(
    (t) => t.id === snapshot.home_team.id,
  );
  const awayFullTeam = gameState.teams.find(
    (t) => t.id === snapshot.away_team.id,
  );
  const homeTeamColor = homeFullTeam?.colors?.primary || "#10b981";
  const awayTeamColor = awayFullTeam?.colors?.primary || "#6366f1";

  const userScore =
    userSide === "Home" ? snapshot.home_score : snapshot.away_score;
  const oppScore =
    userSide === "Home" ? snapshot.away_score : snapshot.home_score;

  // The match score stays the regulation/ET score; a level tie decided on
  // penalties resolves the verdict via the shootout tally instead.
  const shootout = snapshot.penalty_shootout;
  const userPens =
    userSide === "Home" ? shootout?.home_scored : shootout?.away_scored;
  const oppPens =
    userSide === "Home" ? shootout?.away_scored : shootout?.home_scored;

  const resultType =
    userScore > oppScore
      ? "win"
      : userScore < oppScore
        ? "loss"
        : userPens !== undefined && oppPens !== undefined && userPens !== oppPens
          ? userPens > oppPens
            ? "win"
            : "loss"
          : "draw";

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

  const suggestedTalks: TeamTalkTone[] =
    resultType === "win"
      ? ["praise", "calm", "motivational"]
      : resultType === "loss"
        ? ["motivational", "assertive", "disappointed"]
        : ["calm", "motivational", "assertive"];

  const handleDeliverTalk = async () => {
    if (!selectedTalk || talkPending) return;
    const context =
      resultType === "win"
        ? "winning"
        : resultType === "loss"
          ? "losing"
          : "drawing";
    setTalkPending(true);
    setTalkError(null);
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
      setTalkDelivered(true);
    } catch (err) {
      console.error("Team talk failed:", err);
      setTalkError(t("match.teamTalkFailed"));
    } finally {
      setTalkPending(false);
    }
  };

  const tabs: { id: PostMatchTab; label: string; icon: React.ReactNode }[] = [
    ...(!isSpectator && userSide
      ? [
          {
            id: "teamTalk" as PostMatchTab,
            label: t("match.postMatchTeamTalk"),
            icon: <MessageCircle className="w-4 h-4" />,
          },
        ]
      : []),
    {
      id: "matchReport",
      label: t("match.matchReport"),
      icon: <FileText className="w-4 h-4" />,
    },
    {
      id: "playerRatings",
      label: t("match.playerRatings"),
      icon: <Users className="w-4 h-4" />,
    },
    {
      id: "tactics",
      label: t("match.tacticsTab"),
      icon: <Shield className="w-4 h-4" />,
    },
  ];

  return (
    <div className="min-h-screen bg-gray-100 text-gray-900 dark:bg-navy-900 dark:text-white flex flex-col transition-colors duration-300">
      {/* Result Header */}
      <header
        className={`border-b border-gray-200 dark:border-navy-700 px-4 py-6 transition-colors duration-300 ${
          resultType === "win"
            ? "bg-linear-to-r from-primary-100 via-white to-primary-100 dark:from-primary-900/50 dark:via-navy-900 dark:to-primary-900/50"
            : resultType === "loss"
              ? "bg-linear-to-r from-red-100 via-white to-red-100 dark:from-red-900/30 dark:via-navy-900 dark:to-red-900/30"
              : "bg-linear-to-r from-gray-200 via-white to-gray-200 dark:from-navy-800 dark:via-navy-900 dark:to-navy-800"
        }`}
      >
        <div className="text-center relative">
          <ThemeToggle className="absolute right-0 top-0" />

          {/* Result badge */}
          {!isSpectator && userSide && (
            <div className="mb-3">
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
              <TeamLogo
                team={homeFullTeam ?? makeTeamFallback(snapshot.home_team.name)}
                className="w-14 h-14 rounded-xl flex items-center justify-center font-heading font-bold text-lg overflow-hidden"
                imageClassName="h-10 w-10 object-contain drop-shadow"
                style={{
                  backgroundColor: homeTeamColor + "30",
                  borderColor: homeTeamColor,
                  borderWidth: 2,
                }}
              />
              <p className="font-heading font-bold text-base text-gray-800 dark:text-gray-200">
                {snapshot.home_team.name}
              </p>
            </div>

            <div className="flex items-center gap-5">
              <span className="text-5xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
                {snapshot.home_score}
              </span>
              <div className="text-center">
                <p className="text-xs font-heading uppercase tracking-widest text-accent-700 dark:text-accent-400">
                  {t("match.fullTime")}
                </p>
                <p className="text-base font-heading font-bold text-gray-500 dark:text-gray-500">
                  {t("match.ft")}
                </p>
                {shootout && (
                  <p className="text-sm font-heading font-bold text-gray-500 dark:text-gray-400 tabular-nums whitespace-nowrap">
                    {t("match.pen")} {shootout.home_scored}–{shootout.away_scored}
                  </p>
                )}
              </div>
              <span className="text-5xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
                {snapshot.away_score}
              </span>
            </div>

            <div className="flex items-center gap-4">
              <p className="font-heading font-bold text-base text-gray-800 dark:text-gray-200">
                {snapshot.away_team.name}
              </p>
              <TeamLogo
                team={awayFullTeam ?? makeTeamFallback(snapshot.away_team.name)}
                className="w-14 h-14 rounded-xl flex items-center justify-center font-heading font-bold text-lg overflow-hidden"
                imageClassName="h-10 w-10 object-contain drop-shadow"
                style={{
                  backgroundColor: awayTeamColor + "30",
                  borderColor: awayTeamColor,
                  borderWidth: 2,
                }}
              />
            </div>
          </div>
        </div>
      </header>

      {/* Sticky Action Bar */}
      <div className="sticky top-0 z-10 bg-white dark:bg-navy-800 border-b border-gray-200 dark:border-navy-700 shadow-sm transition-colors duration-300">
        <div className="px-6 py-3 flex items-center justify-center gap-3">
          {isSpectator ? (
            <button
              type="button"
              onClick={onFinish}
              className="flex items-center gap-2 px-6 py-2 bg-linear-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 rounded-lg font-heading font-bold uppercase tracking-wider text-sm text-white shadow-md shadow-primary-500/20 transition-all"
            >
              {t("match.continueDashboard")}
              <ChevronRight className="w-4 h-4" />
            </button>
          ) : (
            <>
              <button
                type="button"
                onClick={onFinish}
                className="flex items-center gap-2 px-5 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-navy-700 dark:hover:bg-navy-600 rounded-lg font-heading font-bold uppercase tracking-wider text-sm text-gray-700 dark:text-gray-300 transition-colors"
              >
                {t("match.skip")}
              </button>
              <button
                type="button"
                onClick={onContinue}
                className="flex items-center gap-2 px-6 py-2 bg-linear-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 rounded-lg font-heading font-bold uppercase tracking-wider text-sm text-white shadow-md shadow-primary-500/20 transition-all"
              >
                {t("match.continue")}
                <ChevronRight className="w-4 h-4" />
              </button>
            </>
          )}
        </div>
      </div>

      {/* Tab Bar */}
      <div className="bg-white dark:bg-navy-800 border-b border-gray-200 dark:border-navy-700 transition-colors duration-300">
        <div className="px-6">
          <div className="flex gap-1" role="tablist">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                id={`tab-${tab.id}`}
                type="button"
                role="tab"
                aria-selected={activeTab === tab.id}
                aria-controls={`tabpanel-${tab.id}`}
                onClick={() => setActiveTab(tab.id)}
                className={`flex items-center gap-2 px-5 py-3 text-sm font-heading font-bold uppercase tracking-wider border-b-2 transition-colors ${
                  activeTab === tab.id
                    ? "border-primary-500 text-primary-600 dark:text-primary-400"
                    : "border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200"
                }`}
              >
                {tab.icon}
                {tab.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-auto">
        <div className="px-6 py-6">
          {/* Team Talk Tab */}
          {!isSpectator && userSide && (
            <div
              id="tabpanel-teamTalk"
              role="tabpanel"
              aria-labelledby="tab-teamTalk"
              hidden={activeTab !== "teamTalk"}
              className="max-w-2xl mx-auto"
            >
              {!talkDelivered ? (
                <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-6 transition-colors duration-300">
                  <div className="flex items-center gap-2 mb-4">
                    <MessageCircle className="w-4 h-4 text-accent-400" />
                    <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                      {t("match.postMatchTeamTalk")}
                    </h3>
                  </div>
                  <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
                    {t("match.addressPlayers")}
                  </p>
                  <div className="grid grid-cols-2 gap-3 mb-4">
                    {teamTalkOptions.map((opt) => {
                      const isSuggested = suggestedTalks.includes(opt.id);
                      return (
                        <button
                          key={opt.id}
                          type="button"
                          onClick={() => setSelectedTalk(opt.id)}
                          className={`flex items-center gap-3 p-4 rounded-xl text-left transition-all ${
                            selectedTalk === opt.id
                              ? "bg-primary-500/20 ring-2 ring-primary-500/50"
                              : "bg-gray-50 hover:bg-gray-100 dark:bg-navy-700/50 dark:hover:bg-navy-700"
                          }`}
                        >
                          <span className="text-2xl">
                            {getTalkIcon(opt.icon)}
                          </span>
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-1.5">
                              <p
                                className={`text-sm font-heading font-bold truncate ${
                                  selectedTalk === opt.id
                                    ? "text-primary-400"
                                    : "text-gray-800 dark:text-gray-200"
                                }`}
                              >
                                {opt.label}
                              </p>
                              {isSuggested && (
                                <Star className="w-3 h-3 text-accent-400 shrink-0" />
                              )}
                            </div>
                            <p className="text-[11px] text-gray-500 dark:text-gray-400 truncate">
                              {opt.description}
                            </p>
                          </div>
                        </button>
                      );
                    })}
                  </div>
                  {selectedTalk && (
                    <button
                      type="button"
                      onClick={handleDeliverTalk}
                      disabled={talkPending}
                      className={`w-full py-3 rounded-xl font-heading font-bold text-sm uppercase tracking-wider transition-colors ${
                        talkPending
                          ? "bg-gray-300 dark:bg-navy-600 text-gray-500 cursor-not-allowed"
                          : "bg-primary-500/20 hover:bg-primary-500/30 text-primary-400"
                      }`}
                    >
                      {talkPending
                        ? t("match.delivering")
                        : t("match.deliverTeamTalk")}
                    </button>
                  )}
                  {talkError && (
                    <p className="text-sm text-red-500 mt-2">{talkError}</p>
                  )}
                </div>
              ) : (
                <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-6 transition-colors duration-300">
                  <div className="flex items-center gap-3 mb-5">
                    <span className="text-2xl">
                      {getTalkIcon(selectedTalk || "")}
                    </span>
                    <div>
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
                  </div>
                  {talkResults.length > 0 && (
                    <div className="rounded-lg overflow-hidden border border-gray-200 dark:border-navy-700">
                      <table className="w-full text-xs">
                        <thead>
                          <tr className="bg-gray-50 dark:bg-navy-700/50">
                            <th className="text-left px-3 py-2 font-heading uppercase tracking-wider text-gray-500 dark:text-gray-400">
                              {t("match.player")}
                            </th>
                            <th className="text-right px-3 py-2 font-heading uppercase tracking-wider text-gray-500 dark:text-gray-400">
                              Δ
                            </th>
                            <th className="px-3 py-2 w-24"></th>
                            <th className="text-right px-3 py-2 font-heading uppercase tracking-wider text-gray-500 dark:text-gray-400">
                              %
                            </th>
                          </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-100 dark:divide-navy-700">
                          {talkResults.map((r) => (
                            <tr
                              key={r.player_id}
                              className="hover:bg-gray-50 dark:hover:bg-navy-700/30 transition-colors"
                            >
                              <td className="px-3 py-2 text-gray-700 dark:text-gray-300 font-medium truncate max-w-0 w-full">
                                {r.player_name}
                              </td>
                              <td
                                className={`px-3 py-2 text-right font-heading font-bold tabular-nums ${
                                  r.delta > 0
                                    ? "text-green-500"
                                    : r.delta < 0
                                      ? "text-red-400"
                                      : "text-gray-400"
                                }`}
                              >
                                {r.delta > 0 ? "+" : ""}
                                {r.delta}
                              </td>
                              <td className="px-3 py-2">
                                <div className="h-1.5 bg-gray-200 dark:bg-navy-600 rounded-full overflow-hidden">
                                  <div
                                    className={`h-full rounded-full transition-all ${
                                      r.new_morale >= 70
                                        ? "bg-green-500"
                                        : r.new_morale >= 40
                                          ? "bg-yellow-500"
                                          : "bg-red-500"
                                    }`}
                                    style={{ width: `${r.new_morale}%` }}
                                  />
                                </div>
                              </td>
                              <td className="px-3 py-2 text-right text-gray-500 dark:text-gray-400 tabular-nums font-heading">
                                {r.new_morale}
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}

          {/* Match Report Tab */}
          <div
            id="tabpanel-matchReport"
            role="tabpanel"
            aria-labelledby="tab-matchReport"
            hidden={activeTab !== "matchReport"}
            className="grid grid-cols-2 gap-6"
          >
              {/* Scorers */}
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
                  <p className="text-xs text-gray-600 dark:text-gray-500">
                    {t("match.noGoals")}
                  </p>
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
                <div className="flex justify-center mb-3">
                  <PossessionDonut
                    homePct={snapshot.home_possession_pct}
                    awayPct={snapshot.away_possession_pct}
                    homeTeamName={snapshot.home_team.name}
                    awayTeamName={snapshot.away_team.name}
                    homeColor={homeTeamColor}
                    awayColor={awayTeamColor}
                    label={t("match.possession")}
                  />
                </div>
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

              {/* Key Events */}
              <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
                <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
                  {t("match.matchEvents")}
                </h3>
                {keyEvents.length === 0 ? (
                  <p className="text-xs text-gray-600 dark:text-gray-500">
                    {t("match.quietMatch")}
                  </p>
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

              {/* Substitutions */}
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

          {/* Player Ratings Tab */}
          <div
            id="tabpanel-playerRatings"
            role="tabpanel"
            aria-labelledby="tab-playerRatings"
            hidden={activeTab !== "playerRatings"}
            className="grid grid-cols-2 gap-6"
          >
            {(["Home", "Away"] as const).map((side) => (
              <PlayerRatingsPanel
                key={side}
                snapshot={snapshot}
                side={side}
                teamColor={side === "Home" ? homeTeamColor : awayTeamColor}
                userSide={userSide}
              />
            ))}
          </div>

          {/* Tactics Tab */}
          <div
            id="tabpanel-tactics"
            role="tabpanel"
            aria-labelledby="tab-tactics"
            hidden={activeTab !== "tactics"}
            className="grid grid-cols-2 gap-6"
          >
            {/* Goal Sources — spans both columns */}
            {(() => {
              const homeSrc = computeGoalSources(snapshot.events, "Home");
              const awaySrc = computeGoalSources(snapshot.events, "Away");
              const homeTotal = homeSrc.openPlay + homeSrc.corners + homeSrc.freekicks + homeSrc.penalties || 1;
              const awayTotal = awaySrc.openPlay + awaySrc.corners + awaySrc.freekicks + awaySrc.penalties || 1;
              const sourceKeys: { key: keyof typeof homeSrc; label: string }[] = [
                { key: "openPlay", label: t("match.openPlay") },
                { key: "corners", label: t("match.cornersGoals") },
                { key: "freekicks", label: t("match.freekickGoals") },
                { key: "penalties", label: t("match.penaltyGoals") },
              ];
              return (
                <div className="col-span-2 bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
                  <div className="flex items-center gap-2 mb-3">
                    <BarChart3 className="w-4 h-4 text-gray-500 dark:text-gray-400" />
                    <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                      {t("match.goalSources")}
                    </h3>
                  </div>
                  <div className="space-y-2">
                    {sourceKeys.map(({ key, label }) => {
                      const hv = homeSrc[key];
                      const av = awaySrc[key];
                      const homePct = Math.round((hv / homeTotal) * 100);
                      const awayPct = Math.round((av / awayTotal) * 100);
                      const rowTotal = hv + av;
                      const homeBarPct = rowTotal > 0 ? Math.round((hv / rowTotal) * 100) : 0;
                      const awayBarPct = rowTotal > 0 ? 100 - homeBarPct : 0;
                      return (
                        <div key={key} className="mb-1 last:mb-0">
                          <div className="flex justify-between text-xs mb-0.5">
                            <span className="font-heading font-bold text-primary-400 tabular-nums">
                              {hv} <span className="font-normal text-gray-500">({homePct}%)</span>
                            </span>
                            <span className="text-gray-600 dark:text-gray-500 font-heading uppercase tracking-wider text-[10px]">
                              {label}
                            </span>
                            <span className="font-heading font-bold text-indigo-400 tabular-nums">
                              <span className="font-normal text-gray-500">({awayPct}%)</span> {av}
                            </span>
                          </div>
                          <div className="flex h-1 bg-gray-300 dark:bg-navy-700 rounded-full overflow-hidden">
                            <div className="h-full bg-primary-500" style={{ width: `${homeBarPct}%` }} />
                            <div className="h-full bg-indigo-500" style={{ width: `${awayBarPct}%` }} />
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              );
            })()}

            {/* Player roles per side */}
            {(["Home", "Away"] as const).map((side) => {
              const team = side === "Home" ? snapshot.home_team : snapshot.away_team;
              const teamColor = side === "Home" ? homeTeamColor : awayTeamColor;
              return (
                <div
                  key={side}
                  className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300"
                >
                  <div className="flex items-center gap-2 mb-1">
                    <div className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: teamColor }} />
                    <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                      {team.name}
                    </h3>
                  </div>
                  <p className="text-[10px] text-gray-500 dark:text-gray-500 font-heading uppercase tracking-wider mb-3">
                    {team.formation} · {t(`common.playStyles.${team.play_style}` as never, team.play_style)}
                  </p>
                  <div className="flex flex-col gap-0.5 max-h-52 overflow-auto">
                    {team.players.map((p) => (
                      <div key={p.id} className="flex items-center gap-2 text-xs py-0.5">
                        <span className="text-gray-500 dark:text-gray-500 text-[10px] font-heading uppercase w-6 shrink-0">
                          {p.position.charAt(0)}
                        </span>
                        <span className="text-gray-700 dark:text-gray-300 truncate flex-1">{p.name}</span>
                        <span className="text-gray-500 dark:text-gray-400 text-[10px] font-heading shrink-0">
                          {t(`tactics.playerRoles.${p.role ?? "Standard"}` as never, p.role ?? "Standard")}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              );
            })}
          </div>

        </div>
      </div>
    </div>
  );
}
