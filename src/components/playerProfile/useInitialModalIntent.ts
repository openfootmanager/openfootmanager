import { useEffect, useState } from "react";

interface UseInitialModalIntentArgs {
  /** The caller asked for this modal — e.g. the player arrived from an inbox action. */
  requested: boolean;
  /** Whether this manager may act on the player at all. */
  allowed: boolean;
  isOpen: boolean;
  open: () => void;
  /**
   * Changing this arms the intent again. Shared by every intent on a profile,
   * so switching player — or switching which modal was asked for — re-arms all
   * of them together, as it did when one effect reset them both.
   */
  resetKey: string;
}

/**
 * Open a modal once, on arrival, when the caller asked for it.
 *
 * "Once" is the whole point: without the consumed flag the effect would reopen
 * the modal every time the player closed it, since the prop asking for it never
 * goes away.
 */
export function useInitialModalIntent({
  requested,
  allowed,
  isOpen,
  open,
  resetKey,
}: UseInitialModalIntentArgs): void {
  const [hasConsumed, setHasConsumed] = useState(false);

  useEffect(() => {
    setHasConsumed(false);
  }, [resetKey]);

  useEffect(() => {
    if (!allowed || !requested || isOpen || hasConsumed) {
      return;
    }

    setHasConsumed(true);
    open();
    // `open` is a fresh closure each render and is deliberately not a dep:
    // including it would re-run this on every render.
  }, [allowed, hasConsumed, isOpen, requested]);
}
