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
  const [slice, setSlice] = useState<PlayersPage | null>(null);
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
        setSlice(result);
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

  return { query, patchQuery, slice, error, refetch };
}
