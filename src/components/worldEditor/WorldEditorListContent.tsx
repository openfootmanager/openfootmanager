import { TeamsTab } from "../menu/PackageEditor/TeamsTab";
import { PlayersTab } from "../menu/PackageEditor/PlayersTab";
import { ConfederationsTab } from "../menu/PackageEditor/ConfederationsTab";
import { CountriesTab } from "../menu/PackageEditor/CountriesTab";
import { NamesTab } from "../menu/PackageEditor/NamesTab";
import { CompetitionsTab } from "../menu/PackageEditor/CompetitionsTab";
import { EntityListPanel } from "./EntityListPanel";
import type {
  CompetitionDef,
  ConfederationDef,
  CountryDef,
  EditTab,
  NamesDefinition,
  PlayerDef,
  TeamDef,
} from "../menu/PackageEditor/types";
import type { FormPanel } from "./WorldEditorFormPanel";

type ListEditorAPI = {
  editingIndex: number | null;
  handleAdd: () => void;
  handleSelect: (i: number) => void;
  handleDelete: (i: number) => void;
};

interface WorldEditorListContentProps {
  selectedSection: EditTab;
  formPanel: FormPanel;
  teams: TeamDef[];
  players: PlayerDef[];
  confederations: ConfederationDef[];
  countries: CountryDef[];
  competitions: CompetitionDef[];
  names: NamesDefinition;
  projectDir?: string;
  teamEditor: ListEditorAPI;
  playerEditor: ListEditorAPI;
  confEditor: ListEditorAPI;
  countryEditor: ListEditorAPI;
  compEditor: ListEditorAPI;
  namesEditor: {
    editingPoolKey: string;
    handleAddPool: () => void;
    handleSelectPool: (key: string) => void;
    handleDeletePool: (key: string) => void;
  };
}

export function WorldEditorListContent({
  selectedSection,
  formPanel,
  teams,
  players,
  confederations,
  countries,
  competitions,
  names,
  projectDir,
  teamEditor,
  playerEditor,
  confEditor,
  countryEditor,
  compEditor,
  namesEditor,
}: WorldEditorListContentProps) {
  return (
    <EntityListPanel>
      {selectedSection === "teams" && (
        <TeamsTab
          teams={teams}
          projectDir={projectDir}
          onAdd={teamEditor.handleAdd}
          onEdit={teamEditor.handleSelect}
          onDelete={teamEditor.handleDelete}
          selectedIndex={formPanel === "team" ? teamEditor.editingIndex : null}
          onSelect={teamEditor.handleSelect}
        />
      )}
      {selectedSection === "players" && (
        <PlayersTab
          players={players}
          teams={teams}
          onAdd={playerEditor.handleAdd}
          onEdit={playerEditor.handleSelect}
          onDelete={playerEditor.handleDelete}
          selectedIndex={formPanel === "player" ? playerEditor.editingIndex : null}
          onSelect={playerEditor.handleSelect}
          projectDir={projectDir}
        />
      )}
      {selectedSection === "confederations" && (
        <ConfederationsTab
          confederations={confederations}
          onAdd={confEditor.handleAdd}
          onEdit={confEditor.handleSelect}
          onDelete={confEditor.handleDelete}
          selectedIndex={formPanel === "confederation" ? confEditor.editingIndex : null}
          onSelect={confEditor.handleSelect}
        />
      )}
      {selectedSection === "countries" && (
        <CountriesTab
          countries={countries}
          onAdd={countryEditor.handleAdd}
          onEdit={countryEditor.handleSelect}
          onDelete={countryEditor.handleDelete}
          selectedIndex={formPanel === "country" ? countryEditor.editingIndex : null}
          onSelect={countryEditor.handleSelect}
        />
      )}
      {selectedSection === "names" && (
        <NamesTab
          names={names}
          onAdd={namesEditor.handleAddPool}
          onEdit={namesEditor.handleSelectPool}
          onDelete={namesEditor.handleDeletePool}
          selectedKey={formPanel === "names-pool" ? namesEditor.editingPoolKey : null}
          onSelect={namesEditor.handleSelectPool}
        />
      )}
      {selectedSection === "competitions" && (
        <CompetitionsTab
          competitions={competitions}
          projectDir={projectDir}
          onAdd={compEditor.handleAdd}
          onEdit={compEditor.handleSelect}
          onDelete={compEditor.handleDelete}
          selectedIndex={formPanel === "competition" ? compEditor.editingIndex : null}
          onSelect={compEditor.handleSelect}
        />
      )}
    </EntityListPanel>
  );
}
