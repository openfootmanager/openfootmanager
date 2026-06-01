import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { parseFormationNeeds, condColor, statColor, starterOvrColor, starterBadgeStyle, getStatVal, POSITION_KEY_STATS } from "./PreMatchLineup";
import PreMatchLineup from "./PreMatchLineup";
import type { EnginePlayerData, EngineTeamData } from "./types";

// Mock react-i18next
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, arg?: unknown) => {
      if (typeof arg === "string") {
        return arg;
      }

      if (
        typeof arg === "object" &&
        arg !== null &&
        "count" in arg &&
        typeof (arg as Record<string, unknown>).count !== "undefined"
      ) {
        return `${key}:${String((arg as Record<string, unknown>).count)}`;
      }

      if (key === "match.selectForSwap") {
        return "Select for swap";
      }

      if (key === "match.swapWithSelectedStarter") {
        return "Swap with selected starter";
      }

      return key;
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
  ovr: 70,
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
    expect(statColor(75)).toBe("text-primary-500 dark:text-primary-400 font-bold");
    expect(statColor(99)).toBe("text-primary-500 dark:text-primary-400 font-bold");
  });

  it("returns theme-safe neutral for medium stats (60-74)", () => {
    expect(statColor(60)).toBe("text-gray-700 dark:text-gray-200");
    expect(statColor(74)).toBe("text-gray-700 dark:text-gray-200");
  });

  it("returns subdued but readable neutral for low stats (< 60)", () => {
    expect(statColor(59)).toBe("text-gray-500 dark:text-gray-400");
    expect(statColor(0)).toBe("text-gray-500 dark:text-gray-400");
  });
});

describe("starterOvrColor", () => {
  it("uses theme-safe colors for each OVR tier", () => {
    expect(starterOvrColor(72)).toBe("text-primary-600 dark:text-primary-400");
    expect(starterOvrColor(58)).toBe("text-gray-700 dark:text-gray-300");
    expect(starterOvrColor(44)).toBe("text-red-600 dark:text-red-400");
  });
});

describe("starterBadgeStyle", () => {
  it("keeps dark team colors as the badge accent", () => {
    expect(starterBadgeStyle("#008866")).toEqual({
      backgroundColor: "#00886630",
      color: "#008866",
      borderColor: "#00886655",
      borderWidth: 1,
      borderStyle: "solid",
    });
  });

  it("normalizes 3-digit hex colors before applying alpha variants", () => {
    expect(starterBadgeStyle("#123")).toEqual({
      backgroundColor: "#11223330",
      color: "#112233",
      borderColor: "#11223355",
      borderWidth: 1,
      borderStyle: "solid",
    });
  });

  it("falls back to a readable neutral badge for very light team colors", () => {
    expect(starterBadgeStyle("#ffffff")).toEqual({
      backgroundColor: "#f8fafc",
      color: "#334155",
      borderColor: "#cbd5e1",
      borderWidth: 1,
      borderStyle: "solid",
    });
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

  it("offers a context menu action to select a starter for swapping", () => {
    const onSelectStarter = vi.fn();

    render(<PreMatchLineup {...defaultProps} onSelectStarter={onSelectStarter} />);

    fireEvent.contextMenu(screen.getByTestId("pre-match-starter-m1"));
    fireEvent.click(
      within(screen.getByRole("menu")).getByRole("button", {
        name: "Select for swap",
      }),
    );

    expect(onSelectStarter).toHaveBeenCalledWith("m1");
  });

  it("shows swap prompt when a starter is selected", () => {
    render(<PreMatchLineup {...defaultProps} selectedStarterId="m1" />);
    expect(screen.getByText("match.swapPrompt")).toBeInTheDocument();
    expect(screen.getByText("match.cancel")).toBeInTheDocument();
  });

  it("offers a context menu action to clear the selected starter", () => {
    const onSelectStarter = vi.fn();

    render(
      <PreMatchLineup
        {...defaultProps}
        selectedStarterId="m1"
        onSelectStarter={onSelectStarter}
      />,
    );

    fireEvent.contextMenu(screen.getByTestId("pre-match-starter-m1"));
    fireEvent.click(
      within(screen.getByRole("menu")).getByRole("button", {
        name: "match.cancel",
      }),
    );

    expect(onSelectStarter).toHaveBeenCalledWith(null);
  });

  it("calls onSwap when bench player is clicked while a starter is selected", () => {
    const onSwap = vi.fn();
    render(<PreMatchLineup {...defaultProps} selectedStarterId="m1" onSwap={onSwap} />);
    fireEvent.click(screen.getByText("Bench One"));
    expect(onSwap).toHaveBeenCalledWith("b1");
  });

  it("offers a context menu action to swap in a bench player", () => {
    const onSwap = vi.fn();

    render(
      <PreMatchLineup
        {...defaultProps}
        selectedStarterId="m1"
        onSwap={onSwap}
      />,
    );

    fireEvent.contextMenu(screen.getByTestId("pre-match-bench-b1"));
    fireEvent.click(
      within(screen.getByRole("menu")).getByRole("button", {
        name: "Swap with selected starter",
      }),
    );

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
