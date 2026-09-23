const PLAYER_EVENT_PREFIX_TO_GROUP: Record<string, string> = {
  morale_talk_: "moraleCrisis",
  bench_complaint_: "benchComplaint",
  happy_player_: "happyPlayer",
  contract_concern_: "contractConcern",
};

const PLAYER_EVENT_OPTION_ID_TO_KEY: Record<string, string> = {
  encourage: "encourage",
  promise_time: "promiseTime",
  work_harder: "workHarder",
  explain: "explain",
  promise_chance: "promiseChance",
  prove_yourself: "proveYourself",
  praise_back: "praiseBack",
  stay_professional: "stayProfessional",
  higher_expectations: "higherExpectations",
  reassure: "reassure",
  noncommittal: "noncommittal",
  no_renewal: "noRenewal",
};

export function inferPlayerEventGroup(messageId: string): string | undefined {
  for (const [prefix, group] of Object.entries(PLAYER_EVENT_PREFIX_TO_GROUP)) {
    if (messageId.startsWith(prefix)) {
      return group;
    }
  }

  return undefined;
}

export function inferPlayerEventActionLabelKey(
  messageId: string,
  actionId: string,
): string | undefined {
  if (!inferPlayerEventGroup(messageId)) {
    return undefined;
  }

  if (actionId === "respond") {
    return "be.msg.playerEvent.respond";
  }

  return undefined;
}

export function inferPlayerEventOptionBaseKey(
  messageId: string,
  optionId: string,
): string | undefined {
  const group = inferPlayerEventGroup(messageId);
  const optionKey = PLAYER_EVENT_OPTION_ID_TO_KEY[optionId];

  if (!group || !optionKey) {
    return undefined;
  }

  return `be.msg.playerEvent.options.${group}.${optionKey}`;
}
