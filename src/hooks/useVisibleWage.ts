import { HIDDEN_WAGE } from "../lib/playerKnowledge";
import { getVisibleWage, type VisibleWage } from "../services/scoutingService";
import { type KnowledgeAnswer, useKnowledgeAnswer } from "./useKnowledgeAnswer";

/**
 * One player's weekly wage as the club may see it (exact, approximate or hidden), from the backend,
 * asked again when `revision` changes. While the answer is on its way the state is loading and
 * after a failure it is failed (with a retry): neither carries a figure, so another player's,
 * career's or day's never flashes on screen, and neither reads as hidden (`useKnowledgeAnswer`).
 */
export function useVisibleWage(playerId: string, revision: string): KnowledgeAnswer<VisibleWage> {
  return useKnowledgeAnswer(`wage|${playerId}|${revision}`, () =>
    getVisibleWage(playerId).then((answer) => answer ?? HIDDEN_WAGE),
  );
}
