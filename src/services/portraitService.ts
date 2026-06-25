import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";

import { findNextFixture } from "../lib/fixtures";
import { resolveLocalMediaPath } from "../lib/mediaAssets";
import type { GameStateData, PlayerData } from "../store/gameStore";

export interface PlayerPortraitIdentity {
  id?: string | null;
  full_name?: string | null;
  match_name?: string | null;
  nationality?: string | null;
  date_of_birth?: string | null;
}

export interface GeneratedPlayerPortrait {
  generator: string;
  cacheKey: string;
  sourceId: string;
  cachePath: string;
  dataUrl: string | null;
  imageUrl: string;
  generated: boolean;
  renderMs: number;
  elapsedMs: number;
  width: number;
  height: number;
}

type GeneratedPlayerPortraitCommandResponse = Omit<
  GeneratedPlayerPortrait,
  "imageUrl"
>;

export interface PrewarmPlayerPortraitRecord {
  playerId: string;
  cacheKey: string;
  sourceId: string;
  cachePath: string;
  generated: boolean;
  renderMs: number;
  elapsedMs: number;
  dataUrl: string | null;
}

export interface PrewarmPlayerPortraitsResponse {
  generator: string;
  requestedCount: number;
  generatedCount: number;
  cachedCount: number;
  failedCount: number;
  renderMs: number;
  elapsedMs: number;
  records: PrewarmPlayerPortraitRecord[];
}

const runtimePortraitRequests = new Map<
  string,
  Promise<GeneratedPlayerPortrait | null>
>();
const queuedBackgroundPrewarmKeys = new Set<string>();
const DEFAULT_BACKGROUND_PREWARM_LIMIT = 48;
const DEFAULT_BACKGROUND_PREWARM_BATCH_SIZE = 4;
const DEFAULT_BACKGROUND_PREWARM_BATCH_DELAY_MS = 150;
const noop = () => undefined;

export function canGenerateRuntimePlayerPortraits(): boolean {
  try {
    return isTauri();
  } catch {
    return false;
  }
}

export function runtimePortraitIdentityKey(
  player: PlayerPortraitIdentity,
): string {
  return [
    player.id,
    player.full_name,
    player.match_name,
    player.nationality,
    player.date_of_birth,
  ]
    .map((part) => part?.trim().toLowerCase() || "-")
    .join("|");
}

function toPortraitRequest(player: PlayerPortraitIdentity) {
  const fallbackId =
    player.id ||
    player.full_name ||
    player.match_name ||
    runtimePortraitIdentityKey(player);

  return {
    playerId: fallbackId,
    fullName: player.full_name ?? null,
    matchName: player.match_name ?? null,
    nationality: player.nationality ?? null,
    dateOfBirth: player.date_of_birth ?? null,
  };
}

function isPortraitEligiblePlayer(player: PlayerData): boolean {
  return !player.retired && !resolveLocalMediaPath(player.media?.face);
}

export function selectManagerSquadPortraitPlayers(
  gameState: GameStateData,
): PlayerData[] {
  const teamId = gameState.manager.team_id;
  if (!teamId) {
    return [];
  }

  return gameState.players.filter(
    (player) => player.team_id === teamId && isPortraitEligiblePlayer(player),
  );
}

export function selectBackgroundPortraitPlayers(
  gameState: GameStateData,
  limit = DEFAULT_BACKGROUND_PREWARM_LIMIT,
): PlayerData[] {
  const managerTeamId = gameState.manager.team_id;
  if (!managerTeamId) {
    return [];
  }

  const managerSquadIds = new Set(
    selectManagerSquadPortraitPlayers(gameState).map((player) => player.id),
  );
  const nextFixture = gameState.league
    ? findNextFixture(gameState.league.fixtures, managerTeamId)
    : undefined;
  const nextOpponentTeamId = nextFixture
    ? nextFixture.home_team_id === managerTeamId
      ? nextFixture.away_team_id
      : nextFixture.home_team_id
    : null;

  return gameState.players
    .map((player, index) => ({ player, index }))
    .filter(
      ({ player }) =>
        isPortraitEligiblePlayer(player) && !managerSquadIds.has(player.id),
    )
    .sort((a, b) => {
      const aPriority = a.player.team_id === nextOpponentTeamId ? 0 : 1;
      const bPriority = b.player.team_id === nextOpponentTeamId ? 0 : 1;
      return aPriority - bPriority || a.index - b.index;
    })
    .slice(0, limit)
    .map(({ player }) => player);
}

function backgroundPortraitPrewarmKeyForPlayers(
  gameState: GameStateData,
  players: PlayerData[],
): string {
  return [
    gameState.manager.id ?? "-",
    gameState.manager.team_id ?? "-",
    gameState.clock.start_date,
    players.map((player) => player.id).join(","),
  ].join("|");
}

export function getBackgroundPortraitPrewarmKey(
  gameState: GameStateData,
  options: { limit?: number } = {},
): string | null {
  const players = selectBackgroundPortraitPlayers(gameState, options.limit);
  if (players.length === 0) {
    return null;
  }

  return backgroundPortraitPrewarmKeyForPlayers(gameState, players);
}

export async function prewarmPlayerPortraits(
  players: PlayerPortraitIdentity[],
): Promise<PrewarmPlayerPortraitsResponse | null> {
  if (!canGenerateRuntimePlayerPortraits() || players.length === 0) {
    return null;
  }

  return invoke<PrewarmPlayerPortraitsResponse>("prewarm_player_portraits", {
    requests: players.map(toPortraitRequest),
  }).catch((error) => {
    console.warn("Failed to prewarm runtime player portraits", error);
    return null;
  });
}

export async function prewarmManagerSquadPortraits(
  gameState: GameStateData,
): Promise<PrewarmPlayerPortraitsResponse | null> {
  const players = selectManagerSquadPortraitPlayers(gameState);
  const result = await prewarmPlayerPortraits(players);

  if (result) {
    console.info("[portraits] manager squad prewarm", {
      requested: result.requestedCount,
      generated: result.generatedCount,
      cached: result.cachedCount,
      failed: result.failedCount,
      renderMs: result.renderMs,
      elapsedMs: result.elapsedMs,
    });
  }

  return result;
}

export function queueBackgroundPortraitPrewarm(
  gameState: GameStateData,
  options: {
    delayMs?: number;
    batchSize?: number;
    batchDelayMs?: number;
    limit?: number;
  } = {},
): () => void {
  if (!canGenerateRuntimePlayerPortraits()) {
    return noop;
  }

  const players = selectBackgroundPortraitPlayers(gameState, options.limit);
  if (players.length === 0) {
    return noop;
  }

  const key = backgroundPortraitPrewarmKeyForPlayers(gameState, players);
  if (queuedBackgroundPrewarmKeys.has(key)) {
    return noop;
  }
  queuedBackgroundPrewarmKeys.add(key);

  const delayMs = options.delayMs ?? 400;
  const batchSize = options.batchSize ?? DEFAULT_BACKGROUND_PREWARM_BATCH_SIZE;
  const batchDelayMs =
    options.batchDelayMs ?? DEFAULT_BACKGROUND_PREWARM_BATCH_DELAY_MS;
  let cancelled = false;
  let timeoutId: ReturnType<typeof globalThis.setTimeout> | null = null;

  const finish = () => {
    queuedBackgroundPrewarmKeys.delete(key);
    if (timeoutId !== null) {
      globalThis.clearTimeout(timeoutId);
      timeoutId = null;
    }
  };

  const scheduleBatch = (startIndex: number, delay: number) => {
    timeoutId = globalThis.setTimeout(() => runBatch(startIndex), delay);
  };

  const runBatch = (startIndex: number) => {
    timeoutId = null;
    if (cancelled) {
      finish();
      return;
    }

    const batch = players.slice(startIndex, startIndex + batchSize);
    if (batch.length === 0) {
      finish();
      return;
    }

    void prewarmPlayerPortraits(batch)
      .then((result) => {
        if (cancelled) {
          finish();
          return;
        }

        if (!result) {
          finish();
          return;
        }

        console.debug("[portraits] background prewarm batch", {
          startIndex,
          requested: result.requestedCount,
          generated: result.generatedCount,
          cached: result.cachedCount,
          failed: result.failedCount,
          renderMs: result.renderMs,
          elapsedMs: result.elapsedMs,
        });

        if (startIndex + batchSize < players.length) {
          scheduleBatch(startIndex + batchSize, batchDelayMs);
        } else {
          finish();
        }
      })
      .catch((error) => {
        console.warn("Background runtime player portrait prewarm failed", error);
        finish();
      });
  };

  scheduleBatch(0, delayMs);

  return () => {
    cancelled = true;
    finish();
  };
}

export function getRuntimeGeneratedPlayerPortrait(
  player: PlayerPortraitIdentity,
): Promise<GeneratedPlayerPortrait | null> {
  if (!canGenerateRuntimePlayerPortraits()) {
    return Promise.resolve(null);
  }

  const key = runtimePortraitIdentityKey(player);
  const existing = runtimePortraitRequests.get(key);
  if (existing) {
    return existing;
  }

  const request = invoke<GeneratedPlayerPortraitCommandResponse>("generate_player_portrait", {
    request: toPortraitRequest(player),
  })
    .then((portrait) => ({
      ...portrait,
      imageUrl: portrait.dataUrl ?? convertFileSrc(portrait.cachePath),
    }))
    .catch((error) => {
      console.warn("Failed to generate runtime player portrait", error);
      return null;
    })
    .finally(() => {
      runtimePortraitRequests.delete(key);
    });

  runtimePortraitRequests.set(key, request);
  return request;
}
