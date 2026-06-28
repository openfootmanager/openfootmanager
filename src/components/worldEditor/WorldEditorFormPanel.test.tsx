import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { WorldEditorFormPanel, type FormPanel } from "./WorldEditorFormPanel";
import type { PlayerDef } from "../menu/PackageEditor/types";

// Mock react-i18next so keys render verbatim.
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (k: string) => k }),
}));

// Mock PlayerForm to a thin shim that surfaces which editor buffer it is bound
// to (editing.id) and lets us trigger the save callback. This keeps the test
// focused on the form-panel -> editor wiring rather than PlayerForm internals.
vi.mock("../menu/PackageEditor/PlayerForm", () => ({
  PlayerForm: ({
    editing,
    onSave,
  }: {
    editing: PlayerDef;
    onSave: () => void;
  }) => (
    <div>
      <span data-testid="bound-id">{editing.id}</span>
      <button type="button" onClick={onSave}>
        save
      </button>
    </div>
  ),
}));

// The other form children are never rendered for the player/youth panels but
// must still import cleanly; stub the heavier ones.
vi.mock("../menu/PackageEditor/MetadataForm", () => ({ MetadataForm: () => null }));
vi.mock("../menu/PackageEditor/TeamForm", () => ({ TeamForm: () => null }));
vi.mock("../menu/PackageEditor/ConfederationForm", () => ({ ConfederationForm: () => null }));
vi.mock("../menu/PackageEditor/CountryForm", () => ({ CountryForm: () => null }));
vi.mock("../menu/PackageEditor/StaffForm", () => ({ StaffForm: () => null }));
vi.mock("../menu/PackageEditor/NamesPoolForm", () => ({ NamesPoolForm: () => null }));
vi.mock("../menu/PackageEditor/CompetitionForm", () => ({ CompetitionForm: () => null }));
vi.mock("../menu/PackageEditor/IssueList", () => ({ IssueList: () => null }));

function makeEditor(id: string) {
  return {
    editing: { id } as PlayerDef,
    editingIndex: 0,
    revision: 0,
    handleSave: vi.fn(),
    updateField: vi.fn(),
  };
}

function renderPanel(formPanel: FormPanel, playerId: string, youthId: string) {
  const playerEditor = makeEditor(playerId);
  const youthEditor = makeEditor(youthId);
  const stub = makeEditor("x");
  render(
    <WorldEditorFormPanel
      formPanel={formPanel}
      isBusy={false}
      projectDir=""
      meta={{} as never}
      onMetaChange={() => {}}
      onSaveMetadata={() => {}}
      counts={{ teams: 0, players: 0, confederations: 0, countries: 0, competitions: 0, namePools: 0 }}
      issues={[]}
      teamEditor={stub as never}
      confEditor={stub as never}
      countryEditor={stub as never}
      playerEditor={playerEditor as never}
      youthEditor={youthEditor as never}
      staffEditor={stub as never}
      compEditor={stub as never}
      confederations={[]}
      teams={[]}
      editingPoolKey=""
      editingPool={{ first_names: [], last_names: [] }}
      isNewPool={false}
      namesPoolRevision={0}
      poolKeys={[]}
      onSavePool={() => {}}
      onBack={() => {}}
    />,
  );
  return { playerEditor, youthEditor };
}

describe("WorldEditorFormPanel player/youth wiring", () => {
  it("binds the first-team panel to playerEditor", () => {
    const { playerEditor, youthEditor } = renderPanel("player", "first-team-p", "youth-p");
    expect(screen.getByTestId("bound-id").textContent).toBe("first-team-p");
    fireEvent.click(screen.getByText("save"));
    expect(playerEditor.handleSave).toHaveBeenCalledTimes(1);
    expect(youthEditor.handleSave).not.toHaveBeenCalled();
  });

  it("binds the youth panel to youthEditor", () => {
    const { playerEditor, youthEditor } = renderPanel("youth", "first-team-p", "youth-p");
    expect(screen.getByTestId("bound-id").textContent).toBe("youth-p");
    fireEvent.click(screen.getByText("save"));
    expect(youthEditor.handleSave).toHaveBeenCalledTimes(1);
    expect(playerEditor.handleSave).not.toHaveBeenCalled();
  });
});
