import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { getPositionOvr, parseFormationNeeds, condColor, statColor, getStatVal, POSITION_KEY_STATS } from "./PreMatchLineup";
import PreMatchLineup from "./PreMatchLineup";
import type { EnginePlayerData, EngineTeamData } from "./types";

// Mock react-i18next
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown> | string) => {
      if (typeof opts === "string") {
        return opts;
      }

      return opts?.count !== undefined ? `${key}:${opts.count}` : key;
    },
  }),
}));

// ---------------------------------------------------------------------------
// Minimal fixture
// ---------------------------------------------------------------------------

const makePlayer = (overrides: Partial<EnginePlayerData> = {}): EnginePlayerData => ({
  id: "p1",
  name: "Test Player",
  position: "Midfielder",
  condition: 100,
  pace: 70, stamina: 70, strength: 70, agility: 70,
  passing: 70, shooting: 70, tackling: 70, dribbling: 70,
  defending: 70, positioning: 70, vision: 70, decisions: 70,
  composure: 70, aggression: 50, teamwork: 70,
  leadership: 50, handling: 70, reflexes: 70, aerial: 70,
  traits: [],
  ...overrides,
});

// ---------------------------------------------------------------------------
// getPositionOvr
// ---------------------------------------------------------------------------

describe("getPositionOvr", () => {
  it("calculates Goalkeeper OVR from the canonical goalkeeper weighting", () => {
    const gk = makePlayer({ position: "Goalkeeper", handling: 80, reflexes: 80, aerial: 60, positioning: 70, composure: 60 });
    expect(getPositionOvr(gk)).toBe(74);
  });

  it("calculates Defender OVR from the canonical center-back weighting", () => {
    const def = makePlayer({ position: "Defender", defending: 80, tackling: 75, strength: 70, positioning: 65, aerial: 60 });
    expect(getPositionOvr(def)).toBe(71);
  });

  it("calculates Midfielder OVR from the canonical central-midfielder weighting", () => {
    const mid = makePlayer({ position: "Midfielder", passing: 80, vision: 75, decisions: 70, stamina: 65, dribbling: 60, teamwork: 55 });
    expect(getPositionOvr(mid)).toBe(70);
  });

  it("calculates Forward OVR from the canonical striker weighting", () => {
    const fwd = makePlayer({ position: "Forward", shooting: 85, pace: 80, dribbling: 75, composure: 70, strength: 65, positioning: 60 });
    expect(getPositionOvr(fwd)).toBe(73);
  });

  it("falls back to the default shared weighting for unknown positions", () => {
    const unknown = makePlayer({ position: "Sweeper" });
    expect(getPositionOvr(unknown)).toBe(70);
  });
});

// ---------------------------------------------------------------------------
// parseFormationNeeds
// ---------------------------------------------------------------------------

describe("parseFormationNeeds", () => {
  it("parses standard 3-part formations", () => {
    expect(parseFormationNeeds("4-4-2")).toEqual({ Goalkeeper: 1, Defender: 4, Midfielder: 4, Forward: 2 });
    expect(parseFormationNeeds("4-3-3")).toEqual({ Goalkeeper: 1, Defender: 4, Midfielder: 3, Forward: 3 });
    expect(parseFormationNeeds("3-5-2")).toEqual({ Goalkeeper: 1, Defender: 3, Midfielder: 5, Forward: 2 });
  });

  it("parses 4-part formations (always 1 GK, mid = sum of middle parts)", () => {
    expect(parseFormationNeeds("4-2-3-1")).toEqual({ Goalkeeper: 1, Defender: 4, Midfielder: 5, Forward: 1 });
    expect(parseFormationNeeds("4-1-4-1")).toEqual({ Goalkeeper: 1, Defender: 4, Midfielder: 5, Forward: 1 });
    expect(parseFormationNeeds("3-4-1-2")).toEqual({ Goalkeeper: 1, Defender: 3, Midfielder: 5, Forward: 2 });
    expect(parseFormationNeeds("4-3-2-1")).toEqual({ Goalkeeper: 1, Defender: 4, Midfielder: 5, Forward: 1 });
  });

  it("returns default 4-4-2 for unparsable input", () => {
    expect(parseFormationNeeds("invalid")).toEqual({ Goalkeeper: 1, Defender: 4, Midfielder: 4, Forward: 2 });
    expect(parseFormationNeeds("")).toEqual({ Goalkeeper: 1, Defender: 4, Midfielder: 4, Forward: 2 });
  });
});

// ---------------------------------------------------------------------------
// condColor
// ---------------------------------------------------------------------------

describe("condColor", () => {
  it("returns primary for high condition (>= 75)", () => {
    expect(condColor(75)).toBe("text-primary-400");
    expect(condColor(100)).toBe("text-primary-400");
  });

  it("returns amber for medium condition (50-74)", () => {
    expect(condColor(50)).toBe("text-amber-400");
    expect(condColor(74)).toBe("text-amber-400");
  });

  it("returns red for low condition (< 50)", () => {
    expect(condColor(49)).toBe("text-red-400");
    expect(condColor(0)).toBe("text-red-400");
  });
});

// ---------------------------------------------------------------------------
// statColor
// ---------------------------------------------------------------------------

describe("statColor", () => {
  it("returns primary + bold for high stats (>= 75)", () => {
    expect(statColor(75)).toBe("text-primary-400 font-bold");
    expect(statColor(99)).toBe("text-primary-400 font-bold");
  });

  it("returns gray-200 for medium stats (60-74)", () => {
    expect(statColor(60)).toBe("text-gray-200");
    expect(statColor(74)).toBe("text-gray-200");
  });

  it("returns gray-500 for low stats (< 60)", () => {
    expect(statColor(59)).toBe("text-gray-500");
    expect(statColor(0)).toBe("text-gray-500");
  });
});

// ---------------------------------------------------------------------------
// getStatVal
// ---------------------------------------------------------------------------

describe("getStatVal", () => {
  const player = makePlayer({ pace: 85, shooting: 72 });

  it("returns the attribute value for a valid key", () => {
    expect(getStatVal(player, "pace")).toBe(85);
    expect(getStatVal(player, "shooting")).toBe(72);
  });

  it("returns 0 for a non-existent key", () => {
    expect(getStatVal(player, "nonexistent")).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// POSITION_KEY_STATS
// ---------------------------------------------------------------------------

describe("POSITION_KEY_STATS", () => {
  it("has entries for all four positions", () => {
    expect(Object.keys(POSITION_KEY_STATS)).toEqual(["Goalkeeper", "Defender", "Midfielder", "Forward"]);
  });

  it("each position has 3 stat entries with label and key", () => {
    for (const pos of Object.keys(POSITION_KEY_STATS)) {
      expect(POSITION_KEY_STATS[pos]).toHaveLength(3);
      for (const stat of POSITION_KEY_STATS[pos]) {
        expect(stat).toHaveProperty("label");
        expect(stat).toHaveProperty("key");
      }
    }
  });
});

// ---------------------------------------------------------------------------
// PreMatchLineup component
// ---------------------------------------------------------------------------

const makeTeam = (overrides: Partial<EngineTeamData> = {}): EngineTeamData => ({
  id: "team1",
  name: "Test FC",
  formation: "4-4-2",
  play_style: "Balanced",
  players: [
    makePlayer({ id: "gk1", name: "GK One", position: "Goalkeeper", handling: 80, reflexes: 80, aerial: 60, positioning: 70, composure: 65 }),
    makePlayer({ id: "d1", name: "Def One", position: "Defender" }),
    makePlayer({ id: "m1", name: "Mid One", position: "Midfielder" }),
    makePlayer({ id: "f1", name: "Fwd One", position: "Forward", shooting: 85, pace: 80 }),
  ],
  ...overrides,
});

const defaultProps = {
  userTeam: makeTeam(),
  userBench: [makePlayer({ id: "b1", name: "Bench One", position: "Midfielder", condition: 90 })],
  oppTeam: makeTeam({ id: "opp", name: "Rival United", formation: "3-5-2", play_style: "Counter" }),
  userColor: "#00ff00",
  homeTeamColor: "#ff0000",
  awayTeamColor: "#0000ff",
  userSide: "Home" as const,
  formationNeeds: { Goalkeeper: 1, Defender: 4, Midfielder: 4, Forward: 2 },
  selectedStarterId: null as string | null,
  isAutoSelecting: false,
  onSelectStarter: vi.fn(),
  onSwap: vi.fn(),
  onAutoSelect: vi.fn(),
};

describe("PreMatchLineup component", () => {
  it("renders starting XI header and player names", () => {
    render(<PreMatchLineup {...defaultProps} />);
    expect(screen.getByText("match.startingXI")).toBeInTheDocument();
    expect(screen.getByText("GK One")).toBeInTheDocument();
    expect(screen.getByText("Def One")).toBeInTheDocument();
    expect(screen.getByText("Mid One")).toBeInTheDocument();
    expect(screen.getByText("Fwd One")).toBeInTheDocument();
  });

  it("renders bench section with substitutes", () => {
    render(<PreMatchLineup {...defaultProps} />);
    expect(screen.getByText("match.substitutes")).toBeInTheDocument();
    expect(screen.getByText("Bench One")).toBeInTheDocument();
  });

  it("renders opponent info", () => {
    render(<PreMatchLineup {...defaultProps} />);
    expect(screen.getByText("match.opponent")).toBeInTheDocument();
    expect(screen.getByText("Rival United")).toBeInTheDocument();
    expect(screen.getByText("3-5-2 · Counter")).toBeInTheDocument();
  });

  it("renders auto-select button and calls onAutoSelect", () => {
    const onAutoSelect = vi.fn();
    render(<PreMatchLineup {...defaultProps} onAutoSelect={onAutoSelect} />);
    const autoBtn = screen.getByText("match.autoSelectXI");
    expect(autoBtn).toBeInTheDocument();
    fireEvent.click(autoBtn.closest("button")!);
    expect(onAutoSelect).toHaveBeenCalledOnce();
  });

  it("shows 'selecting' text when isAutoSelecting is true", () => {
    render(<PreMatchLineup {...defaultProps} isAutoSelecting={true} />);
    expect(screen.getByText("match.selecting")).toBeInTheDocument();
  });

  it("calls onSelectStarter when a starter is clicked", () => {
    const onSelectStarter = vi.fn();
    render(<PreMatchLineup {...defaultProps} onSelectStarter={onSelectStarter} />);
    fireEvent.click(screen.getByText("Mid One"));
    expect(onSelectStarter).toHaveBeenCalledWith("m1");
  });

  it("shows swap prompt when a starter is selected", () => {
    render(<PreMatchLineup {...defaultProps} selectedStarterId="m1" />);
    expect(screen.getByText("match.swapPrompt")).toBeInTheDocument();
    expect(screen.getByText("match.cancel")).toBeInTheDocument();
  });

  it("calls onSwap when bench player is clicked while a starter is selected", () => {
    const onSwap = vi.fn();
    render(<PreMatchLineup {...defaultProps} selectedStarterId="m1" onSwap={onSwap} />);
    fireEvent.click(screen.getByText("Bench One"));
    expect(onSwap).toHaveBeenCalledWith("b1");
  });

  it("renders formation fit indicators", () => {
    render(<PreMatchLineup {...defaultProps} />);
    // With 1 GK and formationNeeds.Goalkeeper=1, should show 1/1
    expect(screen.getByText("match.formationFit")).toBeInTheDocument();
  });

  it("shows empty bench message when no bench players", () => {
    render(<PreMatchLineup {...defaultProps} userBench={[]} />);
    expect(screen.getByText("match.noBenchAvailable2")).toBeInTheDocument();
  });

  it("renders player condition percentages", () => {
    render(<PreMatchLineup {...defaultProps} />);
    // All default players have condition=100, bench has 90
    const pcts = screen.getAllByText("100%");
    expect(pcts.length).toBeGreaterThanOrEqual(4);
    expect(screen.getByText("90%")).toBeInTheDocument();
  });
});
