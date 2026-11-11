import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { installScrollGeometry } from "../../../test-utils/scrollGeometry";
import { CompetitionForm } from "./CompetitionForm";
import { emptyCompetition, emptyTeam } from "./helpers";
import type { CompetitionDef, TeamDef } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../../hooks/useAssetDataUrl", () => ({
  useAssetDataUrl: () => null,
  evictAssetDataUrl: vi.fn(),
}));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { language: "en" },
  }),
}));

function teams(count: number): TeamDef[] {
  return Array.from({ length: count }, (_, i) => ({
    ...emptyTeam(),
    id: `t${i}`,
    name: `Team ${i}`,
    country: i % 2 === 0 ? "ENG" : "ESP",
  }));
}

function renderForm(options: { teams?: TeamDef[]; explicit?: string[] } = {}) {
  const updateField = vi.fn();
  const editing: CompetitionDef = {
    ...emptyCompetition(),
    participants: { explicit: options.explicit ?? [] },
  };
  render(
    <CompetitionForm
      editing={editing}
      editingIndex={0}
      isBusy={false}
      teams={options.teams ?? teams(5)}
      confederations={[]}
      onBack={() => {}}
      onSave={() => {}}
      updateField={updateField}
      onAssetError={() => {}}
    />,
  );
  return { updateField };
}

const addButton = () => screen.getByRole("button", { name: "worldEditor.addTeam" });
const searchBox = () => screen.getByPlaceholderText("worldEditor.searchTeams");

describe("CompetitionForm: the explicit participants picker", () => {
  /**
   * Given a competition with explicit participants and five teams
   * When Add team is opened
   * Then every team not already in the competition is offered
   */
  it("offers the teams that are not already participants", () => {
    renderForm({ explicit: ["t1"] });

    fireEvent.click(addButton());

    const offered = screen.getAllByRole("option").map((option) => option.textContent);
    expect(offered).toHaveLength(4);
    expect(screen.queryByRole("option", { name: /Team 1\b/ })).not.toBeInTheDocument();
  });

  /**
   * Given the picker open
   * When a team is picked with the pointer
   * Then it is added to the explicit participants and the picker closes
   */
  it("adds the picked team and closes", () => {
    const { updateField } = renderForm();
    fireEvent.click(addButton());

    fireEvent.mouseDown(screen.getByRole("option", { name: /Team 3/ }));

    expect(updateField).toHaveBeenCalledWith("participants", { explicit: ["t3"] });
    expect(screen.queryByPlaceholderText("worldEditor.searchTeams")).not.toBeInTheDocument();
  });

  /**
   * Given the picker open
   * When something is typed in the search field
   * Then only the teams whose name contains it (any case) are offered
   */
  it("filters the offered teams by name", () => {
    renderForm();
    fireEvent.click(addButton());

    fireEvent.change(searchBox(), { target: { value: "TEAM 2" } });

    expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual([
      expect.stringContaining("Team 2"),
    ]);
  });

  /**
   * Given a search that matches nothing
   * When the list is read
   * Then it says so
   */
  it("says so when nothing matches", () => {
    renderForm();
    fireEvent.click(addButton());

    fireEvent.change(searchBox(), { target: { value: "zzz" } });

    expect(screen.queryAllByRole("option")).toHaveLength(0);
    expect(screen.getByText("menu.noResults")).toBeInTheDocument();
  });

  /**
   * Given every team already a participant
   * When the form is shown
   * Then Add team is disabled
   */
  it("disables Add team when every team is already in", () => {
    renderForm({ teams: teams(2), explicit: ["t0", "t1"] });

    expect(addButton()).toBeDisabled();
  });

  /**
   * Given the picker open on the keyboard
   * When ArrowDown then Enter are pressed in the search field
   * Then the second offered team is added
   */
  it("picks with the arrow keys and Enter", () => {
    const { updateField } = renderForm();
    fireEvent.click(addButton());

    fireEvent.keyDown(searchBox(), { key: "ArrowDown" });
    fireEvent.keyDown(searchBox(), { key: "Enter" });

    expect(updateField).toHaveBeenCalledWith("participants", { explicit: ["t1"] });
  });

  /**
   * Given the picker open
   * When Escape is pressed
   * Then it closes and nothing is added
   */
  it("closes on Escape without adding anything", () => {
    const { updateField } = renderForm();
    fireEvent.click(addButton());

    fireEvent.keyDown(searchBox(), { key: "Escape" });

    expect(screen.queryByPlaceholderText("worldEditor.searchTeams")).not.toBeInTheDocument();
    expect(updateField).not.toHaveBeenCalled();
  });
});

describe("CompetitionForm: the participants picker on a very large package", () => {
  // jsdom lays nothing out: the fixture gives the listbox's scroller its 12rem (192 px) height.
  let restoreGeometry: () => void;
  beforeEach(() => {
    restoreGeometry = installScrollGeometry({ viewportHeight: 192 });
  });
  afterEach(() => restoreGeometry());

  /**
   * Given a package of 545 teams (the mega-sized one)
   * When Add team is opened
   * Then a window of options is in the page, not 545, each saying its place in the whole list
   */
  it("renders a window of the teams, not every team", () => {
    renderForm({ teams: teams(545) });

    fireEvent.click(addButton());

    const rendered = screen.getAllByRole("option");
    expect(rendered.length).toBeGreaterThan(5);
    expect(rendered.length).toBeLessThan(30);
    expect(rendered[0]).toHaveAttribute("aria-setsize", "545");
  });

  /**
   * Given the picker open on the first of 545
   * When End is pressed and then Enter
   * Then the last team is in the page, active, and added
   */
  it("reaches the last team with End and adds it", async () => {
    const { updateField } = renderForm({ teams: teams(545) });
    fireEvent.click(addButton());

    fireEvent.keyDown(searchBox(), { key: "End" });
    await waitFor(() => {
      const id = searchBox().getAttribute("aria-activedescendant");
      expect(id ? document.getElementById(id) : null).toHaveTextContent("Team 544");
    });
    fireEvent.keyDown(searchBox(), { key: "Enter" });

    expect(updateField).toHaveBeenCalledWith("participants", { explicit: ["t544"] });
  });
});
