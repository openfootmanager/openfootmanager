import { useEffect, useState } from "react";

/** A list the backend owns, asked for again whenever `refreshKey` changes; empty if it fails. */
export function useBackendList<T>(fetchList: () => Promise<T[]>, refreshKey: string): T[] {
  const [list, setList] = useState<T[]>([]);

  useEffect(() => {
    let cancelled = false;
    fetchList()
      .then((fetched) => {
        if (!cancelled) setList(fetched);
      })
      .catch(() => {
        if (!cancelled) setList([]);
      });
    return () => {
      cancelled = true;
    };
    // fetchList is a module-level service function, so only the refresh key re-asks.
  }, [refreshKey]);

  return list;
}
