import { useEffect, useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { GameStateData } from "../../store/gameStore";
import { MatchSnapshot, MatchEvent, MinuteResult, SimSpeed, SPEED_MS } from "./types";
import { getEventDisplay, getPlayerName, phaseLabel } from "./helpers";
import { Badge } from "../ui";
import { useSettingsStore } from "../../store/settingsStore";
import { EventFeed, MatchStats, Lineups } from "./MatchPanels";
import MatchScreenLayout from "./MatchScreenLayout";
import { SubPanel } from "./SubPanel";
import {
  Play, Pause, FastForward, SkipForward,
  Clock, Users, BarChart3, MessageSquare, RefreshCw,
  ChevronRight, Zap, Shield, Crosshair,
  Target, Flag
} from "lucide-react";

type ActivePanel = "events" | "stats" | "lineups";

interface MatchLiveProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  userSide: "Home" | "Away" | null;
  isSpectator: boolean;
  importantEvents: MatchEvent[];
  onSnapshotUpdate: (snap: MatchSnapshot) => void;
  onImportantEvent: (evt: MatchEvent) => void;
  onHalfTime: () => void;
  onFullTime: () => void;
}

export default function MatchLive({
  snapshot, gameState, userSide, isSpectator,
  importantEvents, onSnapshotUpdate, onImportantEvent,
  onHalfTime, onFullTime,
}: MatchLiveProps) {
  const { t } = useTranslation();
  const { settings } = useSettingsStore();
  const initialSpeed: SimSpeed = (settings.match_speed === "slow" || settings.match_speed === "fast") ? settings.match_speed : "normal";
  const [speed, setSpeed] = useState<SimSpeed>(initialSpeed);
  const [activePanel, setActivePanel] = useState<ActivePanel>("events");
  const [isRunning, setIsRunning] = useState(true);
  const [showSubPanel, setShowSubPanel] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const eventFeedRef = useRef<HTMLDivElement>(null);
  // Track phases we've already signaled to avoid double-firing
  const signaledRef = useRef<Set<string>>(new Set());

  const homeTeamColor = gameState.teams.find(t => t.id === snapshot.home_team.id)?.colors?.primary || "#10b981";
  const awayTeamColor = gameState.teams.find(t => t.id === snapshot.away_team.id)?.colors?.primary || "#6366f1";

  const isFinished = snapshot.phase === "Finished";

  // Step the match forward one minute
  const stepMatch = useCallback(async () => {
    try {
      const results = await invoke<MinuteResult[]>("step_live_match", { minutes: 1 });
      if (results.length > 0) {
        const lastResult = results[results.length - 1];

        // Collect important events
        for (const r of results) {
          for (const evt of r.events) {
            const display = getEventDisplay(evt);
            if (display.important) {
              onImportantEvent(evt);
            }
          }
        }

        // Fetch full snapshot
        const snap = await invoke<MatchSnapshot>("get_match_snapshot");
        onSnapshotUpdate(snap);

        // Check for phase transitions that should pause
        const phase = lastResult.phase;
        if (phase === "HalfTime" && !signaledRef.current.has("HalfTime")) {
          signaledRef.current.add("HalfTime");
          setIsRunning(false);
          setSpeed("paused");
          // Small delay so the last event renders before transitioning
          setTimeout(() => onHalfTime(), 600);
          return;
        }

        if (phase === "ExtraTimeHalfTime" && !signaledRef.current.has("ExtraTimeHalfTime")) {
          signaledRef.current.add("ExtraTimeHalfTime");
          setIsRunning(false);
          setSpeed("paused");
          setTimeout(() => onHalfTime(), 600);
          return;
        }

        if (lastResult.is_finished && !signaledRef.current.has("Finished")) {
          signaledRef.current.add("Finished");
          setIsRunning(false);
          setSpeed("paused");
          setTimeout(() => onFullTime(), 600);
          return;
        }
      }
    } catch (err) {
      console.error("Failed to step match:", err);
      setIsRunning(false);
    }
  }, [onSnapshotUpdate, onImportantEvent, onHalfTime, onFullTime]);

  // Auto-step timer
  useEffect(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }

    if (isRunning && speed !== "paused" && !isFinished && !showSubPanel) {
      timerRef.current = setTimeout(async () => {
        await stepMatch();
      }, SPEED_MS[speed]);
    }

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [isRunning, speed, snapshot.current_minute, snapshot.phase, stepMatch, isFinished, showSubPanel]);

  // Auto-scroll event feed
  useEffect(() => {
    if (eventFeedRef.current) {
      eventFeedRef.current.scrollTop = eventFeedRef.current.scrollHeight;
    }
  }, [importantEvents.length]);

  // Apply substitution
  const handleSubstitution = async (playerOffId: string, playerOnId: string) => {
    if (!userSide || isSpectator) return;
    try {
      const snap = await invoke<MatchSnapshot>("apply_match_command", {
        command: { Substitute: { side: userSide, player_off_id: playerOffId, player_on_id: playerOnId } }
      });
      onSnapshotUpdate(snap);
      setShowSubPanel(false);
    } catch (err) {
      console.error("Substitution failed:", err);
    }
  };

  const handleFormationChange = async (formation: string) => {
    if (!userSide || isSpectator) return;
    try {
      const snap = await invoke<MatchSnapshot>("apply_match_command", {
        command: { ChangeFormation: { side: userSide, formation } }
      });
      onSnapshotUpdate(snap);
    } catch (err) {
      console.error("Formation change failed:", err);
    }
  };

  const handlePlayStyleChange = async (playStyle: string) => {
    if (!userSide || isSpectator) return;
    try {
      const snap = await invoke<MatchSnapshot>("apply_match_command", {
        command: { ChangePlayStyle: { side: userSide, play_style: playStyle } }
      });
      onSnapshotUpdate(snap);
    } catch (err) {
      console.error("Play style change failed:", err);
    }
  };

  return (
    <MatchScreenLayout
      headerClassName="bg-linear-to-r from-gray-200 via-white to-gray-200 dark:from-navy-800 dark:via-navy-900 dark:to-navy-800"
      headerContentClassName="max-w-7xl py-3"
      contentClassName="overflow-hidden"
      header={
        <>
          <div className="flex items-center justify-between gap-4">
            {/* Live indicator */}
            <div className="flex items-center gap-2">
              {isRunning && (
                <span className="relative flex h-2.5 w-2.5">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-red-500" />
                </span>
              )}
              <span className="text-xs font-heading uppercase tracking-widest text-gray-500 dark:text-gray-400">
                {isRunning ? t('match.live') : t('match.paused')}
              </span>
            </div>

            {/* Scoreboard */}
            <div className="flex items-center gap-6">
              <div className="flex items-center gap-3">
                <div className="text-right">
                  <p className="font-heading font-bold text-sm uppercase tracking-wider text-gray-800 dark:text-gray-200">
                    {snapshot.home_team.name}
                  </p>
                  <p className="text-xs text-gray-500 dark:text-gray-400">{snapshot.home_team.formation}</p>
                </div>
                <div
                  className="w-10 h-10 rounded-lg flex items-center justify-center font-heading font-bold text-sm"
                  style={{ backgroundColor: homeTeamColor + "30", borderColor: homeTeamColor, borderWidth: 2 }}
                >
                  {snapshot.home_team.name.substring(0, 3).toUpperCase()}
                </div>
              </div>

              <div className="flex items-center gap-3">
                <span className="text-4xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">{snapshot.home_score}</span>
                <div className="flex flex-col items-center">
                  <span className="text-xs font-heading uppercase tracking-widest text-accent-700 dark:text-accent-400">
                    {phaseLabel(snapshot.phase, t)}
                  </span>
                  <span className="text-2xl font-heading font-bold text-gray-500 dark:text-gray-400">{snapshot.current_minute}'</span>
                </div>
                <span className="text-4xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">{snapshot.away_score}</span>
              </div>

              <div className="flex items-center gap-3">
                <div
                  className="w-10 h-10 rounded-lg flex items-center justify-center font-heading font-bold text-sm"
                  style={{ backgroundColor: awayTeamColor + "30", borderColor: awayTeamColor, borderWidth: 2 }}
                >
                  {snapshot.away_team.name.substring(0, 3).toUpperCase()}
                </div>
                <div className="text-left">
                  <p className="font-heading font-bold text-sm uppercase tracking-wider text-gray-800 dark:text-gray-200">
                    {snapshot.away_team.name}
                  </p>
                  <p className="text-xs text-gray-500 dark:text-gray-400">{snapshot.away_team.formation}</p>
                </div>
              </div>
            </div>

            <div className="flex items-center gap-2">
              <Clock className="w-4 h-4 text-gray-500 dark:text-gray-400" />
              <span className="text-sm font-heading text-gray-500 dark:text-gray-400 tabular-nums w-8">{snapshot.current_minute}'</span>
            </div>
          </div>

          {/* Possession bar */}
          <div className="mt-2">
            <div className="flex items-center gap-2 text-xs">
              <span className="font-heading font-bold text-primary-400 w-12 text-right">
                {snapshot.home_possession_pct.toFixed(0)}%
              </span>
              <div className="flex-1 h-1.5 bg-gray-300 dark:bg-navy-700 rounded-full overflow-hidden flex transition-colors duration-300">
                <div className="h-full bg-primary-500 transition-all duration-500" style={{ width: `${snapshot.home_possession_pct}%` }} />
                <div className="h-full bg-indigo-500 transition-all duration-500" style={{ width: `${snapshot.away_possession_pct}%` }} />
              </div>
              <span className="font-heading font-bold text-indigo-400 w-12">
                {snapshot.away_possession_pct.toFixed(0)}%
              </span>
            </div>
          </div>
        </>
      }
    >

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Panel: Event Feed + Stats */}
        <div className="flex-1 flex flex-col">
          <div className="flex bg-white dark:bg-navy-800 border-b border-gray-200 dark:border-navy-700 transition-colors duration-300">
            {([
              { id: "events" as ActivePanel, label: t('match.events'), icon: <MessageSquare className="w-4 h-4" /> },
              { id: "stats" as ActivePanel, label: t('match.stats'), icon: <BarChart3 className="w-4 h-4" /> },
              { id: "lineups" as ActivePanel, label: t('match.lineups'), icon: <Users className="w-4 h-4" /> },
            ]).map(tab => (
              <button
                key={tab.id}
                onClick={() => setActivePanel(tab.id)}
                className={`flex items-center gap-2 px-5 py-3 font-heading font-bold text-xs uppercase tracking-wider transition-colors border-b-2 ${activePanel === tab.id
                    ? "text-primary-500 dark:text-primary-400 border-primary-500 bg-primary-50 dark:bg-navy-700/50"
                    : "text-gray-500 dark:text-gray-400 border-transparent hover:text-gray-700 dark:hover:text-gray-300"
                  }`}
              >
                {tab.icon}
                {tab.label}
              </button>
            ))}
          </div>

          <div className="flex-1 overflow-auto p-4">
            {activePanel === "events" && <EventFeed events={importantEvents} snapshot={snapshot} feedRef={eventFeedRef} />}
            {activePanel === "stats" && <MatchStats snapshot={snapshot} />}
            {activePanel === "lineups" && <Lineups snapshot={snapshot} />}
          </div>
        </div>

        {/* Right Panel: Controls */}
        <aside className="w-72 bg-white dark:bg-navy-800 border-l border-gray-200 dark:border-navy-700 flex flex-col transition-colors duration-300">
          {/* Speed Controls */}
          <div className="p-4 border-b border-gray-200 dark:border-navy-700">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">{t('match.simSpeed')}</h3>
            <div className="flex gap-1">
              {([
                { id: "paused" as SimSpeed, icon: <Pause className="w-4 h-4" />, label: t('match.pause') },
                { id: "slow" as SimSpeed, icon: <Play className="w-4 h-4" />, label: t('match.slow') },
                { id: "normal" as SimSpeed, icon: <Play className="w-4 h-4" />, label: t('match.normal') },
                { id: "fast" as SimSpeed, icon: <FastForward className="w-4 h-4" />, label: t('match.fast') },
                { id: "instant" as SimSpeed, icon: <SkipForward className="w-4 h-4" />, label: t('match.max') },
              ]).map(s => (
                <button
                  key={s.id}
                  onClick={() => { setSpeed(s.id); setIsRunning(s.id !== "paused"); }}
                  className={`flex-1 flex flex-col items-center gap-1 py-2 rounded-lg text-xs font-heading uppercase tracking-wider transition-all ${speed === s.id ? "bg-primary-500/20 text-primary-500 dark:text-primary-400 ring-1 ring-primary-500/50" : "text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-navy-700"
                    }`}
                >
                  {s.icon}
                  <span className="text-[10px]">{s.label}</span>
                </button>
              ))}
            </div>
            {speed === "paused" && (
              <button
                onClick={stepMatch}
                className="w-full mt-2 flex items-center justify-center gap-2 py-2 bg-gray-200 hover:bg-gray-300 dark:bg-navy-700 dark:hover:bg-navy-600 rounded-lg text-sm font-heading uppercase tracking-wider text-gray-700 dark:text-gray-300 transition-colors"
              >
                <ChevronRight className="w-4 h-4" />
                {t('match.step1Min')}
              </button>
            )}
          </div>

          {/* User Controls */}
          {!isSpectator && userSide && (
            <div className="p-4 border-b border-gray-200 dark:border-navy-700 flex flex-col gap-2">
              <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-1">{t('match.teamControls')}</h3>
              <button
                onClick={() => setShowSubPanel(!showSubPanel)}
                className="flex items-center gap-2 px-3 py-2 bg-gray-200 hover:bg-gray-300 dark:bg-navy-700 dark:hover:bg-navy-600 rounded-lg text-sm font-heading uppercase tracking-wider text-gray-700 dark:text-gray-300 transition-colors"
              >
                <RefreshCw className="w-4 h-4" />
                {t('match.subs')} ({userSide === "Home" ? snapshot.home_subs_made : snapshot.away_subs_made}/{snapshot.max_subs})
              </button>
              <div>
                <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1">{t('match.formation')}</p>
                <div className="flex flex-wrap gap-1">
                  {["4-4-2", "4-3-3", "3-5-2", "4-5-1", "4-2-3-1", "3-4-3"].map(f => {
                    const cur = userSide === "Home" ? snapshot.home_team.formation : snapshot.away_team.formation;
                    return (
                      <button key={f} onClick={() => handleFormationChange(f)}
                        className={`px-2 py-1 rounded text-xs font-heading transition-colors ${cur === f ? "bg-primary-500/20 text-primary-500 dark:text-primary-400 ring-1 ring-primary-500/50" : "bg-gray-100 text-gray-600 hover:text-gray-900 dark:bg-navy-700 dark:text-gray-400 dark:hover:text-gray-300"}`}
                      >{f}</button>
                    );
                  })}
                </div>
              </div>
              <div>
                <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1">{t('match.playStyle')}</p>
                <div className="flex flex-wrap gap-1">
                  {[
                    { id: "Balanced", icon: <Target className="w-3 h-3" /> },
                    { id: "Attacking", icon: <Zap className="w-3 h-3" /> },
                    { id: "Defensive", icon: <Shield className="w-3 h-3" /> },
                    { id: "Possession", icon: <RefreshCw className="w-3 h-3" /> },
                    { id: "Counter", icon: <Crosshair className="w-3 h-3" /> },
                    { id: "HighPress", icon: <Flag className="w-3 h-3" /> },
                  ].map(s => {
                    const cur = userSide === "Home" ? snapshot.home_team.play_style : snapshot.away_team.play_style;
                    return (
                      <button key={s.id} onClick={() => handlePlayStyleChange(s.id)}
                        className={`flex items-center gap-1 px-2 py-1 rounded text-xs font-heading transition-colors ${cur === s.id ? "bg-primary-500/20 text-primary-500 dark:text-primary-400 ring-1 ring-primary-500/50" : "bg-gray-100 text-gray-600 hover:text-gray-900 dark:bg-navy-700 dark:text-gray-400 dark:hover:text-gray-300"}`}
                      >{s.icon}{t(`common.playStyles.${s.id}`, s.id)}</button>
                    );
                  })}
                </div>
              </div>
            </div>
          )}

          {/* Key Events sidebar */}
          <div className="p-4 flex-1 overflow-auto">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">{t('match.keyEvents')}</h3>
            <div className="flex flex-col gap-1.5">
              {importantEvents
                .filter(e => ["Goal", "PenaltyGoal", "YellowCard", "RedCard", "SecondYellow", "Substitution", "PenaltyMiss", "Injury"].includes(e.event_type))
                .slice(-12).reverse()
                .map((evt, i) => {
                  const display = getEventDisplay(evt);
                  return (
                    <div key={i} className="flex items-center gap-2 text-xs">
                      <span className="text-gray-600 dark:text-gray-500 tabular-nums w-6 text-right font-heading">{evt.minute}'</span>
                      <span>{display.icon}</span>
                      <span className={`${display.color} font-medium truncate`}>{getPlayerName(snapshot, evt.player_id)}</span>
                      <Badge variant={evt.side === "Home" ? "primary" : "accent"} size="sm">
                        {evt.side === "Home" ? snapshot.home_team.name.substring(0, 3) : snapshot.away_team.name.substring(0, 3)}
                      </Badge>
                    </div>
                  );
                })}
              {importantEvents.length === 0 && <p className="text-gray-600 dark:text-gray-500 text-xs">{t('match.noEventsYet')}</p>}
            </div>
          </div>
        </aside>
      </div>

      {/* Substitution Modal */}
      {showSubPanel && userSide && (
        <SubPanel snapshot={snapshot} side={userSide} onSubstitute={handleSubstitution} onClose={() => setShowSubPanel(false)} />
      )}
    </MatchScreenLayout>
  );
}

