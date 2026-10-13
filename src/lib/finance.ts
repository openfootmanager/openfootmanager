import type { PlayerData } from "../store/gameStore";

/** Stored wages and wage budgets are already weekly euros. */
export function weeklyWageAmount(amount: number): number {
  return Math.max(0, Math.floor(amount));
}

function clampPercent(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }

  return Math.max(0, Math.min(100, value));
}

export function getPlayerAnnualWageCommitment(player: PlayerData, teamId?: string | null): number {
  const annualWage = Math.max(0, player.wage);

  if (!teamId) {
    return annualWage;
  }

  const activeLoan = player.active_loan ?? null;
  if (!activeLoan) {
    return player.team_id === teamId ? annualWage : 0;
  }

  const loanTeamContributionPct = clampPercent(activeLoan.wage_contribution_pct);
  const loanTeamShare = Math.floor((annualWage * loanTeamContributionPct) / 100);

  if (activeLoan.loan_team_id === teamId) {
    return loanTeamShare;
  }

  if (activeLoan.parent_team_id === teamId) {
    return annualWage - loanTeamShare;
  }

  return 0;
}
