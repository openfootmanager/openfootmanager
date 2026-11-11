import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { i18nReady } from "../../../i18n";
import type { PositionRatings } from "../../../store/generated/PositionRatings";
import { createPlayer } from "../../../test-utils/factories";
import { ratingTierClass } from "../../../lib/ratingTier";
import PlayerProfileAttributesTab, {
  type PlayerProfileAttributesTabProps,
} from "./PlayerProfileAttributesTab";

vi.mock("../../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});

beforeAll(async () => {
  await i18nReady;
});

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

function props(
  overrides: Partial<PlayerProfileAttributesTabProps> = {},
): PlayerProfileAttributesTabProps {
  const player = createPlayer({ natural_position: "Striker" });
  return {
    attributes: { ...player.attributes, shooting: 84, pace: 55, stamina: 30 },
    isGoalkeeper: false,
    ratings: { status: "answered", value: striker },
    scoutedOn: null,
    scoutOutOfDate: false,
    language: "en",
    potentialOpinion: <p>Opinion block</p>,
    ...overrides,
  };
}

const card = () => within(screen.getByRole("region", { name: "Attributes" }));
const rowOf = (name: string) => card().getByText(name, { selector: "span" }).closest("li");

describe("PlayerProfileAttributesTab", () => {
  /**
   * Given an outfield player
   * When the tab is drawn
   * Then it has a Technical, a Mental and a Physical column, each headed with its average
   */
  it("has three columns, each with its average", () => {
    render(<PlayerProfileAttributesTab {...props()} />);

    const headings = card()
      .getAllByRole("heading", { level: 4 })
      .map((heading) => heading.textContent);
    expect(headings).toEqual(["Technical", "Mental", "Physical"]);
    // The attributes are 60 everywhere but shooting 84, pace 55 and stamina 30: Technical
    // (60+84+60+60+60)/5 = 64.8, Mental 60, Physical (55+30+60+60+60)/5 = 53.
    const averageOf = (column: string) =>
      (card().getByRole("heading", { level: 4, name: column }).parentElement as HTMLElement)
        .textContent;
    expect(averageOf("Technical")).toContain("average 65");
    expect(averageOf("Mental")).toContain("average 60");
    expect(averageOf("Physical")).toContain("average 53");
  });

  /**
   * Given a goalkeeper
   * When the tab is drawn
   * Then Goalkeeping replaces Technical
   */
  it("has Goalkeeping in place of Technical for a goalkeeper", () => {
    render(<PlayerProfileAttributesTab {...props({ isGoalkeeper: true })} />);

    expect(
      card()
        .getAllByRole("heading", { level: 4 })
        .map((h) => h.textContent),
    ).toEqual(["Goalkeeper", "Mental", "Physical"]);
  });

  /**
   * Given a striker, whose natural position weighs shooting most and does not weigh stamina
   * When the rows are read
   * Then shooting has three dots, pace two (12 percent), aerial one, and stamina none, each said in
   *      words as "weight N of 3"
   */
  it("shows how much each attribute counts at his natural position", () => {
    render(<PlayerProfileAttributesTab {...props()} />);

    const dots = (name: string, spoken: string) => {
      const group = within(rowOf(name) as HTMLElement).getByLabelText(spoken);
      return {
        total: group.children.length,
        filled: [...group.children].filter((dot) => dot.classList.contains("bg-fg-muted")).length,
        empty: [...group.children].filter((dot) => dot.classList.contains("bg-line")).length,
      };
    };
    // What is painted, not only what is said: filled dots of three.
    expect(dots("Shooting", "weight 3 of 3")).toEqual({ total: 3, filled: 3, empty: 0 });
    expect(dots("Pace", "weight 2 of 3")).toEqual({ total: 3, filled: 2, empty: 1 });
    expect(dots("Aerial", "weight 1 of 3")).toEqual({ total: 3, filled: 1, empty: 2 });
    expect(dots("Stamina", "weight 0 of 3")).toEqual({ total: 3, filled: 0, empty: 3 });
  });

  /**
   * Given a position whose backend importance for aerial is 3 although its weight is only 4 percent
   * When the rows are read
   * Then aerial has three dots: the dots are the backend's importance, not a cut of the weight made
   *      here
   */
  it("reads the importance the backend gives, not the weight", () => {
    const odd: PositionRatings = {
      ...striker,
      rows: [
        {
          ...striker.rows[0],
          weights: [
            { attribute: "aerial", weight: 4, importance: 3 },
            { attribute: "shooting", weight: 26, importance: 1 },
          ],
        },
      ],
    };
    render(
      <PlayerProfileAttributesTab {...props({ ratings: { status: "answered", value: odd } })} />,
    );

    expect(within(rowOf("Aerial") as HTMLElement).getByLabelText("weight 3 of 3")).toBeVisible();
    expect(within(rowOf("Shooting") as HTMLElement).getByLabelText("weight 1 of 3")).toBeVisible();
  });

  /**
   * Given a striker whose key attributes are shooting, positioning and decisions
   * When the rows are read
   * Then those three are highlighted, each says "Key attribute" to a screen reader (not shown), the
   *      others do not, and there is no explanatory line over the columns
   */
  it("highlights the natural position's key attributes, with no explanatory line", () => {
    render(<PlayerProfileAttributesTab {...props()} />);

    for (const name of ["Shooting", "Positioning", "Decisions"]) {
      const row = rowOf(name) as HTMLElement;
      expect(row).toHaveAttribute("data-key", "true");
      expect(within(row).getByText("Key attribute")).toHaveClass("sr-only");
    }
    expect(rowOf("Pace")).not.toHaveAttribute("data-key", "true");
    expect(within(rowOf("Pace") as HTMLElement).queryByText("Key attribute")).toBeNull();
    // The visible cue: a tinted, semibold row for each key attribute, and neither on the others.
    for (const name of ["Shooting", "Positioning", "Decisions"]) {
      expect(rowOf(name)).toHaveClass("bg-selected-tint", "font-semibold");
    }
    expect(rowOf("Pace")).not.toHaveClass("bg-selected-tint");
    expect(rowOf("Pace")).not.toHaveClass("font-semibold");
    expect(card().queryByText(/what the game weighs most/)).toBeNull();
    expect(card().queryByText(/dots = how much/)).toBeNull();
  });

  /**
   * Given the columns
   * When they are laid out
   * Then they are three side by side only where the card itself is wide (a container breakpoint, not
   *      the window's), and stack below it, so a name is never cut to a letter in a narrow window
   */
  it("stacks the columns in a narrow card", () => {
    render(<PlayerProfileAttributesTab {...props()} />);
    const grid = card().getAllByRole("heading", { level: 4 })[0].closest("[data-columns]");

    expect(grid).toHaveClass("@[56rem]:grid-cols-3");
    expect(grid).toHaveClass("grid-cols-1");
    expect(grid).not.toHaveClass("md:grid-cols-3");
  });

  /**
   * Given an attribute of 84 and one of 30
   * When their values are read
   * Then each is painted in the tone of the backend's tier for it, never a colour chosen from the
   *      number here
   */
  it("paints a value in its backend tier's tone", () => {
    render(<PlayerProfileAttributesTab {...props()} />);

    const value = (name: string) =>
      (rowOf(name) as HTMLElement).querySelector("[data-value]") as HTMLElement;
    expect(value("Shooting").textContent).toBe("84");
    expect(value("Shooting")).toHaveClass(...ratingTierClass("S", "text").split(" "));
    expect(value("Stamina")).toHaveClass(...ratingTierClass("Rest", "text").split(" "));
  });

  /**
   * Given the potential opinion block, and attributes from the club's own staff
   * When the footer is read
   * Then the opinion is there, and the scouted date is a dash (no scout reported on him)
   */
  it("shows the opinion and a dash for the scouted date when no scout reported", () => {
    render(<PlayerProfileAttributesTab {...props()} />);

    expect(card().getByText("Opinion block")).toBeInTheDocument();
    expect(card().getByText("Scouted").parentElement).toHaveTextContent("–");
  });

  /**
   * Given attributes from a scout's report of 12 May 2031 that is out of date
   * When the footer is read
   * Then it shows the report's date and says it is out of date
   */
  it("shows the scouted date, and when the report is out of date", () => {
    render(
      <PlayerProfileAttributesTab {...props({ scoutedOn: "2031-05-12", scoutOutOfDate: true })} />,
    );

    expect(card().getByText("Scouted").parentElement).toHaveTextContent(/May 12, 2031/);
    expect(card().getByText("Out of date")).toBeInTheDocument();
  });

  /**
   * Given a player whose attributes the club cannot read
   * When the tab is drawn
   * Then it says they are hidden and offers no columns
   */
  it("says the attributes are hidden when the club cannot read them", () => {
    render(
      <PlayerProfileAttributesTab
        {...props({ attributes: null, ratings: { status: "answered", value: null } })}
      />,
    );

    expect(card().getByText("Attributes Hidden")).toBeInTheDocument();
    expect(card().queryByRole("heading", { level: 4 })).toBeNull();
  });

  /**
   * Given readable attributes and a position-ratings request that failed
   * When the tab is drawn
   * Then the attributes are all there without dots or highlight, and a retry is offered
   */
  it("lists the attributes without weights when the ratings failed, with a retry", () => {
    const retry = vi.fn();
    render(<PlayerProfileAttributesTab {...props({ ratings: { status: "failed", retry } })} />);

    expect(rowOf("Shooting")).not.toHaveAttribute("data-key", "true");
    expect(card().queryByLabelText(/weight \d of 3/)).toBeNull();
    fireEvent.click(card().getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledTimes(1);
  });

  /**
   * Given the tab
   * When it is drawn
   * Then there are no position cards: his ratings at each position are not repeated here
   */
  it("has no position cards", () => {
    render(<PlayerProfileAttributesTab {...props()} />);

    expect(screen.queryByRole("region", { name: "Positions" })).toBeNull();
    expect(screen.queryByText("Natural position")).toBeNull();
  });

  /**
   * Given the tab's card
   * When it is drawn
   * Then it is positioned, so its screen-reader text cannot stretch the page
   */
  it("is positioned so its screen-reader text stays inside it", () => {
    render(<PlayerProfileAttributesTab {...props()} />);

    expect(screen.getByRole("region", { name: "Attributes" })).toHaveClass("relative");
  });
});
