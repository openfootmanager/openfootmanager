import { beforeAll, describe, it, expect, vi } from "vitest";
import { i18nReady } from "../i18n";
import { render, screen, fireEvent } from "@testing-library/react";
import ScoutPlayerCard from "./ScoutPlayerCard";
import type { ScoutReportData } from "../store/gameStore";

// Mock react-i18next — return keys as-is. The scout's words come from the backend's keys through the
// real resolver, so they read as the locale writes them.
vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => undefined },
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en" },
  }),
}));

beforeAll(async () => {
  await i18nReady;
});

function fullReport(overrides?: Partial<ScoutReportData>): ScoutReportData {
  return {
    player_id: "p1",
    player_name: "Carlos Silva",
    position: "Midfielder",
    nationality: "BR",
    dob: "1998-03-15",
    team_name: "Rival FC",
    pace: 72,
    shooting: 65,
    passing: 78,
    dribbling: 70,
    defending: 55,
    physical: 60,
    condition: 90,
    morale: 75,
    avg_rating: 67,
    rating_key: "common.scoutRatings.good",
    potential_key: "common.scoutPotential.strong",
    confidence_key: "common.scoutConfidence.high",
    ...overrides,
  };
}

describe("ScoutPlayerCard", () => {
  it("renders player name and position", () => {
    render(<ScoutPlayerCard report={fullReport()} />);
    expect(screen.getByText("Carlos Silva")).toBeInTheDocument();
    expect(screen.getByText(/common\.positions\.Midfielder/)).toBeInTheDocument();
  });

  it("renders team name when provided", () => {
    render(<ScoutPlayerCard report={fullReport()} />);
    expect(screen.getByText("Rival FC")).toBeInTheDocument();
  });

  it("does not render team name when null", () => {
    render(<ScoutPlayerCard report={fullReport({ team_name: null })} />);
    expect(screen.queryByText("Rival FC")).not.toBeInTheDocument();
  });

  it("renders discovered attributes with values", () => {
    render(<ScoutPlayerCard report={fullReport()} />);
    // All 6 attrs are revealed → should see values
    expect(screen.getByText("72")).toBeInTheDocument();
    expect(screen.getByText("65")).toBeInTheDocument();
    expect(screen.getByText("78")).toBeInTheDocument();
    expect(screen.getByText("70")).toBeInTheDocument();
    expect(screen.getByText("55")).toBeInTheDocument();
    expect(screen.getByText("60")).toBeInTheDocument();
  });

  it("shows 'undiscovered' for null attributes", () => {
    const report = fullReport({
      pace: null,
      shooting: null,
      passing: null,
      dribbling: null,
    });
    render(<ScoutPlayerCard report={report} />);
    // 4 hidden attrs → 4 "undiscovered" labels
    const undiscovered = screen.getAllByText("scouting.undiscovered");
    expect(undiscovered).toHaveLength(4);
    // 2 revealed attrs still show values
    expect(screen.getByText("55")).toBeInTheDocument();
    expect(screen.getByText("60")).toBeInTheDocument();
  });

  it("shows discovered count", () => {
    const report = fullReport({ pace: null, shooting: null });
    render(<ScoutPlayerCard report={report} />);
    expect(screen.getByText(/4\/6/)).toBeInTheDocument();
  });

  it("renders rating badge", () => {
    render(<ScoutPlayerCard report={fullReport()} />);
    expect(screen.getByText(/Good/)).toBeInTheDocument();
    expect(screen.getByText(/~67/)).toBeInTheDocument();
  });

  it("renders potential badge", () => {
    render(<ScoutPlayerCard report={fullReport()} />);
    expect(screen.getByText("Strong development potential")).toBeInTheDocument();
  });

  it("renders confidence badge", () => {
    render(<ScoutPlayerCard report={fullReport()} />);
    expect(screen.getByText(/High/)).toBeInTheDocument();
  });

  it("shows condition when available", () => {
    render(<ScoutPlayerCard report={fullReport()} />);
    expect(screen.getByText(/90%/)).toBeInTheDocument();
  });

  it("hides condition when null", () => {
    render(<ScoutPlayerCard report={fullReport({ condition: null })} />);
    expect(screen.queryByText(/scouting\.condition/)).not.toBeInTheDocument();
  });

  it("shows morale when available", () => {
    render(<ScoutPlayerCard report={fullReport()} />);
    expect(screen.getByText(/75\/100/)).toBeInTheDocument();
  });

  it("hides morale when null", () => {
    render(<ScoutPlayerCard report={fullReport({ morale: null })} />);
    expect(screen.queryByText(/scouting\.morale/)).not.toBeInTheDocument();
  });

  it("calls onPlayerClick with player_id when clicked", () => {
    const onClick = vi.fn();
    render(<ScoutPlayerCard report={fullReport()} onPlayerClick={onClick} />);
    // Click the card wrapper
    fireEvent.click(screen.getByText("Carlos Silva"));
    expect(onClick).toHaveBeenCalledWith("p1");
  });

  it("renders DOB", () => {
    render(<ScoutPlayerCard report={fullReport()} />);
    expect(screen.getByText("1998-03-15")).toBeInTheDocument();
  });
});
