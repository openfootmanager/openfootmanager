import type { CompetitionCalendarData, StaffData } from "../../../store/types";

export interface TeamColorsDef {
  primary: string;
  secondary: string;
}

export type KitPattern = "Solid" | "Stripes" | "Hoops" | "HalfAndHalf" | "Diagonal";

export interface TeamDef {
  id: string;
  name: string;
  shortName: string;
  city: string;
  country: string;
  colors: TeamColorsDef;
  playStyle: string;
  stadiumName: string;
  reputationRange: [number, number] | null;
  financeRange: [number, number] | null;
  logo: string | null;
  kitPattern: KitPattern | null;
  foundedYear: number | null;
}

export interface WorldMetaDef {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  license: string;
  packageType: string;
  gameMinVersion: string;
  baseYear: number | null;
  formatVersion: number;
  defaultActiveRegions: string[];
  defaultActiveCompetitions: string[];
  logo: string | null;
  // Optional overrides for the league auto-generated when a package has teams
  // but no competitions. Edited from the Metadata section.
  fallbackLeague?: FallbackLeagueConfig | null;
}

export interface FallbackLeagueConfig {
  name?: string | null;
  legs?: number | null;
  scope?: CompetitionScope | null;
}

export interface PackageIssue {
  code: string;
  file: string;
  params: Record<string, string>;
}

// ---------------------------------------------------------------------------
// Confederation & Country
// ---------------------------------------------------------------------------

export interface ConfederationDef {
  id: string;
  name: string;
}

export interface CountryDef {
  id: string;
  name: string;
  confederation: string;
}

// ---------------------------------------------------------------------------
// Player
// ---------------------------------------------------------------------------

export type Position =
  | "Goalkeeper"
  | "Defender"
  | "Midfielder"
  | "Forward"
  | "RightBack"
  | "CenterBack"
  | "LeftBack"
  | "RightWingBack"
  | "LeftWingBack"
  | "DefensiveMidfielder"
  | "CentralMidfielder"
  | "AttackingMidfielder"
  | "RightMidfielder"
  | "LeftMidfielder"
  | "RightWinger"
  | "LeftWinger"
  | "Striker";

export interface PlayerAttributesDef {
  pace: number;
  stamina: number;
  strength: number;
  agility: number;
  passing: number;
  shooting: number;
  tackling: number;
  dribbling: number;
  defending: number;
  positioning: number;
  vision: number;
  decisions: number;
  composure: number;
  aggression: number;
  teamwork: number;
  leadership: number;
  handling: number;
  reflexes: number;
  aerial: number;
}

export type Footedness = "Left" | "Right" | "Both";

/** One spell in a player's authored career history. */
export interface PlayerCareerEntryDef {
  /** The calendar year the season began in. */
  season: number;
  /**
   * A team defined in this package. Absent for a club the package does not define:
   * `teamName` is what the profile shows either way.
   */
  teamId?: string | null;
  teamName: string;
  appearances: number;
  goals: number;
  assists: number;
}

export interface PlayerDef {
  id: string;
  name: string;
  firstName: string;
  lastName: string;
  club: string;
  nationality: string;
  position: Position;
  dateOfBirth: string | null;
  /**
   * Age at the world's opening year, an alternative to `dateOfBirth` that the
   * generator derives a birth date from. The editor authors `dateOfBirth`, but
   * a hand-written package may carry this instead, so the type has to admit it
   * or the field is invisible to everything here.
   */
  age?: number | null;
  overall: number | null;
  /**
   * Career ceiling, independent of whether ability was given as `overall` or an
   * `attributes` block — which is why it sits outside the choice between them.
   * Null hands it back to the engine's age-based roll, as every package written
   * before the field existed does.
   */
  potential: number | null;
  attributes: PlayerAttributesDef | null;
  photo?: string | null;
  footedness?: Footedness | null;
  youth?: boolean;
  /**
   * Contract, wage and status. Every one is optional and absent means the engine
   * generates it as it always did, so a package written before these fields
   * existed is unchanged. The limits and the either-or between `contractEnd` and
   * `contractLength` live in the backend validator, not here.
   */
  contractStart?: string | null;
  contractEnd?: string | null;
  contractLength?: number | null;
  /** Weekly. */
  wage?: number | null;
  value?: number | null;
  weakFoot?: number | null;
  alternatePositions?: Position[];
  condition?: number | null;
  morale?: number | null;
  careerHistory?: PlayerCareerEntryDef[];
}

// ---------------------------------------------------------------------------
// Staff
// ---------------------------------------------------------------------------

export type StaffRole = StaffData["role"];

export interface StaffAttributesDef {
  coaching: number;
  judgingAbility: number;
  judgingPotential: number;
  physiotherapy: number;
}

export interface StaffDef {
  id: string;
  firstName: string;
  lastName: string;
  club: string;
  nationality: string;
  role: StaffRole;
  attributes: StaffAttributesDef | null;
  specialization?: string | null;
  dateOfBirth?: string | null;
  age?: number | null;
}

// ---------------------------------------------------------------------------
// Names
// ---------------------------------------------------------------------------

export interface NamePool {
  first_names: string[];
  last_names: string[];
}

export interface NamesDefinition {
  version: number;
  description: string;
  pools: Record<string, NamePool>;
}

// ---------------------------------------------------------------------------
// Competition
// ---------------------------------------------------------------------------

export type CompetitionType =
  | "League"
  | "Cup"
  | "ContinentalClub"
  | "InternationalClub"
  | "InternationalNation"
  | "FriendlyCup";

export type CompetitionScope = "Domestic" | "Regional" | "Continental" | "International";
export type CompetitionFormat = "LeagueTable" | "Knockout" | "GroupAndKnockout";
export type SelectorKind = "topByReputation" | "allInCountry" | "allInRegion" | "championsOf";

export interface FormatDef {
  kind: CompetitionFormat;
  legs?: number;
  groupSize?: number;
  qualifiersPerGroup?: number;
  bestThirdQualifiers?: number;
}

export interface SelectorSpec {
  kind: SelectorKind;
  country?: string;
  region?: string;
  count?: number;
  excludeCompetitions: string[];
  sourceCompetition?: string;
}

export interface ParticipantSpec {
  explicit?: string[];
  selector?: SelectorSpec;
}

export interface CompetitionDef {
  id: string;
  name: string;
  type: CompetitionType;
  scope: CompetitionScope;
  regionId?: string;
  countryId?: string;
  requiredRegionIds?: string[];
  priority: number;
  format: FormatDef;
  participants: ParticipantSpec;
  berths?: unknown[];
  calendar?: CompetitionCalendarData | null;
  seasonStartMonth?: number;
  seasonStartDay?: number;
  nameKey?: string;
  logo?: string | null;
}

// ---------------------------------------------------------------------------
// Aggregate project data
// ---------------------------------------------------------------------------

export interface PackageProjectData {
  meta: WorldMetaDef;
  confederations: ConfederationDef[];
  countries: CountryDef[];
  teams: TeamDef[];
  players: PlayerDef[];
  staff: StaffDef[];
  names: NamesDefinition | null;
  competitions: CompetitionDef[];
  issues: PackageIssue[];
}

export type EditTab =
  | "metadata"
  | "confederations"
  | "countries"
  | "teams"
  | "players"
  | "youth"
  | "staff"
  | "names"
  | "competitions";
