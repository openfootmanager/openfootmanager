import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { GameStateData } from "../../store/gameStore";
import JobOpportunitiesCard from "./JobOpportunitiesCard";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (
      key: string,
      fallbackOrParams?: string | Record<string, string | number>,
      maybeParams?: Record<string, string | number>,
    ) => {
      const params =
        typeof fallbackOrParams === "object" ? fallbackOrParams : maybeParams;
      const fallback =
        typeof fallbackOrParams === "string" ? fallbackOrParams : undefined;
      if (key === "jobs.opportunitiesTitle") return "Job Opportunities";
      if (key === "jobs.applyButton") return "Apply";
      if (key === "jobs.applicationSent") return "Applying...";
      if (key === "jobs.hired") return "You have been appointed manager!";
      if (key === "jobs.rejected") return "Your application was unsuccessful.";
      if (key === "jobs.noJobs") return "No positions currently available.";
      if (key === "jobs.refresh") return "Check for new positions";
      if (key === "jobs.sameTeam") return "You are already managing that club.";
      if (key === "jobs.notBetterClub")
        return "You can only apply for clubs that are a step up from your current one.";
      if (key === "jobs.leaguePosition")
        return `Last Season: ${params?.position}`;
      if (key === "jobs.switchConfirmTitle") return "Leave your current club?";
      if (key === "jobs.switchConfirmBody")
        return `Accepting this opportunity will end your tenure at ${params?.currentClub} and move you to ${params?.newClub}.`;
      if (key === "jobs.switchConfirmAccept") return "Accept new role";
      if (key === "common.cancel") return "Cancel";
      return fallback ?? key;
    },
  }),
}));

const { getAvailableJobsMock, applyForJobMock } = vi.hoisted(() => ({
  getAvailableJobsMock: vi.fn(),
  applyForJobMock: vi.fn(),
}));

vi.mock("../../services/jobService", () => ({
  getAvailableJobs: (...args: unknown[]) => getAvailableJobsMock(...args),
  applyForJob: (...args: unknown[]) => applyForJobMock(...args),
}));

function createGameState(): GameStateData {
  return {
    clock: { current_date: "2026-11-01", start_date: "2026-07-01" },
    manager: {
      id: "mgr1",
      first_name: "Alex",
      last_name: "Boss",
      date_of_birth: "1980-01-01",
      nationality: "England",
      reputation: 500,
      satisfaction: 50,
      fan_approval: 50,
      team_id: null,
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
    messages: [],
    news: [],
    league: null,
    scouting_assignments: [],
    board_objectives: [],
  } as unknown as GameStateData;
}

function createEmployedGameState(): GameStateData {
  const state = createGameState();
  state.manager.team_id = "team1";
  (state.teams as unknown[]).push({
    id: "team1",
    name: "Old FC",
    short_name: "OLD",
    country: "England",
    city: "Oldville",
    stadium: "Old Ground",
    capacity: 20000,
    reputation: 500,
    manager_id: "mgr1",
    history: [],
  } as unknown as never);
  return state;
}

describe("JobOpportunitiesCard", () => {
  beforeEach(() => {
    getAvailableJobsMock.mockReset();
    applyForJobMock.mockReset();
  });

  it("shows the loading spinner before jobs resolve", () => {
    getAvailableJobsMock.mockReturnValue(new Promise(() => {}));

    render(
      <JobOpportunitiesCard
        gameState={createGameState()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(screen.getByText("Job Opportunities")).toBeInTheDocument();
    // Spinner is visible (no empty-state text, no job rows)
    expect(
      screen.queryByText("No positions currently available."),
    ).not.toBeInTheDocument();
  });

  it("renders the empty state when no jobs are returned", async () => {
    getAvailableJobsMock.mockResolvedValue([]);

    render(
      <JobOpportunitiesCard
        gameState={createGameState()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(
      await screen.findByText("No positions currently available."),
    ).toBeInTheDocument();
  });

  it("renders returned jobs with team name, city and last league position", async () => {
    getAvailableJobsMock.mockResolvedValue([
      {
        team_id: "team2",
        team_name: "New FC",
        city: "Newville",
        reputation: 480,
        last_league_position: 7,
      },
    ]);

    render(
      <JobOpportunitiesCard
        gameState={createGameState()}
        onGameUpdate={vi.fn()}
      />,
    );

    expect(await screen.findByText("New FC")).toBeInTheDocument();
    expect(screen.getByText("Newville")).toBeInTheDocument();
    expect(screen.getByText("Last Season: 7")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Apply" }),
    ).toBeInTheDocument();
  });

  it("shows a success message and propagates updated game state on hire", async () => {
    const hiredGame = createGameState();
    hiredGame.manager.team_id = "team2";
    getAvailableJobsMock.mockResolvedValue([
      {
        team_id: "team2",
        team_name: "New FC",
        city: "Newville",
        reputation: 480,
        last_league_position: 7,
      },
    ]);
    applyForJobMock.mockResolvedValue({ result: "hired", game: hiredGame });

    const onGameUpdate = vi.fn();
    render(
      <JobOpportunitiesCard
        gameState={createGameState()}
        onGameUpdate={onGameUpdate}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Apply" }));

    expect(
      await screen.findByText("You have been appointed manager!"),
    ).toBeInTheDocument();
    expect(applyForJobMock).toHaveBeenCalledWith("team2");
    expect(onGameUpdate).toHaveBeenCalledWith(hiredGame);
  });

  it("shows a failure message and refreshes jobs when rejected", async () => {
    getAvailableJobsMock
      .mockResolvedValueOnce([
        {
          team_id: "team2",
          team_name: "New FC",
          city: "Newville",
          reputation: 480,
          last_league_position: 7,
        },
      ])
      .mockResolvedValueOnce([]);
    applyForJobMock.mockResolvedValue({
      result: "rejected",
      game: createGameState(),
    });

    render(
      <JobOpportunitiesCard
        gameState={createGameState()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Apply" }));

    expect(
      await screen.findByText("Your application was unsuccessful."),
    ).toBeInTheDocument();
    await waitFor(() =>
      expect(getAvailableJobsMock).toHaveBeenCalledTimes(2),
    );
  });

  it("shows the switch-club confirm dialog when an employed manager applies", async () => {
    getAvailableJobsMock.mockResolvedValue([
      {
        team_id: "team3",
        team_name: "Elite FC",
        city: "Elitetown",
        reputation: 800,
        last_league_position: 2,
      },
    ]);

    render(
      <JobOpportunitiesCard
        gameState={createEmployedGameState()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Apply" }));

    expect(
      await screen.findByTestId("switch-club-confirm-modal"),
    ).toBeInTheDocument();
    // No application has been sent yet — the modal must gate the call.
    expect(applyForJobMock).not.toHaveBeenCalled();
    expect(
      screen.getByText(/end your tenure at Old FC/i),
    ).toBeInTheDocument();
  });

  it("does not apply when the switch-club confirm is cancelled", async () => {
    getAvailableJobsMock.mockResolvedValue([
      {
        team_id: "team3",
        team_name: "Elite FC",
        city: "Elitetown",
        reputation: 800,
        last_league_position: 2,
      },
    ]);

    render(
      <JobOpportunitiesCard
        gameState={createEmployedGameState()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Apply" }));
    fireEvent.click(await screen.findByRole("button", { name: "Cancel" }));

    expect(applyForJobMock).not.toHaveBeenCalled();
    expect(
      screen.queryByTestId("switch-club-confirm-modal"),
    ).not.toBeInTheDocument();
  });

  it("applies after the user confirms the switch-club dialog", async () => {
    const hiredGame = createEmployedGameState();
    hiredGame.manager.team_id = "team3";
    getAvailableJobsMock.mockResolvedValue([
      {
        team_id: "team3",
        team_name: "Elite FC",
        city: "Elitetown",
        reputation: 800,
        last_league_position: 2,
      },
    ]);
    applyForJobMock.mockResolvedValue({ result: "hired", game: hiredGame });

    const onGameUpdate = vi.fn();
    render(
      <JobOpportunitiesCard
        gameState={createEmployedGameState()}
        onGameUpdate={onGameUpdate}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Apply" }));
    fireEvent.click(
      await screen.findByRole("button", { name: "Accept new role" }),
    );

    await waitFor(() => expect(applyForJobMock).toHaveBeenCalledWith("team3"));
    expect(
      await screen.findByText("You have been appointed manager!"),
    ).toBeInTheDocument();
    expect(onGameUpdate).toHaveBeenCalledWith(hiredGame);
  });

  it("shows a not-better-club error when the backend reports not_better_club", async () => {
    getAvailableJobsMock.mockResolvedValue([
      {
        team_id: "team2",
        team_name: "Lower Div FC",
        city: "Smalltown",
        reputation: 300,
        last_league_position: null,
      },
    ]);
    applyForJobMock.mockResolvedValue({
      result: "not_better_club",
      game: createEmployedGameState(),
    });

    render(
      <JobOpportunitiesCard
        gameState={createEmployedGameState()}
        onGameUpdate={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Apply" }));
    fireEvent.click(
      await screen.findByRole("button", { name: "Accept new role" }),
    );

    expect(
      await screen.findByText(
        "You can only apply for clubs that are a step up from your current one.",
      ),
    ).toBeInTheDocument();
  });

  it("shows a same-team error when the backend reports same_team", async () => {
    getAvailableJobsMock.mockResolvedValue([
      {
        team_id: "team1",
        team_name: "Old FC",
        city: "Oldville",
        reputation: 500,
        last_league_position: null,
      },
    ]);
    applyForJobMock.mockResolvedValue({
      result: "same_team",
      game: createEmployedGameState(),
    });

    render(
      <JobOpportunitiesCard
        gameState={createEmployedGameState()}
        onGameUpdate={vi.fn()}
      />,
    );

    // Applying to the manager's current club bypasses the switch-confirm
    // modal — the backend's same_team result surfaces directly as an error.
    fireEvent.click(await screen.findByRole("button", { name: "Apply" }));

    expect(
      await screen.findByText("You are already managing that club."),
    ).toBeInTheDocument();
  });

  it("refreshes the list when the refresh button is clicked", async () => {
    getAvailableJobsMock
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([
        {
          team_id: "team3",
          team_name: "Refreshed FC",
          city: "Elsewhere",
          reputation: 500,
          last_league_position: null,
        },
      ]);

    render(
      <JobOpportunitiesCard
        gameState={createGameState()}
        onGameUpdate={vi.fn()}
      />,
    );

    await screen.findByText("No positions currently available.");

    fireEvent.click(
      screen.getByRole("button", { name: "Check for new positions" }),
    );

    expect(await screen.findByText("Refreshed FC")).toBeInTheDocument();
  });
});
