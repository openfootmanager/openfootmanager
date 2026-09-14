import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  clearOldMessages,
  deleteMessage,
  deleteMessages,
  markAllMessagesRead,
  markMessageRead,
  resolveMessageAction,
} from "./inboxService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("inboxService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("marks a single message as read", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(markMessageRead("m1")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("mark_message_read", {
      messageId: "m1",
    });
  });

  it("resolves a message action", async () => {
    const response = { game: { manager: { id: "manager-1" } } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(resolveMessageAction("m1", "a1", "o1")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("resolve_message_action", {
      messageId: "m1",
      actionId: "a1",
      optionId: "o1",
    });
  });

  it("marks all messages as read", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(markAllMessagesRead()).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("mark_all_messages_read");
  });

  it("clears old messages", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(clearOldMessages()).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("clear_old_messages");
  });

  it("deletes a single message", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(deleteMessage("m1")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("delete_message", {
      messageId: "m1",
    });
  });

  it("deletes multiple messages", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(deleteMessages(["m1", "m2"])).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("delete_messages", {
      messageIds: ["m1", "m2"],
    });
  });
});
