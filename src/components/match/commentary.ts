import type { TFunction } from "i18next";
import type {
  MatchEvent,
  MatchSnapshot,
  EventDetail,
  DangerBand,
  SaveQuality,
  FoulSeverity,
  GoalContext,
} from "./types";
import { getPlayerName } from "./helpers";

/** Event types that get the full headline + prose treatment. */
const COMMENTARY_EVENTS = new Set([
  "Goal",
  "PenaltyGoal",
  "PenaltyMiss",
  "PenaltyAwarded",
  "ShotSaved",
  "ShotOffTarget",
  "ShotBlocked",
  "Foul",
  "YellowCard",
  "RedCard",
  "SecondYellow",
  "Injury",
  "Substitution",
  "KickOff",
  "HalfTime",
  "SecondHalfStart",
  "FullTime",
]);

export interface Commentary {
  headline: string;
  line: string;
}

/** Stable, RNG-free hash so a given event always renders the same variant. */
function hashEvent(evt: MatchEvent): number {
  const key = `${evt.minute}|${evt.event_type}|${evt.player_id ?? ""}`;
  let h = 5381;
  for (let i = 0; i < key.length; i++) {
    h = ((h << 5) + h + key.charCodeAt(i)) | 0;
  }
  return Math.abs(h);
}

/**
 * Map an event's truthful detail to a commentary sub-key (camelCase to match
 * the i18n keys). Returns null when the base key should be used.
 */
function detailVariant(detail: EventDetail | null | undefined): string | null {
  if (!detail) return null;
  if ("Shot" in detail) {
    const map: Record<DangerBand, string> = {
      Speculative: "speculative",
      Decent: "decent",
      BigChance: "bigChance",
    };
    return map[detail.Shot.danger] ?? null;
  }
  if ("Save" in detail) {
    const map: Record<SaveQuality, string> = {
      Routine: "routine",
      Strong: "strong",
      WorldClass: "worldClass",
    };
    return map[detail.Save.quality] ?? null;
  }
  if ("Foul" in detail) {
    const map: Record<FoulSeverity, string | null> = {
      Soft: null,
      Hard: "hard",
      Reckless: "reckless",
    };
    const val = map[detail.Foul.severity];
    return val !== undefined ? val : null;
  }
  if ("Goal" in detail) {
    const map: Record<GoalContext, string> = {
      Opener: "opener",
      Equaliser: "equaliser",
      Extends: "extends",
      Consolation: "consolation",
    };
    return map[detail.Goal.context] ?? null;
  }
  return null;
}

/** Count goals scored by a player up to and including this event. */
function goalTally(evt: MatchEvent, snapshot: MatchSnapshot): number {
  if (!evt.player_id) return 0;
  // `minute <=` (not an index/identity comparison) is intentional: the rendered
  // event is not always reference-identical to the entry in snapshot.events, so
  // indexOf would fail. The engine resolves at most one shot per minute, so a
  // same-minute same-player double goal cannot occur and this cannot overcount.
  return snapshot.events.filter(
    (e) =>
      (e.event_type === "Goal" || e.event_type === "PenaltyGoal") &&
      e.player_id === evt.player_id &&
      e.minute <= evt.minute,
  ).length;
}

/**
 * Resolve the variant sub-key, with goal tally (hat-trick/brace) taking
 * precedence over goal context.
 */
function variantKey(evt: MatchEvent, snapshot: MatchSnapshot): string | null {
  if (evt.event_type === "Goal" || evt.event_type === "PenaltyGoal") {
    const tally = goalTally(evt, snapshot);
    if (tally >= 3) return "hattrick";
    if (tally === 2) return "brace";
  }
  return detailVariant(evt.detail);
}

/** Manual interpolation since the variant string is a value, not a key. */
function interpolate(template: string, tokens: Record<string, string>): string {
  return template.replace(/\{\{(\w+)\}\}/g, (_, name: string) => tokens[name] ?? "");
}

function pickLine(
  t: TFunction,
  baseKey: string,
  variant: string | null,
  hash: number,
  tokens: Record<string, string>,
): Commentary | null {
  // Try the refined variant first, then fall back to the base key.
  const candidates = variant ? [`${baseKey}.${variant}`, baseKey] : [baseKey];
  for (const key of candidates) {
    const lines = t(`${key}.lines`, { returnObjects: true }) as
      | Record<string, string>
      | string;
    if (!lines || typeof lines !== "object") continue;
    const values = Object.values(lines);
    if (values.length === 0) continue;
    const template = values[hash % values.length];
    if (typeof template !== "string") continue;
    const headline = t(`${key}.headline`, { defaultValue: "" });
    return { headline, line: interpolate(template, tokens) };
  }
  return null;
}

export function getCommentary(
  evt: MatchEvent,
  snapshot: MatchSnapshot,
  t: TFunction,
): Commentary | null {
  if (!COMMENTARY_EVENTS.has(evt.event_type)) return null;

  const isHome = evt.side === "Home";
  const team = isHome ? snapshot.home_team.name : snapshot.away_team.name;
  const opponent = isHome ? snapshot.away_team.name : snapshot.home_team.name;
  const player = getPlayerName(snapshot, evt.player_id);
  const victim = getPlayerName(snapshot, evt.secondary_player_id);

  const tokens: Record<string, string> = { team, opponent, player, victim };
  const baseKey = `match.commentary.${evt.event_type}`;
  const variant = variantKey(evt, snapshot);
  const hash = hashEvent(evt);

  return pickLine(t, baseKey, variant, hash, tokens);
}
