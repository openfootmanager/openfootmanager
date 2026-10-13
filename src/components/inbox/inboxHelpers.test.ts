import { describe, expect, it } from "vitest";

import type { MessageData } from "../../store/gameStore";
import {
  getFilteredMessages,
  getNavigationTarget,
  isPlayerEventMessage,
  sortInboxMessages,
  toggleVisibleSelection,
  UNREAD_FILTER,
} from "./inboxHelpers";

function createMessage(overrides: Partial<MessageData> = {}): MessageData {
  return {
    id: "m1",
    subject: "Subject",
    body: "Body",
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
      match_result: null,
    },
    ...overrides,
  };
}

describe("inboxHelpers", () => {
  it("filters unread and category-specific message sets", () => {
    const messages = [
      createMessage({ id: "m1", read: false, category: "System" }),
      createMessage({ id: "m2", read: true, category: "Finance" }),
      createMessage({ id: "m3", read: false, category: "Finance" }),
    ];

    expect(getFilteredMessages(messages, UNREAD_FILTER).map((message) => message.id)).toEqual([
      "m1",
      "m3",
    ]);
    expect(getFilteredMessages(messages, "Finance").map((message) => message.id)).toEqual([
      "m2",
      "m3",
    ]);
  });

  it("sorts inbox messages by newest or oldest date", () => {
    const messages = [
      createMessage({ id: "m1", date: "2025-01-03" }),
      createMessage({ id: "m2", date: "2025-01-01" }),
      createMessage({ id: "m3", date: "2025-01-02" }),
    ];

    expect(sortInboxMessages(messages, "newest").map((message) => message.id)).toEqual([
      "m1",
      "m3",
      "m2",
    ]);
    expect(sortInboxMessages(messages, "oldest").map((message) => message.id)).toEqual([
      "m2",
      "m3",
      "m1",
    ]);
  });

  it("maps team, tab, and simple routes into dashboard navigation targets", () => {
    expect(getNavigationTarget("/team/team-99")).toEqual({
      tab: "__selectTeam",
      context: { messageId: "team-99" },
      shouldResolveAction: false,
    });

    expect(getNavigationTarget("/player/player-99")).toEqual({
      tab: "__selectPlayer",
      context: { messageId: "player-99" },
      shouldResolveAction: false,
    });

    expect(getNavigationTarget("/dashboard?tab=Squad")).toEqual({
      tab: "Squad",
      shouldResolveAction: true,
    });

    expect(getNavigationTarget("/transfers")).toEqual({
      tab: "Transfers",
      shouldResolveAction: true,
    });
  });

  it("recognizes player-event message prefixes", () => {
    expect(isPlayerEventMessage("morale_talk_p1")).toBe(true);
    expect(isPlayerEventMessage("contract_concern_p2")).toBe(true);
    expect(isPlayerEventMessage("plain_message")).toBe(false);
  });
});

describe("toggleVisibleSelection", () => {
  /**
   * Given some messages are selected and others are visible but not selected
   * When the visible set is toggled
   * Then every visible message becomes selected and a selection outside the filter is kept.
   */
  it("selects every visible message and keeps a hidden selection", () => {
    expect(toggleVisibleSelection(["hidden", "a"], ["a", "b", "c"])).toEqual([
      "hidden",
      "a",
      "b",
      "c",
    ]);
  });

  /**
   * Given every visible message is already selected
   * When the visible set is toggled
   * Then the visible messages are deselected and a selection outside the filter is kept.
   */
  it("deselects the visible messages when all of them are selected", () => {
    expect(toggleVisibleSelection(["hidden", "a", "b"], ["a", "b"])).toEqual(["hidden"]);
  });

  /**
   * Given nothing is visible
   * When the visible set is toggled
   * Then the selection is unchanged.
   */
  it("does nothing when no message is visible", () => {
    expect(toggleVisibleSelection(["x"], [])).toEqual(["x"]);
  });
});
