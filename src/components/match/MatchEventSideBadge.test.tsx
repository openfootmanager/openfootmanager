import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import MatchEventSideBadge from "./MatchEventSideBadge";

const homeTeam = { short_name: "BRU" };
const awayTeam = { short_name: "SPA" };

describe("MatchEventSideBadge", () => {
  /**
   * Given an away club called Sporting Waregem whose short name is SPA
   * When an away event is badged
   * Then the badge reads the club's short name, as the score header does, not the name's first
   *      three letters (SPO).
   */
  it("labels an event with its club's short name", () => {
    render(<MatchEventSideBadge side="Away" homeTeam={homeTeam} awayTeam={awayTeam} />);

    expect(screen.getByText("SPA")).toBeInTheDocument();
    expect(screen.queryByText("SPO")).not.toBeInTheDocument();
  });

  /**
   * Given the same two clubs
   * When a home event is badged
   * Then it carries the home club's short name.
   */
  it("labels a home event with the home club's short name", () => {
    render(<MatchEventSideBadge side="Home" homeTeam={homeTeam} awayTeam={awayTeam} />);

    expect(screen.getByText("BRU")).toBeInTheDocument();
  });
});
