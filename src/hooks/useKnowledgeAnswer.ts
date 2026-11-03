import { useCallback, useEffect, useState } from "react";

/**
 * Where an answer about what the club knows stands: still on its way, answered (and the answer may
 * itself be "nothing known"), or failed. A failed load is never "nothing known": it carries no data
 * and offers a retry.
 */
export type KnowledgeAnswer<T> =
  | { status: "loading" }
  | { status: "answered"; value: T }
  | { status: "failed"; retry: () => void };

/**
 * Asks `load` for the answer to the question `key` names (a player and the game's knowledge
 * revision), and again whenever `key` changes or a retry is asked for. It fails closed: an answer
 * or a failure belongs to the question it was asked for, so another player's or another
 * revision's never shows, and a failure shows no data at all.
 *
 * `load` is read when `key` changes or a retry is asked for; it need not be stable.
 */
export function useKnowledgeAnswer<T>(key: string, load: () => Promise<T>): KnowledgeAnswer<T> {
  const [attempt, setAttempt] = useState(0);
  const [settled, setSettled] = useState<{
    key: string;
    attempt: number;
    outcome: { ok: true; value: T } | { ok: false };
  } | null>(null);
  const retry = useCallback(() => setAttempt((count) => count + 1), []);

  // `load` is not a dependency: it belongs to `key`, and asking again for every render's new
  // closure would ask forever.
  useEffect(() => {
    let cancelled = false;
    load()
      .then((value) => {
        if (!cancelled) setSettled({ key, attempt, outcome: { ok: true, value } });
      })
      .catch((error: unknown) => {
        console.error(`Failed to load ${key}:`, error);
        if (!cancelled) setSettled({ key, attempt, outcome: { ok: false } });
      });
    return () => {
      cancelled = true;
    };
  }, [key, attempt]);

  if (settled === null || settled.key !== key || settled.attempt !== attempt) {
    return { status: "loading" };
  }
  return settled.outcome.ok
    ? { status: "answered", value: settled.outcome.value }
    : { status: "failed", retry };
}

/** The answered value, or `fallback` while loading or after a failure. */
export function answeredOr<T>(answer: KnowledgeAnswer<T>, fallback: T): T {
  return answer.status === "answered" ? answer.value : fallback;
}
