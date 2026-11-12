import { getPlayerCareer } from "../services/playerCareerService";
import type { PlayerCareer } from "../store/generated/PlayerCareer";
import { type KnowledgeAnswer, useKnowledgeAnswer } from "./useKnowledgeAnswer";

/**
 * One player's career from the backend's one projection (`ofm_core::player_career`): his seasons by
 * club with each move written where it happened, the same rows by club, and his totals; null for
 * an id no active player has. Asked again when `revision` changes (a move, a new day), and loading
 * and failed carry no career, so another player's never shows and a failed load is never "no
 * career" (`useKnowledgeAnswer`).
 */
export function usePlayerCareer(
  playerId: string,
  revision: string,
): KnowledgeAnswer<PlayerCareer | null> {
  return useKnowledgeAnswer(`career|${playerId}|${revision}`, () => getPlayerCareer(playerId));
}
