import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { GameStateData } from "../../store/gameStore";
import {
  MatchSnapshot,
  MatchEvent,
  FORMATIONS,
  PLAY_STYLES,
  getTeamTalkOptions,
  TeamTalkTone,
} from "./types";
import { getEventDisplay, getPlayerName } from "./helpers";
import { getTalkIcon } from "./TeamTalkIcons";
import { SubPanel } from "./SubPanel";
import { Badge, ThemeToggle } from "../ui";
import {
  Play,
  RefreshCw,
  Shield,
  Zap,
  Target,
  Crosshair,
  Flag,
  MessageCircle,
} from "lucide-react";

interface HalfTimeBreakProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  userSide: "Home" | "Away";
  isSpectator: boolean;
  importantEvents: MatchEvent[];
  onResume: () => void;
  onUpdateSnapshot: (snap: MatchSnapshot) => void;
}

const PLAY_STYLE_ICONS: Record<string, React.ReactNode> = {
  Balanced: <Target className="w-3.5 h-3.5" />,
  Attacking: <Zap className="w-3.5 h-3.5" />,
  Defensive: <Shield className="w-3.5 h-3.5" />,
  Possession: <RefreshCw className="w-3.5 h-3.5" />,
  Counter: <Crosshair className="w-3.5 h-3.5" />,
  HighPress: <Flag className="w-3.5 h-3.5" />,
};

export default function HalfTimeBreak({
  snapshot,
  gameState,
  userSide,
  isSpectator,
  importantEvents,
  onResume,
  onUpdateSnapshot,
}: HalfTimeBreakProps) {
  const { t } = useTranslation();
  const teamTalkOptions = getTeamTalkOptions(t);
  const [selectedTalk, setSelectedTalk] = useState<TeamTalkTone | null>(null);
  const [showSubPanel, setShowSubPanel] = useState(false);
  const [talkDelivered, setTalkDelivered] = useState(false);
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

  const userTeam =
    userSide === "Home" ? snapshot.home_team : snapshot.away_team;

  // First half key events
  const firstHalfEvents = importantEvents.filter((e) =>
    [
      "Goal",
      "PenaltyGoal",
      "YellowCard",
      "RedCard",
      "SecondYellow",
      "Injury",
      "PenaltyMiss",
    ].includes(e.event_type),
  );

  const handleFormationChange = async (formation: string) => {
    try {
      const snap = await invoke<MatchSnapshot>("apply_match_command", {
        command: { ChangeFormation: { side: userSide, formation } },
      });
      onUpdateSnapshot(snap);
    } catch (err) {
      console.error("Formation change failed:", err);
    }
  };

  const handlePlayStyleChange = async (playStyle: string) => {
    try {
      const snap = await invoke<MatchSnapshot>("apply_match_command", {
        command: { ChangePlayStyle: { side: userSide, play_style: playStyle } },
      });
      onUpdateSnapshot(snap);
    } catch (err) {
      console.error("Play style change failed:", err);
    }
  };

  const handleSubstitution = async (
    playerOffId: string,
    playerOnId: string,
  ) => {
    try {
      const snap = await invoke<MatchSnapshot>("apply_match_command", {
        command: {
          Substitute: {
            side: userSide,
            player_off_id: playerOffId,
            player_on_id: playerOnId,
          },
        },
      });
      onUpdateSnapshot(snap);
      setShowSubPanel(false);
    } catch (err) {
      console.error("Substitution failed:", err);
    }
  };

  const handleDeliverTalk = async () => {
    if (!selectedTalk) return;
    const userScore =
      userSide === "Home" ? snapshot.home_score : snapshot.away_score;
    const oppScore =
      userSide === "Home" ? snapshot.away_score : snapshot.home_score;
    const context =
      userScore > oppScore
        ? "winning"
        : userScore < oppScore
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
      {/* Header scoreboard */}
      <header className="bg-linear-to-r from-gray-200 via-white to-gray-200 dark:from-navy-800 dark:via-navy-900 dark:to-navy-800 border-b border-gray-200 dark:border-navy-700 px-4 py-4 transition-colors duration-300">
        <div className="max-w-5xl mx-auto relative">
          <ThemeToggle className="absolute right-0 top-0" />
          <div className="flex items-center justify-center gap-8">
            <div className="flex items-center gap-3">
              <div
                className="w-12 h-12 rounded-xl flex items-center justify-center font-heading font-bold"
                style={{
                  backgroundColor: homeTeamColor + "30",
                  borderColor: homeTeamColor,
                  borderWidth: 2,
                }}
              >
                {snapshot.home_team.name.substring(0, 3).toUpperCase()}
              </div>
              <p className="font-heading font-bold text-gray-800 dark:text-gray-200">
                {snapshot.home_team.name}
              </p>
            </div>

            <div className="flex items-center gap-4">
              <span className="text-5xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
                {snapshot.home_score}
              </span>
              <div className="text-center">
                <p className="text-xs font-heading uppercase tracking-widest text-accent-700 dark:text-accent-400">
                  {t("match.halfTime")}
                </p>
                <p className="text-lg font-heading font-bold text-gray-500 dark:text-gray-500">
                  {t("match.ht")}
                </p>
              </div>
              <span className="text-5xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
                {snapshot.away_score}
              </span>
            </div>

            <div className="flex items-center gap-3">
              <p className="font-heading font-bold text-gray-800 dark:text-gray-200">
                {snapshot.away_team.name}
              </p>
              <div
                className="w-12 h-12 rounded-xl flex items-center justify-center font-heading font-bold"
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

          {/* Possession bar */}
          <div className="max-w-md mx-auto mt-3">
            <div className="flex items-center gap-2 text-xs">
              <span className="font-heading font-bold text-primary-400 w-12 text-right">
                {snapshot.home_possession_pct.toFixed(0)}%
              </span>
              <div className="flex-1 h-1.5 bg-gray-300 dark:bg-navy-700 rounded-full overflow-hidden flex transition-colors duration-300">
                <div
                  className="h-full bg-primary-500 transition-all"
                  style={{ width: `${snapshot.home_possession_pct}%` }}
                />
                <div
                  className="h-full bg-indigo-500 transition-all"
                  style={{ width: `${snapshot.away_possession_pct}%` }}
                />
              </div>
              <span className="font-heading font-bold text-indigo-400 w-12">
                {snapshot.away_possession_pct.toFixed(0)}%
              </span>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex-1 overflow-auto">
        <div className="max-w-5xl mx-auto px-6 py-6 grid grid-cols-3 gap-6">
          {/* Left: First Half Summary */}
          <div className="flex flex-col gap-4">
            <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
              <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
                {t("match.firstHalfEvents")}
              </h3>
              {firstHalfEvents.length === 0 ? (
                <p className="text-xs text-gray-600 dark:text-gray-500">
                  {t("match.noMajorEvents")}
                </p>
              ) : (
                <div className="flex flex-col gap-2">
                  {firstHalfEvents.map((evt, i) => {
                    const display = getEventDisplay(evt);
                    return (
                      <div key={i} className="flex items-center gap-2 text-xs">
                        <span className="text-gray-600 dark:text-gray-500 tabular-nums w-6 text-right font-heading">
                          {evt.minute}'
                        </span>
                        <span>{display.icon}</span>
                        <span
                          className={`${display.color} font-medium truncate`}
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
          </div>

          {/* Center: Team Talk (user only) */}
          <div className="flex flex-col gap-4">
            {!isSpectator ? (
              <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
                <div className="flex items-center gap-2 mb-4">
                  <MessageCircle className="w-4 h-4 text-accent-400" />
                  <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                    {t("match.teamTalk")}
                  </h3>
                </div>

                {!talkDelivered ? (
                  <>
                    <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
                      {t("match.teamTalkPrompt")}
                    </p>
                    <div className="flex flex-col gap-2">
                      {teamTalkOptions.map((opt) => (
                        <button
                          key={opt.id}
                          onClick={() => setSelectedTalk(opt.id)}
                          className={`flex items-center gap-3 p-3 rounded-lg text-left transition-all ${
                            selectedTalk === opt.id
                              ? "bg-primary-500/20 ring-2 ring-primary-500/50"
                              : "bg-gray-100 hover:bg-gray-200 dark:bg-navy-700/50 dark:hover:bg-navy-700"
                          }`}
                        >
                          <span className="text-xl">
                            {getTalkIcon(opt.icon)}
                          </span>
                          <div>
                            <p
                                className={`text-sm font-heading font-bold ${
                                  selectedTalk === opt.id
                                    ? "text-primary-400"
                                    : "text-gray-800 dark:text-gray-200"
                                }`}
                              >
                                {opt.label}
                              </p>
                              <p className="text-[11px] text-gray-500 dark:text-gray-400">
                                {opt.description}
                              </p>
                          </div>
                        </button>
                      ))}
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
              <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 flex flex-col items-center justify-center py-8 transition-colors duration-300">
                <p className="text-xs font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1">
                  {t("match.spectatorMode")}
                </p>
                <p className="text-sm text-gray-500 dark:text-gray-400 text-center">
                  {t("match.spectatorHT")}
                </p>
              </div>
            )}
          </div>

          {/* Right: Tactical Changes (user only) */}
          <div className="flex flex-col gap-4">
            {!isSpectator && (
              <>
                {/* Formation */}
                <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
                  <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
                    {t("match.formation")}
                  </h3>
                  <div className="grid grid-cols-3 gap-1.5">
                    {FORMATIONS.map((f) => (
                      <button
                        key={f}
                        onClick={() => handleFormationChange(f)}
                        className={`py-2 rounded-lg text-xs font-heading font-bold transition-all ${
                          userTeam.formation === f
                            ? "bg-primary-500/20 text-primary-400 ring-1 ring-primary-500/50"
                            : "bg-gray-100 text-gray-600 hover:text-gray-900 dark:bg-navy-700 dark:text-gray-400 dark:hover:text-gray-300"
                        }`}
                      >
                        {f}
                      </button>
                    ))}
                  </div>
                </div>

                {/* Play Style */}
                <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
                  <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
                    {t("match.playStyle")}
                  </h3>
                  <div className="grid grid-cols-2 gap-1.5">
                    {PLAY_STYLES.map((style) => (
                      <button
                        key={style}
                        onClick={() => handlePlayStyleChange(style)}
                        className={`flex items-center gap-1.5 py-2 px-3 rounded-lg text-xs font-heading font-bold transition-all ${
                          userTeam.play_style === style
                            ? "bg-primary-500/20 text-primary-400 ring-1 ring-primary-500/50"
                            : "bg-gray-100 text-gray-600 hover:text-gray-900 dark:bg-navy-700 dark:text-gray-400 dark:hover:text-gray-300"
                        }`}
                      >
                        {PLAY_STYLE_ICONS[style]}
                        {t(`common.playStyles.${style}`)}
                      </button>
                    ))}
                  </div>
                </div>

                {/* Substitutions */}
                <div className="bg-white dark:bg-navy-800 rounded-xl border border-gray-200 dark:border-navy-700 shadow-sm p-4 transition-colors duration-300">
                  <div className="flex items-center justify-between mb-3">
                    <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400">
                      {t("match.substitutions")}
                    </h3>
                    <Badge variant="neutral" size="sm">
                      {userSide === "Home"
                        ? snapshot.home_subs_made
                        : snapshot.away_subs_made}
                      /{snapshot.max_subs}
                    </Badge>
                  </div>

                  <button
                    onClick={() => setShowSubPanel(true)}
                    className="w-full flex items-center justify-center gap-2 py-2.5 bg-gray-200 hover:bg-gray-300 dark:bg-navy-700 dark:hover:bg-navy-600 rounded-lg text-sm font-heading uppercase tracking-wider text-gray-700 dark:text-gray-300 transition-colors"
                  >
                    <RefreshCw className="w-4 h-4" />
                    {t("match.makeSubstitution")}
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      </div>

      {/* Footer */}
      <footer className="bg-white dark:bg-navy-800 border-t border-gray-200 dark:border-navy-700 px-6 py-4 transition-colors duration-300">
        <div className="max-w-5xl mx-auto flex justify-between items-center">
          <p className="text-xs text-gray-600 dark:text-gray-500 font-heading uppercase tracking-wider">
            {isSpectator
              ? t("match.waitingSecondHalf")
              : t("match.makeChanges")}
          </p>
          <button
            onClick={onResume}
            className="flex items-center gap-3 px-8 py-3 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 rounded-xl font-heading font-bold uppercase tracking-wider text-sm text-white shadow-lg shadow-primary-500/20 transition-all"
          >
            <Play className="w-4 h-4" />
            {t("match.resumeMatch")}
          </button>
        </div>
      </footer>

      {/* Substitution Modal — reuses the full SubPanel from MatchLive */}
      {showSubPanel && (
        <SubPanel
          snapshot={snapshot}
          side={userSide}
          onSubstitute={handleSubstitution}
          onClose={() => setShowSubPanel(false)}
        />
      )}
    </div>
  );
}
