import type { StaffData } from "../../../store/types";

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

export interface PlayerDef {
  id: string;
  name: string;
  firstName: string;
  lastName: string;
  club: string;
  nationality: string;
  position: Position;
  dateOfBirth: string | null;
  overall: number | null;
  attributes: PlayerAttributesDef | null;
  photo?: string | null;
  foot?: Footedness | null;
  youth?: boolean;
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

export type EditorView =
  | "home"
  | "edit"
  | "team"
  | "confederation"
  | "country"
  | "player"
  | "names-pool"
  | "competition"
  | "staff";

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
