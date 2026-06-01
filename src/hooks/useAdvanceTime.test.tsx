import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState, type JSX } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../store/gameStore";
import { useAdvanceTime } from "./useAdvanceTime";

const navigateMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("react-router-dom", () => ({
  useNavigate: () => navigateMock,
}));

const mockedInvoke = vi.mocked(invoke);

function HookHarness(props: {
  defaultMatchMode?: "live" | "spectator" | "delegate";
  hasMatchToday: boolean;
}): JSX.Element {
  const [, setGameState] = useState<GameStateData | null>(null);
  const {
    blockerModal,
    handleConfirmMatch,
    handleContinue,
    handleSkipToMatchDay,
    showMatchConfirm,
  } =
    useAdvanceTime(
      (state) => setGameState(state),
      props.hasMatchToday,
      props.defaultMatchMode,
      true,
      false,
    );

  return (
    <div>
      <button onClick={() => void handleContinue()}>Continue</button>
      <button onClick={handleConfirmMatch}>Confirm Match</button>
      <button onClick={() => void handleSkipToMatchDay()}>Skip</button>
      <div data-testid="show-match-confirm">{String(showMatchConfirm)}</div>
      <div data-testid="blocker-count">
        {blockerModal?.blockers.length ?? 0}
      </div>
    </div>
  );
}

describe("useAdvanceTime", function (): void {
  beforeEach(function resetMocks(): void {
    mockedInvoke.mockReset();
    navigateMock.mockReset();
  });

  it("shows match confirmation before advancing on match day", async function (): Promise<void> {
    render(<HookHarness hasMatchToday defaultMatchMode="live" />);

    fireEvent.click(screen.getByRole("button", { name: "Continue" }));

    await waitFor(function (): void {
      expect(screen.getByTestId("show-match-confirm")).toHaveTextContent(
        "true",
      );
    });

    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  it("navigates to the live match with snapshot and fixture state after confirmation", async function (): Promise<void> {
    const snapshot = {
      phase: "PreKickOff",
      current_minute: 0,
      home_score: 0,
      away_score: 0,
      possession: "Home",
      ball_zone: "Midfield",
    };

    mockedInvoke.mockResolvedValueOnce({
      action: "live_match",
      fixture_index: 7,
      mode: "live",
      snapshot,
    });

    render(<HookHarness hasMatchToday defaultMatchMode="live" />);

    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    fireEvent.click(screen.getByRole("button", { name: "Confirm Match" }));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("advance_time_with_mode", {
        mode: "live",
      });
    });

    expect(navigateMock).toHaveBeenCalledWith("/match", {
      state: {
        fixtureIndex: 7,
        mode: "live",
        snapshot,
      },
    });
  });

  it("checks blocking actions before a normal continue and stops when blockers exist", async function (): Promise<void> {
    mockedInvoke.mockResolvedValueOnce([
      {
        id: "urgent_messages",
        severity: "info",
        tab: "Inbox",
        text: "1 urgent unread message(s)",
      },
    ]);

    render(<HookHarness hasMatchToday={false} defaultMatchMode="spectator" />);

    fireEvent.click(screen.getByRole("button", { name: "Continue" }));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("check_blocking_actions");
      expect(screen.getByTestId("blocker-count")).toHaveTextContent("1");
    });

    expect(mockedInvoke).toHaveBeenCalledTimes(1);
    expect(navigateMock).not.toHaveBeenCalled();
  });

  it("advances time on a normal day when blocker checks return empty", async function (): Promise<void> {
    const advancedGame = {
      clock: {
        current_date: "2026-07-02",
        start_date: "2026-07-01",
      },
    } as GameStateData;

    mockedInvoke.mockResolvedValueOnce([]).mockResolvedValueOnce({
      action: "advanced",
      game: advancedGame,
    });

    render(<HookHarness hasMatchToday={false} defaultMatchMode="live" />);

    fireEvent.click(screen.getByRole("button", { name: "Continue" }));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenNthCalledWith(1, "check_blocking_actions");
      expect(mockedInvoke).toHaveBeenNthCalledWith(
        2,
        "advance_time_with_mode",
        {
          mode: "live",
        },
      );
    });

    expect(navigateMock).not.toHaveBeenCalled();
  });

  it("checks blocking actions before skipping to match day and stops when blockers exist", async function (): Promise<void> {
    mockedInvoke.mockResolvedValueOnce([
      {
        id: "contract_expiry",
        severity: "warning",
        tab: "Finances",
        text: "A contract decision is still pending",
      },
    ]);

    render(<HookHarness hasMatchToday={false} defaultMatchMode="live" />);

    fireEvent.click(screen.getByRole("button", { name: "Skip" }));

    await waitFor(function (): void {
      expect(mockedInvoke).toHaveBeenCalledWith("check_blocking_actions");
      expect(screen.getByTestId("blocker-count")).toHaveTextContent("1");
    });

    expect(mockedInvoke).toHaveBeenCalledTimes(1);
  });
});
