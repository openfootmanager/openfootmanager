import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  clearOldMessages,
  deleteMessage,
  deleteMessages,
  fetchArchivedMessages,
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
      careerToken: 0,
      messageId: "m1",
    });
  });

  it("resolves a message action", async () => {
    const response = { game: { manager: { id: "manager-1" } } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(resolveMessageAction("m1", "a1", "o1")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("resolve_message_action", {
      careerToken: 0,
      messageId: "m1",
      actionId: "a1",
      optionId: "o1",
    });
  });

  it("marks all messages as read", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(markAllMessagesRead()).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("mark_all_messages_read", { careerToken: 0 });
  });

  it("clears old messages", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(clearOldMessages()).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("clear_old_messages", { careerToken: 0 });
  });

  it("deletes a single message", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(deleteMessage("m1")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("delete_message", {
      careerToken: 0,
      messageId: "m1",
    });
  });

  it("deletes multiple messages", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(deleteMessages(["m1", "m2"])).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("delete_messages", {
      careerToken: 0,
      messageIds: ["m1", "m2"],
    });
  });

  it("asks for the first page of archived mail with no cursor", async () => {
    const response = { items: [], next_before: null };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(fetchArchivedMessages(null)).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("get_archived_messages", {
      before: null,
      limit: 50,
    });
  });

  it("continues archived mail from the cursor the last page returned", async () => {
    mockedInvoke.mockResolvedValueOnce({ items: [], next_before: null });

    await fetchArchivedMessages({ date: "2030-01-01", id: "m1" }, 10);

    expect(mockedInvoke).toHaveBeenCalledWith("get_archived_messages", {
      before: { date: "2030-01-01", id: "m1" },
      limit: 10,
    });
  });
});
