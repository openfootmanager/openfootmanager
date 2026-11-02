import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { FormDots } from "./FormDots";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "common.recentForm": "Recent form",
        "common.won": "V",
        "common.drawn": "E",
        "common.lost": "D",
        "common.wonFull": "Vitórias",
        "common.drawnFull": "Empates",
        "common.lostFull": "Derrotas",
      })[key] ?? key,
  }),
}));

/** What a screen reader reads of an element: its text, less what is hidden from it. */
function spoken(element: Element): string {
  const copy = element.cloneNode(true) as Element;
  for (const hidden of copy.querySelectorAll('[aria-hidden="true"]')) hidden.remove();
  return copy.textContent?.trim() ?? "";
}

describe("FormDots", () => {
  /**
   * Given a club's last five results as the backend keeps them (W, D, W, W, L, oldest first)
   * When they are drawn in a locale whose letters differ from English (Portuguese: V, E, D)
   * Then they are a list named "Recent form", oldest first, each showing the locale's letter and
   *      read as its word, never the backend's English letter.
   */
  it("draws each result as the locale's letter, read as its word, oldest first", () => {
    render(<FormDots results={["W", "D", "W", "W", "L"]} />);

    const list = screen.getByRole("list", { name: "Recent form" });
    const items = within(list).getAllByRole("listitem");
    expect(items.map((item) => item.querySelector('[aria-hidden="true"]')?.textContent)).toEqual([
      "V",
      "E",
      "V",
      "V",
      "D",
    ]);
    expect(items.map((item) => spoken(item))).toEqual([
      "Vitórias",
      "Empates",
      "Vitórias",
      "Vitórias",
      "Derrotas",
    ]);
  });

  /**
   * Given a win, a draw and a loss
   * When they are drawn
   * Then each takes its result's tint and text tone from the role tokens (positive, neutral,
   *      danger), as a Badge does: a tint keeps the letter readable where a solid fill would not, and
   *      the colour adds to the letter and the word rather than standing alone.
   */
  it("tones each result from the role tokens", () => {
    render(<FormDots results={["W", "D", "L"]} />);

    const [win, draw, loss] = screen.getAllByRole("listitem");
    expect(win).toHaveClass("bg-positive-tint", "text-positive");
    expect(draw).toHaveClass("bg-sunken");
    expect(loss).toHaveClass("bg-danger-tint", "text-danger");
  });

  /**
   * Given a club with no results yet, or a value the backend never sends
   * When the form is drawn
   * Then nothing is drawn for no results, and an unknown value is left out rather than shown raw.
   */
  it("draws nothing for no results, and leaves out a value it doesn't know", () => {
    const { container } = render(<FormDots results={[]} />);
    expect(container).toBeEmptyDOMElement();

    render(<FormDots results={["W", "?"]} />);
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
  });
});
