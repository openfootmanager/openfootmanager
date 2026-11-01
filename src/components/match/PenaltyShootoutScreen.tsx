import {
  type ReactNode,
  type RefObject,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";

import { getMatchSnapshot, stepLiveMatch } from "../../services/liveMatchService";
import type { GameStateData } from "../../store/gameStore";
import { useSettingsStore } from "../../store/settingsStore";
import { resolveBackendError } from "../../utils/backendI18n";
import { Button, TeamLogo } from "../ui";
import { KickSheet } from "./KickSheet";
import { sendMatchCommand } from "./matchCommands";
import { MatchSpeedControl } from "./MatchSpeedControl";
import { ShootoutOrderPicker } from "./ShootoutOrderPicker";
import { isOrderChanged } from "./shootoutOrder";
import {
  buildKickSheet,
  isShootoutKick,
  type KickSheet as Sheet,
  REGULAR_ROUNDS,
  sideKicks,
} from "./shootoutSheet";
import {
  type EnginePlayerData,
  type MatchEvent,
  type MatchSnapshot,
  type ShootoutOrder,
  SPEED_MS,
} from "./types";
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

/** The order the snapshot gives the user's own side; the opponent's is never sent. */
function ownOrder(snapshot: MatchSnapshot, userSide: "Home" | "Away" | null): ShootoutOrder | null {
  const shootout = snapshot.penalty_shootout;
  if (!userSide || !shootout) return null;
  return (userSide === "Home" ? shootout.home_order : shootout.away_order) ?? null;
}

/**
 * Step 1, the order picker, and what it needs: the order the engine first gave, the one being set,
 * and sending it when the shootout starts. The picker shows only before the first kick, for a side
 * the user manages, unless the player chose in Settings to skip it.
 */
function useShootoutOrderStep(
  snapshot: MatchSnapshot,
  userSide: "Home" | "Away" | null,
  onSnapshotUpdate: (snap: MatchSnapshot) => void,
) {
  const { t } = useTranslation();
  const { settings } = useSettingsStore();
  const skipPicker = settings.skip_shootout_picker;
  const [autoOrder] = useState(() => ownOrder(snapshot, userSide));
  // Held, not derived: between Start and the first kick the snapshot still shows no kick taken.
  const [step, setStep] = useState<"pick" | "kicks">(() =>
    autoOrder &&
    !skipPicker &&
    snapshot.phase === "PenaltyShootout" &&
    !snapshot.events.some(isShootoutKick)
      ? "pick"
      : "kicks",
  );
  const [order, setOrder] = useState<readonly string[]>(() => autoOrder?.player_ids ?? []);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sheetHeadingRef = useRef<HTMLHeadingElement>(null);
  const focusSheet = useRef(false);

  useEffect(() => {
    if (step !== "kicks" || !focusSheet.current) return;
    focusSheet.current = false;
    sheetHeadingRef.current?.focus();
  }, [step]);

  const start = useCallback(async () => {
    if (!userSide || !autoOrder) return;
    // The engine holds the auto order already; only a changed one is sent.
    if (isOrderChanged(autoOrder.player_ids, order)) {
      setStarting(true);
      try {
        const command = { SetShootoutOrder: { side: userSide, player_ids: [...order] } };
        onSnapshotUpdate(await sendMatchCommand(command));
      } catch (refusal) {
        setError(resolveBackendError(refusal) || t("match.shootout.orderFailed"));
        return;
      } finally {
        setStarting(false);
      }
    }
    setError(null);
    focusSheet.current = true;
    setStep("kicks");
  }, [userSide, autoOrder, order, onSnapshotUpdate, t]);

  return { step, autoOrder, order, setOrder, starting, error, start, sheetHeadingRef };
}

/** The user's players the shootout can name, on the pitch first, then the bench. */
function ownPlayers(snapshot: MatchSnapshot, userSide: "Home" | "Away" | null) {
  const players = new Map<string, EnginePlayerData>();
  if (!userSide) return players;
  const team = userSide === "Home" ? snapshot.home_team : snapshot.away_team;
  const bench = userSide === "Home" ? snapshot.home_bench : snapshot.away_bench;
  for (const player of [...bench, ...team.players]) players.set(player.id, player);
  return players;
}

/**
 * The penalty shootout, as a kick sheet: the score band with each side's kicks at a glance, the
 * sheet with every kicker and result round by round, and the live match's own speed control. It
 * steps one kick at a time, never batched, because watching each one is the point. Before the
 * first kick, the manager may set his side's order (step 1), with the sheet previewing it.
 */
export default function PenaltyShootoutScreen({
  snapshot,
  gameState,
  userSide,
  onSnapshotUpdate,
  onImportantEvent,
  onFullTime,
}: PenaltyShootoutScreenProps) {
  const { t } = useTranslation();
  const { pace, isRunning, pause, togglePause, selectPace } = useMatchPace("normal");
  const picker = useShootoutOrderStep(snapshot, userSide, onSnapshotUpdate);
  const picking = picker.step === "pick";
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
  const { autoOrder, order } = picker;
  const sheet = useMemo(
    () =>
      buildKickSheet(
        snapshot,
        picking && userSide && autoOrder
          ? { side: userSide, order, kickers: autoOrder.kickers }
          : undefined,
      ),
    [snapshot, picking, userSide, autoOrder, order],
  );

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
    // The engine does not wait for the order: no kick is taken while the picker shows.
    if (picking || !isRunning || stepFailed || finished) return;
    timerRef.current = setTimeout(() => void stepMatch(), SPEED_MS[pace]);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [
    picking,
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

      {picking && userSide && autoOrder ? (
        <div className="mx-auto grid w-full max-w-page flex-1 items-start gap-4 p-4 md:grid-cols-[minmax(0,1fr)_18rem] lg:grid-cols-[minmax(0,1fr)_22rem]">
          <ShootoutOrderPicker
            order={order}
            autoOrder={autoOrder.player_ids}
            kickers={autoOrder.kickers}
            players={ownPlayers(snapshot, userSide)}
            penaltyTakerId={
              (userSide === "Home" ? snapshot.home_set_pieces : snapshot.away_set_pieces)
                .penalty_taker
            }
            starting={picker.starting}
            error={picker.error}
            onOrderChange={picker.setOrder}
            onStart={() => void picker.start()}
          />
          <KickSheetPanel
            sheet={sheet}
            homeName={homeName}
            awayName={awayName}
            headingRef={picker.sheetHeadingRef}
            note={t("match.shootout.opponentOrderHidden", {
              team: userSide === "Home" ? awayName : homeName,
            })}
          />
        </div>
      ) : (
        <div className="mx-auto grid w-full max-w-page flex-1 items-start gap-4 p-4 md:grid-cols-[minmax(0,1fr)_18rem] lg:grid-cols-[minmax(0,1fr)_22rem]">
          <KickSheetPanel
            sheet={sheet}
            homeName={homeName}
            awayName={awayName}
            headingRef={picker.sheetHeadingRef}
          />

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
      )}
    </div>
  );
}

/** The kick sheet under its heading, which takes the focus when the kicks begin. */
function KickSheetPanel({
  sheet,
  homeName,
  awayName,
  headingRef,
  note,
}: {
  sheet: Sheet;
  homeName: string;
  awayName: string;
  headingRef: RefObject<HTMLHeadingElement | null>;
  note?: string;
}) {
  const { t } = useTranslation();
  return (
    <section
      aria-labelledby="kick-sheet-heading"
      className="overflow-hidden rounded-xl bg-panel shadow-sm"
    >
      <h2
        id="kick-sheet-heading"
        ref={headingRef}
        tabIndex={-1}
        className="flex h-panel-header items-center border-b border-line px-4 font-heading text-heading font-bold text-fg focus:outline-none"
      >
        {t("match.shootout.kickSheet")}
      </h2>
      <KickSheet sheet={sheet} homeName={homeName} awayName={awayName} />
      {note ? <p className="px-4 py-3 text-caption text-fg-muted">{note}</p> : null}
    </section>
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
