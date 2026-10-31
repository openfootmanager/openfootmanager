import { render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import PenaltyShootoutScreen, { KickRow } from "./PenaltyShootoutScreen";
import { makeGameState, makeSnapshot } from "./matchScreenFixtures";
import type { MatchEvent, MatchSnapshot } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (k: string, params?: { n?: number }) => (params?.n ? `${k}:${params.n}` : k),
  }),
}));

/** The cells' accessible names, left to right. */
const cellNames = () => screen.getAllByRole("img").map((cell) => cell.getAttribute("aria-label"));

describe("KickRow", (): void => {
  it("shows future cells for untaken kicks", (): void => {
    render(<KickRow label="Home" kicks={[]} maxRounds={5} />);
    const cells = screen.getAllByText("?");
    expect(cells).toHaveLength(5);
  });

  /**
   * Given a side that missed its first penalty and scored its second
   * When its row is drawn
   * Then it reads miss, then goal, in the order the kicks were taken, not the goals first.
   */
  it("draws each kick in the order it was taken", (): void => {
    render(<KickRow label="Home" kicks={[false, true]} maxRounds={5} />);
    expect(cellNames()).toEqual([
      "match.shootout.kickMissed:1",
      "match.shootout.kickScored:2",
      "match.shootout.kickToCome:3",
      "match.shootout.kickToCome:4",
      "match.shootout.kickToCome:5",
    ]);
    const [first, second] = screen.getAllByRole("img");
    expect(first).toHaveTextContent("✗");
    expect(second).toHaveTextContent("⚽");
  });

  /**
   * Given kicks taken and to come
   * When a screen reader reads the row
   * Then the row is named for its club, and each cell says which kick it is and how it went, not
   * only a symbol and a colour.
   */
  it("names each kick for a screen reader", (): void => {
    render(<KickRow label="Away FC" kicks={[true, false, true]} maxRounds={5} />);
    const row = screen.getByRole("group", { name: "Away FC" });
    expect(
      within(row).getByRole("img", { name: "match.shootout.kickScored:1" }),
    ).toBeInTheDocument();
    expect(
      within(row).getByRole("img", { name: "match.shootout.kickMissed:2" }),
    ).toBeInTheDocument();
    expect(
      within(row).getByRole("img", { name: "match.shootout.kickToCome:4" }),
    ).toBeInTheDocument();
  });

  it("expands cells beyond maxRounds when more kicks were taken (sudden death)", (): void => {
    render(
      <KickRow label="Home" kicks={[true, true, false, true, false, true, false]} maxRounds={5} />,
    );
    expect(screen.getAllByText("⚽")).toHaveLength(4);
    expect(screen.getAllByText("✗")).toHaveLength(3);
    expect(screen.queryAllByText("?")).toHaveLength(0);
  });
});

const kick = (side: "Home" | "Away", event_type: "ShootoutGoal" | "ShootoutMiss"): MatchEvent => ({
  minute: 120,
  event_type,
  side,
  zone: "Box",
  player_id: null,
  secondary_player_id: null,
});

describe("PenaltyShootoutScreen", (): void => {
  afterEach((): void => {
    vi.useRealTimers();
  });

  /**
   * Given a home goal in extra time, then a shootout where the home side missed its first kick and
   * scored its second, and the away side has not kicked yet
   * When the screen draws the kick grid
   * Then the home row reads miss, then goal, as the kick events happened: not goals first, and
   * without the extra-time goal, which shares the event log but is no kick.
   */
  it("draws the home row from that side's kick events in order", (): void => {
    // The screen steps the shootout on a timer; held still, it draws only the snapshot it was given.
    vi.useFakeTimers();
    const snapshot: MatchSnapshot = {
      ...makeSnapshot(),
      phase: "PenaltyShootout",
      events: [
        { ...kick("Home", "ShootoutGoal"), minute: 112, event_type: "Goal" },
        kick("Home", "ShootoutMiss"),
        kick("Home", "ShootoutGoal"),
      ],
      penalty_shootout: {
        home_taken: 2,
        away_taken: 0,
        home_scored: 1,
        away_scored: 0,
        sudden_death: false,
      },
    };

    render(
      <PenaltyShootoutScreen
        snapshot={snapshot}
        gameState={makeGameState()}
        userSide="Home"
        isSpectator={false}
        importantEvents={[]}
        onSnapshotUpdate={() => {}}
        onImportantEvent={() => {}}
        onFullTime={() => {}}
      />,
    );

    // The grid comes before the event feed, so the first two kick symbols are the home row's.
    const firstTwo = screen
      .getAllByText(/^[⚽✗]$/)
      .slice(0, 2)
      .map((cell) => cell.textContent);
    expect(firstTwo).toEqual(["✗", "⚽"]);
    expect(screen.getByRole("img", { name: "match.shootout.kickMissed:1" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "match.shootout.kickScored:2" })).toBeInTheDocument();
  });
});
