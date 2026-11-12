import type { PlayerData, StaffData, TeamData } from "../../store/gameStore";

/** Why a contract cannot be ended now, when it cannot. */
export type TerminationBlock = "no_contract" | "on_loan";

/**
 * What stops the manager releasing this player outright, if anything. Both follow the backend's own
 * refusals (`playerHasNoActiveContract`, `playerOnActiveLoan`), so the sheet and the actions menu
 * say so before a click instead of after it. The missing contract is named first.
 */
export function terminationBlockedBy(player: PlayerData): TerminationBlock | null {
  if (!player.contract_end) return "no_contract";
  if (player.active_loan) return "on_loan";
  return null;
}

/** What the club-wide previews read from the game. */
export interface ClubStateInputs {
  players: PlayerData[];
  staff: StaffData[];
  teams: Pick<TeamData, "id" | "wage_budget" | "formation" | "finance">[];
}

const loanKey = (loan: PlayerData["active_loan"]): string =>
  loan ? `${loan.parent_team_id}>${loan.loan_team_id}@${loan.wage_contribution_pct}` : "";

/**
 * What the club-wide previews depend on, as one string: everything the club's wage bill and its
 * squad-safety verdict read. That is its playing squad (who is in it, wages, contract ends,
 * injuries), the players loaned out or in whose wage it pays a share of (the loan's clubs and
 * share), its staff and their wages (`calc_wages` counts them), its wage budget, its formation
 * (the squad-safety verdict reads it) and its cash (the runway reads it). A signing, a
 * release, a hire or a loan made while the sheet is open, by the manager elsewhere or by an agent
 * over MCP, changes it, so both previews are asked again; other clubs' people and the order of the
 * lists do not.
 *
 * The game carries no revision counter to ask instead, so the key is built from the inputs.
 */
export function clubStateKey(game: ClubStateInputs, teamId: string | null): string {
  if (!teamId) return "";
  const players = game.players
    .filter(
      (player) =>
        player.team_id === teamId ||
        player.active_loan?.parent_team_id === teamId ||
        player.active_loan?.loan_team_id === teamId,
    )
    .map((player) =>
      [
        player.id,
        player.team_id ?? "",
        player.wage,
        player.contract_end ?? "",
        player.injury ? "injured" : "",
        loanKey(player.active_loan),
      ].join(":"),
    );
  const staff = game.staff
    .filter((member) => member.team_id === teamId)
    .map((member) => `staff:${member.id}:${member.wage}`);
  const team = game.teams.find((candidate) => candidate.id === teamId);
  return `${[...players, ...staff].sort().join("|")}#${team?.wage_budget ?? ""}#${team?.formation ?? ""}#${team?.finance ?? ""}`;
}
