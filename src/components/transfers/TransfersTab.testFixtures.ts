import type { GameStateData, PlayerData, StaffData, TeamData } from "../../store/gameStore";
export function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "User FC",
    short_name: "USR",
    country: "England",
    city: "London",
    stadium_name: "User Ground",
    stadium_capacity: 25000,
    finance: 5000000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 2000000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "Physical",
    training_intensity: "Medium",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: {
      primary: "#111111",
      secondary: "#ffffff",
    },
    facilities: {
      training: 1,
      medical: 1,
      scouting: 1,
    },
    starting_xi_ids: [],
    match_roles: {
      captain: null,
      vice_captain: null,
      penalty_taker: null,
      free_kick_taker: null,
      corner_taker: null,
    },
    form: [],
    history: [],
    ...overrides,
  };
}
export function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "England",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 60,
      stamina: 60,
      strength: 60,
      agility: 60,
      passing: 60,
      shooting: 60,
      tackling: 60,
      dribbling: 60,
      defending: 60,
      positioning: 60,
      vision: 60,
      decisions: 60,
      composure: 60,
      aggression: 60,
      teamwork: 60,
      leadership: 60,
      handling: 30,
      reflexes: 30,
      aerial: 60,
    },
    condition: 90,
    morale: 70,
    injury: null,
    team_id: "team-1",
    retired: false,
    contract_end: "2028-06-30",
    wage: 1000,
    market_value: 1000000,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [
      {
        id: "offer-1",
        from_team_id: "team-2",
        fee: 900000,
        wage_offered: 0,
        last_manager_fee: null,
        negotiation_round: 1,
        suggested_counter_fee: null,
        status: "Pending",
        date: "2026-08-01",
      },
    ],
    traits: [],
    ...overrides,
  };
}
export function createScout(overrides: Partial<StaffData> = {}): StaffData {
  return {
    id: "staff-1",
    first_name: "Sam",
    last_name: "Scout",
    date_of_birth: "1985-01-01",
    nationality: "England",
    role: "Scout",
    attributes: {
      coaching: 20,
      judgingAbility: 65,
      judgingPotential: 70,
      physiotherapy: 10,
    },
    team_id: "team-1",
    specialization: null,
    wage: 1000,
    contract_end: "2027-06-30",
    ...overrides,
  };
}
export function createGameState(players: PlayerData[] = [createPlayer()]): GameStateData {
  return {
    clock: {
      current_date: "2026-08-01T12:00:00Z",
      start_date: "2026-07-01T12:00:00Z",
    },
    manager: {
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "England",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team-1",
      career_stats: {
        matches_managed: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
      career_history: [],
    },
    teams: [
      createTeam(),
      createTeam({
        id: "team-2",
        name: "Buyer FC",
        short_name: "BUY",
        manager_id: null,
      }),
    ],
    players,
    staff: [],
    messages: [],
    news: [],
    league: {
      id: "league-1",
      name: "Premier Division",
      season: 1,
      fixtures: [],
      standings: [],
    },
    scouting_assignments: [],
    board_objectives: [],
    season_context: {
      phase: "InSeason",
      season_start: "2026-07-01",
      season_end: "2027-05-31",
      days_until_season_start: null,
      transfer_window: {
        status: "Open",
        opens_on: "2026-06-01",
        closes_on: "2026-08-31",
        days_until_opens: null,
        days_remaining: 30,
      },
    },
  };
}
export const transferTestTranslation = (key: string, params?: Record<string, string | number>) => {
  if (key === "finances.perWeekSuffix") return "/wk";
  if (key === "finances.perWeekSuffix") return "/wk";
  if (key === "finances.perYearSuffix") return "/yr";
  if (key === "common.nResults") return `${params?.count} results`;
  if (key === "common.action") return "Action";
  if (key === "common.viewTeam") return "View team";
  if (key === "common.freeAgent") return "Free Agent";
  if (key === "scouting.nextPage") return "Next page";
  if (key === "scouting.previousPage") return "Previous page";
  if (key === "players.showingRange")
    return `Showing ${params?.from}-${params?.to} of ${params?.total}`;
  if (key === "dashboard.players") return "Players";
  if (key === "transfers.myTransferList") return "My Transfer List";
  if (key === "transfers.transferMarket") return "Transfer Market";
  if (key === "transfers.myTransferList") return "My Transfer List";
  if (key === "transfers.freeAgents") return "Free Agents";
  if (key === "transfers.transfer") return "TRANSFER";
  if (key === "transfers.loan") return "LOAN";
  if (key === "transfers.loanMarket") return "Loan Market";
  if (key === "transfers.offers") return "Offers";
  if (key === "transfers.noFreeAgents") return "No free agents available.";
  if (key === "transfers.counterOffer") return "Counter Offer";
  if (key === "transfers.counterAmount") return "Counter Amount";
  if (key === "transfers.submitCounter") return "Submit Counter";
  if (key === "transfers.close") return "Close";
  if (key === "transfers.counter") return "Counter";
  if (key === "transfers.offerContract") return "Offer Contract";
  if (key === "transfers.makeOffer") return "Make Offer";
  if (key === "transfers.bid") return "Bid";
  if (key === "transfers.loanOffer") return "Loan Offer";
  if (key === "transfers.counterLoanOffer") return "Counter Loan Offer";
  if (key === "transfers.makeBid") return "Make Transfer Bid";
  if (key === "transfers.makeLoanOffer") return "Make Loan Offer";
  if (key === "transfers.bidAmount") return "Bid Amount (€M)";
  if (key === "transfers.loanEndDate") return "Loan End Date";
  if (key === "transfers.loanPeriod") return "Loan Length";
  if (key === "transfers.loanPeriodThreeMonths") return "3 months";
  if (key === "transfers.loanPeriodJanuaryWindow") return "Until January window";
  if (key === "transfers.loanPeriodEndOfSeason") return "Until end of season";
  if (key === "transfers.loanPeriodTwelveMonths") return "12 months";
  if (key === "transfers.loanPeriodCurrentOffer") return "Current offer date";
  if (key === "transfers.loanEndsOn") return `Loan ends on ${params?.endDate}`;
  if (key === "transfers.noLoanPeriodAvailable") return "No valid loan length.";
  if (key === "transfers.loanPeriodUnavailableRules") return "outside loan rules";
  if (key === "transfers.loanPeriodUnavailableContract") return "contract expires first";
  if (key === "transfers.loanWageContribution") return "Wage Contribution (%)";
  if (key === "transfers.loanWageContributionManual") return "Manual percentage";
  if (key === "transfers.loanWageSummary") return `${params?.percent}% wages: ${params?.wage}`;
  if (key === "transfers.loanToBuyOption") return "Loan-to-buy option";
  if (key === "transfers.loanToBuyOptionDesc") return "Include a permanent purchase clause.";
  if (key === "transfers.buyOptionFee") return "Buy Option Fee";
  if (key === "transfers.buyOptionFeeShort") return `Option ${params?.fee}`;
  if (key === "transfers.loanBuyOptionSummary") return `Permanent option at ${params?.fee}`;
  if (key === "transfers.exerciseBuyOption") return "Exercise Option";
  if (key === "transfers.submitLoanOffer") return "Submit Loan Offer";
  if (key === "transfers.submitLoanCounter") return "Submit Counter";
  if (key === "transfers.loanOfferAccepted") return "Loan accepted";
  if (key === "transfers.loanOfferRejected") return "Loan rejected";
  if (key === "transfers.loanCounterAccepted") return "Loan counter accepted";
  if (key === "transfers.loanCounterRejected") return "Loan counter rejected";
  if (key === "transfers.loanCounterCountered") return "They pushed back with adjusted loan terms.";
  if (key === "transfers.loanCounterSuggestedTerms")
    return `Suggested terms: ${params?.percent}% wages until ${params?.endDate}`;
  if (key === "transfers.loanCounterSuggestedBuyOption")
    return `Suggested buy option: ${params?.fee}`;
  if (key === "transfers.loanOfferTerms")
    return `Loan ${params?.percent}% wages until ${params?.endDate}`;
  if (key === "transfers.acceptLoanOffer") return "Accept Loan";
  if (key === "transfers.rejectLoanOffer") return "Reject Loan";
  if (key === "transfers.submitBid") return "Submit Bid";
  if (key === "transfers.bidImpactTitle") return "Projected impact";
  if (key === "transfers.bidImpactTransferBudget")
    return `Transfer budget ${params?.before} -> ${params?.after}`;
  if (key === "transfers.bidImpactBalance")
    return `Club balance ${params?.before} -> ${params?.after}`;
  if (key === "transfers.bidImpactWagePressure")
    return `Projected wage budget usage ${params?.percent}%`;
  if (key === "transfers.bidImpactOverTransferBudget")
    return "This bid exceeds your transfer budget";
  if (key === "transfers.bidImpactOverBalance") return "This bid would push the club into debt";
  if (key === "transfers.resumeNegotiationHint") return "Talks are still live with this club.";
  if (key === "transfers.resumeNegotiationHeadline")
    return "The other club are waiting for your next move.";
  if (key === "transfers.resumeNegotiationDetail")
    return `Their last signal pointed toward ${params?.fee}.`;
  if (key === "transfers.negotiationHistory") return "Recent exchange";
  if (key === "transfers.lastBidLabel") return "Your last bid";
  if (key === "transfers.lastClubSignalLabel") return "Their last signal";
  if (key === "transfers.lastCounterLabel") return "Your last counter";
  if (key === "transfers.currentOfferLabel") return "Their current offer";
  if (key === "transfers.offerStatusPending") return "Live";
  if (key === "transfers.offerStatusPendingRegistration") return "Pending registration";
  if (key === "transfers.offerStatusAccepted") return "Accepted";
  if (key === "transfers.offerStatusRejected") return "Rejected";
  if (key === "transfers.offerStatusWithdrawn") return "Talks cooled off";
  if (key === "transfers.negotiationExpiredError")
    return "Talks cooled off before you could answer. Start a new negotiation if the club comes back.";
  if (key === "transfers.acceptOffer") return "Accept";
  if (key === "transfers.rejectOffer") return "Reject";
  if (key === "transfers.negotiationPulse") return "Negotiation pulse";
  if (key === "transfers.negotiationRound") return `Round ${params?.count}`;
  if (key === "transfers.negotiationPatience") return "Patience";
  if (key === "transfers.negotiationTension") return "Tension";
  if (key === "transfers.counterCountered") return "They pushed back with a lower number.";
  if (key === "transfers.transferFeedbackCounterHeadline")
    return "They want more before shaking hands.";
  if (key === "transfers.transferFeedbackCounterDetail")
    return `The bid was close enough to keep talking, but their side are signalling a price nearer ${params?.fee}.`;
  if (key === "transfers.transferFeedbackScheduledHeadline")
    return "Deal agreed for the next registration window.";
  if (key === "transfers.transferFeedbackScheduledDetail")
    return `The terms are accepted. Registration is scheduled for ${params?.date}.`;
  if (key === "season.windowClosed") return "Transfer window closed";
  if (key === "season.windowOpensInDays") return `${params?.count} days until the window opens`;
  if (key === "transfers.loanWindowClosedNoticeTitle") return "Transfer window closed";
  if (key === "transfers.loanWindowClosedNoticeDetail")
    return `If accepted, the loan will be registered on ${params?.date}.`;
  if (key === "transfers.loanWindowClosedUnavailableDetail")
    return "Loan registration is unavailable until the next transfer window is scheduled.";
  if (key === "transfers.loanOfferScheduled")
    return `Loan agreed. Registration scheduled for ${params?.date}.`;
  if (key === "transfers.loanCounterScheduled")
    return `Loan agreed. Registration scheduled for ${params?.date}.`;
  if (key === "squad.viewProfile") return "View profile";
  if (key === "squad.addToTransferList") return "Add to transfer list";
  if (key === "squad.removeFromTransferList") return "Remove from transfer list";
  if (key === "squad.addToLoanList") return "Add to loan list";
  if (key === "squad.removeFromLoanList") return "Remove from loan list";
  if (key === "scouting.scoutBtn") return "Scout";
  if (key === "scouting.scoutingInProgress") return "Scouting in progress";
  if (key === "scouting.noScoutsFree") return "No scouts free";
  if (key === "playerProfile.renewalWage") return "Offered Wage";
  if (key === "playerProfile.renewalLength") return "Contract Length";
  if (key === "playerProfile.renewalProjectionTitle") return "Projected financial impact";
  if (key === "playerProfile.renewalProjectionWageBill")
    return `Weekly wage bill ${params?.before} -> ${params?.after}`;
  if (key === "playerProfile.renewalProjectionBudgetUsage")
    return `Wage budget use ${params?.before}% -> ${params?.after}%`;
  if (key === "playerProfile.renewalProjectionRunway")
    return `Cash runway ${params?.before} -> ${params?.after}`;
  if (key === "playerProfile.renewalBudgetWarning") return "Budget warning";
  if (key === "playerProfile.renewalConversationTitle") return "Negotiation pulse";
  if (key === "playerProfile.renewalRound") return `Round ${params?.count}`;
  if (key === "playerProfile.renewalPatience") return "Patience";
  if (key === "playerProfile.renewalTension") return "Tension";
  if (key === "playerProfile.renewalSubmit") return "Submit Offer";
  if (key === "playerProfile.renewalAccepted") return "Offer accepted";
  if (key === "playerProfile.renewalRejected") return "Offer rejected";
  if (key === "playerProfile.renewalCounter")
    return `Wants more: ${params?.wage} for ${params?.years} years`;
  if (key === "playerProfile.renewalBlocked") return "Talks blocked";
  if (key === "be.error.transfers.playerAlreadyLoaned") return "Player already loaned";
  if (params && typeof params === "object" && "defaultValue" in params) {
    return String(params.defaultValue);
  }
  return key;
};
