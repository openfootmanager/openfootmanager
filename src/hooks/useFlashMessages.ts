import { useState } from "react";

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
 * NOTE: the timers intentionally have no cleanup and no clear-before-set, to
 * preserve the original WorldEditor behaviour exactly. This means a flash fired
 * shortly after another can be cleared early by the earlier timer, and a flash
 * fired right before unmount will call setState after unmount. Both are
 * pre-existing and left untouched by this faithful extraction.
 */
export function useFlashMessages(): FlashMessages {
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  function flashError(msg: string) {
    setErrorMsg(msg);
    setTimeout(() => setErrorMsg(null), FLASH_DURATION_MS);
  }

  function flashSuccess(msg: string) {
    setSuccessMsg(msg);
    setTimeout(() => setSuccessMsg(null), FLASH_DURATION_MS);
  }

  return { errorMsg, successMsg, flashError, flashSuccess };
}
