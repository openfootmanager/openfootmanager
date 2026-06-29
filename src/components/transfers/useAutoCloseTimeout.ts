import { useEffect, useRef } from "react";

interface UseAutoCloseTimeoutResult {
  scheduleAutoClose: (callback: () => void, delayMs: number) => void;
  clearAutoClose: () => void;
}

/**
 * Schedules a one-shot timeout (e.g. to auto-close a negotiation modal after
 * an accepted deal) and guarantees it is cleared on unmount, preventing a
 * setState-after-unmount when the component leaves the tree before the timer
 * fires. Mirrors the ref + cleanup pattern in useTransferBidFlow.
 */
export function useAutoCloseTimeout(): UseAutoCloseTimeoutResult {
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearAutoClose = (): void => {
    if (timeoutRef.current !== null) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  };

  useEffect(() => clearAutoClose, []);

  const scheduleAutoClose = (callback: () => void, delayMs: number): void => {
    clearAutoClose();
    timeoutRef.current = setTimeout(() => {
      timeoutRef.current = null;
      callback();
    }, delayMs);
  };

  return { scheduleAutoClose, clearAutoClose };
}
