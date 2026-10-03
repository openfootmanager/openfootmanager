import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";

import { i18nReady } from "../../i18n";
import { condBgColor } from "../../lib/playerConditionDisplay";
import { PitchToken, type PitchFitTone } from "./PitchToken";

/**
 * #330 — each channel of the token says one thing: the ring is position fit,
 * the bar is condition (length and colour), and the words carry both. Uses the
 * real i18n hook.
 */

function renderToken(condition: number, fitTone: PitchFitTone) {
  render(
    <PitchToken name="Rossi" positionAbbr="ST" ovr={74} condition={condition} fitTone={fitTone} />,
  );
  return screen.getByTestId("pitch-token-condition-fill");
}

describe("PitchToken condition bar", () => {
  beforeAll(async () => {
    await i18nReady;
  });

  /**
   * Given a fully rested player playing out of position
   * When the token renders
   * Then the bar colour is the colour of 95% condition, not a fit warning
   */
  it("does not let an out-of-position fit turn a rested player's bar red", () => {
    const fill = renderToken(95, "out");

    expect(fill.className).toContain(condBgColor(95));
    expect(fill.className).not.toContain("bg-red");
  });

  /**
   * Given a worn-out player in exactly the right slot
   * When the token renders
   * Then the bar is the warning colour for 30% condition
   */
  it("shows a tired in-position player's bar as the low-condition colour", () => {
    const fill = renderToken(30, "exact");

    expect(fill.className).toContain(condBgColor(30));
  });

  /**
   * Given the same condition in every fit tone
   * When each token renders
   * Then the bar colour is identical, and matches the shared condition colour
   *      the training tab uses
   */
  it.each(["exact", "adapted", "out", "empty"] as const)(
    "colours 65 percent condition the same way for %s fit",
    (fitTone) => {
      const fill = renderToken(65, fitTone);

      expect(fill.className).toContain(condBgColor(65));
    },
  );

  /**
   * Given a 74% condition player adapted to the slot
   * When the token renders
   * Then hovering (tooltip) and screen readers (description) get both signals in words
   */
  it("spells out condition and fit in words", () => {
    renderToken(74, "adapted");

    expect(screen.getByText("Condition 74% · Adapted fit")).toHaveClass("sr-only");
    fireEvent.mouseEnter(screen.getByText("Rossi"));
    expect(screen.getByRole("tooltip")).toHaveTextContent("Condition 74% · Adapted fit");
  });

  /**
   * Given an empty-fit token (no slot verdict)
   * When it renders
   * Then only the condition is spelled out
   */
  it("omits the fit when there is none to state", () => {
    renderToken(80, "empty");

    fireEvent.mouseEnter(screen.getByText("Rossi"));
    expect(screen.getByRole("tooltip")).toHaveTextContent("Condition 80%");
  });

  /**
   * Given an out-of-position and an exact player
   * When tokens render
   * Then the ring still encodes fit
   */
  it("keeps the ring as the fit channel", () => {
    const { container } = render(
      <PitchToken name="A" positionAbbr="GK" ovr={70} condition={90} fitTone="out" hidePortrait />,
    );

    expect(container.innerHTML).toContain("ring-red-400");
  });
});
