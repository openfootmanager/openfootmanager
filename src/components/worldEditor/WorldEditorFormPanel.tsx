import { useTranslation } from "react-i18next";
import { MousePointerClick } from "lucide-react";
import { MetadataForm } from "../menu/PackageEditor/MetadataForm";
import { TeamForm } from "../menu/PackageEditor/TeamForm";
import { ConfederationForm } from "../menu/PackageEditor/ConfederationForm";
import { CountryForm } from "../menu/PackageEditor/CountryForm";
import { PlayerForm } from "../menu/PackageEditor/PlayerForm";
import { StaffForm } from "../menu/PackageEditor/StaffForm";
import { NamesPoolForm } from "../menu/PackageEditor/NamesPoolForm";
import { CompetitionForm } from "../menu/PackageEditor/CompetitionForm";
import { IssueList } from "../menu/PackageEditor/IssueList";
import type {
  CompetitionDef,
  ConfederationDef,
  CountryDef,
  NamePool,
  PackageProjectData,
  PlayerDef,
  StaffDef,
  TeamDef,
  WorldMetaDef,
} from "../menu/PackageEditor/types";

export type FormPanel =
  | "empty"
  | "metadata"
  | "team"
  | "confederation"
  | "country"
  | "player"
  | "youth"
  | "staff"
  | "names-pool"
  | "competition"
  | "issues";

type EditorAPI<T> = {
  editing: T;
  editingIndex: number | null;
  revision: number;
  handleSave: () => Promise<void>;
  updateField: <K extends keyof T>(key: K, value: T[K]) => void;
};

interface WorldEditorFormPanelProps {
  formPanel: FormPanel;
  isBusy: boolean;
  projectDir: string;
  // Metadata
  meta: WorldMetaDef;
  onMetaChange: (m: WorldMetaDef) => void;
  onSaveMetadata: () => void;
  counts: {
    teams: number;
    players: number;
    confederations: number;
    countries: number;
    competitions: number;
    namePools: number;
  };
  // Issues
  issues: PackageProjectData["issues"];
  // Entity editors
  teamEditor: EditorAPI<TeamDef>;
  confEditor: EditorAPI<ConfederationDef>;
  countryEditor: EditorAPI<CountryDef>;
  playerEditor: EditorAPI<PlayerDef>;
  youthEditor: EditorAPI<PlayerDef>;
  staffEditor: EditorAPI<StaffDef>;
  compEditor: EditorAPI<CompetitionDef>;
  // Cross-entity data
  confederations: ConfederationDef[];
  teams: TeamDef[];
  // Names pool
  editingPoolKey: string;
  editingPool: NamePool;
  isNewPool: boolean;
  namesPoolRevision: number;
  poolKeys: string[];
  onSavePool: (key: string, pool: NamePool) => void;
  // Navigation
  onBack: () => void;
}

export function WorldEditorFormPanel({
  formPanel,
  isBusy,
  projectDir,
  meta,
  onMetaChange,
  onSaveMetadata,
  counts,
  issues,
  teamEditor,
  confEditor,
  countryEditor,
  playerEditor,
  youthEditor,
  staffEditor,
  compEditor,
  confederations,
  teams,
  editingPoolKey,
  editingPool,
  isNewPool,
  namesPoolRevision,
  poolKeys,
  onSavePool,
  onBack,
}: WorldEditorFormPanelProps) {
  const { t } = useTranslation();

  if (formPanel === "metadata") {
    return (
      <div className="max-w-4xl">
        <h2 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white mb-5">
          {t("worldEditor.metadata")}
        </h2>
        <MetadataForm
          meta={meta}
          onChange={(m) => onMetaChange(m)}
          counts={counts}
          projectDir={projectDir || undefined}
        />
        <button
          type="button"
          onClick={onSaveMetadata}
          disabled={isBusy}
          className="mt-6 px-5 py-2.5 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white rounded-xl font-heading font-bold uppercase tracking-wide text-sm transition-all disabled:opacity-60"
        >
          {t("common.save")}
        </button>
      </div>
    );
  }

  if (formPanel === "issues") {
    return (
      <div className="max-w-2xl">
        <h2 className="text-lg font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white mb-5">
          {t("worldEditor.issuesTitle")}
        </h2>
        {issues.length === 0 ? (
          <p className="text-sm text-gray-400 dark:text-gray-500">
            {t("worldEditor.noIssues")}
          </p>
        ) : (
          <IssueList issues={issues} />
        )}
      </div>
    );
  }

  if (formPanel === "team") {
    return (
      <div className="max-w-3xl">
        <TeamForm
          key={`team-${teamEditor.revision}`}
          editingTeam={teamEditor.editing}
          editingTeamIndex={teamEditor.editingIndex}
          isBusy={isBusy}
          projectDir={projectDir || undefined}
          onBack={onBack}
          onSave={() => { void teamEditor.handleSave(); }}
          updateField={teamEditor.updateField}
        />
      </div>
    );
  }

  if (formPanel === "confederation") {
    return (
      <div className="max-w-lg">
        <ConfederationForm
          key={`conf-${confEditor.revision}`}
          editing={confEditor.editing}
          editingIndex={confEditor.editingIndex}
          isBusy={isBusy}
          onBack={onBack}
          onSave={() => { void confEditor.handleSave(); }}
          updateField={confEditor.updateField}
        />
      </div>
    );
  }

  if (formPanel === "country") {
    return (
      <div className="max-w-lg">
        <CountryForm
          key={`country-${countryEditor.revision}`}
          editing={countryEditor.editing}
          editingIndex={countryEditor.editingIndex}
          confederations={confederations}
          isBusy={isBusy}
          onBack={onBack}
          onSave={() => { void countryEditor.handleSave(); }}
          updateField={countryEditor.updateField}
        />
      </div>
    );
  }

  if (formPanel === "player" || formPanel === "youth") {
    // Youth and first-team players share PlayerForm but are driven by separate
    // editor instances (youthEditor defaults youth:true on add). Bind the form
    // to whichever editor opened it so youth edits/saves don't hit the wrong one.
    const editor = formPanel === "youth" ? youthEditor : playerEditor;
    return (
      <div className="max-w-4xl">
        <PlayerForm
          key={`${formPanel}-${editor.revision}`}
          editing={editor.editing}
          editingIndex={editor.editingIndex}
          isBusy={isBusy}
          teams={teams}
          projectDir={projectDir || undefined}
          onBack={onBack}
          onSave={() => { void editor.handleSave(); }}
          updateField={editor.updateField}
        />
      </div>
    );
  }

  if (formPanel === "staff") {
    return (
      <div className="max-w-2xl">
        <StaffForm
          key={`staff-${staffEditor.revision}`}
          editing={staffEditor.editing}
          editingIndex={staffEditor.editingIndex}
          isBusy={isBusy}
          teams={teams}
          onBack={onBack}
          onSave={() => { void staffEditor.handleSave(); }}
          updateField={staffEditor.updateField}
        />
      </div>
    );
  }

  if (formPanel === "names-pool") {
    return (
      <div className="max-w-lg">
        <NamesPoolForm
          key={`names-pool-${namesPoolRevision}`}
          poolKey={editingPoolKey}
          pool={editingPool}
          isNew={isNewPool}
          isBusy={isBusy}
          takenKeys={poolKeys}
          onBack={onBack}
          onSave={(key, pool) => { onSavePool(key, pool); }}
        />
      </div>
    );
  }

  if (formPanel === "competition") {
    return (
      <div className="max-w-3xl">
        <CompetitionForm
          key={`competition-${compEditor.revision}`}
          editing={compEditor.editing}
          editingIndex={compEditor.editingIndex}
          isBusy={isBusy}
          teams={teams}
          confederations={confederations}
          projectDir={projectDir || undefined}
          onBack={onBack}
          onSave={() => { void compEditor.handleSave(); }}
          updateField={compEditor.updateField}
        />
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center justify-center h-full text-center gap-3">
      <MousePointerClick className="w-10 h-10 text-gray-300 dark:text-navy-600" />
      <p className="text-sm text-gray-400 dark:text-gray-500">
        {t("worldEditor.noItemSelected")}
      </p>
    </div>
  );
}
