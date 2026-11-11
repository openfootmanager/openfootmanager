import type { GameStateData } from "../../store/gameStore";

/**
 * One manager as the Managers directory shows him: the fields of the backend's `ManagerSummary`
 * (ofm-profile's manager plan, M2; its 1 Nov addendum), so the table reads the same row when M2's
 * `get_managers_page` replaces this module's source. M2 adds the reputation's tier and tone, and
 * the record's seasons; until then the reputation is the bare number and the table shows no
 * seasons, since counting them from stint dates here would be a rule the frontend doesn't own.
 */
export interface ManagerRow {
  id: string;
  display_name: string;
  nationality: string;
  /** None while he is between jobs. */
  club: { id: string; name: string } | null;
  /** Holding the job until a permanent appointment. */
  interim: boolean;
  reputation: number;
  /** His career record as stored: it moves at each season's end (#1019). */
  record: { matches: number; trophies: number };
}

/** A club with no manager, as the directory's vacancies list it. */
export interface VacancyRow {
  club_id: string;
  club_name: string;
  club_reputation: number;
}

export interface ManagersDirectory {
  managers: ManagerRow[];
  vacancies: VacancyRow[];
}

/**
 * The Managers directory, built from the game state until M2's `get_managers_page` answers it.
 * This is the one function that swap replaces.
 */
export function managersDirectory(gameState: GameStateData): ManagersDirectory {
  const clubs = new Map(gameState.teams.map((team) => [team.id, team]));
  const managers = (gameState.managers ?? [gameState.manager]).map((manager) => {
    const club = manager.team_id ? clubs.get(manager.team_id) : undefined;
    return {
      id: manager.id,
      display_name: manager.display_name,
      nationality: manager.nationality,
      club: club ? { id: club.id, name: club.name } : null,
      interim: manager.interim === true,
      reputation: manager.reputation,
      record: {
        matches: manager.career_stats.matches_managed,
        trophies: manager.career_stats.trophies,
      },
    };
  });
  const vacancies = gameState.teams
    .filter((team) => team.manager_id == null)
    .map((team) => ({ club_id: team.id, club_name: team.name, club_reputation: team.reputation }));
  return { managers, vacancies };
}
