import { useEffect, useState } from "react";

import { fetchTeamCard } from "../../services/teamsService";

interface Answer {
  teamId: string;
  revision: string;
  averageRating: number | null;
}

/**
 * The club's average rating as the backend's teams card gives it: a number for the manager's own
 * club and his division rivals, null for any other club. It asks again when `revision` moves (see
 * `gameKnowledgeRevision`), and reads null until the answer for this club and this revision lands,
 * so an average the club could see before a club change never shows after it, nor after a
 * failed request.
 */
export function useTeamAverageRating(teamId: string, revision: string): number | null {
  const [answer, setAnswer] = useState<Answer | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetchTeamCard(teamId)
      .then((card) => {
        if (!cancelled) setAnswer({ teamId, revision, averageRating: card?.avg_ovr ?? null });
      })
      .catch((error: unknown) => {
        console.error("Failed to read the club's card:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [teamId, revision]);

  return answer?.teamId === teamId && answer.revision === revision ? answer.averageRating : null;
}
