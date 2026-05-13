import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { MessageData } from "../../store/gameStore";
import HomeRecentMessagesCard from "./HomeRecentMessagesCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => {
      if (key === "inbox.openMessage") return "Open message";
      if (key === "home.viewAll") return "View All";
      if (key === "home.recentMessages") return "Recent Messages";
      if (key === "home.noMessages") return "No messages.";
      return key;
    },
  }),
}));

function createMessage(overrides: Partial<MessageData> = {}): MessageData {
  return {
    id: "message-1",
    subject: "Welcome",
    body: "Welcome to the club",
    sender: "Board",
    sender_role: "Board",
    date: "2025-01-10",
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

describe("HomeRecentMessagesCard", () => {
  it("renders recent messages and delegates inbox navigation", () => {
    const onNavigate = vi.fn();

    render(
      <HomeRecentMessagesCard
        messages={[createMessage()]}
        lang="en"
        onNavigate={onNavigate}
      />,
    );

    expect(screen.getByText("Recent Messages")).toBeInTheDocument();
    expect(screen.getByText("Welcome")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Welcome"));

    expect(onNavigate).toHaveBeenCalledWith("Inbox", { messageId: "message-1" });
  });

  it("renders the empty state when there are no recent messages", () => {
    render(<HomeRecentMessagesCard messages={[]} lang="en" />);

    expect(screen.getByText("No messages.")).toBeInTheDocument();
  });

  it("offers a context menu action to open the message in inbox", () => {
    const onNavigate = vi.fn();

    render(
      <HomeRecentMessagesCard
        messages={[createMessage()]}
        lang="en"
        onNavigate={onNavigate}
      />,
    );

    fireEvent.contextMenu(screen.getByText("Welcome"));
    fireEvent.click(screen.getByRole("button", { name: "Open message" }));

    expect(onNavigate).toHaveBeenCalledWith("Inbox", { messageId: "message-1" });
  });
});