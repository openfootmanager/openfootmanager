import { useEffect, useState, useRef, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import type { GameStateData } from "../../store/gameStore";
import {
  type MatchSnapshot,
  type RoleChangeOutcome,
  type SubstitutionOutcome,
  type SubstitutionSwap,
  type MatchEvent,
  type MinuteResult,
  SPEED_MS,
  MINUTES_PER_TICK,
  FORMATIONS,
} from "./types";
import MatchEventSideBadge from "./MatchEventSideBadge";
import { getEventDisplay, getPlayerName, makeTeamFallback, phaseLabel } from "./helpers";
import { TeamLogo } from "../ui";
import { useSettingsStore } from "../../store/settingsStore";
import { EventFeed, MatchStats, Lineups } from "./MatchPanels";
import MatchScreenLayout from "./MatchScreenLayout";
import { SubPanel } from "./SubPanel";
import { applyRoleChange } from "./applyRoleChange";
import { sendMatchCommand } from "./matchCommands";
import { applySubstitutions } from "./applySubstitutions";
import { type MatchPace, MatchSpeedControl } from "./MatchSpeedControl";
import { useMatchPace } from "./useMatchPace";
import {
  Clock,
  Users,
  BarChart3,
  MessageSquare,
  RefreshCw,
  Zap,
  Shield,
  Crosshair,
  Target,
  Flag,
} from "lucide-react";
import { invokeForCareer } from "../../services/careerInvoke";

type ActivePanel = "events" | "stats" | "lineups";

interface MatchLiveProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  userSide: "Home" | "Away" | null;
  isSpectator: boolean;
  importantEvents: MatchEvent[];
  preferredSpeed?: "slow" | "normal" | "fast";
  onPreferredSpeedChange?: (speed: "slow" | "normal" | "fast") => void;
  onSnapshotUpdate: (snap: MatchSnapshot) => void;
  onImportantEvent: (evt: MatchEvent) => void;
  onHalfTime: (phase: "HalfTime" | "ExtraTimeHalfTime") => void;
  onFullTime: () => void;
  onPenaltyShootout?: () => void;
}

export default function MatchLive({
  snapshot,
  gameState,
  userSide,
  isSpectator,
  importantEvents,
  preferredSpeed,
  onPreferredSpeedChange,
  onSnapshotUpdate,
  onImportantEvent,
  onHalfTime,
  onFullTime,
  onPenaltyShootout,
}: MatchLiveProps) {
  const { t } = useTranslation();
  const { settings } = useSettingsStore();
  const initialPace: MatchPace =
    preferredSpeed ??
    (settings.match_speed === "slow" || settings.match_speed === "fast"
      ? settings.match_speed
      : "normal");
  const [activePanel, setActivePanel] = useState<ActivePanel>("events");
  const { pace, isRunning, pause, togglePause, selectPace } = useMatchPace(
    initialPace,
    onPreferredSpeedChange,
  );
  const [showSubPanel, setShowSubPanel] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const eventFeedRef = useRef<HTMLDivElement>(null);
  // Track phases we've already signaled to avoid double-firing
  const signaledRef = useRef<Set<string>>(new Set());
  // Set when a step reaches half time, extra-time half time, the shootout or the end: the screen
  // hands over 600ms later, and nothing may step the match on in between (a Space or a Step there
  // started the second half before the break). stepMatch checks the ref, the one guard every step
  // goes through; the state switches the speed control's Space off. MatchLive remounts per stage,
  // so it never needs clearing.
  const transitioningRef = useRef(false);
  const [transitioning, setTransitioning] = useState(false);
  const beginTransition = useCallback(() => {
    transitioningRef.current = true;
    setTransitioning(true);
    pause();
  }, [pause]);

  const homeFullTeam = gameState.teams.find((t) => t.id === snapshot.home_team.id);
  const awayFullTeam = gameState.teams.find((t) => t.id === snapshot.away_team.id);
  const homeTeam = homeFullTeam ?? makeTeamFallback(snapshot.home_team.name);
  const awayTeam = awayFullTeam ?? makeTeamFallback(snapshot.away_team.name);
  const homeTeamColor = homeFullTeam?.colors?.primary || "#10b981";
  const awayTeamColor = awayFullTeam?.colors?.primary || "#6366f1";

  const playerJerseyMap = useMemo(() => {
    const m = new Map<string, number>();
    for (const p of gameState.players) {
      if (p.jersey_number != null) m.set(p.id, p.jersey_number);
    }
    return m;
  }, [gameState.players]);

  const isFinished = snapshot.phase === "Finished";
  // The engine refuses substitutions from the final whistle of extra time on (#1043), so the control
  // closes with it rather than opening a panel whose every change comes back refused.
  const subsClosed = snapshot.phase === "ExtraTimeEnd" || snapshot.phase === "PenaltyShootout";

  // Reads only `lastResult` for phase transitions, which is sound because step_many stops on
  // entering any phase that needs the manager — so a half time, shootout or finish is always the
  // last entry of a batch, never buried in the middle. See `phase_needs_manager` in
  // ofm_core/live_match_manager.rs; MINUTES_PER_TICK on this side is what makes batches possible.
  const stepMatch = useCallback(
    async (minutes: number) => {
      if (transitioningRef.current) return;
      try {
        const results = await invokeForCareer<MinuteResult[]>("step_live_match", { minutes });
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
            beginTransition();
            // Small delay so the last event renders before transitioning
            setTimeout(() => onHalfTime("HalfTime"), 600);
            return;
          }

          if (phase === "ExtraTimeHalfTime" && !signaledRef.current.has("ExtraTimeHalfTime")) {
            signaledRef.current.add("ExtraTimeHalfTime");
            beginTransition();
            setTimeout(() => onHalfTime("ExtraTimeHalfTime"), 600);
            return;
          }

          if (phase === "PenaltyShootout" && !signaledRef.current.has("PenaltyShootout")) {
            signaledRef.current.add("PenaltyShootout");
            beginTransition();
            setTimeout(() => onPenaltyShootout?.(), 600);
            return;
          }

          if (lastResult.is_finished && !signaledRef.current.has("Finished")) {
            signaledRef.current.add("Finished");
            beginTransition();
            setTimeout(() => onFullTime(), 600);
            return;
          }
        }
      } catch (err) {
        console.error("Failed to step match:", err);
        pause();
      }
    },
    [
      pause,
      beginTransition,
      onSnapshotUpdate,
      onImportantEvent,
      onHalfTime,
      onFullTime,
      onPenaltyShootout,
    ],
  );

  // Auto-step timer
  useEffect(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }

    if (isRunning && !isFinished && !showSubPanel) {
      timerRef.current = setTimeout(async () => {
        await stepMatch(MINUTES_PER_TICK[pace]);
      }, SPEED_MS[pace]);
    }

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [
    isRunning,
    pace,
    snapshot.current_minute,
    snapshot.phase,
    stepMatch,
    isFinished,
    showSubPanel,
  ]);

  // Auto-scroll event feed
  useEffect(() => {
    if (eventFeedRef.current) {
      eventFeedRef.current.scrollTop = eventFeedRef.current.scrollHeight;
    }
  }, [importantEvents.length]);

  // Apply the queued substitutions; the screen closes only when every one went through.
  const handleSubstitutions = async (swaps: SubstitutionSwap[]): Promise<SubstitutionOutcome> => {
    if (!userSide || isSpectator) {
      return { applied: true };
    }
    const outcome = await applySubstitutions(userSide, swaps, onSnapshotUpdate);
    if (outcome.applied) {
      setShowSubPanel(false);
    }
    return outcome;
  };

  const handleFormationChange = async (formation: string) => {
    if (!userSide || isSpectator) return;
    try {
      const snap = await sendMatchCommand({ ChangeFormation: { side: userSide, formation } });
      onSnapshotUpdate(snap);
    } catch (err) {
      console.error("Formation change failed:", err);
    }
  };

  const handleRoleChange = async (playerId: string, role: string): Promise<RoleChangeOutcome> => {
    if (!userSide || isSpectator) return { applied: true };
    return applyRoleChange(userSide, playerId, role, onSnapshotUpdate);
  };

  const handlePlayStyleChange = async (playStyle: string) => {
    if (!userSide || isSpectator) return;
    try {
      const snap = await sendMatchCommand({
        ChangePlayStyle: { side: userSide, play_style: playStyle },
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
                {isRunning ? t("match.live") : t("match.paused")}
              </span>
            </div>

            {/* Scoreboard */}
            <div className="flex items-center gap-6">
              <div className="flex items-center gap-3">
                <div className="text-right">
                  <p className="font-heading font-bold text-sm uppercase tracking-wider text-gray-800 dark:text-gray-200">
                    {snapshot.home_team.name}
                  </p>
                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    {snapshot.home_team.formation}
                  </p>
                </div>
                <TeamLogo
                  team={homeTeam}
                  className="w-10 h-10 rounded-lg flex items-center justify-center font-heading font-bold text-sm overflow-hidden"
                  imageClassName="h-8 w-8 object-contain drop-shadow"
                  style={{
                    backgroundColor: `${homeTeamColor}30`,
                    borderColor: homeTeamColor,
                    borderWidth: 2,
                  }}
                />
              </div>

              <div className="flex items-center gap-3">
                <span className="text-4xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
                  {snapshot.home_score}
                </span>
                <div className="flex flex-col items-center">
                  <span className="text-xs font-heading uppercase tracking-widest text-accent-700 dark:text-accent-400">
                    {phaseLabel(snapshot.phase, t)}
                  </span>
                  <span className="text-2xl font-heading font-bold text-gray-500 dark:text-gray-400">
                    {snapshot.current_minute}'
                  </span>
                </div>
                <span className="text-4xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
                  {snapshot.away_score}
                </span>
              </div>

              <div className="flex items-center gap-3">
                <TeamLogo
                  team={awayTeam}
                  className="w-10 h-10 rounded-lg flex items-center justify-center font-heading font-bold text-sm overflow-hidden"
                  imageClassName="h-8 w-8 object-contain drop-shadow"
                  style={{
                    backgroundColor: `${awayTeamColor}30`,
                    borderColor: awayTeamColor,
                    borderWidth: 2,
                  }}
                />
                <div className="text-left">
                  <p className="font-heading font-bold text-sm uppercase tracking-wider text-gray-800 dark:text-gray-200">
                    {snapshot.away_team.name}
                  </p>
                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    {snapshot.away_team.formation}
                  </p>
                </div>
              </div>
            </div>

            <div className="flex items-center gap-2">
              <Clock className="w-4 h-4 text-gray-500 dark:text-gray-400" />
              <span className="text-sm font-heading text-gray-500 dark:text-gray-400 tabular-nums w-8">
                {snapshot.current_minute}'
              </span>
            </div>
          </div>

          {/* Possession bar */}
          <div className="mt-2">
            <div className="flex items-center gap-2 text-xs">
              <span className="font-heading font-bold text-primary-400 w-12 text-right">
                {snapshot.home_possession_pct.toFixed(0)}%
              </span>
              <div className="flex-1 h-1.5 bg-gray-300 dark:bg-navy-700 rounded-full overflow-hidden flex transition-colors duration-300">
                <div
                  className="h-full bg-primary-500 transition-all duration-500"
                  style={{ width: `${snapshot.home_possession_pct}%` }}
                />
                <div
                  className="h-full bg-indigo-500 transition-all duration-500"
                  style={{ width: `${snapshot.away_possession_pct}%` }}
                />
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
            {[
              {
                id: "events" as ActivePanel,
                label: t("match.events"),
                icon: <MessageSquare className="w-4 h-4" />,
              },
              {
                id: "stats" as ActivePanel,
                label: t("match.stats"),
                icon: <BarChart3 className="w-4 h-4" />,
              },
              {
                id: "lineups" as ActivePanel,
                label: t("match.lineups"),
                icon: <Users className="w-4 h-4" />,
              },
            ].map((tab) => (
              <button
                type="button"
                key={tab.id}
                onClick={() => setActivePanel(tab.id)}
                className={`flex items-center gap-2 px-5 py-3 font-heading font-bold text-xs uppercase tracking-wider transition-colors border-b-2 ${
                  activePanel === tab.id
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
            {activePanel === "events" && (
              <EventFeed
                events={importantEvents}
                snapshot={snapshot}
                feedRef={eventFeedRef}
                playerJerseyMap={playerJerseyMap}
              />
            )}
            {activePanel === "stats" && <MatchStats snapshot={snapshot} />}
            {activePanel === "lineups" && <Lineups snapshot={snapshot} />}
          </div>
        </div>

        {/* Right Panel: Controls */}
        <aside className="w-72 bg-panel border-l border-line flex flex-col transition-colors duration-300">
          {/* Speed Controls */}
          <div className="p-4 border-b border-line">
            <MatchSpeedControl
              paused={!isRunning}
              pace={pace}
              onTogglePause={togglePause}
              onSelectPace={selectPace}
              onStep={() => stepMatch(1)}
              // No Space while the screen hands over or a dialog is open over the match.
              enabled={!transitioning && !isFinished && !showSubPanel}
            />
          </div>

          {/* User Controls */}
          {!isSpectator && userSide && (
            <div className="p-4 border-b border-gray-200 dark:border-navy-700 flex flex-col gap-2">
              <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-1">
                {t("match.teamControls")}
              </h3>
              <button
                type="button"
                onClick={() => setShowSubPanel(!showSubPanel)}
                disabled={subsClosed}
                aria-describedby={subsClosed ? "subs-closed-reason" : undefined}
                className="flex items-center gap-2 px-3 py-2 bg-gray-200 hover:bg-gray-300 dark:bg-navy-700 dark:hover:bg-navy-600 rounded-lg text-sm font-heading uppercase tracking-wider text-gray-700 dark:text-gray-300 transition-colors disabled:cursor-not-allowed disabled:opacity-50"
              >
                <RefreshCw className="w-4 h-4" />
                {t("match.subs")} (
                {userSide === "Home" ? snapshot.home_subs_made : snapshot.away_subs_made}/
                {snapshot.max_subs})
              </button>
              {subsClosed && (
                <p id="subs-closed-reason" className="text-caption text-fg-muted">
                  {t("match.subsClosed")}
                </p>
              )}
              <div>
                <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1">
                  {t("match.formation")}
                </p>
                <div className="flex flex-wrap gap-1">
                  {FORMATIONS.map((f) => {
                    const cur =
                      userSide === "Home"
                        ? snapshot.home_team.formation
                        : snapshot.away_team.formation;
                    return (
                      <button
                        type="button"
                        key={f}
                        onClick={() => handleFormationChange(f)}
                        className={`px-2 py-1 rounded text-xs font-heading transition-colors ${cur === f ? "bg-primary-500/20 text-primary-500 dark:text-primary-400 ring-1 ring-primary-500/50" : "bg-gray-100 text-gray-600 hover:text-gray-900 dark:bg-navy-700 dark:text-gray-400 dark:hover:text-gray-300"}`}
                      >
                        {f}
                      </button>
                    );
                  })}
                </div>
              </div>
              <div>
                <p className="text-[10px] font-heading uppercase tracking-widest text-gray-600 dark:text-gray-500 mb-1">
                  {t("match.playStyle")}
                </p>
                <div className="flex flex-wrap gap-1">
                  {[
                    { id: "Balanced", icon: <Target className="w-3 h-3" /> },
                    { id: "Attacking", icon: <Zap className="w-3 h-3" /> },
                    { id: "Defensive", icon: <Shield className="w-3 h-3" /> },
                    { id: "Possession", icon: <RefreshCw className="w-3 h-3" /> },
                    { id: "Counter", icon: <Crosshair className="w-3 h-3" /> },
                    { id: "HighPress", icon: <Flag className="w-3 h-3" /> },
                  ].map((s) => {
                    const cur =
                      userSide === "Home"
                        ? snapshot.home_team.play_style
                        : snapshot.away_team.play_style;
                    return (
                      <button
                        type="button"
                        key={s.id}
                        onClick={() => handlePlayStyleChange(s.id)}
                        className={`flex items-center gap-1 px-2 py-1 rounded text-xs font-heading transition-colors ${cur === s.id ? "bg-primary-500/20 text-primary-500 dark:text-primary-400 ring-1 ring-primary-500/50" : "bg-gray-100 text-gray-600 hover:text-gray-900 dark:bg-navy-700 dark:text-gray-400 dark:hover:text-gray-300"}`}
                      >
                        {s.icon}
                        {t(`common.playStyles.${s.id}`, s.id)}
                      </button>
                    );
                  })}
                </div>
              </div>
            </div>
          )}

          {/* Key Events sidebar */}
          <div className="p-4 flex-1 overflow-auto">
            <h3 className="text-xs font-heading font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-3">
              {t("match.keyEvents")}
            </h3>
            <div className="flex flex-col gap-1.5">
              {importantEvents
                .filter((e) =>
                  [
                    "Goal",
                    "PenaltyGoal",
                    "YellowCard",
                    "RedCard",
                    "SecondYellow",
                    "Substitution",
                    "PenaltyMiss",
                    "Injury",
                  ].includes(e.event_type),
                )
                .slice(-12)
                .reverse()
                .map((evt, i) => {
                  const display = getEventDisplay(evt);
                  return (
                    <div key={i} className="flex items-center gap-2 text-xs">
                      <span className="text-gray-600 dark:text-gray-500 tabular-nums w-6 text-right font-heading">
                        {evt.minute}'
                      </span>
                      <span>{display.icon}</span>
                      <span className={`${display.color} font-medium truncate`}>
                        {getPlayerName(snapshot, evt.player_id)}
                      </span>
                      <MatchEventSideBadge
                        side={evt.side}
                        homeTeam={homeTeam}
                        awayTeam={awayTeam}
                      />
                    </div>
                  );
                })}
              {importantEvents.length === 0 && (
                <p className="text-gray-600 dark:text-gray-500 text-xs">{t("match.noEventsYet")}</p>
              )}
            </div>
          </div>
        </aside>
      </div>

      {/* Substitution Modal */}
      {showSubPanel && userSide && !subsClosed && (
        <SubPanel
          snapshot={snapshot}
          side={userSide}
          onSubstitute={handleSubstitutions}
          onFormationChange={handleFormationChange}
          onPlayStyleChange={handlePlayStyleChange}
          onChangeRole={handleRoleChange}
          onClose={() => setShowSubPanel(false)}
        />
      )}
    </MatchScreenLayout>
  );
}
