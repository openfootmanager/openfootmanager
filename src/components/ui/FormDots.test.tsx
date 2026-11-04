import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { FormDots } from "./FormDots";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        // Spanish, where the standings headers (PG, PE, PP) differ from the form letters (G, E, P)
        // and the table's plural words from one match's.
        "common.recentForm": "Recent form",
        "common.won": "PG",
        "common.drawn": "PE",
        "common.lost": "PP",
        "common.wonFull": "Ganados",
        "common.drawnFull": "Empatados",
        "common.lostFull": "Perdidos",
        "common.formResult.W": "G",
        "common.formResult.D": "E",
        "common.formResult.L": "P",
        "match.victory": "Victoria",
        "match.draw": "Empate",
        "match.defeat": "Derrota",
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
   * When they are drawn in Spanish, whose form letters (G, E, P) differ from both English and its
   *      own standings headers (PG, PE, PP)
   * Then they are a list named "Recent form", oldest first, each showing the form letter and read
   *      as one match's word (Victoria, not the table's Ganados), never the backend's English letter.
   */
  it("draws each result as the locale's letter, read as its word, oldest first", () => {
    render(<FormDots results={["W", "D", "W", "W", "L"]} />);

    const list = screen.getByRole("list", { name: "Recent form" });
    const items = within(list).getAllByRole("listitem");
    expect(items.map((item) => item.querySelector('[aria-hidden="true"]')?.textContent)).toEqual([
      "G",
      "E",
      "G",
      "G",
      "P",
    ]);
    expect(items.map((item) => spoken(item))).toEqual([
      "Victoria",
      "Empate",
      "Victoria",
      "Victoria",
      "Derrota",
    ]);
  });

  /**
   * Given a club's form, drawn without bullets (list-style: none)
   * When a screen reader on WebKit, Tauri's webview, reaches it
   * Then it is still announced as a list, because the list says its role outright: WebKit drops
   *      the semantics of an unstyled list otherwise, whatever its aria-label says.
   */
  it("says outright that the form is a list, which WebKit drops from an unstyled one", () => {
    render(<FormDots results={["W", "D", "L"]} />);

    expect(screen.getByRole("list", { name: "Recent form" })).toHaveAttribute("role", "list");
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
    // With Badge's inset line: sunken alone vanishes on a white panel.
    expect(draw).toHaveClass("bg-sunken", "ring-1", "ring-line");
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

    // Including a key every object inherits, which isn't a result either.
    render(<FormDots results={["W", "?", "constructor"]} />);
    expect(screen.getAllByRole("listitem")).toHaveLength(1);
  });
});
