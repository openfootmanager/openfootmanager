import { describe, expect, it } from "vitest";

import { buildPlayerProfileRelationship } from "./PlayerProfile.viewModel";
import type { GameStateData, PlayerData } from "../../store/gameStore";

function player(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    team_id: "team-1",
    retired: false,
    ...overrides,
  } as PlayerData;
}

function gameState(overrides: Partial<GameStateData> = {}): GameStateData {
  return {
    manager: { team_id: "team-1" },
    staff: [],
    ...overrides,
  } as GameStateData;
}

const assistant = {
  id: "staff-1",
  team_id: "team-1",
  role: "AssistantManager",
};

describe("buildPlayerProfileRelationship", () => {
  describe("who may act on the contract", () => {
    // The distinction that caused a wrong test elsewhere: the club id settles
    // ownership, so `isOwnClub: false` does not make a player at the manager's
    // own club off limits.
    it("counts a player at the manager's club as theirs even when isOwnClub is false", () => {
      const result = buildPlayerProfileRelationship(
        player(),
        gameState(),
        false,
      );

      expect(result.isContractOwnerClub).toBe(true);
      expect(result.isManagerOwnedProfile).toBe(true);
    });

    it("does not claim a player at another club", () => {
      const result = buildPlayerProfileRelationship(
        player({ team_id: "team-2" }),
        gameState(),
        false,
      );

      expect(result.isManagerOwnedProfile).toBe(false);
      expect(result.isManagerSquadProfile).toBe(false);
    });

    it("trusts isOwnClub when the club ids do not settle it", () => {
      const result = buildPlayerProfileRelationship(
        player({ team_id: "team-2" }),
        gameState(),
        true,
      );

      expect(result.isManagerOwnedProfile).toBe(true);
    });

    it("has no owner to compare against for an unemployed manager", () => {
      const result = buildPlayerProfileRelationship(
        player(),
        gameState({ manager: { team_id: null } } as Partial<GameStateData>),
        false,
      );

      expect(result.isContractOwnerClub).toBe(false);
      expect(result.isManagerOwnedProfile).toBe(false);
    });
  });

  describe("loans", () => {
    const loanedIn = player({
      team_id: "team-1",
      active_loan: { parent_team_id: "team-2", loan_team_id: "team-1" },
    } as Partial<PlayerData>);

    // A loaned-in player turns out every week without the contract being the
    // manager's to touch — the two answers have to differ.
    it("separates playing for the manager from being contracted to them", () => {
      const result = buildPlayerProfileRelationship(loanedIn, gameState(), true);

      expect(result.isManagerSquadProfile).toBe(true);
      expect(result.isManagerOwnedProfile).toBe(false);
      expect(result.isManagerLoanClub).toBe(true);
    });

    it("keeps the contract with the parent club when the manager owns it", () => {
      const loanedOut = player({
        team_id: "team-2",
        active_loan: { parent_team_id: "team-1", loan_team_id: "team-2" },
      } as Partial<PlayerData>);

      const result = buildPlayerProfileRelationship(
        loanedOut,
        gameState(),
        false,
      );

      expect(result.contractOwnerTeamId).toBe("team-1");
      expect(result.isManagerOwnedProfile).toBe(true);
      expect(result.isManagerLoanClub).toBe(false);
    });

    it("will not let isOwnClub override a loan's parent club", () => {
      const otherClubsLoan = player({
        team_id: "team-3",
        active_loan: { parent_team_id: "team-2", loan_team_id: "team-3" },
      } as Partial<PlayerData>);

      const result = buildPlayerProfileRelationship(
        otherClubsLoan,
        gameState(),
        true,
      );

      expect(result.isManagerOwnedProfile).toBe(false);
    });
  });

  describe("free agency", () => {
    it("treats a club-less player as a free agent", () => {
      const result = buildPlayerProfileRelationship(
        player({ team_id: null }),
        gameState(),
        false,
      );

      expect(result.isFreeAgent).toBe(true);
    });

    it("does not offer a retired player as a free agent", () => {
      const result = buildPlayerProfileRelationship(
        player({ team_id: null, retired: true }),
        gameState(),
        false,
      );

      expect(result.isFreeAgent).toBe(false);
    });
  });

  describe("delegation", () => {
    it("finds an assistant at the manager's own club", () => {
      const result = buildPlayerProfileRelationship(
        player(),
        gameState({ staff: [assistant] } as Partial<GameStateData>),
        true,
      );

      expect(result.hasAssistantManager).toBe(true);
    });

    it("ignores an assistant employed by another club", () => {
      const result = buildPlayerProfileRelationship(
        player(),
        gameState({
          staff: [{ ...assistant, team_id: "team-2" }],
        } as Partial<GameStateData>),
        true,
      );

      expect(result.hasAssistantManager).toBe(false);
    });

    it("ignores staff who are not assistant managers", () => {
      const result = buildPlayerProfileRelationship(
        player(),
        gameState({
          staff: [{ ...assistant, role: "Scout" }],
        } as Partial<GameStateData>),
        true,
      );

      expect(result.hasAssistantManager).toBe(false);
    });
  });

  it("reports a let-expire intent the manager already set", () => {
    const result = buildPlayerProfileRelationship(
      player({
        morale_core: { renewal_state: { exit_intent: { kind: "let_expire" } } },
      } as Partial<PlayerData>),
      gameState(),
      true,
    );

    expect(result.hasLetExpireIntent).toBe(true);
  });
});
