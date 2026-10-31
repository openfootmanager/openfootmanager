import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { getMatchSnapshot, stepLiveMatch } from "../../services/liveMatchService";
import type { GameStateData } from "../../store/gameStore";
import { Button, TeamLogo } from "../ui";
import { KickSheet } from "./KickSheet";
import { MatchSpeedControl } from "./MatchSpeedControl";
import {
  buildKickSheet,
  isShootoutKick,
  type KickSheet as Sheet,
  REGULAR_ROUNDS,
  sideKicks,
} from "./shootoutSheet";
import { type MatchEvent, type MatchSnapshot, SPEED_MS } from "./types";
import { useMatchPace } from "./useMatchPace";

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

/**
 * The penalty shootout, as a kick sheet: the score band with each side's kicks at a glance, the
 * sheet with every kicker and result round by round, and the live match's own speed control. It
 * steps one kick at a time, never batched, because watching each one is the point.
 */
export default function PenaltyShootoutScreen({
  snapshot,
  gameState,
  onSnapshotUpdate,
  onImportantEvent,
  onFullTime,
}: PenaltyShootoutScreenProps) {
  const { t } = useTranslation();
  const { pace, isRunning, pause, togglePause, selectPace } = useMatchPace("normal");
  const [stepFailed, setStepFailed] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const finishedRef = useRef(false);
  // Set while a step is awaiting the engine. A second step then (a double-click on Next kick, or
  // Retry over the timer's step) would take two kicks at once, or ask for a kick after the last.
  const stepInFlightRef = useRef(false);
  const [finished, setFinished] = useState(false);

  const homeFullTeam = gameState.teams.find((tm) => tm.id === snapshot.home_team.id);
  const awayFullTeam = gameState.teams.find((tm) => tm.id === snapshot.away_team.id);
  const homeName = snapshot.home_team.name;
  const awayName = snapshot.away_team.name;
  const ps = snapshot.penalty_shootout;
  const sheet = useMemo(() => buildKickSheet(snapshot), [snapshot]);

  const stepMatch = useCallback(async () => {
    if (finishedRef.current || stepInFlightRef.current) return;
    stepInFlightRef.current = true;
    try {
      const results = await stepLiveMatch(1);
      for (const result of results) {
        for (const evt of result.events) {
          if (isShootoutKick(evt)) {
            onImportantEvent(evt);
          }
        }
      }
      onSnapshotUpdate(await getMatchSnapshot());
      if (results[results.length - 1]?.is_finished) {
        finishedRef.current = true;
        setFinished(true);
        pause();
        setTimeout(() => onFullTime(), 800);
      }
    } catch {
      // The shootout can't go on without the engine; say so and let the player try again, rather
      // than stopping with nothing on screen.
      setStepFailed(true);
      pause();
    } finally {
      stepInFlightRef.current = false;
    }
  }, [onSnapshotUpdate, onImportantEvent, onFullTime, pause]);

  // Resuming after a failed step is a retry too, so it clears the message.
  const resumeOrPause = useCallback(() => {
    setStepFailed(false);
    togglePause();
  }, [togglePause]);

  const retry = useCallback(() => {
    setStepFailed(false);
    void stepMatch();
  }, [stepMatch]);

  useEffect(() => {
    if (!isRunning || stepFailed || finished) return;
    timerRef.current = setTimeout(() => void stepMatch(), SPEED_MS[pace]);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [
    isRunning,
    stepFailed,
    finished,
    pace,
    snapshot.current_minute,
    snapshot.events.length,
    stepMatch,
  ]);

  const status = ps?.sudden_death
    ? t("match.shootout.suddenDeath")
    : sheet.currentRound !== null && sheet.currentRound <= REGULAR_ROUNDS
      ? t("match.shootout.round", { n: sheet.currentRound })
      : null;

  return (
    <div className="flex min-h-screen flex-col bg-page text-fg">
      <ScoreBand
        snapshot={snapshot}
        sheet={sheet}
        homeLogo={homeFullTeam && <TeamLogo team={homeFullTeam} />}
        awayLogo={awayFullTeam && <TeamLogo team={awayFullTeam} />}
        status={status}
      />
      <LastKickAnnouncement snapshot={snapshot} sheet={sheet} />

      <div className="mx-auto grid w-full max-w-page flex-1 items-start gap-4 p-4 md:grid-cols-[minmax(0,1fr)_18rem] lg:grid-cols-[minmax(0,1fr)_22rem]">
        <section
          aria-labelledby="kick-sheet-heading"
          className="overflow-hidden rounded-xl bg-panel shadow-sm"
        >
          <h2
            id="kick-sheet-heading"
            className="flex h-panel-header items-center border-b border-line px-4 font-heading text-heading font-bold text-fg"
          >
            {t("match.shootout.kickSheet")}
          </h2>
          <KickSheet sheet={sheet} homeName={homeName} awayName={awayName} />
        </section>

        <aside className="flex flex-col gap-4 rounded-xl bg-panel p-4 shadow-sm">
          {stepFailed && (
            <div role="alert" className="flex flex-col gap-2 rounded-lg bg-danger-tint p-3">
              <p className="text-body font-semibold text-danger">
                {t("match.shootout.stepFailed")}
              </p>
              <Button variant="outline" size="sm" onClick={retry} className="self-start">
                {t("common.retry")}
              </Button>
            </div>
          )}
          <MatchSpeedControl
            paused={!isRunning}
            pace={pace}
            onTogglePause={resumeOrPause}
            onSelectPace={selectPace}
            onStep={() => void stepMatch()}
            stepLabel={t("match.shootout.nextKick")}
            enabled={!finished}
          />
        </aside>
      </div>
    </div>
  );
}

/** The always-dark band: both clubs, the shootout score, each side's kicks, and where it stands. */
function ScoreBand({
  snapshot,
  sheet,
  homeLogo,
  awayLogo,
  status,
}: {
  snapshot: MatchSnapshot;
  sheet: Sheet;
  homeLogo: ReactNode;
  awayLogo: ReactNode;
  status: string | null;
}) {
  const { t } = useTranslation();
  const ps = snapshot.penalty_shootout;
  const home = ps?.home_scored ?? 0;
  const away = ps?.away_scored ?? 0;
  const live = snapshot.phase === "PenaltyShootout";
  const homeKicks = sideKicks(snapshot.events, "Home").map((e) => e.event_type === "ShootoutGoal");
  const awayKicks = sideKicks(snapshot.events, "Away").map((e) => e.event_type === "ShootoutGoal");
  // As many cells as the sheet has rounds while the shootout is on; once over, only kicks taken.
  const cells = (kicks: boolean[]) => (live ? sheet.rows.length : kicks.length);

  return (
    <header className="bg-chrome text-chrome-fg">
      <div className="mx-auto grid w-full max-w-page grid-cols-[1fr_auto_1fr] items-center gap-4 px-4 py-3">
        <div className="flex min-w-0 items-center gap-3">
          {homeLogo}
          <div className="flex min-w-0 flex-col gap-1.5">
            <span className="truncate font-heading text-heading font-bold">
              {snapshot.home_team.name}
            </span>
            <KickRow
              label={snapshot.home_team.name}
              kicks={homeKicks}
              maxRounds={cells(homeKicks)}
            />
          </div>
        </div>
        <div className="flex flex-col items-center gap-1 text-center">
          <h1 className="font-heading text-caption font-bold uppercase tracking-label text-chrome-fg-muted">
            {t("match.penaltyShootout")}
          </h1>
          <p className="font-heading text-display font-bold tabular-nums">
            <span className="sr-only">
              {t("match.shootout.shootoutScore", { h: home, a: away })}
            </span>
            <span aria-hidden="true">
              {home} – {away}
            </span>
          </p>
          <p className="text-caption text-chrome-fg-muted">
            {t("match.shootout.afterExtraTime", {
              home: snapshot.home_score,
              away: snapshot.away_score,
            })}
          </p>
          {status && (
            <p
              className={`rounded-full px-2 py-0.5 font-heading text-caption font-bold uppercase tracking-label ${
                snapshot.penalty_shootout?.sudden_death
                  ? "bg-warning-tint text-warning"
                  : "text-chrome-fg-muted"
              }`}
            >
              {status}
            </p>
          )}
        </div>
        <div className="flex min-w-0 items-center justify-end gap-3 text-right">
          <div className="flex min-w-0 flex-col items-end gap-1.5">
            <span className="truncate font-heading text-heading font-bold">
              {snapshot.away_team.name}
            </span>
            <KickRow
              label={snapshot.away_team.name}
              kicks={awayKicks}
              maxRounds={cells(awayKicks)}
            />
          </div>
          {awayLogo}
        </div>
      </div>
    </header>
  );
}

/** Reads each kick out once, as it lands: "Kick 6, Beta FC: Bob missed." */
function LastKickAnnouncement({ snapshot, sheet }: { snapshot: MatchSnapshot; sheet: Sheet }) {
  const { t } = useTranslation();
  const kicks = snapshot.events.filter(isShootoutKick);
  const last = kicks[kicks.length - 1];
  let message = "";
  if (last) {
    const n = sideKicks(snapshot.events, last.side).length;
    const cell = sheet.rows[n - 1]?.[last.side === "Home" ? "home" : "away"];
    const team = last.side === "Home" ? snapshot.home_team.name : snapshot.away_team.name;
    const kicker = cell && "kickerName" in cell && cell.kickerName ? cell.kickerName : team;
    message = t(
      last.event_type === "ShootoutGoal"
        ? "match.shootout.kickAnnounceScored"
        : "match.shootout.kickAnnounceMissed",
      { n, team, kicker },
    );
  }
  return (
    <p role="status" className="sr-only">
      {message}
    </p>
  );
}

/**
 * One side's kicks, in the order they were taken (true: scored), then the ones still to come.
 * Each cell is named for a screen reader ("Kick 2, missed"), so the result is never only a symbol
 * and a colour.
 */
export function KickRow({
  label,
  kicks,
  maxRounds,
}: {
  label: string;
  kicks: readonly boolean[];
  maxRounds: number;
}) {
  const { t } = useTranslation();
  const cells = Math.max(maxRounds, kicks.length);

  return (
    <span role="group" aria-label={label} className="flex flex-wrap gap-1">
      {Array.from({ length: cells }).map((_, i) => {
        const n = i + 1;
        if (i >= kicks.length) {
          return (
            <span
              key={n}
              role="img"
              aria-label={t("match.shootout.kickToCome", { n })}
              className="size-4 rounded-full border-2 border-chrome-fg-muted"
            />
          );
        }
        const scored = kicks[i];
        return (
          <span
            key={n}
            role="img"
            aria-label={t(scored ? "match.shootout.kickScored" : "match.shootout.kickMissed", {
              n,
            })}
            className={`grid size-4 place-items-center rounded-full font-heading text-caption font-bold leading-none ${
              scored ? "bg-positive-tint text-positive" : "bg-danger-tint text-danger"
            }`}
          >
            <span aria-hidden="true">{scored ? "✓" : "✗"}</span>
          </span>
        );
      })}
    </span>
  );
}
