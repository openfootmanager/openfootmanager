import { getPotentialOpinion } from "../services/scoutingService";
import type { PotentialOpinion } from "../store/generated/PotentialOpinion";
import { type KnowledgeAnswer, useKnowledgeAnswer } from "./useKnowledgeAnswer";

/**
 * The club's opinion of one player's potential, from the backend's one rule: a range, never the
 * number, or null when the club holds none. Asked again when `revision` changes (see
 * `gameKnowledgeRevision`); loading and failed carry no opinion, so another player's or another
 * club's never shows, and a failed load is never "no opinion" (`useKnowledgeAnswer`).
 */
export function usePotentialOpinion(
  playerId: string,
  revision: string,
): KnowledgeAnswer<PotentialOpinion | null> {
  return useKnowledgeAnswer(`opinion|${playerId}|${revision}`, () => getPotentialOpinion(playerId));
}
