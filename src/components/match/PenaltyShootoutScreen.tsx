import { useEffect, useRef, useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { GameStateData } from "../../store/gameStore";
import {
  MatchSnapshot,
  MatchEvent,
  MinuteResult,
  SimSpeed,
  SPEED_MS,
} from "./types";
import {
  Play, Pause, FastForward, SkipForward,
} from "lucide-react";
import { TeamLogo } from "../ui";

interface PenaltyShootoutScreenProps {
  snapshot: MatchSnapshot;
  gameState: GameStateData;
  userSide: "Home" | "Away" | null;
  isSpectator: boolean;
  importantEvents: MatchEvent[];
  onSnapshotUpdate: (snap: MatchSnapshot) => void;
  onImportantEvent: (evt: MatchEvent) => void;
  onFullTime: () => void;
}

// Only true shootout kicks: an in-match PenaltyAwarded from regulation/ET
// lives in the same snapshot.events log and must not appear in this feed.
const SHOOTOUT_EVENTS = new Set([
  "ShootoutGoal",
  "ShootoutMiss",
]);

export default function PenaltyShootoutScreen({
  snapshot,
  gameState,
  onSnapshotUpdate,
  onImportantEvent,
  onFullTime,
}: PenaltyShootoutScreenProps) {
  const { t } = useTranslation();
  const [speed, setSpeed] = useState<SimSpeed>("normal");
  const [isRunning, setIsRunning] = useState(true);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const signaledRef = useRef(false);

  const homeFullTeam = gameState.teams.find(
    (tm) => tm.id === snapshot.home_team.id,
  );
  const awayFullTeam = gameState.teams.find(
    (tm) => tm.id === snapshot.away_team.id,
  );

  const ps = snapshot.penalty_shootout;
  const roundNumber = ps ? Math.max(ps.home_taken, ps.away_taken) : 0;

  const stepMatch = useCallback(async () => {
    try {
      const results = await invoke<MinuteResult[]>("step_live_match", {
        minutes: 1,
      });
      if (results.length > 0) {
        for (const r of results) {
          for (const evt of r.events) {
            if (SHOOTOUT_EVENTS.has(evt.event_type)) {
              onImportantEvent(evt);
            }
          }
        }

        const snap = await invoke<MatchSnapshot>("get_match_snapshot");
        onSnapshotUpdate(snap);

        const lastResult = results[results.length - 1];
        if (lastResult.is_finished && !signaledRef.current) {
          signaledRef.current = true;
          setIsRunning(false);
          setSpeed("paused");
          setTimeout(() => onFullTime(), 800);
        }
      }
    } catch (err) {
      console.error("Failed to step penalty shootout:", err);
      setIsRunning(false);
    }
  }, [onSnapshotUpdate, onImportantEvent, onFullTime]);

  useEffect(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }

    if (isRunning && speed !== "paused") {
      timerRef.current = setTimeout(async () => {
        await stepMatch();
      }, SPEED_MS[speed]);
    }

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [isRunning, speed, snapshot.current_minute, snapshot.phase, stepMatch]);

  const shootoutEvents = snapshot.events.filter((e) =>
    SHOOTOUT_EVENTS.has(e.event_type),
  );

  return (
    <div className="min-h-screen bg-gray-100 dark:bg-navy-900 flex flex-col items-center justify-center px-4 py-8 transition-colors duration-300">
      {/* Header */}
      <div className="w-full max-w-lg mb-6 text-center">
        <p className="text-xs font-heading uppercase tracking-widest text-accent-600 dark:text-accent-400 mb-1">
          {ps?.sudden_death
            ? t("match.shootout.suddenDeath")
            : roundNumber > 0
              ? t("match.shootout.round", { n: roundNumber })
              : t("match.penaltyShootout")}
        </p>
        <h1 className="text-2xl font-heading font-bold text-gray-900 dark:text-white">
          {t("match.penaltyShootout")}
        </h1>
      </div>

      {/* Score card */}
      <div className="w-full max-w-lg bg-white dark:bg-navy-800 rounded-2xl shadow-lg p-6 mb-4">
        <div className="flex items-center justify-between gap-4">
          {/* Home */}
          <div className="flex flex-col items-center gap-2 flex-1">
            {homeFullTeam && <TeamLogo team={homeFullTeam} />}
            <span className="font-heading font-semibold text-gray-900 dark:text-white text-sm text-center">
              {snapshot.home_team.name}
            </span>
            <span className="text-3xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
              {ps?.home_scored ?? 0}
            </span>
          </div>

          {/* vs */}
          <div className="text-gray-400 dark:text-gray-500 font-heading font-bold text-xl">
            –
          </div>

          {/* Away */}
          <div className="flex flex-col items-center gap-2 flex-1">
            {awayFullTeam && <TeamLogo team={awayFullTeam} />}
            <span className="font-heading font-semibold text-gray-900 dark:text-white text-sm text-center">
              {snapshot.away_team.name}
            </span>
            <span className="text-3xl font-heading font-bold text-gray-900 dark:text-white tabular-nums">
              {ps?.away_scored ?? 0}
            </span>
          </div>
        </div>

        {/* Kick grid */}
        {ps && (
          <div className="mt-6 space-y-3">
            <KickRow
              label={snapshot.home_team.name}
              taken={ps.home_taken}
              scored={ps.home_scored}
              maxRounds={ps.sudden_death ? ps.home_taken + 1 : 5}
            />
            <KickRow
              label={snapshot.away_team.name}
              taken={ps.away_taken}
              scored={ps.away_scored}
              maxRounds={ps.sudden_death ? ps.away_taken + 1 : 5}
            />
          </div>
        )}
      </div>

      {/* Event feed */}
      {shootoutEvents.length > 0 && (
        <div className="w-full max-w-lg bg-white dark:bg-navy-800 rounded-xl p-4 mb-4 space-y-1">
          {shootoutEvents.slice(-8).map((evt, i) => (
            <div
              key={i}
              className="flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300"
            >
              <span className="text-gray-400 dark:text-gray-500 tabular-nums w-6 text-right">
                {evt.minute}&apos;
              </span>
              <span
                className={
                  evt.event_type === "ShootoutGoal"
                    ? "text-green-600 dark:text-green-400 font-semibold"
                    : "text-red-500 dark:text-red-400"
                }
              >
                {evt.event_type === "ShootoutGoal" ? "⚽" : "✗"}
              </span>
              <span>
                {evt.side === "Home"
                  ? snapshot.home_team.name
                  : snapshot.away_team.name}
              </span>
            </div>
          ))}
        </div>
      )}

      {/* Speed controls */}
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => {
            setIsRunning((r) => !r);
            if (!isRunning) setSpeed("normal");
            else setSpeed("paused");
          }}
          className="p-2 rounded-full bg-white dark:bg-navy-700 shadow hover:shadow-md transition-all"
          aria-label={isRunning ? t("match.pause") : t("match.live")}
        >
          {isRunning ? (
            <Pause className="w-5 h-5 text-gray-700 dark:text-gray-200" />
          ) : (
            <Play className="w-5 h-5 text-gray-700 dark:text-gray-200" />
          )}
        </button>
        <button
          type="button"
          onClick={() => { setSpeed("fast"); setIsRunning(true); }}
          className="p-2 rounded-full bg-white dark:bg-navy-700 shadow hover:shadow-md transition-all"
          aria-label={t("match.fast")}
        >
          <FastForward className="w-5 h-5 text-gray-700 dark:text-gray-200" />
        </button>
        <button
          type="button"
          onClick={async () => {
            setIsRunning(false);
            await stepMatch();
          }}
          className="p-2 rounded-full bg-white dark:bg-navy-700 shadow hover:shadow-md transition-all"
          aria-label={t("match.step1Min")}
        >
          <SkipForward className="w-5 h-5 text-gray-700 dark:text-gray-200" />
        </button>
      </div>
    </div>
  );
}

export function KickRow({
  label,
  taken,
  scored,
  maxRounds,
}: {
  label: string;
  taken: number;
  scored: number;
  maxRounds: number;
}) {
  const cells = Math.max(maxRounds, taken);

  return (
    <div className="flex items-center gap-3">
      <span className="text-xs text-gray-500 dark:text-gray-400 w-20 truncate text-right">
        {label}
      </span>
      <div className="flex gap-1.5 flex-wrap">
        {Array.from({ length: cells }).map((_, i) => {
          if (i >= taken) {
            return (
              <span
                key={i}
                className="w-6 h-6 rounded-full border-2 border-gray-200 dark:border-gray-600 flex items-center justify-center text-xs text-gray-300"
              >
                ?
              </span>
            );
          }
          const isGoal = i < scored;
          return (
            <span
              key={i}
              className={`w-6 h-6 rounded-full flex items-center justify-center text-sm ${
                isGoal
                  ? "bg-green-100 dark:bg-green-900/40 text-green-600 dark:text-green-400"
                  : "bg-red-100 dark:bg-red-900/40 text-red-500 dark:text-red-400"
              }`}
            >
              {isGoal ? "⚽" : "✗"}
            </span>
          );
        })}
      </div>
    </div>
  );
}
