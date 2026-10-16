import { useCallback, useEffect, useState } from "react";

import {
  fetchPlayersPage,
  type PlayersPage,
  type PlayersPageQuery,
} from "../services/playersService";
import { getErrorMessage, resolveTranslatedErrorMessage } from "../utils/errorMessage";

type Translate = Parameters<typeof resolveTranslatedErrorMessage>[1];

/**
 * One filtered, sorted, paged list of players, answered by the backend for every screen that
 * lists them. The query is state; `patchQuery` changes it and goes back to page 1 unless the
 * change is the page itself; `refetch` asks again for the same query after the game changed.
 * An answer to an older query is dropped, so a slow response never overwrites a newer page.
 */
export function usePlayersPage(initialQuery: PlayersPageQuery, translate: Translate) {
  const [query, setQuery] = useState<PlayersPageQuery>(initialQuery);
  const [answer, setAnswer] = useState<{ query: PlayersPageQuery; page: PlayersPage } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refetchKey, setRefetchKey] = useState(0);

  const patchQuery = useCallback((patch: Partial<PlayersPageQuery>) => {
    setQuery((current) => ({ ...current, ...patch, ...("page" in patch ? {} : { page: 1 }) }));
  }, []);

  const refetch = useCallback(() => setRefetchKey((key) => key + 1), []);

  useEffect(() => {
    let cancelled = false;
    fetchPlayersPage(query)
      .then((result) => {
        if (cancelled) return;
        setAnswer({ query, page: result });
        setError(null);
      })
      .catch((failure) => {
        if (cancelled) return;
        setError(resolveTranslatedErrorMessage(getErrorMessage(failure), translate));
      });
    return () => {
      cancelled = true;
    };
  }, [query, refetchKey, translate]);

  // A game update can leave fewer players than the page the user is on (a day passes, a scout is
  // sent): the backend then answers that page empty, and pagination hides once one page is left.
  // Move the query back to the last page that has rows.
  useEffect(() => {
    if (answer === null || answer.query !== query) return;
    const lastPage = Math.max(1, Math.ceil(answer.page.total / query.page_size));
    if (query.page > lastPage) {
      setQuery((current) => (current.page > lastPage ? { ...current, page: lastPage } : current));
    }
  }, [answer, query]);

  // `answeredQuery` is the query the slice answers: a screen that swaps the kind of list can tell
  // the old list from the new one while the new one is still on its way.
  return {
    query,
    patchQuery,
    slice: answer?.page ?? null,
    answeredQuery: answer?.query ?? null,
    error,
    refetch,
  };
}
