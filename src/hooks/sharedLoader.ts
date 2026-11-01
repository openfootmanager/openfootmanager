/** How long to wait before asking again after the nth failure in a row; the last delay repeats. */
const RETRY_DELAYS_MS = [1_000, 2_000, 5_000, 15_000];

export interface SharedLoader<T> {
  /**
   * Starts showing `key`'s value to `show` (at once if it is cached) and asks the backend for it if
   * nobody has. Returns the unsubscribe.
   */
  subscribe: (key: string, show: (value: T) => void) => () => void;
  /** The cached value for `key`, or undefined before it arrives. */
  cached: (key: string) => T | undefined;
  /** Forgets everything, so a test can start from the first request again. */
  reset: () => void;
}

/**
 * One backend value per key, shared by every screen that mounts a hook for it: a request serves all
 * mounts. A failed request is dropped so the next mount asks again, and while a screen stays
 * mounted it asks again on its own (a screen that waits for the value would otherwise stay disabled
 * for good); whichever request finally succeeds hands the value to every mount of its key,
 * including one whose own request failed. Cancelled, and failures forgotten, when the last mount
 * leaves.
 */
export function createSharedLoader<T>(
  fetchValue: (key: string) => Promise<T>,
  describe: string,
): SharedLoader<T> {
  const pending = new Map<string, Promise<T>>();
  const resolved = new Map<string, T>();
  const mounted = new Set<{ key: string; show: (value: T) => void }>();
  let failures = 0;
  let retryTimer: ReturnType<typeof setTimeout> | null = null;

  function cancelRetry(): void {
    if (retryTimer !== null) clearTimeout(retryTimer);
    retryTimer = null;
  }

  function mountedKeys(): Set<string> {
    return new Set([...mounted].map((mount) => mount.key));
  }

  function scheduleRetry(): void {
    if (retryTimer !== null || mounted.size === 0) return;
    const delay = RETRY_DELAYS_MS[Math.min(failures - 1, RETRY_DELAYS_MS.length - 1)];
    retryTimer = setTimeout(() => {
      retryTimer = null;
      // Everyone left, or another request already won: nothing to ask for.
      for (const key of mountedKeys()) {
        if (!resolved.has(key)) load(key).catch(report);
      }
    }, delay);
  }

  function report(error: unknown): void {
    console.error(`Failed to load the ${describe}:`, error);
  }

  function load(key: string): Promise<T> {
    if (resolved.has(key)) return Promise.resolve(resolved.get(key) as T);
    let request = pending.get(key);
    if (request) return request;
    request = fetchValue(key)
      .then((value) => {
        resolved.set(key, value);
        // The value is cached now; a finished request left here would answer a later subscribe
        // with a promise that tells nobody.
        pending.delete(key);
        // A key no mount shows any more (the club's squad moved on) is not kept.
        const live = mountedKeys();
        for (const stale of resolved.keys())
          if (stale !== key && !live.has(stale)) resolved.delete(stale);
        for (const mount of mounted) if (mount.key === key) mount.show(value);
        // Another mounted key may still be waiting on its retry: only stop when none is.
        if ([...live].every((mountedKey) => resolved.has(mountedKey))) {
          failures = 0;
          cancelRetry();
        }
        return value;
      })
      .catch((error: unknown) => {
        pending.delete(key);
        failures += 1;
        scheduleRetry();
        throw error;
      });
    pending.set(key, request);
    return request;
  }

  return {
    subscribe(key, show) {
      const mount = { key, show };
      mounted.add(mount);
      // The value can land between a mount's render and its effect; nothing would tell it then.
      if (resolved.has(key)) show(resolved.get(key) as T);
      load(key).catch(report);
      return () => {
        mounted.delete(mount);
        // No screen is waiting any more: a timer left running would fire on a later mount's behalf
        // at the old deadline, and the old failures would stretch that mount's first retry.
        if (mounted.size === 0) {
          cancelRetry();
          failures = 0;
        }
      };
    },
    cached: (key) => resolved.get(key),
    reset() {
      pending.clear();
      resolved.clear();
      failures = 0;
      cancelRetry();
      mounted.clear();
    },
  };
}
