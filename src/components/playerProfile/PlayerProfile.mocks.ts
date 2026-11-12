import type { PlayerKnowledge, VisibleWage } from "../../services/scoutingService";

/**
 * TEST-ONLY: imported by the player profile's `*.test.tsx` suites and by nothing in production.
 *
 * The bodies of the module mocks every profile suite installs, and the state a suite sets to say
 * what they answer. Each suite keeps its own `vi.mock(...)` lines (vitest hoists them per file) and
 * each factory imports its body from here, so the suites share one copy and the state objects below
 * are the very ones the mocked hooks read.
 */

// What the backend's contract warning stages read as on a badge, by player id; a test says which
// players are at risk. The hook that asks for them has its own test.
export const riskLevels = {
  current: {} as Record<string, "critical" | "warning" | "stable">,
};

export const contractWarningStagesMock = {
  useContractRiskLevels: () => (playerId: string) => riskLevels.current[playerId] ?? "stable",
};

// What the club knows about each player, by id, as the backend's knowledge rule would answer; a test
// names the rivals it shows. Everyone else reads as the club's own, which most of these tests render.
// The hook that asks for it has its own test.
// Each revision the profile hands the hook is kept, so a test can see when it would ask again.
export const knowledge = {
  current: {} as Record<string, PlayerKnowledge>,
  revisions: [] as string[],
};

export const playerKnowledgeMock = {
  usePlayerKnowledge: (_playerIds: string[], revision: string) => {
    knowledge.revisions.push(revision);
    return (playerId: string) => knowledge.current[playerId] ?? ({ kind: "own" } as const);
  },
};

// The wage the club may see, by player id, as the backend's `visible_wage` would answer. Everyone
// else reads as the club's own player on his current wage, which the profile's revision key ends
// with, as the backend would answer after a renewal.
export const wages = {
  current: {} as Record<string, VisibleWage>,
  revisions: [] as string[],
};

export const visibleWageMock = {
  useVisibleWage: (playerId: string, revision: string) => {
    wages.revisions.push(revision);
    return {
      status: "answered" as const,
      value:
        wages.current[playerId] ??
        ({ kind: "exact", weekly: Number(JSON.parse(revision).at(-1)) } as const),
    };
  },
};

// The club holds no opinion of anyone's potential in these tests; PlayerProfile.potential.test.tsx
// shows one, through the real hook. The hook has its own test.
export const potentialOpinionMock = {
  usePotentialOpinion: () => ({ status: "answered", value: null }),
};
// Nor its ratings at positions: PlayerProfilePositionRatings and usePositionRatings have their own.
export const positionRatingsMock = {
  usePositionRatings: () => ({ status: "answered", value: null }),
};

export const reactI18nextMock = {
  initReactI18next: {
    type: "3rdParty",
    init: () => {},
  },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "common.back") return "Back";
      if (key === "playerProfile.tabs.overview") return "Overview tab";
      if (key === "playerProfile.tabs.attributes") return "Attributes tab";
      if (key === "playerProfile.tabs.career") return "Career tab";
      if (key === "playerProfile.tabs.contract") return "Contract tab";
      if (key === "common.contract") return "Contract";
      if (key === "common.renewContract") return "Renew Contract";
      if (key === "common.cancel") return "Cancel";
      if (key === "common.done") return "Done";
      if (key === "common.submit") return "Submit";
      if (key === "common.condition") return "Condition";
      if (key === "common.morale") return "Morale";
      if (key === "common.value") return "Value";
      if (key === "common.wage") return "Wage";
      if (key === "common.age") return "Age";
      if (key.startsWith("common.ordinal.")) {
        const suffixes: Record<string, string> = { one: "st", two: "nd", few: "rd", other: "th" };
        return `${params?.n}${suffixes[key.split(".")[2]] ?? "th"}`;
      }
      if (key === "common.viewTeam") return "View team";
      if (key === "common.freeAgent") return "Free Agent";
      if (key === "common.unknown") return "Unknown";
      if (key === "finances.perWeekSuffix") return "/wk";
      if (key === "finances.perYearSuffix") return "/yr";
      if (key === "finances.marketValue") return "Market Value";
      if (key === "finances.contractRiskCritical") return "Critical";
      if (key === "finances.contractRiskWarning") return "Warning";
      if (key === "finances.contractRiskStable") return "Stable";
      if (key === "finances.contractExpiresOn") return `Expires ${params?.date}`;
      if (key === "playerProfile.contractInfo") return "Contract Info";
      if (key === "playerProfile.overview.now") return "Now";
      if (key === "playerProfile.overview.details") return "Details";
      if (key === "playerProfile.overview.previousMatches") return "Previous matches";
      if (key === "playerProfile.overview.contractUntil") return `until ${params?.date}`;
      if (key === "playerProfile.dateOfBirth") return "Date of Birth";
      if (key === "playerProfile.weeklyWage") return "Weekly Wage";
      if (key === "playerProfile.approximateWage") return `about ${params?.wage}`;
      if (key === "playerProfile.annualWage") return "Annual Wage";
      if (key === "playerProfile.noContract") return "No Contract";
      if (key === "playerProfile.yearsRemaining") return "Years Remaining";
      if (key === "playerProfile.contractRisk") return "Contract Risk";
      if (key === "playerProfile.renewalTitle") return "Renew Contract";
      if (key === "playerProfile.renewalWage") return "Offered Wage";
      if (key === "playerProfile.renewalLength") return "Contract Length";
      if (key === "playerProfile.renewalLengthYears") return `${params?.count} years`;
      if (key === "playerProfile.renewalSubmit") return "Submit Offer";
      if (key === "playerProfile.renewalBudgetWarning") return "Exceeds wage budget";
      if (key === "playerProfile.renewalInvalidWage") return "Enter a valid weekly wage";
      if (key === "playerProfile.renewalAccepted") return "Offer accepted";
      if (key === "playerProfile.renewalRejected") return "Offer rejected";
      if (key === "playerProfile.renewalCounter")
        return `Wants more: ${params?.wage}/wk for ${params?.years} years`;
      if (key === "playerProfile.renewalBlocked")
        return "Talks are blocked after your earlier decision";
      if (key === "playerProfile.renewalCooledOff")
        return "Previous talks cooled off, so this starts as a fresh conversation.";
      if (key === "playerProfile.letContractExpire") return "Let Expire";
      if (key === "playerProfile.reopenContractTalks") return "Reopen Talks";
      if (key === "playerProfile.terminateContract") return "Terminate Now";
      if (key === "playerProfile.terminateContractTitle") return "Terminate Contract";
      if (key === "playerProfile.terminateContractBody")
        return `Release ${params?.name} immediately.`;
      if (key === "playerProfile.terminationSeverance") return "Severance";
      if (key === "playerProfile.projectedHealthyPlayers") return "Healthy players after release";
      if (key === "playerProfile.terminationUnsafe")
        return "This release would leave the squad unable to field a matchday XI.";
      if (key === "playerProfile.confirmTerminateContract") return "Terminate Contract";
      if (key === "playerProfile.delegateRenewal") return "Delegate to Assistant";
      if (key === "playerProfile.renewalDelegateMissingReport")
        return "Assistant report did not include this player.";
      if (key === "be.error.contracts.noAssistantManagerAssigned")
        return "Hire an assistant manager before delegating contract talks.";
      if (key === "be.error.contracts.playerHasNoActiveContract")
        return "This player does not have an active contract.";
      if (key === "playerProfile.renewalConversationTitle") return "Negotiation pulse";
      if (key === "playerProfile.renewalProjectionTitle") return "Projected financial impact";
      if (key === "playerProfile.renewalProjectionWageBill")
        return `Weekly wage bill ${params?.before} -> ${params?.after}`;
      if (key === "playerProfile.renewalProjectionBudgetUsage")
        return `Wage budget use ${params?.before}% -> ${params?.after}%`;
      if (key === "playerProfile.renewalProjectionRunway")
        return `Cash runway ${params?.before} -> ${params?.after}`;
      if (key === "playerProfile.renewalRound") return `Round ${params?.count}`;
      if (key === "playerProfile.renewalPatience") return "Patience";
      if (key === "playerProfile.renewalTension") return "Tension";
      if (key === "playerProfile.renewalFeedbackFirmHeadline")
        return "They want stronger terms before moving.";
      if (key === "playerProfile.renewalFeedbackFirmDetail")
        return "The discussion is still open, but wage level and contract length need to feel clearly worthwhile from their side.";
      if (key === "playerProfile.loanStatus") return "Loan Status";
      if (key === "playerProfile.loanedInFrom") return `On loan from ${params?.team}`;
      if (key === "playerProfile.loanedOutTo") return `On loan at ${params?.team}`;
      if (key === "playerProfile.loanBetweenClubs")
        return `On loan from ${params?.parent} to ${params?.loanTeam}`;
      if (key === "playerProfile.loanUntil") return `Until ${params?.date}`;
      if (key === "playerProfile.loanWageSplit")
        return `${params?.percent}% wages covered by loan club`;
      if (key === "playerProfile.loanBuyOption") return `Buy option ${params?.fee}`;
      if (key === "playerProfile.attributes") return "Attributes";
      if (key === "playerProfile.seasonStats") return "Season Stats";
      if (key === "playerProfile.advancedStats") return "Advanced Stats";
      if (key === "playerProfile.shots") return "Shots";
      if (key === "playerProfile.shotsOnTarget") return "Shots On Target";
      if (key === "playerProfile.passes") return "Passes";
      if (key === "playerProfile.tacklesWon") return "Tackles Won";
      if (key === "playerProfile.interceptions") return "Interceptions";
      if (key === "playerProfile.foulsCommitted") return "Fouls Committed";
      if (key === "playerProfile.per90") return "Per 90";
      if (key === "playerProfile.per90Value") return `${params?.value} per 90`;
      if (key === "playerProfile.accurateValue") return `${params?.value} accurate`;
      if (key === "playerProfile.passAccuracy") return "Pass Accuracy";
      if (key === "playerProfile.percentile") return "Percentile";
      if (key === "playerProfile.percentileUnavailable") return "Percentile unavailable";
      if (key === "playerProfile.recentMatches") return "Recent Matches";
      if (key === "playerProfile.vs") return "vs";
      if (key === "playerProfile.noRecentMatches") return "No recent match data";
      if (key === "playerProfile.careerHistory") return "Career History";
      if (key === "playerProfile.noCareer") return "No Career";
      if (key === "playerProfile.movementHistory") return "Movement History";
      if (key === "playerProfile.noMovementHistory") return "No movement history";
      if (key === "playerProfile.movementPermanentTransfer") return "Transfer";
      if (key === "playerProfile.movementLoanStart") return "Loan";
      if (key === "playerProfile.movementLoanReturn") return "Loan return";
      if (key === "playerProfile.movementLoanToBuy") return "Loan-to-buy";
      if (key === "playerProfile.movementFreeAgentSigning") return "Free agent";
      if (key === "playerProfile.movementReleased") return "Released";
      if (key === "playerProfile.movementFromTo") return `${params?.from} -> ${params?.to}`;
      if (key === "playerProfile.movementTo") return `Joined ${params?.to}`;
      if (key === "playerProfile.movementFrom") return `Left ${params?.from}`;
      if (key === "playerProfile.movementFee") return `Fee ${params?.fee}`;
      if (key === "playerProfile.movementLoanUntil") return `Loan until ${params?.date}`;
      if (key === "scouting.noScoutsHint") return "Hire a scout first";
      if (key === "scouting.scoutingInProgress") return "Scouting in progress";
      if (key === "scouting.scoutBtn") return "Scout";
      if (key === "finances.wagePerWeek") return "Wage/wk";
      if (key === "transfers.offerContract") return "Offer Contract";
      if (key === "transfers.close") return "Close";
      if (key === "transfers.submitting") return "Submitting...";
      if (key === "transfers.playerValue") return `Value: ${params?.value}`;
      return key;
    },
    i18n: { language: "en" },
  }),
};

/** Laid over the real `backendI18n`: the banners' one resolver, with this suite's own texts. */
export const backendI18nOverrides = {
  resolveBackendError: (error: unknown) => {
    const message = error instanceof Error ? error.message : String(error);
    if (message === "be.error.contracts.noAssistantManagerAssigned") {
      return "Hire an assistant manager before delegating contract talks.";
    }
    if (message === "be.error.contracts.playerHasNoActiveContract") {
      return "This player does not have an active contract.";
    }
    return message;
  },
};

export const countriesMock = {
  countryName: () => "England",
  isValidCountryCode: () => true,
  normaliseNationality: (value: string) => value,
  resolveCountryFlagCode: () => "GB",
};
