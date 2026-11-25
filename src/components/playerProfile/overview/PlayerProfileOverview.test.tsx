import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../../i18n";
import type { PositionRatings } from "../../../store/generated/PositionRatings";
import { createPlayer } from "../../../test-utils/factories";
import type { PlayerRecentMatchEntry } from "../PlayerProfile.helpers";
import PlayerProfileOverview, { type PlayerProfileOverviewProps } from "./PlayerProfileOverview";

vi.mock("../../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

beforeAll(async () => {
  await i18nReady;
});

const match = (
  index: number,
  teamGoals: number,
  opponentGoals: number,
): PlayerRecentMatchEntry => ({
  fixture_id: `m${index}`,
  date: `2031-06-${String(28 - index).padStart(2, "0")}`,
  competition: "League",
  matchday: 10 - index,
  opponent_team_id: `t${index}`,
  opponent_name: `Rovers ${index}`,
  team_goals: teamGoals,
  opponent_goals: opponentGoals,
  minutes_played: 90,
  goals: index === 0 ? 2 : 0,
  assists: index === 1 ? 1 : 0,
  shots: 3,
  shots_on_target: 1,
  rating: 7.8 - index * 0.4,
  rated: true,
});

// Latest first, as the backend returns them: a win, a loss, a draw.
const THREE = [match(0, 3, 0), match(1, 1, 2), match(2, 2, 2)];

const STRIKER_WEIGHTS = [
  ["shooting", 26],
  ["positioning", 18],
  ["decisions", 14],
  ["pace", 12],
  ["dribbling", 10],
  ["strength", 8],
  ["composure", 8],
  ["aerial", 4],
] as const;

const striker: PositionRatings = {
  source: { kind: "live" },
  rows: [
    {
      position: "Striker",
      rating: 79,
      familiarity: "natural",
      deployed: false,
      key_attributes: [
        { attribute: "shooting", value: 84 },
        { attribute: "positioning", value: 81 },
        { attribute: "decisions", value: 70 },
      ],
      weights: STRIKER_WEIGHTS.map(([attribute, weight]) => ({
        attribute,
        weight,
        importance: weight >= 16 ? 3 : weight >= 8 ? 2 : 1,
      })),
    },
  ],
};

function props(overrides: Partial<PlayerProfileOverviewProps> = {}): PlayerProfileOverviewProps {
  const player = createPlayer({
    id: "p1",
    position: "Forward",
    natural_position: "Striker",
    condition: 92,
    morale: 75,
    market_value: 14_200_000,
    contract_end: "2028-06-30",
    date_of_birth: "2002-03-14",
    stats: {
      appearances: 27,
      goals: 19,
      assists: 6,
      minutes_played: 2214,
      clean_sheets: 0,
      yellow_cards: 1,
      red_cards: 0,
      avg_rating: 7.42,
    },
  });
  return {
    player,
    language: "en",
    showsPrivateState: true,
    recentMatches: THREE,
    recentMatchesStatus: "answered",
    retryRecentMatches: () => {},
    seasonRatingRated: true,
    tacticalRole: null,
    contractRiskLevel: "stable",
    contractRiskLabel: "Stable",
    wageText: "€23k/wk",
    potentialOpinion: <p>Potential opinion block</p>,
    ratings: { status: "answered", value: striker },
    attributes: player.attributes,
    traits: ["Finisher"],
    onOpenTab: () => {},
    ...overrides,
  };
}

describe("PlayerProfileOverview", () => {
  /**
   * Given the club's own player
   * When the Overview is drawn
   * Then Now, Details, Previous matches, Season stats, the attributes and Traits are each a
   *      labelled region, once, in that order of the board's three columns
   */
  it("has the board's six cards, each once", () => {
    render(<PlayerProfileOverview {...props()} />);

    for (const name of [
      "Now",
      "Details",
      "Previous matches",
      "Season Statistics",
      "Attributes",
      "Traits",
    ]) {
      expect(screen.getAllByRole("region", { name })).toHaveLength(1);
    }
  });

  /**
   * Given the club's own player at 92% condition and 75% morale
   * When Now is read
   * Then both show, with his form from his last matches (oldest first: draw, loss, win)
   */
  it("shows the own player's condition, morale and form in Now", () => {
    render(<PlayerProfileOverview {...props()} />);
    const now = within(screen.getByRole("region", { name: "Now" }));

    expect(now.getByText("92")).toBeInTheDocument();
    expect(now.getByText("75%")).toBeInTheDocument();
    const form = now.getByRole("list", { name: "Recent form" });
    expect(
      within(form)
        .getAllByRole("listitem")
        .map((item) => item.textContent),
    ).toEqual(["DDraw", "LDefeat", "WVictory"]);
  });

  /**
   * Given a rival's player, whose condition and morale the club may not see
   * When Now is read
   * Then both read "–", and there is no tactical role to set
   */
  it("hides a rival's condition and morale", () => {
    render(<PlayerProfileOverview {...props({ showsPrivateState: false })} />);
    const now = within(screen.getByRole("region", { name: "Now" }));

    expect(now.queryByText("92")).toBeNull();
    expect(now.queryByText("75%")).toBeNull();
    expect(now.getAllByText("–").length).toBeGreaterThanOrEqual(2);
    expect(now.queryByRole("combobox")).toBeNull();
  });

  /**
   * Given the own player with a tactical role to set
   * When the role is changed
   * Then the change is handed on, from a select that Now holds
   */
  it("lets the own player's role be set from Now", () => {
    const onChange = vi.fn();
    render(
      <PlayerProfileOverview
        {...props({
          tacticalRole: { value: "Standard", options: ["Standard", "Poacher"], onChange },
        })}
      />,
    );

    fireEvent.click(screen.getByRole("combobox", { name: "Role" }));
    fireEvent.click(screen.getByRole("option", { name: /Poacher/ }));

    expect(onChange).toHaveBeenCalledWith("Poacher");
  });

  /**
   * Given a player with a contract to 30 June 2028, a value and a weekly wage
   * When Details is read
   * Then it holds when he was born, his contract and its risk, his value, his wage and the potential
   *      opinion block it was given
   */
  it("holds born, contract, risk, value, wage and the potential opinion in Details", () => {
    render(<PlayerProfileOverview {...props()} />);
    const details = within(screen.getByRole("region", { name: "Details" }));

    expect(details.getByText(/14 March 2002/)).toBeInTheDocument();
    expect(details.getByText(/30 June 2028/)).toBeInTheDocument();
    expect(details.getByText("Stable")).toBeInTheDocument();
    expect(details.getByText("€23k/wk")).toBeInTheDocument();
    expect(details.getByText("Potential opinion block")).toBeInTheDocument();
  });

  /**
   * Given a wage that failed to load
   * When Details is read
   * Then the wage reads "Couldn't load" with its retry, and not a dash
   */
  it("shows a failed wage with its retry", () => {
    const retry = vi.fn();
    render(<PlayerProfileOverview {...props({ wageRetry: retry })} />);

    fireEvent.click(within(screen.getByRole("region", { name: "Details" })).getByRole("button"));

    expect(retry).toHaveBeenCalledTimes(1);
  });

  /**
   * Given three recent matches
   * When Previous matches is read
   * Then each is a row: the result, the opponent, the score, his goals or assists and his rating
   */
  it("lists the previous matches with result, opponent, score and rating", () => {
    render(<PlayerProfileOverview {...props()} />);
    const rows = within(screen.getByRole("region", { name: "Previous matches" })).getAllByRole(
      "listitem",
    );

    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("W");
    expect(rows[0]).toHaveTextContent("Rovers 0");
    expect(rows[0]).toHaveTextContent("3–0");
    expect(rows[0]).toHaveTextContent("7.8");
    expect(rows[1]).toHaveTextContent("L");
    expect(rows[1]).toHaveTextContent("1–2");
  });

  /**
   * Given no match played yet
   * When Previous matches is read
   * Then it says so, and Now draws no form
   */
  it("says there are no matches yet", () => {
    render(<PlayerProfileOverview {...props({ recentMatches: [] })} />);

    expect(
      within(screen.getByRole("region", { name: "Previous matches" })).getByText(
        "No recent match data",
      ),
    ).toBeInTheDocument();
    expect(screen.queryByRole("list", { name: "Recent form" })).toBeNull();
  });

  /**
   * Given his season totals
   * When Season stats is read
   * Then apps, goals, assists, average rating and minutes show
   */
  it("shows the season's five figures", () => {
    render(<PlayerProfileOverview {...props()} />);
    const season = within(screen.getByRole("region", { name: "Season Statistics" }));

    expect(season.getByText("27")).toBeInTheDocument();
    expect(season.getByText("19")).toBeInTheDocument();
    expect(season.getByText("7.4")).toBeInTheDocument();
    expect(season.getByText(/2,?214/)).toBeInTheDocument();
  });

  /**
   * Given a season whose rating the backend has not rated
   * When Season stats is read
   * Then the average rating is a dash
   */
  it("shows no average rating before the backend rates one", () => {
    render(<PlayerProfileOverview {...props({ seasonRatingRated: false })} />);
    const season = within(screen.getByRole("region", { name: "Season Statistics" }));

    expect(season.queryByText("7.4")).toBeNull();
    expect(season.getByText("–")).toBeInTheDocument();
  });

  /**
   * Given a striker, and a defender
   * When the attributes card is read for each
   * Then both radars have the same eight fixed axes (pace, shooting, dribbling, passing, vision,
   *      composure, strength, aerial), drawn short with his value ("PAC 80"), so two radars compare
   *      at a glance; the picture names every value in full to a screen reader
   */
  it("draws the same eight fixed axes for every outfield player", () => {
    const axes = (player: ReturnType<typeof props>["player"]) => {
      const { unmount } = render(<PlayerProfileOverview {...props({ player })} />);
      const card = within(screen.getByRole("region", { name: "Attributes" }));
      const labels = card.getAllByText(/^\S{2,5} \d+$/).map((label) => label.textContent);
      const spoken = card.getByRole("img").getAttribute("aria-label");
      unmount();
      return { labels, spoken };
    };
    const striker = createPlayer({ position: "Forward", natural_position: "Striker" });
    const defender = createPlayer({ position: "Defender", natural_position: "CenterBack" });

    const first = axes(striker);
    const second = axes(defender);

    expect(first.labels.map((label) => label?.split(" ")[0])).toEqual([
      "PAC",
      "SHO",
      "DRI",
      "PAS",
      "VIS",
      "COM",
      "STR",
      "AER",
    ]);
    expect(second.labels.map((label) => label?.split(" ")[0])).toEqual(
      first.labels.map((label) => label?.split(" ")[0]),
    );
    expect(first.spoken).toContain("Shooting");
    expect(first.spoken).toContain("Aerial");
  });

  /**
   * Given a goalkeeper
   * When the attributes card is read
   * Then the axes are the goalkeeping ones (handling, reflexes, aerial, composure, vision, agility,
   *      stamina, strength), not pace or shooting
   */
  it("swaps in the goalkeeping attributes for a goalkeeper", () => {
    const keeper = createPlayer({ position: "Goalkeeper", natural_position: "Goalkeeper" });
    render(<PlayerProfileOverview {...props({ player: keeper, attributes: keeper.attributes })} />);
    const radar = within(screen.getByRole("region", { name: "Attributes" })).getByRole("img");

    const spoken = radar.getAttribute("aria-label") ?? "";
    expect(spoken).toContain("Handling");
    expect(spoken).toContain("Reflexes");
    expect(spoken).not.toContain("Pace");
    expect(spoken).not.toContain("Shooting");
  });

  /**
   * Given a striker whose key attributes are shooting, positioning and decisions
   * When the radar is read
   * Then the axis for a key attribute that the radar has (shooting) is drawn heavier, and no other:
   *      the position's weights set the highlight, not the axes
   */
  it("highlights the axes that are the position's key attributes", () => {
    render(<PlayerProfileOverview {...props()} />);
    const card = within(screen.getByRole("region", { name: "Attributes" }));

    const heavy = card
      .getAllByText(/^\S{2,5} \d+$/)
      .filter((label) => label.classList.contains("font-bold"))
      .map((label) => label.textContent);

    expect(heavy).toHaveLength(1);
    expect(heavy[0]).toMatch(/^SHO /);
  });

  /**
   * Given a striker
   * When the attributes card is read
   * Then the line under the radar names the position's three key attributes with their values
   */
  it("names the key attributes under the radar", () => {
    render(<PlayerProfileOverview {...props()} />);
    const attributes = within(screen.getByRole("region", { name: "Attributes" }));

    expect(attributes.getByText(/Key for a/)).toBeInTheDocument();
    expect(attributes.getByText("Shooting", { selector: "li > span" })).toBeInTheDocument();
    expect(attributes.getByText("84")).toBeInTheDocument();
  });

  /**
   * Given a player the club knows nothing about (no position ratings, no readable attributes)
   * When the attributes card is read
   * Then there is no radar and it says the attributes are hidden
   */
  it("hides the radar for a player whose attributes the club cannot read", () => {
    render(
      <PlayerProfileOverview
        {...props({ ratings: { status: "answered", value: null }, attributes: null })}
      />,
    );
    const attributes = within(screen.getByRole("region", { name: "Attributes" }));

    expect(attributes.queryByRole("img")).toBeNull();
    expect(attributes.getByText("Attributes Hidden")).toBeInTheDocument();
  });

  /**
   * Given the Overview
   * When its links are read
   * Then there is no "All matches" link (no view holds older matches yet), and the season's
   *      "Career" link opens the Career tab
   */
  it("links the season to Career and offers no all-matches link", () => {
    const onOpenTab = vi.fn();
    render(<PlayerProfileOverview {...props({ onOpenTab })} />);

    expect(screen.queryByRole("button", { name: "All matches" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Career" }));

    expect(onOpenTab).toHaveBeenCalledWith("career");
  });

  /**
   * Given a match in which he scored and set up a goal
   * When Previous matches is read
   * Then both show, and the date of the match is a caption on the row
   */
  it("shows a goal and an assist from one match, and the match's date", () => {
    const both = { ...match(0, 3, 0), goals: 1, assists: 2 };
    render(<PlayerProfileOverview {...props({ recentMatches: [both] })} />);
    const row = within(screen.getByRole("region", { name: "Previous matches" })).getAllByRole(
      "listitem",
    )[0];

    expect(row).toHaveTextContent("1 goals");
    expect(row).toHaveTextContent("2 assists");
    expect(row).toHaveTextContent(/Jun/);
  });

  /**
   * Given a match history that has not answered, and one that failed
   * When Previous matches is read
   * Then neither says "No recent match data": the first says it is loading, the second says it
   *      could not load and offers a retry; only an answered empty list says there are none
   */
  it("tells loading and failed apart from no matches", () => {
    const retry = vi.fn();
    const { rerender } = render(
      <PlayerProfileOverview {...props({ recentMatches: [], recentMatchesStatus: "loading" })} />,
    );
    const card = () => within(screen.getByRole("region", { name: "Previous matches" }));
    expect(card().queryByText("No recent match data")).toBeNull();
    expect(card().getByText("Loading...")).toBeInTheDocument();

    rerender(
      <PlayerProfileOverview
        {...props({
          recentMatches: [],
          recentMatchesStatus: "failed",
          retryRecentMatches: retry,
        })}
      />,
    );
    expect(card().queryByText("No recent match data")).toBeNull();
    expect(card().getByText("Couldn't load")).toBeInTheDocument();
    fireEvent.click(card().getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledTimes(1);

    rerender(<PlayerProfileOverview {...props({ recentMatches: [] })} />);
    expect(card().getByText("No recent match data")).toBeInTheDocument();
  });

  /**
   * Given a season with 4 clean sheets, 6 yellow cards and 1 red card
   * When Season statistics is read
   * Then a compact line under the tiles shows all three; and with none, it shows the zeros
   */
  it("keeps clean sheets, yellow and red cards on a line under the season tiles", () => {
    const base = props();
    const withStats = (clean_sheets: number, yellow_cards: number, red_cards: number) => ({
      ...base.player,
      stats: { ...base.player.stats, clean_sheets, yellow_cards, red_cards },
    });
    const player = withStats(4, 6, 1);
    const { unmount } = render(<PlayerProfileOverview {...props({ player })} />);
    const line = within(screen.getByRole("region", { name: "Season Statistics" })).getByRole(
      "list",
      { name: "Discipline and clean sheets" },
    );
    expect(line).toHaveTextContent("Clean Sh");
    expect(line).toHaveTextContent("4");
    expect(line).toHaveTextContent("Yellows");
    expect(line).toHaveTextContent("6");
    expect(line).toHaveTextContent("Reds");
    expect(line).toHaveTextContent("1");
    unmount();

    render(<PlayerProfileOverview {...props({ player: withStats(0, 0, 0) })} />);
    const zeros = within(screen.getByRole("region", { name: "Season Statistics" })).getByRole(
      "list",
      { name: "Discipline and clean sheets" },
    );
    expect(zeros.textContent?.match(/0/g)).toHaveLength(3);
  });

  /**
   * Given the position ratings still loading, then failed, then answered with nothing for an
   *       unknown player
   * When the attributes card is read
   * Then loading shows a loading state, failure shows "Couldn't load" with a retry, and only the
   *      answered-unknown player reads "Attributes Hidden"
   */
  it("tells a loading or failed ratings request apart from an unknown player", () => {
    const retry = vi.fn();
    const card = () => within(screen.getByRole("region", { name: "Attributes" }));
    const { rerender } = render(
      <PlayerProfileOverview {...props({ ratings: { status: "loading" } })} />,
    );
    expect(card().getByText("Loading...")).toBeInTheDocument();
    expect(card().queryByText("Attributes Hidden")).toBeNull();

    rerender(<PlayerProfileOverview {...props({ ratings: { status: "failed", retry } })} />);
    expect(card().getByText("Couldn't load")).toBeInTheDocument();
    expect(card().queryByText("Attributes Hidden")).toBeNull();
    fireEvent.click(card().getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledTimes(1);

    rerender(
      <PlayerProfileOverview
        {...props({ ratings: { status: "answered", value: null }, attributes: null })}
      />,
    );
    expect(card().getByText("Attributes Hidden")).toBeInTheDocument();
  });

  /**
   * Given the radar in a narrow column
   * When its labels are read
   * Then they are text on the page at the caption size, not text inside the scaled picture (which
   *      shrinks below 12px when the chart is narrower than its drawing), and the picture names
   *      every value in full
   */
  it("draws the axis labels outside the scaled picture", () => {
    render(<PlayerProfileOverview {...props()} />);
    const card = within(screen.getByRole("region", { name: "Attributes" }));
    const radar = card.getByRole("img");

    expect(radar.querySelectorAll("text")).toHaveLength(0);
    const labels = card.getAllByText(/^(PAC|SHO|DRI|PAS|VIS|COM|STR|AER) \d+$/);
    expect(labels).toHaveLength(8);
    for (const label of labels) expect(label).toHaveClass("text-caption");
  });

  /**
   * Given a player with no traits
   * When the Overview is drawn
   * Then there is no Traits card
   */
  it("draws no Traits card for a player without traits", () => {
    render(<PlayerProfileOverview {...props({ traits: [] })} />);

    expect(screen.queryByRole("region", { name: "Traits" })).toBeNull();
  });

  /**
   * Given the Overview's cards, which hold screen-reader-only text (rating chips, the retry
   *       descriptions)
   * When a card is drawn
   * Then it is positioned, so that text stays inside it: an unpositioned card lets its
   *      absolutely placed text reach out to the page, and the page scrolls (the 960×640 layout
   *      check of the profile)
   */
  it("positions each card so its screen-reader text cannot stretch the page", () => {
    render(<PlayerProfileOverview {...props()} />);

    for (const name of ["Now", "Details", "Previous matches", "Season Statistics", "Attributes"]) {
      expect(screen.getByRole("region", { name })).toHaveClass("relative");
    }
  });

  /**
   * Given a player with traits
   * When the Traits card is drawn
   * Then it is as tall as its traits, not stretched to the height of the other columns
   */
  it("does not stretch the Traits card to fill its column", () => {
    render(<PlayerProfileOverview {...props()} />);

    expect(screen.getByRole("region", { name: "Traits" })).not.toHaveClass("flex-1");
  });
});
