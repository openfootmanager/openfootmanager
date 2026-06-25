import type {
  CompetitionDef,
  ConfederationDef,
  CountryDef,
  NamesDefinition,
  PlayerDef,
  TeamDef,
  WorldMetaDef,
} from "./types";

export interface SamplePackage {
  meta: WorldMetaDef;
  confederations: ConfederationDef[];
  countries: CountryDef[];
  teams: TeamDef[];
  players: PlayerDef[];
  names: NamesDefinition;
  competitions: CompetitionDef[];
}

export const MINI_LEAGUE_SAMPLE: SamplePackage = {
  meta: {
    id: "northshire-mini-league",
    name: "Northshire Mini League",
    description:
      "A four-team example league for the Northshire region. Great starting point for learning the .ofm format.",
    version: "1.0.0",
    author: "",
    license: "CC0-1.0",
    packageType: "database",
    gameMinVersion: "0.3.0",
    baseYear: 2026,
    formatVersion: 1,
    defaultActiveRegions: [],
    defaultActiveCompetitions: ["northshire-premier"],
  },
  confederations: [{ id: "europe", name: "Europe" }],
  countries: [{ id: "ENG", name: "England", confederation: "europe" }],
  teams: [
    {
      id: "northshire-fc",
      name: "Northshire FC",
      shortName: "NSH",
      city: "Northshire",
      country: "ENG",
      colors: { primary: "#003366", secondary: "#ffffff" },
      playStyle: "Balanced",
      stadiumName: "Castle Park",
      reputationRange: [400, 650],
      financeRange: [800000, 3000000],
      logo: null,
    },
    {
      id: "westbrook-united",
      name: "Westbrook United",
      shortName: "WBU",
      city: "Westbrook",
      country: "ENG",
      colors: { primary: "#cc0000", secondary: "#ffffff" },
      playStyle: "Attacking",
      stadiumName: "Westbrook Arena",
      reputationRange: [350, 600],
      financeRange: [600000, 2500000],
      logo: null,
    },
    {
      id: "eastgate-city",
      name: "Eastgate City",
      shortName: "EGC",
      city: "Eastgate",
      country: "ENG",
      colors: { primary: "#0066cc", secondary: "#ffcc00" },
      playStyle: "Pressing",
      stadiumName: "City Ground",
      reputationRange: [300, 550],
      financeRange: [500000, 2000000],
      logo: null,
    },
    {
      id: "southfield-rovers",
      name: "Southfield Rovers",
      shortName: "SFR",
      city: "Southfield",
      country: "ENG",
      colors: { primary: "#006600", secondary: "#ffffff" },
      playStyle: "Defensive",
      stadiumName: "Rovers Stadium",
      reputationRange: [280, 500],
      financeRange: [400000, 1500000],
      logo: null,
    },
  ],
  players: [] as PlayerDef[],
  names: { version: 1, description: "", pools: {} },
  competitions: [
    {
      id: "northshire-premier",
      name: "Northshire Premier League",
      type: "League",
      scope: "Domestic",
      countryId: "ENG",
      priority: 10,
      format: { kind: "LeagueTable", legs: 2 },
      participants: {
        explicit: ["northshire-fc", "westbrook-united", "eastgate-city", "southfield-rovers"],
      },
      seasonStartMonth: 8,
      seasonStartDay: 1,
    },
  ],
};
