import { useTranslation } from "react-i18next";
import { ArrowLeft, CheckCircle, Package, Loader2 } from "lucide-react";
import { IssueList } from "./IssueList";
import { MetadataForm } from "./MetadataForm";
import { TeamsTab } from "./TeamsTab";
import { ConfederationsTab } from "./ConfederationsTab";
import { CountriesTab } from "./CountriesTab";
import { PlayersTab } from "./PlayersTab";
import { NamesTab } from "./NamesTab";
import { CompetitionsTab } from "./CompetitionsTab";
import type {
  CompetitionDef,
  ConfederationDef,
  CountryDef,
  EditTab,
  NamesDefinition,
  PackageIssue,
  PlayerDef,
  TeamDef,
  WorldMetaDef,
} from "./types";

interface EditViewProps {
  tab: EditTab;
  meta: WorldMetaDef;
  confederations: ConfederationDef[];
  countries: CountryDef[];
  teams: TeamDef[];
  players: PlayerDef[];
  names: NamesDefinition;
  competitions: CompetitionDef[];
  issues: PackageIssue[];
  isBusy: boolean;
  successMsg: string | null;
  errorMsg: string | null;
  onBack: () => void;
  onTabChange: (t: EditTab) => void;
  onSave: () => void;
  onValidate: () => void;
  onBuild: () => void;
  onMetaChange: (m: WorldMetaDef) => void;
  onAddConfederation: () => void;
  onEditConfederation: (index: number) => void;
  onDeleteConfederation: (index: number) => void;
  onAddCountry: () => void;
  onEditCountry: (index: number) => void;
  onDeleteCountry: (index: number) => void;
  onAddTeam: () => void;
  onEditTeam: (index: number) => void;
  onDeleteTeam: (index: number) => void;
  onAddPlayer: () => void;
  onEditPlayer: (index: number) => void;
  onDeletePlayer: (index: number) => void;
  onAddPool: () => void;
  onEditPool: (key: string) => void;
  onDeletePool: (key: string) => void;
  onAddCompetition: () => void;
  onEditCompetition: (index: number) => void;
  onDeleteCompetition: (index: number) => void;
}

export function EditView({
  tab,
  meta,
  confederations,
  countries,
  teams,
  players,
  names,
  competitions,
  issues,
  isBusy,
  successMsg,
  errorMsg,
  onBack,
  onTabChange,
  onSave,
  onValidate,
  onBuild,
  onMetaChange,
  onAddConfederation,
  onEditConfederation,
  onDeleteConfederation,
  onAddCountry,
  onEditCountry,
  onDeleteCountry,
  onAddTeam,
  onEditTeam,
  onDeleteTeam,
  onAddPlayer,
  onEditPlayer,
  onDeletePlayer,
  onAddPool,
  onEditPool,
  onDeletePool,
  onAddCompetition,
  onEditCompetition,
  onDeleteCompetition,
}: EditViewProps) {
  const { t } = useTranslation();

  const tabClass = (active: boolean) =>
    `flex-shrink-0 px-3 py-1.5 rounded-md text-xs font-heading font-bold uppercase tracking-wider transition-all whitespace-nowrap ${
      active
        ? "bg-white dark:bg-navy-600 text-gray-900 dark:text-white shadow-sm"
        : "text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200"
    }`;

  const tabs: { key: EditTab; label: string }[] = [
    { key: "metadata", label: t("packageEditor.metadata") },
    {
      key: "confederations",
      label: t("packageEditor.confederations", { count: confederations.length }),
    },
    { key: "countries", label: t("packageEditor.countries", { count: countries.length }) },
    { key: "teams", label: t("packageEditor.teams", { count: teams.length }) },
    { key: "players", label: t("packageEditor.players", { count: players.length }) },
    { key: "names", label: t("packageEditor.names") },
    {
      key: "competitions",
      label: t("packageEditor.competitions", { count: competitions.length }),
    },
  ];

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <button
            onClick={onBack}
            className="text-gray-400 hover:text-gray-700 dark:hover:text-white transition-colors p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-navy-600"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <h2 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
            {t("packageEditor.title")}
          </h2>
        </div>
        <button
          onClick={onSave}
          disabled={isBusy}
          className="text-xs font-heading font-bold uppercase tracking-wider text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300 disabled:opacity-50 transition-colors"
        >
          {isBusy ? <Loader2 className="w-3.5 h-3.5 animate-spin inline" /> : null}{" "}
          {t("common.save")}
        </button>
      </div>

      <div className="overflow-x-auto -mx-1 px-1">
        <div className="flex gap-1 rounded-lg bg-gray-100 dark:bg-navy-700 p-1 min-w-max">
          {tabs.map(({ key, label }) => (
            <button key={key} onClick={() => onTabChange(key)} className={tabClass(tab === key)}>
              {label}
            </button>
          ))}
        </div>
      </div>

      {successMsg && (
        <div className="text-xs text-green-700 dark:text-green-400 bg-green-50 dark:bg-green-500/10 border border-green-200 dark:border-green-500/30 rounded-lg px-3 py-2 flex items-center gap-1.5">
          <CheckCircle className="w-3.5 h-3.5 flex-shrink-0" />
          {successMsg}
        </div>
      )}
      {errorMsg && (
        <div className="text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/30 rounded-lg px-3 py-2">
          {errorMsg}
        </div>
      )}

      <IssueList issues={issues} />

      {tab === "metadata" && <MetadataForm meta={meta} onChange={onMetaChange} />}
      {tab === "confederations" && (
        <ConfederationsTab
          confederations={confederations}
          onAdd={onAddConfederation}
          onEdit={onEditConfederation}
          onDelete={onDeleteConfederation}
        />
      )}
      {tab === "countries" && (
        <CountriesTab
          countries={countries}
          onAdd={onAddCountry}
          onEdit={onEditCountry}
          onDelete={onDeleteCountry}
        />
      )}
      {tab === "teams" && (
        <TeamsTab teams={teams} onAdd={onAddTeam} onEdit={onEditTeam} onDelete={onDeleteTeam} />
      )}
      {tab === "players" && (
        <PlayersTab
          players={players}
          onAdd={onAddPlayer}
          onEdit={onEditPlayer}
          onDelete={onDeletePlayer}
        />
      )}
      {tab === "names" && (
        <NamesTab names={names} onAdd={onAddPool} onEdit={onEditPool} onDelete={onDeletePool} />
      )}
      {tab === "competitions" && (
        <CompetitionsTab
          competitions={competitions}
          onAdd={onAddCompetition}
          onEdit={onEditCompetition}
          onDelete={onDeleteCompetition}
        />
      )}

      <div className="flex gap-2 pt-1">
        <button
          onClick={onValidate}
          disabled={isBusy}
          className="flex-1 py-2 rounded-xl border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-xs font-heading font-bold uppercase tracking-wider text-gray-700 dark:text-gray-200 hover:border-primary-400 dark:hover:border-primary-500 transition-all disabled:opacity-50 flex items-center justify-center gap-1.5"
        >
          {isBusy ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <CheckCircle className="w-3.5 h-3.5" />
          )}
          {t("packageEditor.validate")}
        </button>
        <button
          onClick={onBuild}
          disabled={isBusy}
          className="flex-1 py-2 rounded-xl bg-gradient-to-r from-accent-500 to-accent-600 hover:from-accent-600 hover:to-accent-700 text-white text-xs font-heading font-bold uppercase tracking-wider transition-all disabled:opacity-50 flex items-center justify-center gap-1.5"
        >
          {isBusy ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <Package className="w-3.5 h-3.5" />
          )}
          {t("packageEditor.build")}
        </button>
      </div>
    </div>
  );
}
