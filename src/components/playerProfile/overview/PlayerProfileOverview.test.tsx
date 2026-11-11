import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../../i18n";
import type { PositionRatings } from "../../../store/generated/PositionRatings";
import { createPlayer } from "../../../test-utils/factories";
import type { PlayerRecentMatchEntry } from "../PlayerProfileRecentMatchesCard";
import PlayerProfileOverview, { type PlayerProfileOverviewProps } from "./PlayerProfileOverview";

vi.mock("../../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

beforeAll(async () => {
  await i18nReady;
});

const t = (key: string, options?: Record<string, string | number>) =>
  options ? `${key}:${JSON.stringify(options)}` : key;

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
      weights: STRIKER_WEIGHTS.map(([attribute, weight]) => ({ attribute, weight })),
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
    seasonRatingRated: true,
    tacticalRole: null,
    contractRiskLevel: "stable",
    contractRiskLabel: "Stable",
    wageText: "€23k/wk",
    potentialOpinion: <p>Potential opinion block</p>,
    positionRatings: striker,
    attributes: player.attributes,
    traits: ["Finisher"],
    onOpenTab: () => {},
    t,
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
      "playerProfile.overview.now",
      "playerProfile.overview.details",
      "playerProfile.overview.previousMatches",
      "playerProfile.seasonStats",
      "playerProfile.attributes",
      "playerProfile.overview.traits",
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
    const now = within(screen.getByRole("region", { name: "playerProfile.overview.now" }));

    expect(now.getByText("92")).toBeInTheDocument();
    expect(now.getByText("75%")).toBeInTheDocument();
    const form = now.getByRole("list", { name: "common.recentForm" });
    expect(
      within(form)
        .getAllByRole("listitem")
        .map((item) => item.textContent),
    ).toEqual(["common.formResult.D", "common.formResult.L", "common.formResult.W"]);
  });

  /**
   * Given a rival's player, whose condition and morale the club may not see
   * When Now is read
   * Then both read "–", and there is no tactical role to set
   */
  it("hides a rival's condition and morale", () => {
    render(<PlayerProfileOverview {...props({ showsPrivateState: false })} />);
    const now = within(screen.getByRole("region", { name: "playerProfile.overview.now" }));

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

    fireEvent.change(screen.getByRole("combobox", { name: "tactics.playerRoleLabel" }), {
      target: { value: "Poacher" },
    });

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
    const details = within(screen.getByRole("region", { name: "playerProfile.overview.details" }));

    expect(details.getByText(/March 14, 2002/)).toBeInTheDocument();
    expect(details.getByText(/June 30, 2028/)).toBeInTheDocument();
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

    fireEvent.click(
      within(screen.getByRole("region", { name: "playerProfile.overview.details" })).getByRole(
        "button",
      ),
    );

    expect(retry).toHaveBeenCalledTimes(1);
  });

  /**
   * Given three recent matches
   * When Previous matches is read
   * Then each is a row: the result, the opponent, the score, his goals or assists and his rating
   */
  it("lists the previous matches with result, opponent, score and rating", () => {
    render(<PlayerProfileOverview {...props()} />);
    const rows = within(
      screen.getByRole("region", { name: "playerProfile.overview.previousMatches" }),
    ).getAllByRole("listitem");

    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("common.formResult.W");
    expect(rows[0]).toHaveTextContent("Rovers 0");
    expect(rows[0]).toHaveTextContent("3–0");
    expect(rows[0]).toHaveTextContent("7.8");
    expect(rows[1]).toHaveTextContent("common.formResult.L");
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
      within(
        screen.getByRole("region", { name: "playerProfile.overview.previousMatches" }),
      ).getByText("playerProfile.noRecentMatches"),
    ).toBeInTheDocument();
    expect(screen.queryByRole("list", { name: "common.recentForm" })).toBeNull();
  });

  /**
   * Given his season totals
   * When Season stats is read
   * Then apps, goals, assists, average rating and minutes show
   */
  it("shows the season's five figures", () => {
    render(<PlayerProfileOverview {...props()} />);
    const season = within(screen.getByRole("region", { name: "playerProfile.seasonStats" }));

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
    const season = within(screen.getByRole("region", { name: "playerProfile.seasonStats" }));

    expect(season.queryByText("7.4")).toBeNull();
    expect(season.getByText("–")).toBeInTheDocument();
  });

  /**
   * Given a striker, whose natural row weighs eight attributes
   * When the attributes card is read
   * Then the radar has those eight axes (what the game weighs there), each with his value, and the
   *      line names the position's three key attributes with their values
   */
  it("draws the radar from the position's weights, and the key attributes under it", () => {
    render(<PlayerProfileOverview {...props()} />);
    const attributes = within(screen.getByRole("region", { name: "playerProfile.attributes" }));

    const radar = attributes.getByRole("img");
    expect(radar.getAttribute("aria-label")).toContain("common.attributes.shooting");
    expect(radar.querySelectorAll("text")).toHaveLength(8);
    expect(attributes.getByText(/playerProfile.overview.keyFor/)).toBeInTheDocument();
    expect(
      attributes.getByText(/common.attributes.shooting/, { selector: "span" }),
    ).toBeInTheDocument();
    expect(attributes.getByText("84")).toBeInTheDocument();
  });

  /**
   * Given a player the club knows nothing about (no position ratings, no readable attributes)
   * When the attributes card is read
   * Then there is no radar and it says the attributes are hidden
   */
  it("hides the radar for a player whose attributes the club cannot read", () => {
    render(<PlayerProfileOverview {...props({ positionRatings: null, attributes: null })} />);
    const attributes = within(screen.getByRole("region", { name: "playerProfile.attributes" }));

    expect(attributes.queryByRole("img")).toBeNull();
    expect(attributes.getByText("playerProfile.attributesHidden")).toBeInTheDocument();
  });

  /**
   * Given the Overview's links to his matches and his career
   * When "All matches" or "Career" is pressed
   * Then the Career tab is asked for
   */
  it("opens the Career tab from the matches and season links", () => {
    const onOpenTab = vi.fn();
    render(<PlayerProfileOverview {...props({ onOpenTab })} />);

    fireEvent.click(screen.getByRole("button", { name: "playerProfile.overview.allMatches" }));
    fireEvent.click(screen.getByRole("button", { name: "playerProfile.tabs.career" }));

    expect(onOpenTab).toHaveBeenNthCalledWith(1, "career");
    expect(onOpenTab).toHaveBeenNthCalledWith(2, "career");
  });

  /**
   * Given a player with no traits
   * When the Overview is drawn
   * Then there is no Traits card
   */
  it("draws no Traits card for a player without traits", () => {
    render(<PlayerProfileOverview {...props({ traits: [] })} />);

    expect(screen.queryByRole("region", { name: "playerProfile.overview.traits" })).toBeNull();
  });
});
