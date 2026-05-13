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
        "inbox.chooseResponseOutcomeVaries": "Choose your response — outcome varies",
        "inbox.deleteMessage": "Delete message",
        "inbox.effectOutcomeLabel": "Outcome",
        "inbox.markAsRead": "Mark as read",
        "inbox.openMessage": "Open message",
        "scouting.youthTargetLabel": "Youth target",
        "scouting.youthAnyPosition": "Any position",
        "common.positions.Defender": "Defender",
        "common.positions.Midfielder": "Midfielder",
        "common.positions.Forward": "Forward",
        "inbox.sortByDate": "Sort messages by date",
        "inbox.sortLabel": "Sort",
        "inbox.sortNewest": "Newest first",
        "inbox.sortOldest": "Oldest first",
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
      effect: "Player beams at the praise. Morale +3",
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
      effect: "Player beams at the praise. Morale +3",
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
});
