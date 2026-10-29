import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { MatchMiniPitch } from "./MatchMiniPitch";
import { createPlayer } from "../../test-utils/factories";
import { createSnapshot, makePlayer, makeTeam, makeXi } from "../../test-utils/matchFactories";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: vi.fn((path: string) => path),
  invoke: vi.fn(),
  isTauri: vi.fn(() => false),
}));

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, arg?: unknown) => {
      if (key === "match.liveRating" && typeof arg === "object" && arg !== null) {
        return `Rating ${String((arg as { rating: string }).rating)}`;
      }
      if (key === "squad.pitchTokenConditionOnly" && typeof arg === "object" && arg !== null) {
        return `Condition ${String((arg as { condition: number }).condition)}%`;
      }
      if (key === "match.liveGoals" && typeof arg === "object" && arg !== null) {
        return `Goals: ${String((arg as { count: number }).count)}`;
      }
      if (key === "match.liveAssists" && typeof arg === "object" && arg !== null) {
        return `Assists: ${String((arg as { count: number }).count)}`;
      }
      if (key === "match.eventTypes.YellowCard") return "Yellow Card";
      if (key === "match.eventTypes.RedCard") return "Red Card";
      return key;
    },
  }),
}));

/** A token is the element that carries the player's full name (the visible label is only the surname). */
function tokenFor(name: string): HTMLElement {
  const token = screen.getByText(name).parentElement;
  if (!token) throw new Error(`No pitch token for ${name}`);
  return token;
}

/** Where FormationPitch placed a player's token on the pitch. */
function slotOf(name: string): { left: string; top: string } {
  const placed = tokenFor(name).parentElement;
  if (!placed) throw new Error(`${name} is not placed on the pitch`);
  return { left: placed.style.left, top: placed.style.top };
}

/** Every token on one side, by the full names of the players it shows. */
function namesOn(team: string): string[] {
  return within(screen.getByRole("region", { name: team }))
    .getAllByText(/ Player\d+$|^Fresh Legs$/)
    .map((el) => el.textContent ?? "");
}

describe("MatchMiniPitch", () => {
  /** Given a live match, When the pitch renders, Then each side shows its XI under its own name. */
  it("shows both formations side by side, one token per starter", () => {
    render(<MatchMiniPitch snapshot={createSnapshot()} squad={new Map()} />);

    expect(namesOn("Alpha FC")).toEqual(makeXi("Home").map((p) => p.name));
    expect(namesOn("Beta United")).toEqual(makeXi("Away").map((p) => p.name));
  });

  /**
   * Given a starter with a shirt number and a live mark, When the pitch renders,
   * Then the token shows the number, the surname and the mark to one decimal, in
   * the bubble colour of its band.
   */
  it("shows a starter's shirt number, surname and live rating", () => {
    render(
      <MatchMiniPitch
        snapshot={createSnapshot({ player_ratings: { "Home-9": 7.25 } })}
        squad={new Map([["Home-9", createPlayer({ id: "Home-9", jersey_number: 9 })]])}
      />,
    );

    const striker = tokenFor("Home Player9");
    expect(within(striker).getByText("9")).toBeInTheDocument();
    expect(within(striker).getByText("Player9")).toBeInTheDocument();
    expect(within(striker).getByText("7.3")).toHaveClass("bg-primary-700");
    expect(within(striker).getByText("Rating 7.3")).toBeInTheDocument();
  });

  /**
   * Given a starter the engine has not rated yet (under ten minutes played),
   * When the pitch renders, Then the token shows a dash in the unrated bubble and
   * announces no mark.
   */
  it("shows a dash for a player who has not been rated yet", () => {
    render(<MatchMiniPitch snapshot={createSnapshot()} squad={new Map()} />);

    const keeper = tokenFor("Home Player0");
    expect(within(keeper).getByText("–")).toHaveClass("bg-gray-500");
    expect(within(keeper).queryByText(/^Rating/)).not.toBeInTheDocument();
  });

  /**
   * Given a starter whose squad record has a face asset, When the pitch renders,
   * Then his token shows that portrait rather than a bare shirt circle.
   */
  it("shows the player's portrait from the squad's assets", () => {
    const face = "assets/worlds/test-world/players/home-9.png";
    render(
      <MatchMiniPitch
        snapshot={createSnapshot()}
        squad={new Map([["Home-9", createPlayer({ id: "Home-9", media: { face } })]])}
      />,
    );

    // Hidden from the accessibility tree: the token already names the player in text.
    expect(within(tokenFor("Home Player9")).getByRole("img", { hidden: true })).toHaveAttribute(
      "src",
      `/${face}`,
    );
  });

  /**
   * Given a tiring starter at 42% condition, When the pitch renders, Then his token
   * shows a stamina bar filled to his condition and announces it.
   */
  it("shows a stamina bar at the player's live condition", () => {
    const players = makeXi("Home");
    players[6] = makePlayer({ ...players[6], condition: 42 });
    render(
      <MatchMiniPitch
        snapshot={createSnapshot({ home_team: makeTeam({ players }) })}
        squad={new Map()}
      />,
    );

    const tired = tokenFor("Home Player6");
    expect(within(tired).getByText("Condition 42%")).toBeInTheDocument();
    expect(within(tired).getByTestId("live-token-condition-fill")).toHaveStyle({ width: "42%" });
    expect(within(tokenFor("Home Player7")).getByText("Condition 78%")).toBeInTheDocument();
  });

  /**
   * Given a striker with a hat-trick and two assists, When the pitch renders, Then
   * his token shows one ball and one boot, each with its count, so the badges stay
   * the same size however many he gets, and both totals are announced.
   */
  it("shows goals and assists as one counted badge each, however many", () => {
    render(
      <MatchMiniPitch
        snapshot={createSnapshot({
          player_goals: { "Home-9": 3 },
          player_assists: { "Home-9": 2 },
        })}
        squad={new Map()}
      />,
    );

    const striker = tokenFor("Home Player9");
    expect(within(striker).getAllByTestId("live-token-goals")).toHaveLength(1);
    expect(within(striker).getAllByTestId("live-token-assists")).toHaveLength(1);
    expect(within(striker).getByTestId("live-token-goals")).toHaveTextContent("3");
    expect(within(striker).getByTestId("live-token-assists")).toHaveTextContent("2");
    expect(within(striker).getByText("Goals: 3")).toBeInTheDocument();
    expect(within(striker).getByText("Assists: 2")).toBeInTheDocument();
  });

  /**
   * Given a player with one goal and nobody with an assist, When the pitch renders,
   * Then his ball carries no count, and players without either show no badge.
   */
  it("shows a lone goal as a bare ball and nothing for players without one", () => {
    render(
      <MatchMiniPitch
        snapshot={createSnapshot({ player_goals: { "Home-9": 1 } })}
        squad={new Map()}
      />,
    );

    expect(within(tokenFor("Home Player9")).getByTestId("live-token-goals")).toHaveTextContent("");
    expect(
      within(tokenFor("Home Player9")).queryByTestId("live-token-assists"),
    ).not.toBeInTheDocument();
    expect(
      within(tokenFor("Home Player8")).queryByTestId("live-token-goals"),
    ).not.toBeInTheDocument();
  });

  /** Given a booked away player, When the pitch renders, Then his token carries a yellow card. */
  it("marks a booked player with a yellow card", () => {
    render(
      <MatchMiniPitch
        snapshot={createSnapshot({ away_yellows: { "Away-3": 1 } })}
        squad={new Map()}
      />,
    );

    expect(within(tokenFor("Away Player3")).getByText("Yellow Card")).toBeInTheDocument();
    expect(within(tokenFor("Away Player4")).queryByText("Yellow Card")).not.toBeInTheDocument();
  });

  /**
   * Given a player sent off for a second yellow, When the pitch renders, Then he
   * stays in his slot with a red card and no longer shows the yellow.
   */
  it("keeps a sent-off player on the pitch with a red card", () => {
    render(
      <MatchMiniPitch
        snapshot={createSnapshot({ home_yellows: { "Home-5": 2 }, sent_off: ["Home-5"] })}
        squad={new Map()}
      />,
    );

    const sentOff = tokenFor("Home Player5");
    expect(within(sentOff).getByText("Red Card")).toBeInTheDocument();
    expect(within(sentOff).queryByText("Yellow Card")).not.toBeInTheDocument();
    expect(namesOn("Alpha FC")).toHaveLength(11);
  });

  /**
   * Given the pitch on screen, When a substitution arrives in the next snapshot,
   * Then the substitute takes the vacated slot and the player taken off is gone.
   */
  it("puts a substitute in the slot of the player he replaced", () => {
    const { rerender } = render(<MatchMiniPitch snapshot={createSnapshot()} squad={new Map()} />);
    const vacatedSlot = slotOf("Home Player10");

    const players = makeXi("Home");
    players[10] = makePlayer({ id: "Home-sub", name: "Fresh Legs", position: "Forward" });
    rerender(
      <MatchMiniPitch
        snapshot={createSnapshot({ home_team: makeTeam({ players }) })}
        squad={new Map()}
      />,
    );

    expect(slotOf("Fresh Legs")).toEqual(vacatedSlot);
    expect(screen.queryByText("Home Player10")).not.toBeInTheDocument();
  });

  /**
   * Given a 4-4-2 on screen, When the next snapshot switches to 4-5-1, Then the
   * shape is relabelled and the last slot moves from a strike partnership to a
   * lone striker in the centre.
   */
  it("redraws the shape on a formation change", () => {
    const { rerender } = render(<MatchMiniPitch snapshot={createSnapshot()} squad={new Map()} />);
    expect(slotOf("Home Player10").left).not.toBe("50%");

    rerender(
      <MatchMiniPitch
        snapshot={createSnapshot({ home_team: makeTeam({ formation: "4-5-1" }) })}
        squad={new Map()}
      />,
    );

    expect(
      within(screen.getByRole("region", { name: "Alpha FC" })).getByText("4-5-1"),
    ).toBeInTheDocument();
    expect(slotOf("Home Player10").left).toBe("50%");
  });
});
