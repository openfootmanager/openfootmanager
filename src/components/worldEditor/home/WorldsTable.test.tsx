import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import WorldsTable from "./WorldsTable";
import type { WorldRow } from "./worldRows";

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (params?.name !== undefined) return `${key}:${params.name}`;
      if (params?.count !== undefined) return `${key}:${params.count}`;
      if (params?.version !== undefined) return `${key}:${params.version}`;
      return key;
    },
    i18n: { language: "en" },
  }),
}));

const PROJECT: WorldRow = {
  path: "/w/nordic",
  kind: "project",
  name: "Nordic Leagues 2027",
  version: null,
  clubs: 412,
  players: 9860,
  openedAt: "2026-11-09T12:00:00Z",
  check: { kind: "clean" },
};
const INSTALLED: WorldRow = {
  path: "/p/nordic.ofm",
  kind: "installed",
  name: "Nordic Leagues 2027",
  version: "1.2",
  clubs: 412,
  players: 9860,
  openedAt: null,
  check: { kind: "unchecked" },
};
const BROKEN: WorldRow = {
  path: "/w/gone",
  kind: "project",
  name: "gone",
  version: null,
  clubs: 0,
  players: 0,
  openedAt: null,
  check: { kind: "unreadable", errorKey: "be.error.package.notAProjectDirectory" },
};

function renderTable(rows: WorldRow[], overrides: Partial<Parameters<typeof WorldsTable>[0]> = {}) {
  const onOpen = vi.fn();
  render(
    <WorldsTable
      rows={rows}
      busy={false}
      loading={false}
      empty="nothing"
      onOpen={onOpen}
      {...overrides}
    />,
  );
  return onOpen;
}

describe("WorldsTable", () => {
  /**
   * Given a project the editor opened and a clean check, and an installed package
   * When the table is drawn
   * Then each row names the world with its folder or its installed version, its kind, its clubs and
   *      players, when it was last opened and what the last check found; an installed package is
   *      never opened or checked, so it reads a dash and "not checked".
   */
  it("shows each world's place, kind, counts, last opened and last check", () => {
    renderTable([PROJECT, INSTALLED]);

    const rows = screen.getAllByRole("row");
    expect(within(rows[1]).getByText("/w/nordic")).toBeInTheDocument();
    expect(within(rows[1]).getByText("worldEditor.home.kindProject")).toBeInTheDocument();
    expect(within(rows[1]).getByText("412")).toBeInTheDocument();
    expect(within(rows[1]).getByText("9,860")).toBeInTheDocument();
    expect(within(rows[1]).getByText("Nov 9, 2026")).toBeInTheDocument();
    expect(within(rows[1]).getByText("worldEditor.home.noIssues")).toBeInTheDocument();

    expect(within(rows[2]).getByText("worldEditor.home.installedVersion:1.2")).toBeInTheDocument();
    expect(within(rows[2]).getByText("worldEditor.home.kindInstalled")).toBeInTheDocument();
    expect(within(rows[2]).getByText("worldEditor.home.notChecked")).toBeInTheDocument();
    expect(within(rows[2]).getByText("—")).toBeInTheDocument();
  });

  /**
   * Given a project, an installed package, and a folder that can't be read
   * When their buttons are pressed
   * Then a project opens, an installed package opens as an editable copy ("Edit a copy"), and the
   *      unreadable one's button stays enabled (a project with a broken file is what the editor is
   *      for), its chip saying it can't be read.
   */
  it("opens a project, edits a copy of an installed package, and keeps an unreadable folder openable", () => {
    const onOpen = renderTable([PROJECT, INSTALLED, BROKEN]);

    fireEvent.click(
      screen.getByRole("button", { name: "worldEditor.home.openNamed:Nordic Leagues 2027" }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "worldEditor.home.editCopyNamed:Nordic Leagues 2027" }),
    );

    expect(onOpen.mock.calls.map(([row]) => (row as WorldRow).path)).toEqual([
      "/w/nordic",
      "/p/nordic.ofm",
    ]);
    expect(screen.getByText("worldEditor.home.unreadable")).toBeInTheDocument();
    // Why it cannot be read is written under its folder, visible whatever the table's width.
    expect(screen.getByText("be.error.package.notAProjectDirectory")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "worldEditor.home.openNamed:gone" })).toBeEnabled();
  });

  /**
   * Given a project whose recorded open time is not a date
   * When the table is drawn
   * Then the last-opened cell reads a dash and the table still draws: one corrupt record never takes
   *      the screen down.
   */
  it("reads a dash for an open time that is not a date", () => {
    renderTable([{ ...PROJECT, openedAt: "not a date" }]);

    expect(within(screen.getAllByRole("row")[1]).getByText("\u2014")).toBeInTheDocument();
  });

  it("issues a count in its chip, and waits while a world is being opened", () => {
    renderTable([{ ...PROJECT, check: { kind: "issues", count: 2 } }], { busy: true });

    expect(screen.getAllByText("worldEditor.issuesBadge:2").length).toBeGreaterThan(0);
    expect(
      screen.getByRole("button", { name: "worldEditor.home.openNamed:Nordic Leagues 2027" }),
    ).toBeDisabled();
  });

  it("says what to do when there are no worlds", () => {
    renderTable([], { empty: "Start with New world" });

    expect(screen.getByText("Start with New world")).toBeInTheDocument();
  });
});
