import { getPlayerCareer } from "../services/playerCareerService";
import type { PlayerCareer } from "../store/generated/PlayerCareer";
import { type KnowledgeAnswer } from "./useKnowledgeAnswer";

/**
 * One player's career from the backend's one projection. Scaffold: the real hook replaces it.
 */
export function usePlayerCareer(
  _playerId: string,
  _revision: string,
): KnowledgeAnswer<PlayerCareer | null> {
  void getPlayerCareer;
  return { status: "loading" };
}
