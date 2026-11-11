import { answeredOr } from "../../hooks/useKnowledgeAnswer";
import { usePlayerKnowledge } from "../../hooks/usePlayerKnowledge";
import { usePositionRatings } from "../../hooks/usePositionRatings";
import { usePotentialOpinion } from "../../hooks/usePotentialOpinion";
import { useVisibleWage } from "../../hooks/useVisibleWage";
import {
  gameKnowledgeRevision,
  HIDDEN_WAGE,
  knowsPlayerRating,
  knowsPrivateState,
  visibleTraits,
} from "../../lib/playerKnowledge";
import type { GameStateData, PlayerData } from "../../store/gameStore";
import { useScoutedReport } from "./useScoutedReport";

interface UsePlayerProfileKnowledgeArgs {
  player: PlayerData;
  gameState: GameStateData;
  /** His own squad's player: the club never needs a scout's report on him. */
  isManagerSquadProfile: boolean;
}

/**
 * What the manager's club may read about the player, keyed on the knowledge revision: how it knows
 * him, his wage, the opinion of his potential, his position ratings, a scout's report on him, and
 * what of his attributes and traits is readable. Each answer is loading, answered or failed; a
 * failed one shows "Couldn't load" and a retry, never the empty state that means the club knows
 * nothing (#1108).
 */
export function usePlayerProfileKnowledge({
  player,
  gameState,
  isManagerSquadProfile,
}: UsePlayerProfileKnowledgeArgs) {
  const revision = gameKnowledgeRevision(gameState, {
    playerClubId: player.team_id,
    wage: player.wage,
  });
  const knowledge = usePlayerKnowledge([player.id], revision)(player.id);
  const wageAnswer = useVisibleWage(player.id, revision);
  const visibleWage = answeredOr(wageAnswer, HIDDEN_WAGE);
  const wageRetry = wageAnswer.status === "failed" ? wageAnswer.retry : undefined;
  // The club's opinion of his potential: a range from his scout or his coaches, never the number.
  const opinionAnswer = usePotentialOpinion(player.id, revision);
  const potentialOpinion = answeredOr(opinionAnswer, null);
  // His natural position's weights and key attributes: neither follows the lineup, so the
  // knowledge revision is all the request needs.
  const ratingsAnswer = usePositionRatings(player.id, revision);
  const scoutedReport = useScoutedReport(
    player.id,
    !isManagerSquadProfile,
    gameState.clock.current_date,
  );
  const attributesPlayer = scoutedReport
    ? { ...player, attributes: scoutedReport.attributes }
    : player;

  return {
    knowledge,
    showsPrivateState: knowsPrivateState(knowledge),
    ratingKnown: knowsPlayerRating(knowledge),
    visibleWage,
    wageRetry,
    opinionAnswer,
    potentialOpinion,
    ratingsAnswer,
    scoutedReport,
    /** His attributes as the club may read them (a scout's report for a scouted player), or null. */
    readableAttributes:
      knowsPlayerRating(knowledge) || scoutedReport !== null ? attributesPlayer.attributes : null,
    traits: visibleTraits(player.traits ?? [], potentialOpinion),
  };
}

export type PlayerProfileKnowledge = ReturnType<typeof usePlayerProfileKnowledge>;
