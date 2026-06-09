import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "../../i18n";

import type {
  GameStateData,
  MessageAction,
  MessageData,
} from "../../store/gameStore";
import { useSettingsStore } from "../../store/settingsStore";
import InboxTab from "./InboxTab";

const mockTranslationState = vi.hoisted(function () {
  return {
    language: "en",
    translations: {
      en: {
        "common.age": "Age",
        "inbox.chooseResponseOutcomeVaries": "Choose your response — outcome varies",
        "inbox.deleteMessage": "Delete message",
        "inbox.effectOutcomeLabel": "Outcome",
        "inbox.markAsRead": "Mark as read",
        "inbox.openMessage": "Open message",
        "inbox.responded": "Response sent",
        "inbox.youthProspectSigned": "Signed to academy",
        "finances.marketValue": "Market Value",
        "finances.perWeekSuffix": "/wk",
        "finances.wagePerWeek": "Wage/wk",
        "playerProfile.contractInfo": "Contract",
        "scouting.youthTargetLabel": "Youth target",
        "scouting.youthAnyPosition": "Any position",
        "scouting.regionDomestic": "Domestic",
        "scouting.objectiveBalanced": "Balanced",
        "common.positions.Defender": "Defender",
        "common.positions.Forward": "Forward",
        "common.positions.Midfielder": "Midfielder",
        "squad.viewProfile": "View profile",
        "inbox.sortByDate": "Sort messages by date",
        "inbox.sortLabel": "Sort",
        "inbox.sortNewest": "Newest first",
        "inbox.sortOldest": "Oldest first",
        "youthAcademy.growth": "Growth",
        "youthAcademy.ovr": "OVR",
        "youthAcademy.potential": "Potential",
        "youthAcademy.potPromising": "Promising",
      },
      "pt-BR": {
        "inbox.effectOutcomeLabel": "Desfecho",
      },
    } as Record<string, Record<string, string>>,
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-i18next", async (importOriginal) => {
  const actual = await importOriginal<typeof import("react-i18next")>();
  const mockI18n = {
    get language(): string {
      return mockTranslationState.language;
    },
    async changeLanguage(language: string): Promise<string> {
      mockTranslationState.language = language;
      return language;
    },
  };

  return {
    ...actual,
    useTranslation: () => ({
      t: (key: string, value?: unknown) => {
        const resolved =
          mockTranslationState.translations[mockTranslationState.language]?.[
          key
          ];

        if (resolved) {
          return resolved;
        }

        if (typeof value === "string") {
          return value;
        }

        return key;
      },
      i18n: mockI18n,
    }),
  };
});

const mockedInvoke = vi.mocked(invoke);

beforeAll(function defineMatchMedia(): void {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });

  i18n.addResourceBundle(
    "en",
    "translation",
    {
      "test.effectFeedback": "Resolved morale {{delta}}",
      "be.msg.delegatedRenewals.subject":
        "Assistant Report — Contract Renewals",
      "be.msg.delegatedRenewals.body":
        "Boss, I went through our renewal list at {{team}}. {{successes}} completed, {{stalled}} still pending, {{failures}} failed.",
      "be.msg.delegatedRenewals.case.successful":
        "Completed: {{player}} agreed to {{years}} year(s) on {{wage}}/wk.",
      "be.msg.delegatedRenewals.case.stalled":
        "Still difficult: {{player}} — {{detail}}",
      "be.msg.delegatedRenewals.case.failed": "Failed: {{player}} — {{detail}}",
      "be.msg.delegatedRenewals.notes.beyondLimits":
        "Their camp want around {{wage}}/wk for {{years}} years, which is beyond the delegation limits.",
      "be.msg.delegatedRenewals.notes.boardWagePolicy":
        "Board wage policy blocks this renewal. Keep annual wages near {{budget}} while we recover.",
      "be.msg.delegatedRenewals.notes.relationshipBlocked":
        "They are not willing to commit through me under the current relationship and contract situation.",
      "be.msg.youthRecruitmentReport.subject":
        "Scout Report — Youth Recruitment",
      "be.msg.youthRecruitmentReport.bodyAny":
        "{{scout}} has completed the latest youth recruitment search for {{team}}. I found {{count}} prospects in the {{regionLabel}} market with a {{objectiveLabel}} profile.\n\nReview the attached cards and decide who should join the academy.",
      "be.msg.youthRecruitmentReport.bodyTargeted":
        "{{scout}} has completed the latest youth recruitment search for {{team}}. I found {{count}} prospects in the {{regionLabel}} market with a {{objectiveLabel}} profile, focused on {{targetLabel}}.\n\nReview the attached cards and decide who should join the academy.",
      "be.msg.youthRecruitment.option.sign.label": "Sign to academy",
      "be.msg.youthRecruitment.option.sign.description":
        "Offer this prospect a youth contract and add them to the academy.",
      "be.msg.youthRecruitment.option.shortlist.label": "Shortlist",
      "be.msg.youthRecruitment.option.shortlist.description":
        "Keep this prospect under consideration and move them into a separate shortlist message.",
      "be.msg.youthRecruitment.option.discard.label": "Discard",
      "be.msg.youthRecruitment.option.discard.description":
        "Pass on this prospect and remove them from the report.",
    },
    true,
    true,
  );

  i18n.addResourceBundle(
    "de",
    "translation",
    {
      "be.msg.delegatedRenewals.case.successful":
        "Completed: {{player}} agreed to {{years}} year(s) on {{wage}}/wk.",
      "be.msg.delegatedRenewals.case.failed": "Failed: {{player}} — {{detail}}",
      "be.msg.delegatedRenewals.notes.boardWagePolicy":
        "Board wage policy blocks this renewal. Keep annual wages near {{budget}} while we recover.",
    },
    true,
    true,
  );
});

beforeEach(function resetMocks(): void {
  mockedInvoke.mockReset();
  useSettingsStore.setState({
    settings: {
      ...useSettingsStore.getState().settings,
      currency: "EUR",
      language: "en",
    },
    currency: { code: "EUR", symbol: "€", exchange_rate: 1 },
  });
});

function createMessage(overrides: Partial<MessageData> = {}): MessageData {
  return {
    id: "m1",
    subject: "Test Message",
    body: "Test Body",
    sender: "Sender",
    sender_role: "Role",
    date: "2025-01-01",
    read: false,
    category: "System",
    priority: "Normal",
    actions: [],
    context: {
      team_id: null,
      player_id: null,
      fixture_id: null,
      youth_target_position: null,
      match_result: null,
    },
    ...overrides,
  };
}

function createGameState(messages: MessageData[]): GameStateData {
  return {
    clock: {
      current_date: "2025-01-01",
      start_date: "2025-01-01",
    },
    manager: {
      id: "manager-1",
      first_name: "John",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "BR",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "t1",
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
    teams: [],
    players: [],
    staff: [],
    messages,
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  };
}

function createProspect(overrides: Partial<GameStateData["players"][number]> = {}) {
  return {
    id: "prospect-1",
    match_name: "R. Prospect",
    full_name: "Rui Prospect",
    date_of_birth: "2009-03-20",
    nationality: "BR",
    football_nation: "BR",
    position: "Defender",
    natural_position: "Defender",
    alternate_positions: [],
    footedness: "Right",
    weak_foot: 3,
    training_focus: null,
    attributes: {
      pace: 10,
      stamina: 10,
      strength: 10,
      agility: 10,
      passing: 10,
      shooting: 10,
      tackling: 10,
      dribbling: 10,
      defending: 10,
      positioning: 10,
      vision: 10,
      decisions: 10,
      composure: 10,
      aggression: 10,
      teamwork: 10,
      leadership: 10,
      handling: 10,
      reflexes: 10,
      aerial: 10,
    },
    condition: 88,
    morale: 74,
    injury: null,
    team_id: null,
    squad_role: "Youth" as const,
    contract_end: "2028-06-30",
    wage: 950,
    market_value: 180000,
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
    retired: false,
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ovr: 63,
    potential: 72,
    ...overrides,
  };
}

function renderInboxTab(options: {
  gameState: GameStateData;
  initialMessageId?: string | null;
  onGameUpdate?: (state: GameStateData) => void;
  onNavigate?: (tab: string, context?: { messageId?: string }) => void;
}): void {
  render(
    <InboxTab
      gameState={options.gameState}
      initialMessageId={options.initialMessageId}
      onGameUpdate={options.onGameUpdate ?? vi.fn()}
      onNavigate={options.onNavigate}
    />,
  );
}

describe("InboxTab", function (): void {
  it("renders each message exactly once in the list", function (): void {
    const gameState = createGameState([
      createMessage({ id: "m1", subject: "Test Message 1" }),
      createMessage({ id: "m2", subject: "Test Message 2" }),
      createMessage({ id: "m3", subject: "Test Message 3" }),
    ]);

    renderInboxTab({ gameState });

    expect(screen.getAllByText(/Test Message \d/)).toHaveLength(3);
  });

  it("marks an unread message as read when selected", async function (): Promise<void> {
    const updatedGameState = createGameState([
      createMessage({ id: "m1", read: true }),
    ]);
    const onGameUpdate = vi.fn();

    mockedInvoke.mockResolvedValue(updatedGameState);

    renderInboxTab({
      gameState: createGameState([createMessage({ id: "m1" })]),
      onGameUpdate,
    });

    fireEvent.click(screen.getByText("Test Message"));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("mark_message_read", {
        messageId: "m1",
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
  });

  it("sorts messages by date when the sort order changes", function (): void {
    renderInboxTab({
      gameState: createGameState([
        createMessage({
          id: "m1",
          subject: "Newest Message",
          date: "2025-01-03",
          read: true,
        }),
        createMessage({
          id: "m2",
          subject: "Oldest Message",
          date: "2025-01-01",
          read: true,
        }),
        createMessage({
          id: "m3",
          subject: "Middle Message",
          date: "2025-01-02",
          read: true,
        }),
      ]),
    });

    let rows = screen.getAllByTestId(/inbox-row-/);
    expect(within(rows[0]).getByText("Newest Message")).toBeInTheDocument();
    expect(within(rows[1]).getByText("Middle Message")).toBeInTheDocument();
    expect(within(rows[2]).getByText("Oldest Message")).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("combobox", { name: "Sort messages by date" }),
    );
    fireEvent.click(screen.getByRole("option", { name: "Oldest first" }));

    rows = screen.getAllByTestId(/inbox-row-/);
    expect(within(rows[0]).getByText("Oldest Message")).toBeInTheDocument();
    expect(within(rows[1]).getByText("Middle Message")).toBeInTheDocument();
    expect(within(rows[2]).getByText("Newest Message")).toBeInTheDocument();
  });

  it("confirms before deleting a single message", async function (): Promise<void> {
    const onGameUpdate = vi.fn();
    const updatedGameState = createGameState([]);

    mockedInvoke.mockResolvedValue(updatedGameState);

    renderInboxTab({
      gameState: createGameState([createMessage({ id: "m1", read: true })]),
      initialMessageId: "m1",
      onGameUpdate,
    });

    fireEvent.click(screen.getByTestId("inbox-delete-message"));

    expect(
      screen.getByTestId("inbox-delete-confirm-modal"),
    ).toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId("inbox-confirm-delete"));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("delete_message", {
        messageId: "m1",
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
  });

  it("opens the context menu on a message row and requests deletion", function (): void {
    renderInboxTab({
      gameState: createGameState([createMessage({ id: "m1", read: true })]),
    });

    fireEvent.contextMenu(screen.getByTestId("inbox-row-m1"));
    fireEvent.click(screen.getByRole("button", { name: "Delete message" }));

    expect(
      screen.getByTestId("inbox-delete-confirm-modal"),
    ).toBeInTheDocument();
  });

  it("confirms before deleting selected messages in bulk", async function (): Promise<void> {
    const onGameUpdate = vi.fn();
    const updatedGameState = createGameState([
      createMessage({ id: "m3", subject: "Keep Me", read: true }),
    ]);

    mockedInvoke.mockResolvedValue(updatedGameState);

    renderInboxTab({
      gameState: createGameState([
        createMessage({ id: "m1", subject: "Delete Me 1", read: true }),
        createMessage({ id: "m2", subject: "Delete Me 2", read: true }),
        createMessage({ id: "m3", subject: "Keep Me", read: true }),
      ]),
      onGameUpdate,
    });

    fireEvent.click(screen.getByTestId("inbox-toggle-selection-mode"));
    fireEvent.click(screen.getByTestId("inbox-select-message-m1"));
    fireEvent.click(screen.getByTestId("inbox-select-message-m2"));
    fireEvent.click(screen.getByTestId("inbox-delete-selected"));

    expect(
      screen.getByTestId("inbox-delete-confirm-modal"),
    ).toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId("inbox-confirm-delete"));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("delete_messages", {
        messageIds: ["m1", "m2"],
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(updatedGameState);
  });

  it("navigates to a team route without resolving the message action", async function (): Promise<void> {
    const onNavigate = vi.fn();
    const action: MessageAction = {
      id: "action-1",
      label: "Open Team",
      action_type: { NavigateTo: { route: "/team/team-99" } },
      resolved: false,
    };

    renderInboxTab({
      gameState: createGameState([
        createMessage({ id: "m1", read: true, actions: [action] }),
      ]),
      initialMessageId: "m1",
      onNavigate,
    });

    fireEvent.click(screen.getByRole("button", { name: "Open Team" }));

    await waitFor(function (): void {
      expect(onNavigate).toHaveBeenCalledWith("__selectTeam", {
        messageId: "team-99",
      });
    });

    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  it("navigates to a player route without resolving the message action", async function (): Promise<void> {
    const onNavigate = vi.fn();
    const action: MessageAction = {
      id: "action-1",
      label: "Open Player",
      action_type: { NavigateTo: { route: "/player/player-99" } },
      resolved: false,
    };

    renderInboxTab({
      gameState: createGameState([
        createMessage({ id: "m1", read: true, actions: [action] }),
      ]),
      initialMessageId: "m1",
      onNavigate,
    });

    fireEvent.click(screen.getByRole("button", { name: "Open Player" }));

    await waitFor(function (): void {
      expect(onNavigate).toHaveBeenCalledWith("__selectPlayer", {
        messageId: "player-99",
      });
    });

    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  it("navigates to a dashboard tab and still resolves the action", async function (): Promise<void> {
    const onGameUpdate = vi.fn();
    const onNavigate = vi.fn();
    const resolvedGameState = createGameState([
      createMessage({ id: "m1", read: true }),
    ]);
    const action: MessageAction = {
      id: "action-1",
      label: "Open Squad",
      action_type: { NavigateTo: { route: "/dashboard?tab=Squad" } },
      resolved: false,
    };

    mockedInvoke.mockResolvedValue({ game: resolvedGameState, effect: null });

    renderInboxTab({
      gameState: createGameState([
        createMessage({ id: "m1", read: true, actions: [action] }),
      ]),
      initialMessageId: "m1",
      onGameUpdate,
      onNavigate,
    });

    fireEvent.click(screen.getByRole("button", { name: "Open Squad" }));

    await waitFor(function (): void {
      expect(onNavigate).toHaveBeenCalledWith("Squad", undefined);
      expect(mockedInvoke).toHaveBeenCalledWith("resolve_message_action", {
        messageId: "m1",
        actionId: "action-1",
        optionId: null,
      });
    });

    expect(onGameUpdate).toHaveBeenCalledWith(resolvedGameState);
  });

  it("renders localized effect feedback when the backend returns an effect key", async function (): Promise<void> {
    const onGameUpdate = vi.fn();
    const action: MessageAction = {
      id: "respond",
      label: "Respond",
      action_type: {
        ChooseOption: {
          options: [
            {
              id: "praise_back",
              label: "Return the praise",
              description: "Tell them how much you value their contribution.",
            },
          ],
        },
      },
      resolved: false,
    };
    const resolvedGameState = createGameState([
      createMessage({ id: "happy_player_p1", read: true, actions: [action] }),
    ]);

    mockedInvoke.mockResolvedValue({
      game: resolvedGameState,
      effect: "",
      effect_i18n_key: "test.effectFeedback",
      effect_i18n_params: { delta: "+3" },
    });

    renderInboxTab({
      gameState: createGameState([
        createMessage({ id: "happy_player_p1", read: true, actions: [action] }),
      ]),
      initialMessageId: "happy_player_p1",
      onGameUpdate,
    });

    fireEvent.click(screen.getByText("Return the praise"));

    await waitFor(function (): void {
      expect(
        screen.getByText("Outcome: Resolved morale +3"),
      ).toBeInTheDocument();
    });

    expect(onGameUpdate).toHaveBeenCalledWith(resolvedGameState);
  });

  it("renders the outcome label from the active locale", async function (): Promise<void> {
    const previousLanguage = mockTranslationState.language;
    const onGameUpdate = vi.fn();
    const action: MessageAction = {
      id: "respond",
      label: "Respond",
      action_type: {
        ChooseOption: {
          options: [
            {
              id: "praise_back",
              label: "Return the praise",
              description: "Tell them how much you value their contribution.",
            },
          ],
        },
      },
      resolved: false,
    };
    const resolvedGameState = createGameState([
      createMessage({ id: "happy_player_p1", read: true, actions: [action] }),
    ]);

    mockedInvoke.mockResolvedValue({
      game: resolvedGameState,
      effect: "",
      effect_i18n_key: "test.effectFeedback",
      effect_i18n_params: { delta: "+3" },
    });

    mockTranslationState.language = "pt-BR";

    try {
      renderInboxTab({
        gameState: createGameState([
          createMessage({
            id: "happy_player_p1",
            read: true,
            actions: [action],
          }),
        ]),
        initialMessageId: "happy_player_p1",
        onGameUpdate,
      });

      fireEvent.click(screen.getByText("Return the praise"));

      await waitFor(function (): void {
        expect(
          screen.getByText("Desfecho: Resolved morale +3"),
        ).toBeInTheDocument();
      });
    } finally {
      mockTranslationState.language = previousLanguage;
    }
  });

  it("renders delegated renewal report details with settings-aware money formatting", function (): void {
    useSettingsStore.setState({
      settings: {
        ...useSettingsStore.getState().settings,
        currency: "GBP",
        language: "en",
      },
      currency: { code: "GBP", symbol: "£", exchange_rate: 1 },
    });

    renderInboxTab({
      gameState: createGameState([
        createMessage({
          id: "delegated_renewals_2025-01-01_0",
          read: true,
          category: "Contract",
          subject_key: "be.msg.delegatedRenewals.subject",
          body_key: "be.msg.delegatedRenewals.body",
          i18n_params: {
            team: "Test FC",
            successes: "1",
            stalled: "1",
            failures: "1",
          },
          context: {
            team_id: "t1",
            player_id: null,
            fixture_id: null,
            match_result: null,
            delegated_renewal_report: {
              success_count: 1,
              failure_count: 1,
              stalled_count: 1,
              cases: [
                {
                  player_id: "p1",
                  player_name: "Alex Done",
                  status: "successful",
                  agreed_wage: 24000,
                  agreed_years: 3,
                },
                {
                  player_id: "p2",
                  player_name: "Ben Pending",
                  status: "stalled",
                  note_key: "be.msg.delegatedRenewals.notes.beyondLimits",
                  note_params: { wage: "26000", years: "4" },
                },
                {
                  player_id: "p3",
                  player_name: "Chris Failed",
                  status: "failed",
                  note_key:
                    "be.msg.delegatedRenewals.notes.relationshipBlocked",
                  note_params: {},
                },
              ],
            },
          },
        }),
      ]),
      initialMessageId: "delegated_renewals_2025-01-01_0",
    });

    expect(screen.getByTestId("delegated-renewal-report")).toBeInTheDocument();
    expect(
      screen.getByText(
        "Completed: Alex Done agreed to 3 year(s) on £24,000/wk.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Still difficult: Ben Pending — Their camp want around £26,000/wk for 4 years, which is beyond the delegation limits.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Failed: Chris Failed — They are not willing to commit through me under the current relationship and contract situation.",
      ),
    ).toBeInTheDocument();
  });

  it("opens the linked player profile from a message context button", function (): void {
    const onNavigate = vi.fn();
    const gameState = createGameState([
      createMessage({
        id: "injury-player-1",
        category: "Injury",
        read: true,
        context: {
          team_id: "t1",
          player_id: "player-1",
          fixture_id: null,
          match_result: null,
        },
      }),
    ]);
    gameState.players = [
      createProspect({ id: "player-1", full_name: "Rui Prospect" }),
    ];

    renderInboxTab({
      gameState,
      initialMessageId: "injury-player-1",
      onNavigate,
    });

    fireEvent.click(screen.getByRole("button", { name: "View profile: Rui Prospect" }));

    expect(onNavigate).toHaveBeenCalledWith("__selectPlayer", {
      messageId: "player-1",
    });
  });

  it("opens the referenced player profile from delegated renewal reports", function (): void {
    const onNavigate = vi.fn();

    renderInboxTab({
      gameState: createGameState([
        createMessage({
          id: "delegated_renewals_linked_2025-01-01_0",
          read: true,
          category: "Contract",
          context: {
            team_id: "t1",
            player_id: null,
            fixture_id: null,
            match_result: null,
            delegated_renewal_report: {
              success_count: 1,
              failure_count: 0,
              stalled_count: 0,
              cases: [
                {
                  player_id: "p1",
                  player_name: "Alex Done",
                  status: "successful",
                  agreed_wage: 24000,
                  agreed_years: 3,
                },
              ],
            },
          },
        }),
      ]),
      initialMessageId: "delegated_renewals_linked_2025-01-01_0",
      onNavigate,
    });

    fireEvent.click(screen.getByRole("button", { name: "View profile" }));

    expect(onNavigate).toHaveBeenCalledWith("__selectPlayer", {
      messageId: "p1",
    });
  });

  function jobOfferAcceptDeclineAction(targetTeamId: string): MessageAction {
    return {
      id: `respond_${targetTeamId}`,
      label: "Respond",
      action_type: {
        ChooseOption: {
          options: [
            {
              id: "accept",
              label: "Accept the position",
              description: "Take the job",
            },
            {
              id: "decline",
              label: "Decline the offer",
              description: "Stay where you are",
            },
          ],
        },
      },
      resolved: false,
    };
  }

  function gameStateWithCurrentClub(
    messages: MessageData[],
    teamId: string | null,
    teamName = "Old FC",
  ): GameStateData {
    const state = createGameState(messages);
    state.manager.team_id = teamId;
    if (teamId) {
      (state.teams as unknown as Array<{ id: string; name: string }>).push({
        id: teamId,
        name: teamName,
      });
    }
    return state;
  }

  it("shows the switch-club confirm dialog when an employed manager accepts a job offer", function (): void {
    const action = jobOfferAcceptDeclineAction("team2");
    const message = createMessage({
      id: "job_offer_team2_2025-01-01",
      category: "JobOffer",
      read: true,
      actions: [action],
      context: {
        team_id: "team2",
        player_id: null,
        fixture_id: null,
        match_result: null,
      },
    });

    renderInboxTab({
      gameState: gameStateWithCurrentClub([message], "t1", "Old FC"),
      initialMessageId: "job_offer_team2_2025-01-01",
    });

    fireEvent.click(screen.getByText("Accept the position"));

    expect(screen.getByTestId("switch-club-confirm-modal")).toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalledWith(
      "resolve_message_action",
      expect.anything(),
    );
  });

  it("does not invoke the action when the switch-club dialog is cancelled", function (): void {
    const action = jobOfferAcceptDeclineAction("team2");
    const message = createMessage({
      id: "job_offer_team2_2025-01-01",
      category: "JobOffer",
      read: true,
      actions: [action],
      context: {
        team_id: "team2",
        player_id: null,
        fixture_id: null,
        match_result: null,
      },
    });

    renderInboxTab({
      gameState: gameStateWithCurrentClub([message], "t1"),
      initialMessageId: "job_offer_team2_2025-01-01",
    });

    fireEvent.click(screen.getByText("Accept the position"));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(
      screen.queryByTestId("switch-club-confirm-modal"),
    ).not.toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalledWith(
      "resolve_message_action",
      expect.anything(),
    );
  });

  it("invokes the action with optionId=accept when the switch-club dialog is confirmed", async function (): Promise<void> {
    const action = jobOfferAcceptDeclineAction("team2");
    const message = createMessage({
      id: "job_offer_team2_2025-01-01",
      category: "JobOffer",
      read: true,
      actions: [action],
      context: {
        team_id: "team2",
        player_id: null,
        fixture_id: null,
        match_result: null,
      },
    });
    mockedInvoke.mockResolvedValue({
      game: gameStateWithCurrentClub([], "team2", "New FC"),
      effect: null,
      effect_i18n_key: null,
      effect_i18n_params: null,
    });

    renderInboxTab({
      gameState: gameStateWithCurrentClub([message], "t1"),
      initialMessageId: "job_offer_team2_2025-01-01",
    });

    fireEvent.click(screen.getByText("Accept the position"));
    fireEvent.click(screen.getByTestId("switch-club-confirm"));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("resolve_message_action", {
        messageId: "job_offer_team2_2025-01-01",
        actionId: "respond_team2",
        optionId: "accept",
      });
    });
  });

  it("does not show the switch-club dialog when an employed manager declines a job offer", async function (): Promise<void> {
    const action = jobOfferAcceptDeclineAction("team2");
    const message = createMessage({
      id: "job_offer_team2_2025-01-01",
      category: "JobOffer",
      read: true,
      actions: [action],
      context: {
        team_id: "team2",
        player_id: null,
        fixture_id: null,
        match_result: null,
      },
    });
    mockedInvoke.mockResolvedValue({
      game: gameStateWithCurrentClub([], "t1"),
      effect: null,
      effect_i18n_key: null,
      effect_i18n_params: null,
    });

    renderInboxTab({
      gameState: gameStateWithCurrentClub([message], "t1"),
      initialMessageId: "job_offer_team2_2025-01-01",
    });

    fireEvent.click(screen.getByText("Decline the offer"));

    expect(
      screen.queryByTestId("switch-club-confirm-modal"),
    ).not.toBeInTheDocument();
    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("resolve_message_action", {
        messageId: "job_offer_team2_2025-01-01",
        actionId: "respond_team2",
        optionId: "decline",
      });
    });
  });

  it("does not show the switch-club dialog when an unemployed manager accepts a job offer", async function (): Promise<void> {
    const action = jobOfferAcceptDeclineAction("team2");
    const message = createMessage({
      id: "job_offer_team2_2025-01-01",
      category: "JobOffer",
      read: true,
      actions: [action],
      context: {
        team_id: "team2",
        player_id: null,
        fixture_id: null,
        match_result: null,
      },
    });
    mockedInvoke.mockResolvedValue({
      game: gameStateWithCurrentClub([], "team2", "New FC"),
      effect: null,
      effect_i18n_key: null,
      effect_i18n_params: null,
    });

    renderInboxTab({
      gameState: gameStateWithCurrentClub([message], null),
      initialMessageId: "job_offer_team2_2025-01-01",
    });

    fireEvent.click(screen.getByText("Accept the position"));

    expect(
      screen.queryByTestId("switch-club-confirm-modal"),
    ).not.toBeInTheDocument();
    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("resolve_message_action", {
        messageId: "job_offer_team2_2025-01-01",
        actionId: "respond_team2",
        optionId: "accept",
      });
    });
  });

  it("formats delegated renewal money values only once in dot-separated locales", async function (): Promise<void> {
    const previousLanguage = i18n.language;
    const previousSettings = useSettingsStore.getState().settings;

    await i18n.changeLanguage("de");
    useSettingsStore.setState({
      settings: { ...previousSettings, language: "de", currency: "EUR" },
      currency: { code: "EUR", symbol: "€", exchange_rate: 1 },
    });

    try {
      renderInboxTab({
        gameState: createGameState([
          createMessage({
            id: "delegated_renewals_locale_2025-01-01_0",
            read: true,
            category: "Contract",
            context: {
              team_id: "t1",
              player_id: null,
              fixture_id: null,
              match_result: null,
              delegated_renewal_report: {
                success_count: 1,
                failure_count: 1,
                stalled_count: 0,
                cases: [
                  {
                    player_id: "p1",
                    player_name: "Alex Done",
                    status: "successful",
                    agreed_wage: 24000,
                    agreed_years: 3,
                  },
                  {
                    player_id: "p2",
                    player_name: "Chris Failed",
                    status: "failed",
                    note_key: "be.msg.delegatedRenewals.notes.boardWagePolicy",
                    note_params: { budget: "24000" },
                  },
                ],
              },
            },
          }),
        ]),
        initialMessageId: "delegated_renewals_locale_2025-01-01_0",
      });

      expect(
        screen.getByText(
          "Completed: Alex Done agreed to 3 year(s) on €24.000/wk.",
        ),
      ).toBeInTheDocument();
      expect(
        screen.getByText(
          "Failed: Chris Failed — Board wage policy blocks this renewal. Keep annual wages near €24.000 while we recover.",
        ),
      ).toBeInTheDocument();
    } finally {
      await i18n.changeLanguage(previousLanguage);
      useSettingsStore.setState({
        settings: previousSettings,
      });
    }
  });

  it("tells the user that player-event response outcomes vary", function (): void {
    const action: MessageAction = {
      id: "respond",
      label: "Respond",
      action_type: {
        ChooseOption: {
          options: [
            {
              id: "encourage",
              label: "Encourage them",
              description: "Try to lift their spirits.",
            },
          ],
        },
      },
      resolved: false,
    };

    renderInboxTab({
      gameState: createGameState([
        createMessage({
          id: "morale_talk_p1",
          category: "PlayerMorale",
          read: true,
          actions: [action],
        }),
      ]),
      initialMessageId: "morale_talk_p1",
    });

    expect(
      screen.getByText("Choose your response — outcome varies"),
    ).toBeInTheDocument();
  });

  it("shows the selected youth scouting target on youth recruitment reports", function (): void {
    renderInboxTab({
      gameState: createGameState([
        createMessage({
          id: "youth-scout-1",
          category: "ScoutReport",
          read: true,
          subject: "Youth prospect found",
          body: "Scout report body",
          context: {
            team_id: "t1",
            player_id: "p1",
            fixture_id: null,
            youth_target_position: "Defender",
            match_result: null,
          },
        }),
      ]),
      initialMessageId: "youth-scout-1",
    });

    expect(screen.getByText("Youth target")).toBeInTheDocument();
    expect(screen.getByText("Defender")).toBeInTheDocument();
  });

  it("renders translated youth recruitment reports with contract details and signed prospects still visible", function (): void {
    renderInboxTab({
      gameState: createGameState([
        createMessage({
          id: "youth-scout-2",
          category: "ScoutReport",
          read: true,
          subject: "fallback subject",
          body: "fallback body",
          sender: "Joao Scout",
          sender_role: "Scout",
          subject_key: "be.msg.youthRecruitmentReport.subject",
          body_key: "be.msg.youthRecruitmentReport.bodyTargeted",
          i18n_params: {
            scout: "Joao Scout",
            team: "FC Test",
            count: "2",
            regionLabel: "scouting.regionDomestic",
            objectiveLabel: "scouting.objectiveBalanced",
            targetLabel: "common.positions.Defender",
          },
          context: {
            team_id: "t1",
            player_id: null,
            fixture_id: null,
            youth_target_position: "Defender",
            youth_search_region: "Domestic",
            youth_search_objective: "Balanced",
            youth_prospects: [
              createProspect({
                id: "prospect-signed",
                full_name: "Mateus Anchor",
                team_id: "t1",
              }),
              createProspect({
                id: "prospect-open",
                full_name: "Leo Builder",
                team_id: null,
                position: "Forward",
                natural_position: "Forward",
              }),
            ],
            match_result: null,
          },
          actions: [
            {
              id: "prospect:prospect-signed",
              label: "Respond",
              action_type: {
                ChooseOption: {
                  options: [],
                },
              },
              resolved: true,
            },
            {
              id: "prospect:prospect-open",
              label: "Respond",
              action_type: {
                ChooseOption: {
                  options: [
                    {
                      id: "sign",
                      label: "placeholder",
                      description: "placeholder",
                      label_key: "be.msg.youthRecruitment.option.sign.label",
                      description_key:
                        "be.msg.youthRecruitment.option.sign.description",
                    },
                    {
                      id: "shortlist",
                      label: "placeholder",
                      description: "placeholder",
                      label_key: "be.msg.youthRecruitment.option.shortlist.label",
                      description_key:
                        "be.msg.youthRecruitment.option.shortlist.description",
                    },
                    {
                      id: "discard",
                      label: "placeholder",
                      description: "placeholder",
                      label_key: "be.msg.youthRecruitment.option.discard.label",
                      description_key:
                        "be.msg.youthRecruitment.option.discard.description",
                    },
                  ],
                },
              },
              resolved: false,
            },
          ] as MessageAction[],
        }),
      ]),
      initialMessageId: "youth-scout-2",
    });

    expect(screen.getAllByText("Scout Report — Youth Recruitment")).toHaveLength(2);
    expect(screen.getByText("Domestic")).toBeInTheDocument();
    expect(screen.getByText("Balanced")).toBeInTheDocument();
    expect(screen.getAllByText("Signed to academy").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "View profile" })).toBeInTheDocument();
    expect(screen.getAllByText(/Wage\/wk:/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Market Value:/).length).toBeGreaterThan(0);
    expect(
      screen.getByRole("button", { name: "Sign to academy" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Shortlist" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Discard" })).toBeInTheDocument();
  });
});
