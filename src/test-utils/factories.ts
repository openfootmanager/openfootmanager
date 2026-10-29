import type { PlayerData, SeasonContextData, TeamData } from "../store/gameStore";
import type { SquadFitness } from "../services/squadService";
import type { ConditionBand, ConditionStatus, RatedTone } from "../store/types";

export function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "GB",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 500000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 250000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000000", secondary: "#ffffff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

export function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "GB",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 60,
      stamina: 60,
      strength: 60,
      agility: 60,
      passing: 60,
      shooting: 60,
      tackling: 60,
      dribbling: 60,
      defending: 60,
      positioning: 60,
      vision: 60,
      decisions: 60,
      composure: 60,
      aggression: 60,
      teamwork: 60,
      leadership: 60,
      handling: 20,
      reflexes: 20,
      aerial: 60,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-1",
    retired: false,
    contract_end: "2027-06-30",
    wage: 12000,
    market_value: 350000,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

/**
 * The `season_context` a game state carries: the backend always sends one. The default is the
 * quiet start of a career (preseason, window closed, no dates); a test that cares about the season
 * says so with the fields it needs.
 */
export function createSeasonContext(overrides: Partial<SeasonContextData> = {}): SeasonContextData {
  return {
    phase: "Preseason",
    season_complete: false,
    season_start: null,
    season_end: null,
    days_until_season_start: null,
    transfer_window: {
      status: "Closed",
      opens_on: null,
      closes_on: null,
      days_until_opens: null,
      days_remaining: null,
    },
    ...overrides,
  };
}

/** A condition status as the backend sends it, for the band a test names. */
export function createConditionStatus(band: ConditionBand): ConditionStatus {
  const steps: Record<ConditionBand, RatedTone> = {
    Fresh: "good",
    Tiring: "fair",
    Exhausted: "low",
    Critical: "bad",
  };
  return { band, label_key: `be.condition.${band.toLowerCase()}`, tone: steps[band] };
}

/**
 * The band the backend gives a figure, for test data only: a fixture's average must carry the
 * band its number would get, or a test passes on a pairing the game never sends. The rule itself
 * is `fitness_status` in ofm_core's training/fitness_warnings.rs; the screens never compute it.
 */
function backendBandOf(condition: number): ConditionBand {
  const shown = Math.round(condition);
  if (shown < 25) return "Critical";
  if (shown < 40) return "Exhausted";
  if (shown < 75) return "Tiring";
  return "Fresh";
}

/** The backend's squad fitness answer; a test sets what it is about and leaves the rest quiet. */
export function createSquadFitness(overrides: Partial<SquadFitness> = {}): SquadFitness {
  const averageCondition =
    overrides.average_condition === undefined ? 70 : overrides.average_condition;
  return {
    players: 11,
    average_condition: averageCondition,
    // As the backend has it: the band of the average, and none without an average.
    average_status:
      averageCondition === null ? null : createConditionStatus(backendBandOf(averageCondition)),
    exhausted_count: 0,
    critical_count: 0,
    statuses: {},
    ...overrides,
  };
}
