import type {
  CompetitionDef,
  CompetitionFormat,
  CompetitionScope,
  CompetitionType,
  ConfederationDef,
  CountryDef,
  FallbackLeagueConfig,
  NamesDefinition,
  ParticipantSpec,
  PlayerAttributesDef,
  PlayerDef,
  Position,
  SelectorKind,
  SelectorSpec,
  StaffDef,
  TeamDef,
  WorldMetaDef,
} from "./types";

/**
 * Club play styles, matching `play_style_from_str` in the generator exactly.
 * That function falls back to Balanced for any unrecognised name *silently*,
 * so an entry here that the engine does not know produces a Balanced club with
 * no error — which is what "Pressing" used to do.
 */
export const PLAY_STYLES = [
  "Balanced",
  "Attacking",
  "Defensive",
  "Possession",
  "Counter",
  "HighPress",
];
/**
 * Coerce a stored play style to one the engine and the editor both know.
 *
 * The editor used to offer "Pressing", which was never a `PlayStyle` variant —
 * the generator silently read it as Balanced. Packages authored then still
 * carry it, and binding a value the select does not list leaves the control
 * blank and writes the invalid value straight back on save.
 */
export function normalizePlayStyle(value: string | null | undefined): string {
  const trimmed = (value ?? "").trim();
  return PLAY_STYLES.includes(trimmed) ? trimmed : "Balanced";
}

export const PACKAGE_TYPES = ["database", "patch", "assets"];

export const POSITIONS: Position[] = [
  "Goalkeeper",
  "Defender",
  "Midfielder",
  "Forward",
  "RightBack",
  "CenterBack",
  "LeftBack",
  "RightWingBack",
  "LeftWingBack",
  "DefensiveMidfielder",
  "CentralMidfielder",
  "AttackingMidfielder",
  "RightMidfielder",
  "LeftMidfielder",
  "RightWinger",
  "LeftWinger",
  "Striker",
];

// Position badge colours live in src/lib/positionColors.ts, shared with the
// tactics pitch; re-exported here for the players list and the preview card.
export { POSITION_COLOR } from "../../../lib/positionColors";

// Player attribute groups shown both as editable sliders (PlayerForm) and as
// read-only bars (PlayerPreviewCard). Single source so the form and its preview
// render the same attributes in the same order.
export const PLAYER_ATTR_GROUPS = [
  { groupKey: "physical",   keys: ["pace", "stamina", "strength", "agility"] },
  { groupKey: "technical",  keys: ["passing", "shooting", "tackling", "dribbling", "defending"] },
  { groupKey: "mental",     keys: ["positioning", "vision", "decisions", "composure", "aggression", "teamwork", "leadership"] },
  { groupKey: "goalkeeper", keys: ["handling", "reflexes", "aerial"] },
] as const;

export type PlayerAttrKey = typeof PLAYER_ATTR_GROUPS[number]["keys"][number];

export const COMPETITION_TYPES: CompetitionType[] = [
  "League",
  "Cup",
  "ContinentalClub",
  "InternationalClub",
  "InternationalNation",
  "FriendlyCup",
];

export const COMPETITION_SCOPES: CompetitionScope[] = [
  "Domestic",
  "Regional",
  "Continental",
  "International",
];

export const COMPETITION_FORMATS: CompetitionFormat[] = [
  "LeagueTable",
  "Knockout",
  "GroupAndKnockout",
];

export const SELECTOR_KINDS: SelectorKind[] = [
  "topByReputation",
  "allInCountry",
  "allInRegion",
  "championsOf",
];

export function emptyMeta(): WorldMetaDef {
  return {
    id: "",
    name: "",
    description: "",
    version: "1.0.0",
    author: "",
    license: "",
    packageType: "database",
    gameMinVersion: "",
    baseYear: null,
    formatVersion: 1,
    defaultActiveRegions: [],
    defaultActiveCompetitions: [],
    logo: null,
  };
}

export function emptyTeam(): TeamDef {
  return {
    id: "",
    name: "",
    shortName: "",
    city: "",
    country: "",
    colors: { primary: "#cc0000", secondary: "#ffffff" },
    playStyle: "Balanced",
    stadiumName: "",
    reputationRange: null,
    financeRange: null,
    logo: null,
    kitPattern: null,
    foundedYear: null,
  };
}

export function emptyConfederation(): ConfederationDef {
  return { id: "", name: "" };
}

export function emptyCountry(): CountryDef {
  return { id: "", name: "", confederation: "" };
}

export function emptyAttributes(): PlayerAttributesDef {
  return {
    pace: 50,
    stamina: 50,
    strength: 50,
    agility: 50,
    passing: 50,
    shooting: 50,
    tackling: 50,
    dribbling: 50,
    defending: 50,
    positioning: 50,
    vision: 50,
    decisions: 50,
    composure: 50,
    aggression: 50,
    teamwork: 50,
    leadership: 50,
    handling: 50,
    reflexes: 50,
    aerial: 50,
  };
}

export function emptyPlayer(): PlayerDef {
  return {
    id: "",
    name: "",
    firstName: "",
    lastName: "",
    club: "",
    nationality: "",
    position: "Goalkeeper",
    dateOfBirth: null,
    overall: null,
    attributes: null,
    footedness: null,
  };
}

export const STAFF_ROLES = ["AssistantManager", "Coach", "Scout", "Physio"] as const;
export const COACHING_SPECIALIZATIONS = [
  "Fitness", "Technique", "Tactics", "Defending", "Attacking", "GoalKeeping", "Youth",
] as const;

export function emptyStaff(): StaffDef {
  return {
    id: "",
    firstName: "",
    lastName: "",
    club: "",
    nationality: "",
    role: "Coach",
    attributes: null,
    specialization: null,
    dateOfBirth: null,
    age: null,
  };
}

export function toSlug(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64);
}

/**
 * React key for an entity row in the editor's lists.
 *
 * The id is the right identity when there is one: it survives an insert or a
 * delete that shifts every index, so `EntityRow`'s delete-confirmation state
 * stays on the row that owns it. But an id is blank until the entity is named,
 * and a package can arrive with several already blank — `id` is `#[serde(default)]`
 * on the Rust side, and repairing such a package is exactly what the editor is
 * for. Two blank ids would collapse into one key and let that row state attach
 * to the wrong entity.
 *
 * Blank ids therefore fall back to the index, prefixed with a `#` that `toSlug`
 * can never emit, so a generated id cannot collide with a fallback key.
 */
export function entityRowKey(id: string, index: number): string {
  return id || `#row-${index}`;
}

export function emptyNamesDefinition(): NamesDefinition {
  return { version: 1, description: "", pools: {} };
}

export function emptyCompetition(): CompetitionDef {
  return {
    id: "",
    name: "",
    type: "League",
    scope: "Domestic",
    priority: 0,
    format: { kind: "LeagueTable" },
    participants: { explicit: [] },
  };
}

export function parseRangeBound(v: string): number | null {
  if (v === "") return null;
  const n = parseInt(v, 10);
  return Number.isNaN(n) ? null : n;
}

export function makeRange(a: number | null, b: number | null): [number, number] | null {
  if (a == null || b == null) return null;
  return [a, b];
}

export function parsePoolText(text: string): string[] {
  return text
    .split("\n")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

export function poolToText(names: string[]): string {
  return names.join("\n");
}

export function buildParticipantSpec(
  mode: "explicit" | "selector",
  explicitText: string,
  selector: SelectorSpec,
): ParticipantSpec {
  if (mode === "explicit") {
    return { explicit: parsePoolText(explicitText) };
  }
  return { selector };
}

/**
 * Apply a patch to the optional fallback-league config, collapsing back to
 * `null` once every field is blank. Each field is an override of an engine
 * default, so a config with nothing set carries no meaning — dropping it keeps
 * an empty object out of the package manifest.
 */
export function mergeFallbackLeague(
  current: FallbackLeagueConfig | null | undefined,
  patch: Partial<FallbackLeagueConfig>,
): FallbackLeagueConfig | null {
  const next: FallbackLeagueConfig = { ...(current ?? {}), ...patch };
  const isEmpty = !next.name && !next.legs && !next.scope;
  return isEmpty ? null : next;
}
