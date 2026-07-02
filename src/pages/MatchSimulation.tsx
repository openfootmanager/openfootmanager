import { useEffect, useState, useCallback } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { useGameStore, GameStateData } from "../store/gameStore";
import { useSettingsStore } from "../store/settingsStore";
import {
  MatchSnapshot,
  MatchEvent,
  MatchDayStage,
  RoundSummary,
} from "../components/match/types";
import { resolveMatchFixture } from "../components/match/helpers";
import PreMatchSetup from "../components/match/PreMatchSetup";
import MatchLive from "../components/match/MatchLive";
import HalfTimeBreak from "../components/match/HalfTimeBreak";
import PostMatchScreen from "../components/match/PostMatchScreen";
import RoundDigestScreen from "../components/match/RoundDigestScreen";
import PressConference from "../components/match/PressConference";
import PenaltyShootoutScreen from "../components/match/PenaltyShootoutScreen";

// ---------------------------------------------------------------------------
// Multi-stage Match Day Orchestrator
// ---------------------------------------------------------------------------

interface MatchRouteState {
  fixtureIndex?: number;
  mode?: string;
  snapshot?: MatchSnapshot;
}

interface FinishLiveMatchResponse {
  game: GameStateData;
  round_summary?: RoundSummary | null;
}

export default function MatchSimulation() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const routeState = (location.state as MatchRouteState | null) ?? null;
  const matchMode = routeState?.mode || "live";
  const { gameState, setGameState } = useGameStore();
  const { settings } = useSettingsStore();
  const [snapshot, setSnapshot] = useState<MatchSnapshot | null>(
    routeState?.snapshot ?? null,
  );
  const [stage, setStage] = useState<MatchDayStage>("prematch");
  const [importantEvents, setImportantEvents] = useState<MatchEvent[]>([]);
  const [userSide, setUserSide] = useState<"Home" | "Away" | null>(null);
  const [isSpectator, setIsSpectator] = useState(matchMode === "spectator");
  const [roundSummary, setRoundSummary] = useState<RoundSummary | null>(null);
  const [hasFinalizedMatch, setHasFinalizedMatch] = useState(false);
  const [preferredSpeed, setPreferredSpeed] = useState<"slow" | "normal" | "fast">("normal");
  const [hasUserOverriddenSpeed, setHasUserOverriddenSpeed] = useState(false);

  useEffect(() => {
    console.info("[MatchSimulation] mount", {
      fixtureIndex: routeState?.fixtureIndex,
      hasGameState: !!gameState,
      hasRouteSnapshot: !!routeState?.snapshot,
      matchMode,
    });
  }, [gameState, matchMode, routeState?.fixtureIndex, routeState?.snapshot]);

  useEffect(() => {
    if (hasUserOverriddenSpeed) return;
    setPreferredSpeed(
      settings.match_speed === "slow" || settings.match_speed === "fast"
        ? settings.match_speed
        : "normal",
    );
  }, [settings.match_speed, hasUserOverriddenSpeed]);

  // Determine user side from game state
  useEffect(() => {
    if (!gameState || !snapshot) return;
    const utid = gameState.manager.team_id;
    if (!utid) {
      setIsSpectator(true);
      return;
    }
    if (snapshot.home_team.id === utid) setUserSide("Home");
    else if (snapshot.away_team.id === utid) setUserSide("Away");
    else setIsSpectator(true);

    // If mode is spectator, force spectator regardless of team
    if (matchMode === "spectator") setIsSpectator(true);

    console.info("[MatchSimulation] resolveSide", {
      awayTeamId: snapshot.away_team.id,
      homeTeamId: snapshot.home_team.id,
      matchMode,
      managerTeamId: utid,
      resolvedUserSide:
        snapshot.home_team.id === utid
          ? "Home"
          : snapshot.away_team.id === utid
            ? "Away"
            : null,
    });
  }, [gameState, snapshot?.home_team.id, snapshot?.away_team.id, matchMode]);

  useEffect(() => {
    console.info("[MatchSimulation] stage", {
      hasSnapshot: !!snapshot,
      isSpectator,
      stage,
      userSide,
    });
  }, [isSpectator, snapshot, stage, userSide]);

  // Fetch initial snapshot
  useEffect(() => {
    // Don't refetch after finalize — finish_live_match has cleared the backend
    // session, so get_match_snapshot would fail and the catch path would
    // start_live_match a fresh 0-0 session, clobbering the final score.
    if (hasFinalizedMatch) return;
    let isCancelled = false;

    const fetchSnapshot = async () => {
      console.info("[MatchSimulation] fetchSnapshot:start", {
        fixtureIndex: routeState?.fixtureIndex,
        hasRouteSnapshot: !!routeState?.snapshot,
        matchMode,
      });
      try {
        const snap = await invoke<MatchSnapshot | null>("get_match_snapshot");
        if (!snap) throw new Error("No active match snapshot");
        console.info("[MatchSimulation] fetchSnapshot:success", {
          awayPlayers: snap.away_team.players.length,
          awayTeam: snap.away_team.name,
          homePlayers: snap.home_team.players.length,
          homeTeam: snap.home_team.name,
          phase: snap.phase,
        });
        if (!isCancelled) {
          setSnapshot(snap);
        }
        return;
      } catch (snapshotError) {
        console.warn("[MatchSimulation] fetchSnapshot:failed", snapshotError);
        if (typeof routeState?.fixtureIndex !== "number") {
          console.error("Failed to get match snapshot:", snapshotError);
          navigate("/dashboard");
          return;
        }

        try {
          console.info("[MatchSimulation] restoreLiveMatch:start", {
            fixtureIndex: routeState.fixtureIndex,
            matchMode,
          });
          const fixture = gameState?.league?.fixtures?.[routeState.fixtureIndex];
          const competitionsWithET: string[] = ["Cup", "ContinentalClub", "InternationalClub", "InternationalNation", "FriendlyCup"];
          const allowsExtraTime = routeState?.snapshot?.allows_extra_time
            ?? competitionsWithET.includes(fixture?.competition ?? "");
          // Identify the fixture by its teams so the backend can resolve it
          // across all competitions — the raw index may point into a cup while
          // game.league mirrors the domestic league after a restart.
          const restoredSnapshot = await invoke<MatchSnapshot>(
            "start_live_match",
            {
              allowsExtraTime,
              fixtureIndex: routeState.fixtureIndex,
              mode: matchMode,
              homeTeamId: routeState?.snapshot?.home_team?.id ?? null,
              awayTeamId: routeState?.snapshot?.away_team?.id ?? null,
            },
          );

          console.info("[MatchSimulation] restoreLiveMatch:success", {
            awayPlayers: restoredSnapshot.away_team.players.length,
            awayTeam: restoredSnapshot.away_team.name,
            homePlayers: restoredSnapshot.home_team.players.length,
            homeTeam: restoredSnapshot.home_team.name,
            phase: restoredSnapshot.phase,
          });

          if (!isCancelled) {
            setSnapshot(restoredSnapshot);
          }
        } catch (restoreError) {
          console.error("Failed to restore live match session:", restoreError);
          navigate("/dashboard");
        }
      }
    };

    fetchSnapshot();

    return () => {
      isCancelled = true;
    };
  }, [hasFinalizedMatch, gameState, matchMode, navigate, routeState?.fixtureIndex, routeState?.snapshot]);

  // Skip pre-match for spectators
  useEffect(() => {
    if (isSpectator && stage === "prematch") {
      setStage("first_half");
    }
  }, [isSpectator, stage]);

  // Callbacks for stage transitions
  const handleStartMatch = useCallback(() => {
    console.info("[MatchSimulation] handleStartMatch");
    setStage("first_half");
  }, []);

  const handleHalfTime = useCallback((phase: "HalfTime" | "ExtraTimeHalfTime") => {
    console.info("[MatchSimulation] handleHalfTime", { phase });
    if (phase === "ExtraTimeHalfTime") {
      setStage("extra_time_halftime");
    } else {
      setStage("halftime");
    }
  }, []);

  const handleResumeFromHalfTime = useCallback(() => {
    console.info("[MatchSimulation] handleResumeFromHalfTime", { stage });
    setStage(stage === "extra_time_halftime" ? "extra_time_second_half" : "second_half");
  }, [stage]);

  const handlePenaltyShootout = useCallback(() => {
    console.info("[MatchSimulation] handlePenaltyShootout");
    setStage("penalty_shootout");
  }, []);

  const finalizeMatch = useCallback(async (): Promise<boolean> => {
    if (hasFinalizedMatch) {
      return true;
    }

    try {
      console.info("[MatchSimulation] finalizeMatch:start");
      const response =
        await invoke<FinishLiveMatchResponse>("finish_live_match");
      console.info("[MatchSimulation] finalizeMatch:success", {
        hasRoundSummary: !!response.round_summary,
        hasUpdatedGame: !!response.game,
      });
      setGameState(response.game);
      setRoundSummary(response.round_summary ?? null);
      setHasFinalizedMatch(true);
      return true;
    } catch (err) {
      console.error("Failed to finish match:", err);
      return false;
    }
  }, [hasFinalizedMatch, setGameState]);

  const handleFullTime = useCallback(() => {
    console.info("[MatchSimulation] handleFullTime");
    void (async () => {
      const finalized = await finalizeMatch();
      if (finalized) {
        setStage("postmatch");
      }
    })();
  }, [finalizeMatch]);

  const handleFinishMatch = useCallback(async () => {
    console.info("[MatchSimulation] handleFinishMatch:start");
    const finalized = await finalizeMatch();
    if (finalized) {
      navigate("/dashboard");
    }
  }, [finalizeMatch, navigate]);

  const handlePostMatchContinue = useCallback(() => {
    if (isSpectator) {
      void handleFinishMatch();
      return;
    }
    console.info("[MatchSimulation] handlePostMatchContinue");
    setStage("digest");
  }, [isSpectator, handleFinishMatch]);

  const handlePressConference = useCallback(() => {
    console.info("[MatchSimulation] handlePressConference");
    setStage("press");
  }, []);

  const handleSnapshotUpdate = useCallback((snap: MatchSnapshot) => {
    console.info("[MatchSimulation] handleSnapshotUpdate", {
      awayPlayers: snap.away_team.players.length,
      currentMinute: snap.current_minute,
      homePlayers: snap.home_team.players.length,
      phase: snap.phase,
    });
    setSnapshot(snap);
  }, []);

  const handleImportantEvent = useCallback((evt: MatchEvent) => {
    console.info("[MatchSimulation] handleImportantEvent", {
      eventType: evt.event_type,
      minute: evt.minute,
      side: evt.side,
    });
    setImportantEvents((prev) => [...prev, evt]);
  }, []);

  const handlePreferredSpeedChange = useCallback(
    (speed: "slow" | "normal" | "fast") => {
      setHasUserOverriddenSpeed(true);
      setPreferredSpeed(speed);
    },
    [],
  );

  // Loading state
  if (!snapshot || !gameState) {
    return (
      <div className="min-h-screen bg-gray-100 dark:bg-navy-900 flex items-center justify-center transition-colors duration-300">
        <div className="flex flex-col items-center gap-3">
          <div className="w-8 h-8 border-4 border-primary-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider text-sm">
            {t("dashboard.loading")}
          </span>
        </div>
      </div>
    );
  }

  const currentFixture = resolveMatchFixture(
    gameState,
    snapshot,
    routeState?.fixtureIndex,
  );

  // Render the current stage
  switch (stage) {
    case "prematch":
      if (!userSide) return null;
      return (
        <PreMatchSetup
          snapshot={snapshot}
          gameState={gameState}
          currentFixture={currentFixture}
          userSide={userSide}
          onStart={handleStartMatch}
          onUpdateSnapshot={handleSnapshotUpdate}
        />
      );

    case "first_half":
    case "second_half":
    case "extra_time_second_half":
      return (
        <MatchLive
          key={stage}
          snapshot={snapshot}
          gameState={gameState}
          userSide={userSide}
          isSpectator={isSpectator}
          importantEvents={importantEvents}
          preferredSpeed={preferredSpeed}
          onPreferredSpeedChange={handlePreferredSpeedChange}
          onSnapshotUpdate={handleSnapshotUpdate}
          onImportantEvent={handleImportantEvent}
          onHalfTime={handleHalfTime}
          onFullTime={handleFullTime}
          onPenaltyShootout={handlePenaltyShootout}
        />
      );

    case "halftime":
    case "extra_time_halftime":
      if (!userSide) return null;
      return (
        <HalfTimeBreak
          key={stage}
          snapshot={snapshot}
          gameState={gameState}
          userSide={userSide}
          isSpectator={isSpectator}
          importantEvents={importantEvents}
          onResume={handleResumeFromHalfTime}
          onUpdateSnapshot={handleSnapshotUpdate}
        />
      );

    case "penalty_shootout":
      return (
        <PenaltyShootoutScreen
          snapshot={snapshot}
          gameState={gameState}
          userSide={userSide}
          isSpectator={isSpectator}
          importantEvents={importantEvents}
          onSnapshotUpdate={handleSnapshotUpdate}
          onImportantEvent={handleImportantEvent}
          onFullTime={handleFullTime}
        />
      );

    case "postmatch":
      return (
        <PostMatchScreen
          snapshot={snapshot}
          gameState={gameState}
          userSide={userSide}
          isSpectator={isSpectator}
          importantEvents={importantEvents}
          onContinue={handlePostMatchContinue}
          onFinish={handleFinishMatch}
        />
      );

    case "digest": {
      const isLeagueFixture = currentFixture
        ? currentFixture.competition !== "Friendly" &&
          currentFixture.competition !== "PreseasonTournament"
        : roundSummary !== null;
      return (
        <RoundDigestScreen
          snapshot={snapshot}
          gameState={gameState}
          currentFixture={currentFixture}
          userSide={userSide}
          isLeagueFixture={isLeagueFixture}
          roundSummary={roundSummary}
          onPressConference={handlePressConference}
          onFinish={handleFinishMatch}
        />
      );
    }

    case "press":
      if (!userSide) return null;
      return (
        <PressConference
          snapshot={snapshot}
          gameState={gameState}
          userSide={userSide}
          onFinish={handleFinishMatch}
          onGameUpdate={setGameState}
        />
      );

    default:
      return null;
  }
}
