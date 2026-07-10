import { useState } from "react";

import type { GameStateData, PlayerData, TeamData } from "../../store/gameStore";
import type { TransferOfferData } from "../../store/types";
import {
  negotiateTransferPersonalTerms,
  type TransferNegotiationFeedbackData,
  type TransferPersonalTermsResponseData,
} from "../../services/transfersService";
import { resolveBackendError } from "../../utils/backendI18n";

interface UseTransferPersonalTermsFlowArgs {
  gameState: GameStateData;
  onGameUpdate?: (game: GameStateData) => void;
}

const MAX_CONTRACT_YEARS = 5;
const MIN_WAGE = 500;

/** The offer statuses at which the personal-terms conversation has ended. */
const TERMINAL_STATUSES: ReadonlyArray<TransferOfferData["status"]> = [
  "Accepted",
  "PendingRegistration",
  "PersonalTermsFailed",
];

interface UseTransferPersonalTermsFlowResult {
  personalTermsTarget: PlayerData | null;
  wageOffer: string;
  setWageOffer: (value: string) => void;
  contractYears: string;
  setContractYears: (value: string) => void;
  personalTermsRound: number;
  personalTermsStatus: TransferOfferData["status"] | null;
  personalTermsFeedback: TransferNegotiationFeedbackData | null;
  personalTermsSuggestedWage: number | null;
  personalTermsSuggestedYears: number | null;
  personalTermsLoading: boolean;
  personalTermsError: string | null;
  personalTermsTerminal: boolean;
  personalTermsSucceeded: boolean;
  submitDisabled: boolean;
  myTeam: TeamData | null;
  openPersonalTermsNegotiation: (
    player: PlayerData,
    offerId: string,
    buyerTeamId: string,
  ) => void;
  closePersonalTermsNegotiation: () => void;
  submitPersonalTerms: () => Promise<void>;
}

export function useTransferPersonalTermsFlow({
  gameState,
  onGameUpdate,
}: UseTransferPersonalTermsFlowArgs): UseTransferPersonalTermsFlowResult {
  const myTeam =
    gameState.teams.find((team) => team.id === gameState.manager.team_id) ??
    null;

  const [personalTermsTarget, setPersonalTermsTarget] =
    useState<PlayerData | null>(null);
  const [offerId, setOfferId] = useState<string | null>(null);
  const [buyerTeamId, setBuyerTeamId] = useState<string | null>(null);
  const [wageOffer, setWageOffer] = useState("");
  const [contractYears, setContractYears] = useState("");
  const [personalTermsRound, setPersonalTermsRound] = useState(1);
  const [personalTermsStatus, setPersonalTermsStatus] =
    useState<TransferOfferData["status"] | null>(null);
  const [personalTermsFeedback, setPersonalTermsFeedback] =
    useState<TransferNegotiationFeedbackData | null>(null);
  const [personalTermsSuggestedWage, setPersonalTermsSuggestedWage] = useState<
    number | null
  >(null);
  const [personalTermsSuggestedYears, setPersonalTermsSuggestedYears] =
    useState<number | null>(null);
  const [personalTermsLoading, setPersonalTermsLoading] = useState(false);
  const [personalTermsError, setPersonalTermsError] = useState<string | null>(
    null,
  );
  const [personalTermsSucceeded, setPersonalTermsSucceeded] = useState(false);
  // The player has been insulted / is on a cooldown and won't re-engage yet.
  const [personalTermsCooldown, setPersonalTermsCooldown] = useState(false);

  const offeredWage = Number(wageOffer);
  const offeredYears = Number(contractYears);
  const isWageValid = Number.isFinite(offeredWage) && offeredWage >= MIN_WAGE;
  const isYearsValid =
    Number.isInteger(offeredYears) &&
    offeredYears > 0 &&
    offeredYears <= MAX_CONTRACT_YEARS;

  const personalTermsTerminal =
    personalTermsStatus !== null && TERMINAL_STATUSES.includes(personalTermsStatus);

  const findOffer = (
    game: GameStateData,
    playerId: string,
    id: string,
  ): TransferOfferData | undefined =>
    game.players
      .find((player) => player.id === playerId)
      ?.transfer_offers.find((offer) => offer.id === id);

  const openPersonalTermsNegotiation = (
    player: PlayerData,
    id: string,
    team: string,
  ): void => {
    setPersonalTermsTarget(player);
    setOfferId(id);
    setBuyerTeamId(team);

    const offer = findOffer(gameState, player.id, id);
    // Prefill from any live player counter, otherwise from the player's current wage.
    const suggestedWage = offer?.suggested_wage ?? null;
    const suggestedYears = offer?.suggested_contract_years ?? null;
    const defaultWage = suggestedWage ?? Math.max(player.wage, MIN_WAGE);
    setWageOffer(String(Math.ceil(defaultWage / 1000) * 1000));
    setContractYears(String(suggestedYears ?? 3));
    setPersonalTermsRound(Math.max(offer?.personal_terms_round ?? 1, 1));
    setPersonalTermsStatus(offer?.status ?? null);
    setPersonalTermsSuggestedWage(suggestedWage);
    setPersonalTermsSuggestedYears(suggestedYears);
    setPersonalTermsFeedback(null);
    setPersonalTermsError(null);
    setPersonalTermsSucceeded(false);
    setPersonalTermsCooldown(false);
  };

  const closePersonalTermsNegotiation = (): void => {
    if (personalTermsLoading) {
      return;
    }
    setPersonalTermsTarget(null);
    setOfferId(null);
    setBuyerTeamId(null);
    setWageOffer("");
    setContractYears("");
    setPersonalTermsRound(1);
    setPersonalTermsStatus(null);
    setPersonalTermsFeedback(null);
    setPersonalTermsSuggestedWage(null);
    setPersonalTermsSuggestedYears(null);
    setPersonalTermsError(null);
    setPersonalTermsSucceeded(false);
    setPersonalTermsCooldown(false);
  };

  const submitPersonalTerms = async (): Promise<void> => {
    if (
      !personalTermsTarget ||
      !offerId ||
      !buyerTeamId ||
      !isWageValid ||
      !isYearsValid ||
      personalTermsTerminal
    ) {
      return;
    }

    setPersonalTermsLoading(true);
    setPersonalTermsError(null);

    try {
      const response: TransferPersonalTermsResponseData =
        await negotiateTransferPersonalTerms(
          personalTermsTarget.id,
          offerId,
          buyerTeamId,
          Math.round(offeredWage),
          offeredYears,
        );

      onGameUpdate?.(response.game);
      setPersonalTermsStatus(response.status);
      setPersonalTermsFeedback(response.feedback ?? null);
      setPersonalTermsRound(response.personal_terms_round);
      setPersonalTermsSuggestedWage(response.suggested_wage);
      setPersonalTermsSuggestedYears(response.suggested_contract_years);
      setPersonalTermsSucceeded(response.success);

      // An insult or an active cooldown means the player won't re-engage yet —
      // lock submission immediately (don't wait for a second attempt).
      const onCooldown =
        !response.success &&
        (response.error === "be.error.transfers.personalTermsInsulting" ||
          response.error === "be.error.transfers.personalTermsCooldown");
      setPersonalTermsCooldown(onCooldown);
      // Backend returns an i18n key; resolve it so we never render a raw key.
      setPersonalTermsError(
        response.success || !response.error
          ? null
          : (resolveBackendError(response.error) ?? response.error),
      );

      // Prefill the next round's inputs from the player's counter proposal.
      if (response.suggested_wage !== null) {
        setWageOffer(String(response.suggested_wage));
      }
      if (response.suggested_contract_years !== null) {
        setContractYears(String(response.suggested_contract_years));
      }
    } catch (error) {
      setPersonalTermsError(resolveBackendError(error) ?? "Unknown error");
    } finally {
      setPersonalTermsLoading(false);
    }
  };

  return {
    personalTermsTarget,
    wageOffer,
    setWageOffer,
    contractYears,
    setContractYears,
    personalTermsRound,
    personalTermsStatus,
    personalTermsFeedback,
    personalTermsSuggestedWage,
    personalTermsSuggestedYears,
    personalTermsLoading,
    personalTermsError,
    personalTermsTerminal,
    personalTermsSucceeded,
    submitDisabled:
      personalTermsLoading ||
      personalTermsTerminal ||
      personalTermsCooldown ||
      !isWageValid ||
      !isYearsValid,
    myTeam,
    openPersonalTermsNegotiation,
    closePersonalTermsNegotiation,
    submitPersonalTerms,
  };
}
