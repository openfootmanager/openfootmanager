import { useEffect, useState } from "react";

import { getYouthProspectOpinions } from "../services/scoutingService";
import type { PotentialOpinion } from "../store/generated/PotentialOpinion";

const NO_OPINIONS: Readonly<Record<string, PotentialOpinion>> = {};

/**
 * The reporting scout's opinion of each prospect in one youth scouting report, by prospect id,
 * from the backend's one rule. `messageId` null asks nothing (the open message is no youth
 * report). It asks again when `revision` changes (see `gameKnowledgeRevision`), and reads no
 * opinions until the answer for this report and this revision arrives, so another report's
 * opinions never show meanwhile or after a failed request.
 */
export function useYouthProspectOpinions(
  messageId: string | null,
  revision: string,
): Readonly<Record<string, PotentialOpinion>> {
  const [answer, setAnswer] = useState<{
    messageId: string;
    revision: string;
    opinions: Readonly<Record<string, PotentialOpinion>>;
  } | null>(null);

  useEffect(() => {
    if (messageId === null) return;
    let cancelled = false;
    getYouthProspectOpinions(messageId)
      .then((opinions) => {
        // An answer of nothing (a build without the command) reads as no opinions, never a crash.
        if (!cancelled) setAnswer({ messageId, revision, opinions: opinions ?? NO_OPINIONS });
      })
      .catch((error: unknown) => {
        console.error("Failed to read the scout's opinions of the youth prospects:", error);
      });
    return () => {
      cancelled = true;
    };
  }, [messageId, revision]);

  return answer !== null && answer.messageId === messageId && answer.revision === revision
    ? answer.opinions
    : NO_OPINIONS;
}
