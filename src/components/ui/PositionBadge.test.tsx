import { render, screen, waitFor, within } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { resetPositionGroupsCache } from "../../hooks/usePositionGroups";
import { i18nReady } from "../../i18n";
import { PositionBadge, PositionBadges } from "./PositionBadge";

vi.mock("../../services/positionService", () => ({ getPositionGroups: vi.fn() }));

const { getPositionGroups } = await import("../../services/positionService");

/** The backend's table, with one position moved to the wrong group on purpose (see below). */
const groups = [
  { position: "Striker", group: "Forward", rank: 0 },
  { position: "RightWinger", group: "Defender", rank: 0 },
  { position: "CenterBack", group: "Defender", rank: 0 },
];

beforeEach(() => {
  resetPositionGroupsCache();
  vi.mocked(getPositionGroups).mockResolvedValue(groups);
});

const describePosition = (position: string) => ({
  label: position === "Striker" ? "ST" : position === "CenterBack" ? "CB" : "RW",
  name: position === "Striker" ? "Striker" : position === "CenterBack" ? "Center Back" : "Winger",
});

describe("PositionBadge", () => {
  beforeAll(async () => {
    await i18nReady;
  });

  /**
   * Given a player deployed in his natural position
   * When the badge renders
   * Then it carries no marker, but its name still says "Natural position".
   */
  it("shows no marker for a natural position and still names it", () => {
    render(<PositionBadge position="Striker" label="ST" name="Striker" familiarity="natural" />);

    expect(screen.getByText("Striker, Natural position")).toHaveClass("sr-only");
    expect(screen.queryByTestId("familiarity-dot")).not.toBeInTheDocument();
  });

  /**
   * Given a player deployed in a position he can play but is not his own
   * When the badge renders
   * Then it carries a small hollow dot and a dashed outline, and its name says "Can play there".
   */
  it("marks an adapted position with a hollow dot and a dashed outline", () => {
    const { container } = render(
      <PositionBadge position="RightWinger" label="RW" name="Winger" familiarity="adapted" />,
    );

    expect(screen.getByText("Winger, Can play there")).toBeInTheDocument();
    const dot = screen.getByTestId("familiarity-dot");
    expect(dot).toHaveClass("bg-panel", "border-fg-subtle");
    expect(dot).not.toHaveClass("bg-warning-solid");
    expect(container.firstElementChild).toHaveClass("outline-dashed");
  });

  /**
   * Given a player deployed out of position
   * When the badge renders
   * Then it carries the filled warning dot and a solid warning outline, and its name says so.
   */
  it("marks an out-of-position slot with the filled warning dot", () => {
    const { container } = render(
      <PositionBadge position="CenterBack" label="CB" name="Center Back" familiarity="out" />,
    );

    expect(screen.getByText("Center Back, Out of position")).toBeInTheDocument();
    expect(screen.getByTestId("familiarity-dot")).toHaveClass(
      "bg-warning-solid",
      "border-warning-solid",
    );
    expect(container.firstElementChild).toHaveClass("outline-warning-solid");
    expect(container.firstElementChild).not.toHaveClass("outline-dashed");
  });

  /**
   * Given a backend table that puts a winger in the defender group (nothing a TypeScript map says)
   * When the winger's badge renders
   * Then it is toned as the table's group, and as the neutral tone while the table is not here.
   */
  it("tones the badge by the group the backend names", async () => {
    const { container } = render(<PositionBadge position="RightWinger" label="RW" name="Winger" />);
    expect(container.querySelector(".bg-sunken")).not.toBeNull();

    await waitFor(() => expect(container.querySelector(".bg-primary-100")).not.toBeNull());
    expect(container.querySelector(".bg-danger-tint")).toBeNull();
  });

  /**
   * Given a badge for one of the player's other positions
   * When it renders
   * Then it is outlined and not filled with the position's group colour.
   */
  it("draws an alternate position outlined", () => {
    const { container } = render(
      <PositionBadge position="RightWinger" label="RW" name="Winger" kind="alternate" />,
    );

    expect(container.querySelector(".bg-transparent")).not.toBeNull();
    expect(container.querySelector(".bg-danger-tint")).toBeNull();
  });

  /**
   * Given no familiarity
   * When the badge renders
   * Then it states nothing about familiarity, and never uses the word "fit".
   */
  it("says nothing about familiarity when none is given", () => {
    const { container } = render(<PositionBadge position="Striker" label="ST" name="Striker" />);

    expect(container).toHaveTextContent("ST");
    expect(container.textContent?.toLowerCase()).not.toContain("fit");
    expect(screen.queryByTestId("familiarity-dot")).not.toBeInTheDocument();
  });
});

describe("PositionBadges", () => {
  beforeAll(async () => {
    await i18nReady;
  });

  const badges = (container: HTMLElement) =>
    within(container).queryAllByTestId("position-badge-item");

  /**
   * Given a natural position and two alternates, deployed in the natural one
   * When the badges render
   * Then the natural one comes first filled, the alternates follow outlined, and nothing is marked.
   */
  it("lists the natural position first and the alternates after it", () => {
    const { container } = render(
      <PositionBadges
        natural="Striker"
        alternates={["RightWinger", "CenterBack"]}
        deployed="Striker"
        familiarity="natural"
        describe={describePosition}
      />,
    );

    expect(badges(container).map((b) => b.textContent)).toEqual([
      "STStriker, Natural position",
      "RWWinger",
      "CBCenter Back",
    ]);
    expect(screen.queryByTestId("familiarity-dot")).not.toBeInTheDocument();
  });

  /**
   * Given badges drawn in a table row
   * When they are asked to keep to one line
   * Then they never wrap, so the row stays a row's height; elsewhere they wrap as before.
   */
  it("keeps to one line when asked, and wraps otherwise", () => {
    const props = {
      natural: "Striker",
      alternates: ["RightWinger", "CenterBack"],
      describe: describePosition,
    };
    const { container, rerender } = render(<PositionBadges {...props} oneLine />);
    const list = () => badges(container)[0].parentElement;
    expect(list()).toHaveClass("flex-nowrap");
    rerender(<PositionBadges {...props} />);
    expect(list()).toHaveClass("flex-wrap");
  });

  /**
   * Given a player deployed in one of his alternates
   * When the badges render
   * Then that alternate carries the hollow dot, and the natural position carries none.
   */
  it("marks the alternate the player is deployed in as adapted", () => {
    const { container } = render(
      <PositionBadges
        natural="Striker"
        alternates={["RightWinger"]}
        deployed="RightWinger"
        familiarity="adapted"
        describe={describePosition}
      />,
    );

    const [natural, alternate] = badges(container);
    expect(within(natural).queryByTestId("familiarity-dot")).not.toBeInTheDocument();
    expect(within(alternate).getByTestId("familiarity-dot")).toHaveClass("bg-panel");
  });

  /**
   * Given a player deployed in a position that is neither natural nor an alternate
   * When the badges render
   * Then an extra outlined badge for that slot is appended with the warning dot.
   */
  it("appends the deployed slot as out of position when it is not among his positions", () => {
    const { container } = render(
      <PositionBadges
        natural="Striker"
        alternates={["RightWinger"]}
        deployed="CenterBack"
        familiarity="out"
        describe={describePosition}
      />,
    );

    const items = badges(container);
    expect(items).toHaveLength(3);
    expect(within(items[2]).getByTestId("familiarity-dot")).toHaveClass("bg-warning-solid");
    expect(items[2]).toHaveTextContent("CB");
  });

  /**
   * Given more alternates than the row has room for
   * When the badges render with a maximum
   * Then the rest collapse into a +n count, and a deployed slot is never the one hidden.
   */
  it("collapses the overflow into a count", () => {
    const { container } = render(
      <PositionBadges
        natural="Striker"
        alternates={["RightWinger", "CenterBack", "Defender", "Forward"]}
        max={3}
        describe={describePosition}
      />,
    );

    expect(badges(container)).toHaveLength(3);
    expect(screen.getByText("+2")).toBeInTheDocument();
  });

  /**
   * Given the player is deployed in an alternate that would fall into the overflow
   * When the badges render with a maximum
   * Then that alternate stays visible, in place of the last one that fits.
   */
  it("never hides the badge of the slot the player is deployed in", () => {
    const { container } = render(
      <PositionBadges
        natural="Striker"
        alternates={["RightWinger", "CenterBack", "Defender", "Forward"]}
        deployed="Forward"
        familiarity="adapted"
        max={3}
        describe={(position) => ({ label: position.slice(0, 2).toUpperCase(), name: position })}
      />,
    );

    const shown = badges(container).map((b) => b.textContent);
    expect(shown).toHaveLength(3);
    expect(shown.some((text) => text?.startsWith("FO"))).toBe(true);
    expect(screen.getByText("+2")).toBeInTheDocument();
  });
});
