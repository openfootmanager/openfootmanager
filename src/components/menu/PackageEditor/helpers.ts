import type {
  CompetitionDef,
  CompetitionFormat,
  CompetitionScope,
  CompetitionType,
  ConfederationDef,
  CountryDef,
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

export const PLAY_STYLES = ["Balanced", "Attacking", "Defensive", "Counter", "Pressing"];
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
