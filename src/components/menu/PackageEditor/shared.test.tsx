import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { installScrollGeometry } from "../../../test-utils/scrollGeometry";
import { EntityRow, EntityRowList } from "./shared";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

let restoreGeometry: () => void;
beforeEach(() => {
  restoreGeometry = installScrollGeometry({ viewportHeight: 640 });
});
afterEach(() => {
  restoreGeometry();
  document.documentElement.style.fontSize = "";
});

const LONG_SUBTITLE =
  "A very long club name that FC · Some Extraordinarily Long Country Name · Midfielder · 1998-04-12";

describe("EntityRow", () => {
  const renderRow = () =>
    render(
      <EntityRow
        title="Player"
        subtitle={LONG_SUBTITLE}
        onEdit={() => {}}
        onDelete={() => {}}
        editLabel="edit"
        deleteLabel="delete"
      />,
    );

  /**
   * Given a row whose subtitle is far wider than its column
   * When it is rendered
   * Then the subtitle is one line, in the scaling caption size (not a px literal), and the whole text
   *      is in the page once, as the paragraph a screen reader reads, and as its title
   */
  it("clamps a long subtitle to one line in the caption size and keeps its full text", () => {
    renderRow();

    const subtitle = screen.getByText(LONG_SUBTITLE);
    expect(subtitle).toHaveClass("truncate", "text-caption");
    expect(subtitle.className).not.toMatch(/text-\[\d+px\]/);
    expect(subtitle).toHaveAttribute("title", LONG_SUBTITLE);
  });

  /**
   * Given a keyboard user on the row's edit button (no pointer, so no hover title)
   * When the row has focus
   * Then the full subtitle is drawn from the row: the panel shows under `group-focus-within` and
   *      takes its text from the subtitle
   */
  it("shows the whole subtitle while the row has keyboard focus", () => {
    renderRow();

    const panel = screen.getByText(LONG_SUBTITLE).parentElement as HTMLElement;
    expect(panel).toHaveAttribute("data-full", LONG_SUBTITLE);
    expect(panel.className).toContain("group-focus-within:after:block");
    expect(panel.closest(".group")).not.toBeNull();
  });
});

describe("EntityRowList", () => {
  const rows = Array.from({ length: 200 }, (_, i) => ({ i }));
  const renderList = () =>
    render(
      <div data-scroll-container="">
        <EntityRowList
          rows={rows}
          itemKey={(row) => row.i}
          renderRow={(row) => <span>row {row.i}</span>}
          label="Rows"
        />
      </div>,
    );

  /**
   * Given the default text size (a 16px root)
   * When the list is rendered
   * Then a row is 4rem = 64px tall
   */
  it("gives each row 4rem at the default text size", () => {
    document.documentElement.style.fontSize = "16px";
    renderList();

    expect(screen.getAllByRole("listitem")[0]).toHaveStyle({ height: "64px" });
  });

  /**
   * Given the largest text scale (a 24px root)
   * When the list is rendered
   * Then the row grows with the text: 4rem = 96px, and the rows are spaced by the larger step
   */
  it("makes the row height follow the text scale", () => {
    document.documentElement.style.fontSize = "24px";
    renderList();

    const [first, second] = screen.getAllByRole("listitem");
    expect(first).toHaveStyle({ height: "96px", top: "0px" });
    expect(second).toHaveStyle({ top: "108px" });
  });
});
