import { getPositionRatings } from "../services/positionService";
import type { PositionRatings } from "../store/generated/PositionRatings";
import { type KnowledgeAnswer, useKnowledgeAnswer } from "./useKnowledgeAnswer";

/**
 * One player's ratings at the positions he plays, from the backend's one rule, or null when the
 * club knows nothing of his attributes. Asked again when `revision` changes (see
 * `gameKnowledgeRevision`); loading and failed carry no ratings, so another player's never shows,
 * and a failed load is never "nothing known" (`useKnowledgeAnswer`).
 */
export function usePositionRatings(
  playerId: string,
  revision: string,
): KnowledgeAnswer<PositionRatings | null> {
  return useKnowledgeAnswer(`positions|${playerId}|${revision}`, () =>
    getPositionRatings(playerId).then((ratings) => ratings ?? null),
  );
}
