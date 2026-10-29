import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { DataTable, type DataTableColumn } from "./DataTable";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string>) =>
      params?.name ? `${key}:${params.name}` : key,
  }),
}));

interface Player {
  id: string;
  name: string;
  age: number;
  morale: number;
  starter: boolean;
}

const PLAYERS: Player[] = [
  { id: "p1", name: "Anderson", age: 31, morale: 72, starter: true },
  { id: "p2", name: "Navarro", age: 33, morale: 55, starter: false },
];

const COLUMNS: DataTableColumn<Player>[] = [
  { key: "name", header: "Name", cell: (p) => p.name, primary: true, sortable: true },
  { key: "age", header: "Age", cell: (p) => p.age, numeric: true, sortable: true },
  { key: "morale", header: "Morale", cell: (p) => p.morale, numeric: true, drawer: true },
];

function renderTable(props: Partial<Parameters<typeof DataTable<Player>>[0]> = {}) {
  return render(
    <DataTable
      label="Squad"
      columns={COLUMNS}
      rows={PLAYERS}
      rowKey={(p) => p.id}
      rowName={(p) => p.name}
      {...props}
    />,
  );
}

describe("DataTable", () => {
  /**
   * Given a list of players
   * When it is drawn as a table
   * Then it is a table named for the list, with a header per column and a row per player.
   */
  it("is a named table with a header per column and a row per item", () => {
    renderTable();
    const table = screen.getByRole("table", { name: "Squad" });
    expect(
      within(table)
        .getAllByRole("columnheader")
        .map((th) => th.textContent),
    ).toEqual(expect.arrayContaining(["Name", "Age", "Morale"]));
    expect(within(table).getByRole("row", { name: /Anderson/ })).toBeInTheDocument();
    expect(within(table).getByRole("row", { name: /Navarro/ })).toBeInTheDocument();
  });

  /**
   * Given the list sorted by age, oldest first
   * When a screen reader reads the headers
   * Then the age header says it is sorted descending, and the other headers carry no sort.
   */
  it("says which column it is sorted by, on that column's header", () => {
    renderTable({ sort: { key: "age", direction: "desc" }, onSortChange: vi.fn() });
    expect(screen.getByRole("columnheader", { name: "Age" })).toHaveAttribute(
      "aria-sort",
      "descending",
    );
    expect(screen.getByRole("columnheader", { name: "Name" })).not.toHaveAttribute("aria-sort");
  });

  /**
   * Given a sortable column and one that isn't
   * When the player presses the sortable header
   * Then the table asks to sort by it (it never sorts the rows itself); the other header is no
   * button at all.
   */
  it("asks to sort by a sortable column, and offers no button on the others", () => {
    const onSortChange = vi.fn();
    renderTable({ sort: { key: "name", direction: "asc" }, onSortChange });
    fireEvent.click(
      within(screen.getByRole("columnheader", { name: "Age" })).getByRole("button", {
        name: "Age",
      }),
    );
    expect(onSortChange).toHaveBeenCalledWith("age");
    expect(
      within(screen.getByRole("columnheader", { name: "Morale" })).queryByRole("button"),
    ).toBeNull();
  });

  /**
   * Given rows that open a profile
   * When the player activates a row's name (a click, or Enter on it) or clicks elsewhere on the row
   * Then that row is opened; a control in the row's action cell does its own job without opening
   * the row.
   */
  it("opens a row from its name or the row, but not from the row's own action", () => {
    const onOpen = vi.fn();
    const onMenu = vi.fn();
    renderTable({
      onOpen,
      rowAction: {
        header: "Actions",
        cell: (p) => (
          <button type="button" onClick={() => onMenu(p.id)}>
            Menu for {p.name}
          </button>
        ),
      },
    });
    expect(screen.getByRole("columnheader", { name: "Actions" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Anderson" }));
    expect(onOpen).toHaveBeenLastCalledWith(PLAYERS[0]);

    const navarro = screen.getByRole("row", { name: /Navarro/ });
    fireEvent.click(within(navarro).getByText("33"));
    expect(onOpen).toHaveBeenLastCalledWith(PLAYERS[1]);

    onOpen.mockClear();
    fireEvent.click(screen.getByRole("button", { name: "Menu for Navarro" }));
    expect(onMenu).toHaveBeenCalledWith("p2");
    expect(onOpen).not.toHaveBeenCalled();

    // The action cell's own padding, around its control, is the action's too.
    const actionCell = screen.getByRole("button", { name: "Menu for Navarro" }).closest("td");
    fireEvent.click(actionCell as HTMLElement);
    expect(onOpen).not.toHaveBeenCalled();
  });

  /** Given rows that open nothing, then a row's name is plain text, not a button. */
  it("draws no open button when rows open nothing", () => {
    renderTable();
    expect(screen.queryByRole("button", { name: "Anderson" })).toBeNull();
    expect(screen.getByText("Anderson")).toBeInTheDocument();
  });

  /**
   * Given columns that fold away in a narrow window
   * When the player opens a row's details
   * Then the details button says whose they are and whether they are open, and the details show
   * each folded column's name and value.
   */
  it("shows the folded columns in a row's details", () => {
    renderTable();
    const toggle = screen.getByRole("button", { name: "common.detailsFor:Anderson" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    // What the toggle controls exists while closed too, hidden.
    const closed = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(closed).not.toBeNull();
    expect(closed).not.toBeVisible();

    fireEvent.click(toggle);

    expect(toggle).toHaveAttribute("aria-expanded", "true");
    const details = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(details).not.toBeNull();
    expect(details).toHaveTextContent("Morale");
    expect(details).toHaveTextContent("72");
  });

  /**
   * Given a table whose columns name none as primary (or whose primary column folds away)
   * When it is drawn with rows that open and a folded column
   * Then its first column that never folds holds the open button and the details toggle, so a
   * keyboard can still open a row and reach the folded column.
   */
  it("puts the open button and details in the first unfolded column when none is primary", () => {
    const onOpen = vi.fn();
    const columns: DataTableColumn<Player>[] = [
      { key: "name", header: "Name", cell: (p) => p.name },
      { key: "age", header: "Age", cell: (p) => p.age, numeric: true },
      { key: "morale", header: "Morale", cell: (p) => p.morale, numeric: true, drawer: true },
    ];
    renderTable({ columns, onOpen });

    fireEvent.click(screen.getByRole("button", { name: "Anderson" }));
    expect(onOpen).toHaveBeenCalledWith(PLAYERS[0]);
    expect(screen.getByRole("button", { name: "common.detailsFor:Anderson" })).toBeInTheDocument();
  });

  /** Given no folded columns, then rows have no details button. */
  it("offers no details without folded columns", () => {
    renderTable({ columns: COLUMNS.map((column) => ({ ...column, drawer: false })) });
    expect(screen.queryByRole("button", { name: /common.detailsFor/ })).toBeNull();
  });

  /**
   * Given no rows
   * When the table is drawn
   * Then the headers stay, and the empty message says there is nothing to show.
   */
  it("says when there is nothing to show", () => {
    renderTable({ rows: [], empty: "No players match" });
    expect(screen.getAllByRole("columnheader").length).toBeGreaterThan(0);
    expect(screen.getByText("No players match")).toBeInTheDocument();
  });

  /** Given rows still loading, then the table says it is busy and shows the loading word. */
  it("says it is busy while loading", () => {
    renderTable({ rows: [], loading: true });
    expect(screen.getByRole("table", { name: "Squad" })).toHaveAttribute("aria-busy", "true");
    expect(screen.getByText("common.loading")).toBeInTheDocument();
  });

  /**
   * Given a table whose every column asks to fold away in a narrow window
   * When it is drawn
   * Then the first column stays: it still holds the row's open button and its details toggle, so a
   * narrow table never hides a row with no way to reach it.
   */
  it("keeps the first column when every column asks to fold", () => {
    const allFolding = COLUMNS.map((column) => ({ ...column, primary: false, drawer: true }));
    renderTable({ columns: allFolding, onOpen: vi.fn() });
    const nameButton = screen.getByRole("button", { name: "Anderson" });
    expect(nameButton.closest("td")).not.toHaveClass("@max-3xl:hidden");
    expect(screen.getByRole("button", { name: "common.detailsFor:Anderson" })).toBeInTheDocument();
    // The first column is in the row, so its details hold only the others.
    const details = document.getElementById(
      screen
        .getByRole("button", { name: "common.detailsFor:Anderson" })
        .getAttribute("aria-controls") ?? "",
    );
    expect(within(details as HTMLElement).queryByText("Name")).not.toBeInTheDocument();
    expect(within(details as HTMLElement).getByText("Age")).toBeInTheDocument();
  });

  /**
   * Given rows with an accent and a first column that asks to fold
   * When the table is narrow
   * Then the first column stays, so the accent on its edge never folds away with it.
   */
  it("keeps the accented first column when it asks to fold", () => {
    const firstFolds = [{ ...COLUMNS[1], drawer: true }, COLUMNS[0], COLUMNS[2]];
    renderTable({ columns: firstFolds, rowAccent: (p) => (p.starter ? "positive" : null) });
    const firstCell = within(screen.getByRole("row", { name: /Anderson/ })).getByText("31");
    expect(firstCell.closest("td")).toHaveClass("border-l-positive-solid");
    expect(firstCell.closest("td")).not.toHaveClass("@max-3xl:hidden");
    expect(screen.getByRole("columnheader", { name: "Age" })).not.toHaveClass("@max-3xl:hidden");
  });

  /** Given starters, then their rows carry the starter accent, and the others don't. */
  it("marks a row with its accent", () => {
    renderTable({ rowAccent: (p) => (p.starter ? "positive" : null) });
    expect(screen.getByRole("row", { name: /Anderson/ })).toHaveAttribute(
      "data-accent",
      "positive",
    );
    expect(screen.getByRole("row", { name: /Navarro/ })).not.toHaveAttribute("data-accent");
  });
});
