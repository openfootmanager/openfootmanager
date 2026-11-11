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
  /**
   * Given a row whose subtitle is far wider than its column
   * When it is rendered
   * Then the subtitle is clamped to one line (so the row keeps the height the list gives it) and the
   *      whole text stays available as the element's title
   */
  it("clamps a long subtitle to one line and keeps its full text as a title", () => {
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

    const subtitle = screen.getByText(LONG_SUBTITLE);
    expect(subtitle).toHaveClass("truncate");
    expect(subtitle).toHaveAttribute("title", LONG_SUBTITLE);
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
