import { useCallback, useEffect, useState } from "react";

import {
  type ContractTerminationPreviewData,
  previewContractTermination,
  previewRenewalProjection,
  type RenewalProjection,
} from "../../services/contractService";
import type { PlayerData } from "../../store/gameStore";
import { terminationBlockedBy } from "./PlayerProfile.contract";

/** An answer the Contract tab loads, and where it stands. */
export type SheetAnswer<T> =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "answered"; value: T }
  | { status: "failed" };

export interface ContractSheetData {
  /** What releasing him would cost and leave: his own player with a contract, not on loan. */
  termination: SheetAnswer<ContractTerminationPreviewData>;
  /** The club's wage bill and budget, with his current wage in it: his own player's. */
  projection: SheetAnswer<RenewalProjection>;
  /** Asks again for whatever failed. */
  retry: () => void;
}

interface UseContractSheetDataArgs {
  player: PlayerData;
  /** The Contract tab is the one open: nothing is asked for before. */
  active: boolean;
  /** The manager holds his contract: only then is there anything to ask. */
  ownsContract: boolean;
  /** Changes when the game changes under the sheet (an action taken, the day moved on). */
  refreshKey: string;
}

const IDLE = { status: "idle" } as const;

/** An answer tagged with what it is about, so it is never read for another player or a changed game. */
interface Held<T> {
  about: string;
  answer: SheetAnswer<T>;
}

/**
 * The two answers the decision sheet shows before a click: what releasing him costs and leaves, and
 * the club's wage bill with his current wage in it.
 *
 * Asked for only while the Contract tab is open (the profile keeps hidden tabs mounted), only for
 * a contract the manager holds, and the release preview only for a player who can be released (the
 * backend refuses it for one on loan or with no contract end). Each answer is tagged with the
 * player and the refresh key it was asked for, so the page reusing this instance for another
 * player never shows the first one's figures for the second. A failure is `failed`, never the
 * absence of an answer.
 */
export function useContractSheetData({
  player,
  active,
  ownsContract,
  refreshKey,
}: UseContractSheetDataArgs): ContractSheetData {
  const [attempt, setAttempt] = useState(0);
  const [termination, setTermination] = useState<Held<ContractTerminationPreviewData>>({
    about: "",
    answer: IDLE,
  });
  const [projection, setProjection] = useState<Held<RenewalProjection>>({
    about: "",
    answer: IDLE,
  });
  const releasable = terminationBlockedBy(player) === null;
  const asked = active && ownsContract;
  const about = `${player.id}|${refreshKey}|${attempt}`;
  const wage = player.wage;
  const playerId = player.id;

  useEffect(() => {
    if (!asked) return;
    let cancelled = false;
    setProjection({ about, answer: { status: "loading" } });
    previewRenewalProjection(playerId, wage)
      .then((response) => {
        if (cancelled) return;
        setProjection({
          about,
          answer: response?.projection
            ? { status: "answered", value: response.projection }
            : { status: "failed" },
        });
      })
      .catch(() => {
        if (!cancelled) setProjection({ about, answer: { status: "failed" } });
      });
    return () => {
      cancelled = true;
    };
  }, [asked, about, playerId, wage]);

  useEffect(() => {
    if (!asked || !releasable) return;
    let cancelled = false;
    setTermination({ about, answer: { status: "loading" } });
    previewContractTermination(playerId)
      .then((response) => {
        if (cancelled) return;
        setTermination({
          about,
          answer: response?.preview
            ? { status: "answered", value: response.preview }
            : { status: "failed" },
        });
      })
      .catch(() => {
        if (!cancelled) setTermination({ about, answer: { status: "failed" } });
      });
    return () => {
      cancelled = true;
    };
  }, [asked, releasable, about, playerId]);

  const retry = useCallback(() => setAttempt((count) => count + 1), []);

  // An answer is shown only for the player, key and attempt it was asked for; until the new one
  // arrives it is loading (never the previous one's figures).
  const shown = <T>(held: Held<T>, wanted: boolean): SheetAnswer<T> =>
    !wanted ? IDLE : held.about === about ? held.answer : { status: "loading" };

  return {
    termination: shown(termination, asked && releasable),
    projection: shown(projection, asked),
    retry,
  };
}
