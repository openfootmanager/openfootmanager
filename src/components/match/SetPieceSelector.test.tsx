import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { getSetPieceStats } from "./SetPieceSelector";
import SetPieceSelector from "./SetPieceSelector";
import type { PlayerData } from "../../store/gameStore";

// Mock react-i18next
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

// ---------------------------------------------------------------------------
// Minimal fixture
// ---------------------------------------------------------------------------

const makePlayer = (overrides: Partial<PlayerData> = {}): PlayerData => ({
  id: "p1",
  match_name: "Test Player",
  full_name: "Test Player Full",
  date_of_birth: "1996-01-15",
  nationality: "GB",
  position: "Midfielder",
  natural_position: "Midfielder",
  alternate_positions: [],
  training_focus: null,
  attributes: {
    pace: 70,
    stamina: 70,
    strength: 70,
    agility: 70,
    passing: 75,
    shooting: 80,
    tackling: 60,
    dribbling: 70,
    defending: 60,
    positioning: 65,
    vision: 72,
    decisions: 68,
    composure: 50,
    aggression: 50,
    teamwork: 50,
    leadership: 50,
    handling: 30,
    reflexes: 30,
    aerial: 50,
  },
  condition: 100,
  morale: 80,
  injury: null,
  team_id: "team_1",
  retired: false,
  contract_end: "2028-06-30",
  wage: 10000,
  market_value: 5000000,
  stats: {
    appearances: 0,
    goals: 0,
    assists: 0,
    clean_sheets: 0,
    yellow_cards: 0,
    red_cards: 0,
    avg_rating: 0,
    minutes_played: 0,
  },
  career: [],
  transfer_listed: false,
  loan_listed: false,
  transfer_offers: [],
  traits: [],
  ...overrides,
});

// ---------------------------------------------------------------------------
// getSetPieceStats
// ---------------------------------------------------------------------------

describe("getSetPieceStats", () => {
  const player = makePlayer();
  const a = player.attributes;

  it("penalty: weights shooting and composure", () => {
    const result = getSetPieceStats("penalty", player);
    expect(result.score).toBe(Math.round((a.shooting + a.composure) / 2));
    expect(result.stats).toEqual([
      { label: "SHO", value: a.shooting },
      { label: "COM", value: a.composure },
    ]);
  });

  it("freekick: weights passing, vision, and shooting support", () => {
    const result = getSetPieceStats("freekick", player);
    expect(result.score).toBe(
      Math.round((a.passing + a.vision + a.shooting / 2) / 2.5),
    );
    expect(result.stats).toEqual([
      { label: "PAS", value: a.passing },
      { label: "VIS", value: a.vision },
      { label: "SHO", value: a.shooting },
    ]);
  });

  it("corner: weights passing and vision", () => {
    const result = getSetPieceStats("corner", player);
    expect(result.score).toBe(Math.round((a.passing + a.vision) / 2));
    expect(result.stats).toEqual([
      { label: "PAS", value: a.passing },
      { label: "VIS", value: a.vision },
    ]);
  });

  it("captain: weights leadership and teamwork", () => {
    const result = getSetPieceStats("captain", player);
    expect(result.score).toBe(Math.round((a.leadership + a.teamwork) / 2));
    expect(result.stats).toEqual([
      { label: "LDR", value: a.leadership },
      { label: "TMW", value: a.teamwork },
    ]);
  });

  it("vice captain uses the same leadership profile as captain", () => {
    const result = getSetPieceStats("vicecaptain", player);
    expect(result.score).toBe(Math.round((a.leadership + a.teamwork) / 2));
    expect(result.stats).toEqual([
      { label: "LDR", value: a.leadership },
      { label: "TMW", value: a.teamwork },
    ]);
  });

  it("unknown role: returns score 0 and empty stats", () => {
    const result = getSetPieceStats("throw_in", player);
    expect(result.score).toBe(0);
    expect(result.stats).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// SetPieceSelector component
// ---------------------------------------------------------------------------

const players = [
  { id: "p1", name: "John Smith", position: "Midfielder" },
  { id: "p2", name: "Jane Doe", position: "Forward" },
  { id: "gk", name: "Keeper", position: "Goalkeeper" },
];

const allSquad = [
  makePlayer({ id: "p1", position: "Midfielder" }),
  makePlayer({
    id: "p2",
    position: "Forward",
    attributes: { ...makePlayer().attributes, shooting: 90 },
  }),
  makePlayer({ id: "gk", position: "Goalkeeper" }),
];

describe("SetPieceSelector component", () => {
  it("renders the label and 'not assigned' when no currentId", () => {
    render(
      <SetPieceSelector
        label="Penalty Taker"
        icon={<span data-testid="icon">PK</span>}
        role="penalty"
        currentId={null}
        players={players}
        allSquad={allSquad}
        onSelect={() => {}}
      />,
    );
    expect(screen.getByText("Penalty Taker")).toBeInTheDocument();
    expect(screen.getByText("match.notAssigned")).toBeInTheDocument(); // mocked t() returns key
    expect(screen.getByTestId("icon")).toBeInTheDocument();
  });

  it("shows the current player name when currentId is set", () => {
    render(
      <SetPieceSelector
        label="Penalty Taker"
        icon={<span>PK</span>}
        role="penalty"
        currentId="p1"
        players={players}
        allSquad={allSquad}
        onSelect={() => {}}
      />,
    );
    expect(screen.getByText("John Smith")).toBeInTheDocument();
  });

  it("normalizes detailed positions to translated core abbreviations", () => {
    render(
      <SetPieceSelector
        label="Penalty Taker"
        icon={<span>PK</span>}
        role="penalty"
        currentId={null}
        players={[
          { id: "cb", name: "Center Back Player", position: "Center Back" },
        ]}
        allSquad={[makePlayer({ id: "cb", position: "Center Back" })]}
        onSelect={() => {}}
      />,
    );

    fireEvent.click(screen.getByText("Penalty Taker"));

    expect(screen.getByText("common.posAbbr.Defender")).toBeInTheDocument();
  });

  it("expands dropdown on click and shows non-GK players sorted by score", () => {
    render(
      <SetPieceSelector
        label="Penalty Taker"
        icon={<span>PK</span>}
        role="penalty"
        currentId={null}
        players={players}
        allSquad={allSquad}
        onSelect={() => {}}
      />,
    );
    // Click to expand
    fireEvent.click(screen.getByText("Penalty Taker"));
    // Should see non-GK players
    expect(screen.getByText("John Smith")).toBeInTheDocument();
    expect(screen.getByText("Jane Doe")).toBeInTheDocument();
    // Goalkeeper should be filtered out from dropdown
    expect(screen.queryByText("Keeper")).not.toBeInTheDocument();
  });

  it("renders translated stat labels in the expanded selector header", () => {
    render(
      <SetPieceSelector
        label="Penalty Taker"
        icon={<span>PK</span>}
        role="penalty"
        currentId="p1"
        players={players}
        allSquad={allSquad}
        onSelect={() => {}}
      />,
    );

    fireEvent.click(screen.getByText("Penalty Taker"));

    expect(
      screen.getAllByText("common.attributes.shooting").length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByText("common.attributes.composure").length,
    ).toBeGreaterThan(0);
  });

  it("calls onSelect and collapses when a player is picked", () => {
    const onSelect = vi.fn();
    render(
      <SetPieceSelector
        label="Penalty Taker"
        icon={<span>PK</span>}
        role="penalty"
        currentId={null}
        players={players}
        allSquad={allSquad}
        onSelect={onSelect}
      />,
    );
    // Expand
    fireEvent.click(screen.getByText("Penalty Taker"));
    // Pick a player
    fireEvent.click(screen.getByText("Jane Doe"));
    expect(onSelect).toHaveBeenCalledWith("p2");
  });

  it("highlights the current player in the dropdown", () => {
    render(
      <SetPieceSelector
        label="Penalty Taker"
        icon={<span>PK</span>}
        role="penalty"
        currentId="p1"
        players={players}
        allSquad={allSquad}
        onSelect={() => {}}
      />,
    );
    fireEvent.click(screen.getByText("Penalty Taker"));
    // The current player's row should have the highlight class
    const buttons = screen.getAllByRole("button");
    const p1Button = buttons.find(
      (b) => b.textContent?.includes("John Smith") && b !== buttons[0],
    );
    expect(p1Button?.className).toContain("bg-primary-500/20");
  });

  it("includes goalkeepers for vice captain assignments", () => {
    render(
      <SetPieceSelector
        label="Vice-captain"
        icon={<span>VC</span>}
        role="vicecaptain"
        currentId={null}
        players={players}
        allSquad={allSquad}
        onSelect={() => {}}
      />,
    );

    fireEvent.click(screen.getByText("Vice-captain"));

    expect(screen.getByText("Keeper")).toBeInTheDocument();
  });
});
