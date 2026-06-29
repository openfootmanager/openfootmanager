import { useEffect, useRef, useState } from "react";

const FLASH_DURATION_MS = 5000;

export interface FlashMessages {
  errorMsg: string | null;
  successMsg: string | null;
  flashError: (msg: string) => void;
  flashSuccess: (msg: string) => void;
}

/**
 * Transient toast-style messages that auto-dismiss after a fixed delay.
 *
 * Each channel (error / success) owns its own timer, cleared before being
 * re-armed so a rapid second flash can't be dismissed early by the previous
 * one's timer, and cleared on unmount to avoid a setState-after-unmount.
 * Mirrors the ref + cleanup pattern in useAutoCloseTimeout.
 */
export function useFlashMessages(): FlashMessages {
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  const errorTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const successTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (errorTimer.current !== null) clearTimeout(errorTimer.current);
      if (successTimer.current !== null) clearTimeout(successTimer.current);
    };
  }, []);

  function flashError(msg: string) {
    if (errorTimer.current !== null) clearTimeout(errorTimer.current);
    setErrorMsg(msg);
    errorTimer.current = setTimeout(() => {
      errorTimer.current = null;
      setErrorMsg(null);
    }, FLASH_DURATION_MS);
  }

  function flashSuccess(msg: string) {
    if (successTimer.current !== null) clearTimeout(successTimer.current);
    setSuccessMsg(msg);
    successTimer.current = setTimeout(() => {
      successTimer.current = null;
      setSuccessMsg(null);
    }, FLASH_DURATION_MS);
  }

  return { errorMsg, successMsg, flashError, flashSuccess };
}
