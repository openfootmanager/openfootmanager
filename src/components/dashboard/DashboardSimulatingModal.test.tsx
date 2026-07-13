import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { AdvanceRecap, DigestEntry, RecapMatch } from "./advanceRecap";
import DashboardSimulatingModal from "./DashboardSimulatingModal";

// jsdom has no scrollIntoView; the modal auto-scrolls the feed on new entries.
Element.prototype.scrollIntoView = vi.fn();

vi.mock("react-i18next", () => ({
  // The modal pulls in utils/backendI18n → i18n/index, which registers the
  // react plugin at import time; a no-op plugin keeps that init happy.
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "match.shootout.shootoutScore")
        return `Penalties: ${params?.h} - ${params?.a}`;
      return key;
    },
    i18n: { language: "en" },
  }),
}));

function recapWith(overrides: Partial<AdvanceRecap> = {}): AdvanceRecap {
  return {
    advancedTo: "2026-07-11",
    matches: [],
    transfers: [],
    news: [],
    inbox: [],
    hasEvents: true,
    userTransferInWindow: false,
    userNewsInWindow: false,
    ...overrides,
  };
}

function match(overrides: Partial<RecapMatch> = {}): RecapMatch {
  return {
    date: "2026-07-10",
    competition: "World Cup 2026",
    international: true,
    home_team: "Brazil",
    away_team: "France",
    home_goals: 1,
    away_goals: 1,
    involves_user: false,
    ...overrides,
  };
}

function entryWith(matches: RecapMatch[]): DigestEntry {
  return { date: "2026-07-10", recap: recapWith({ matches }) };
}

describe("DashboardSimulatingModal penalty shootouts", () => {
  it("shows the shootout score for a penalty-decided result", () => {
    render(
      <DashboardSimulatingModal
        digestEntries={[entryWith([match({ home_penalties: 4, away_penalties: 2 })])]}
      />,
    );

    expect(screen.getByText("Penalties: 4 - 2")).toBeInTheDocument();
  });

  it("omits the shootout line for a result settled in normal time", () => {
    render(
      <DashboardSimulatingModal
        digestEntries={[entryWith([match({ home_goals: 2, away_goals: 1 })])]}
      />,
    );

    expect(screen.queryByText(/Penalties:/)).not.toBeInTheDocument();
  });
});

describe("DashboardSimulatingModal batch resume", () => {
  it("presents a crunching batch behind a kept feed as in-progress without Close or Stop", () => {
    render(
      <DashboardSimulatingModal
        digestEntries={[entryWith([match()])]}
        isBatchAdvancing
        onStop={vi.fn()}
        onDismiss={vi.fn()}
      />,
    );

    expect(screen.getByText("dashboard.digestAdvancing")).toBeInTheDocument();
    expect(screen.queryByText("dashboard.digestDone")).not.toBeInTheDocument();
    expect(screen.queryByText("common.close")).not.toBeInTheDocument();
    // Only the streaming loop is stoppable; a batch backend call is not.
    expect(screen.queryByText("dashboard.digestStop")).not.toBeInTheDocument();
  });

  it("returns to the done state with Close once the batch lands", () => {
    render(
      <DashboardSimulatingModal
        digestEntries={[entryWith([match()])]}
        onDismiss={vi.fn()}
      />,
    );

    expect(screen.getByText("dashboard.digestDone")).toBeInTheDocument();
    expect(screen.getByText("common.close")).toBeInTheDocument();
  });
});

describe("DashboardSimulatingModal attention-event stop", () => {
  it("lists the event reasons and wires resume and dismiss", () => {
    const onResume = vi.fn();
    const onDismiss = vi.fn();
    render(
      <DashboardSimulatingModal
        digestEntries={[entryWith([])]}
        stopReason={{ kind: "event", events: ["highPriorityInbox", "transferWindow"] }}
        onResume={onResume}
        onDismiss={onDismiss}
      />,
    );

    expect(screen.getByText("dashboard.digestEventStop")).toBeInTheDocument();
    expect(screen.getByText("dashboard.digestEventInbox")).toBeInTheDocument();
    expect(screen.getByText("dashboard.digestEventTransferWindow")).toBeInTheDocument();

    fireEvent.click(screen.getByText("dashboard.digestContinue"));
    expect(onResume).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByText("dashboard.digestClose"));
    expect(onDismiss).toHaveBeenCalledOnce();
  });
});
