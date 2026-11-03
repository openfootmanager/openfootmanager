import type { EnginePlayerData, MatchEvent } from "../types";

export interface PlayerPitchPos {
  playerId: string;
  name: string;
  number: number;
  position: string;
  isGoalkeeper: boolean;
  side: "Home" | "Away";
  x: number; // 0..100 (%)
  y: number; // 0..100 (%)
}

export interface BallPos {
  x: number;
  y: number;
  targetX?: number;
  targetY?: number;
  highlightPlayerId?: string | null;
  actionText?: string;
  isGoal: boolean;
}

const ZONE_X_OFFSETS: Record<string, number> = {
  HomeBox: -7,
  HomeDefense: -4,
  Midfield: 0,
  AwayDefense: 4,
  AwayBox: 8,
};

const ZONE_DEFAULT_X: Record<string, number> = {
  HomeBox: 10,
  HomeDefense: 28,
  Midfield: 50,
  AwayDefense: 72,
  AwayBox: 90,
};

function parseFormationCounts(formation: string): number[] {
  const parts = formation.split("-").map((n) => Number.parseInt(n, 10));
  if (parts.length >= 3 && parts.every((n) => !Number.isNaN(n) && n > 0)) {
    return parts;
  }
  return [4, 4, 2];
}

function distributePlayers(
  players: EnginePlayerData[],
  formation: string,
): { gk: EnginePlayerData | null; lines: EnginePlayerData[][] } {
  const gk = players.find((p) => p.position === "Goalkeeper") ?? players[0] ?? null;
  const outfields = players.filter((p) => p !== gk);

  const counts = parseFormationCounts(formation);
  const lines: EnginePlayerData[][] = [];
  let cursor = 0;

  for (let i = 0; i < counts.length; i++) {
    const count = counts[i];
    if (i === counts.length - 1) {
      lines.push(outfields.slice(cursor));
    } else {
      lines.push(outfields.slice(cursor, cursor + count));
      cursor += count;
    }
  }

  return { gk, lines };
}

function clamp(val: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, val));
}

export function buildPitchCoordinates(
  formationHome: string,
  formationAway: string,
  playersHome: EnginePlayerData[],
  playersAway: EnginePlayerData[],
  sentOff: string[] = [],
  zone = "Midfield",
  _possession: "Home" | "Away" = "Home",
): { home: PlayerPitchPos[]; away: PlayerPitchPos[] } {
  const sentOffSet = new Set(sentOff);
  const zoneShift = ZONE_X_OFFSETS[zone] ?? 0;

  // Layout Home team (attacking Left -> Right: 0% -> 100%)
  const homeDist = distributePlayers(playersHome, formationHome);
  const homePositions: PlayerPitchPos[] = [];

  if (homeDist.gk && !sentOffSet.has(homeDist.gk.id)) {
    const gkX = clamp(5 + zoneShift * 0.3, 2, 12);
    homePositions.push({
      playerId: homeDist.gk.id,
      name: homeDist.gk.name,
      number: 1,
      position: homeDist.gk.position,
      isGoalkeeper: true,
      side: "Home",
      x: gkX,
      y: 50,
    });
  }

  const homeLineCount = homeDist.lines.length;
  homeDist.lines.forEach((linePlayers, lineIdx) => {
    // Spread lines between x = 20% and x = 60%
    const baseX =
      homeLineCount > 1
        ? 20 + (lineIdx * 40) / (homeLineCount - 1)
        : 35;
    const shiftedX = clamp(baseX + zoneShift, 10, 92);

    linePlayers.forEach((player, pIdx) => {
      if (sentOffSet.has(player.id)) return;
      const y = Math.round((100 * (pIdx + 1)) / (linePlayers.length + 1));
      homePositions.push({
        playerId: player.id,
        name: player.name,
        number: pIdx + 2,
        position: player.position,
        isGoalkeeper: false,
        side: "Home",
        x: shiftedX,
        y,
      });
    });
  });

  // Layout Away team (attacking Right -> Left: 100% -> 0%)
  const awayDist = distributePlayers(playersAway, formationAway);
  const awayPositions: PlayerPitchPos[] = [];

  if (awayDist.gk && !sentOffSet.has(awayDist.gk.id)) {
    const gkX = clamp(95 + zoneShift * 0.3, 88, 98);
    awayPositions.push({
      playerId: awayDist.gk.id,
      name: awayDist.gk.name,
      number: 1,
      position: awayDist.gk.position,
      isGoalkeeper: true,
      side: "Away",
      x: gkX,
      y: 50,
    });
  }

  const awayLineCount = awayDist.lines.length;
  awayDist.lines.forEach((linePlayers, lineIdx) => {
    // Symmetrical layout: 100 - (20 + lineIdx * 40 / (N - 1))
    const baseX =
      awayLineCount > 1
        ? 100 - (20 + (lineIdx * 40) / (awayLineCount - 1))
        : 65;
    const shiftedX = clamp(baseX + zoneShift, 8, 90);

    linePlayers.forEach((player, pIdx) => {
      if (sentOffSet.has(player.id)) return;
      const y = Math.round((100 * (pIdx + 1)) / (linePlayers.length + 1));
      awayPositions.push({
        playerId: player.id,
        name: player.name,
        number: pIdx + 2,
        position: player.position,
        isGoalkeeper: false,
        side: "Away",
        x: shiftedX,
        y,
      });
    });
  });

  return { home: homePositions, away: awayPositions };
}

export function calculateBallPosition(params: {
  zone: string;
  possession: "Home" | "Away";
  events: MatchEvent[];
  homePositions: PlayerPitchPos[];
  awayPositions: PlayerPitchPos[];
}): BallPos {
  const { zone, events, homePositions, awayPositions } = params;

  // Check the latest event
  const latestEvent = events.length > 0 ? events[events.length - 1] : null;

  if (latestEvent) {
    if (latestEvent.event_type === "Goal") {
      const isHomeGoal = latestEvent.side === "Home";
      return {
        x: isHomeGoal ? 99 : 1,
        y: 50,
        isGoal: true,
        actionText: "GOAL!",
        highlightPlayerId: latestEvent.player_id,
      };
    }

    if (latestEvent.player_id) {
      const allPositions = [...homePositions, ...awayPositions];
      const playerPos = allPositions.find((p) => p.playerId === latestEvent.player_id);
      if (playerPos) {
        const isShot =
          latestEvent.event_type === "ShotOnTarget" ||
          latestEvent.event_type === "ShotOffTarget" ||
          latestEvent.event_type === "ShotSaved" ||
          latestEvent.event_type === "ShotBlocked";

        return {
          x: playerPos.x,
          y: playerPos.y,
          targetX: isShot ? (latestEvent.side === "Home" ? 98 : 2) : undefined,
          targetY: isShot ? 50 : undefined,
          isGoal: false,
          highlightPlayerId: latestEvent.player_id,
          actionText: latestEvent.event_type,
        };
      }
    }

    if (
      latestEvent.event_type === "ShotOnTarget" ||
      latestEvent.event_type === "ShotOffTarget" ||
      latestEvent.event_type === "ShotSaved" ||
      latestEvent.event_type === "ShotBlocked"
    ) {
      const isHomeShot = latestEvent.side === "Home";
      return {
        x: isHomeShot ? 96 : 4,
        y: 50,
        isGoal: false,
        actionText: "Shot",
        highlightPlayerId: latestEvent.player_id,
      };
    }
  }

  // Zone fallback
  const defaultX = ZONE_DEFAULT_X[zone] ?? 50;

  return {
    x: clamp(defaultX, 5, 95),
    y: 50,
    isGoal: false,
  };
}
