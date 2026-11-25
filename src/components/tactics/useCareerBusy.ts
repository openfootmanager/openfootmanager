import { useCallback, useRef, useState } from "react";

import { useGameStore } from "../../store/gameStore";

/**
 * Whether a write to the team's tactic that the open career started is still out. Each operation is
 * counted under the career epoch it began in: a career change leaves a held operation of the old
 * career behind without keeping the new career's controls off, and when it finally completes it
 * removes only itself, never an operation the new career has started since.
 */
export function useCareerBusy(): { busy: boolean; begin: () => () => void } {
  const careerEpoch = useGameStore((state) => state.careerEpoch);
  const [operations, setOperations] = useState<Array<{ id: number; epoch: number }>>([]);
  const nextId = useRef(0);

  /** Starts an operation; the function it returns ends it. */
  const begin = useCallback(() => {
    const operation = { id: nextId.current++, epoch: useGameStore.getState().careerEpoch };
    setOperations((current) => [...current, operation]);
    return () => setOperations((current) => current.filter((held) => held.id !== operation.id));
  }, []);

  return { busy: operations.some((held) => held.epoch === careerEpoch), begin };
}
