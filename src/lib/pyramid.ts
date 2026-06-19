import type { LeagueData } from "../store/gameStore";

export interface PromotionRelegationZones {
  promotionSlots: number;
  relegationSlots: number;
}

const NO_ZONES: PromotionRelegationZones = { promotionSlots: 0, relegationSlots: 0 };

/** Mirrors the backend's relegation_count: ~one slot per five clubs in the smaller division, at least one. */
function swapCount(upperSize: number, lowerSize: number): number {
  return Math.max(1, Math.floor(Math.min(upperSize, lowerSize) / 5));
}

function isLeagueTable(competition: LeagueData): boolean {
  return !competition.rules || competition.rules.format === "LeagueTable";
}

function divisionSize(competition: LeagueData): number {
  return competition.participant_ids?.length ?? competition.standings.length;
}

/**
 * Promotion/relegation zone sizes for a division's standings table, derived
 * from its neighbours in the country pyramid (league competitions sharing a
 * country, ordered by priority — lower priority is the higher tier).
 */
export function getPromotionRelegationZones(
  competitions: LeagueData[],
  competition: LeagueData,
): PromotionRelegationZones {
  if (!competition.country_id || !isLeagueTable(competition)) {
    return NO_ZONES;
  }

  const myPriority = competition.priority ?? 0;
  const siblings = competitions.filter(
    (other) =>
      other.id !== competition.id &&
      other.country_id === competition.country_id &&
      isLeagueTable(other),
  );

  const above = siblings
    .filter((other) => (other.priority ?? 0) < myPriority)
    .sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0))[0];
  const below = siblings
    .filter((other) => (other.priority ?? 0) > myPriority)
    .sort((a, b) => (a.priority ?? 0) - (b.priority ?? 0))[0];

  return {
    promotionSlots: above ? swapCount(divisionSize(above), divisionSize(competition)) : 0,
    relegationSlots: below ? swapCount(divisionSize(competition), divisionSize(below)) : 0,
  };
}
