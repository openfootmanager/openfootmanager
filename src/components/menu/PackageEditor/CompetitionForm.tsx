import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ImagePlus, X } from "lucide-react";
import { LabeledInput, LabeledSelect, labelClass, inputClass } from "./primitives";
import { useAssetPicker } from "./useAssetPicker";
import { Select } from "../../ui/Select";
import { EntityFormShell } from "./shared";
import { CompetitionPreviewCard } from "./CompetitionPreviewCard";
import { TeamAddPicker } from "./TeamAddPicker";
import {
  COMPETITION_FORMATS,
  COMPETITION_SCOPES,
  COMPETITION_TYPES,
  SELECTOR_KINDS,
  buildParticipantSpec,
  toSlug,
} from "./helpers";
import type {
  CompetitionDef,
  ConfederationDef,
  SelectorKind,
  SelectorSpec,
  TeamDef,
} from "./types";

interface CompetitionFormProps {
  editing: CompetitionDef;
  editingIndex: number | null;
  isBusy: boolean;
  teams?: TeamDef[];
  confederations?: ConfederationDef[];
  projectDir?: string;
  onBack: () => void;
  onSave: () => void;
  updateField: <K extends keyof CompetitionDef>(key: K, value: CompetitionDef[K]) => void;
  /**
   * Writes through to the saved record instead of only the form buffer. The
   * logo pick already copied the image into the package, so the path must not
   * depend on the user remembering to press Save.
   */
  commitField?: <K extends keyof CompetitionDef>(key: K, value: CompetitionDef[K]) => void;
  onAssetError: (err: unknown) => void;
}

function emptySelector(): SelectorSpec {
  return { kind: "topByReputation", excludeCompetitions: [] };
}

function detectParticipantMode(comp: CompetitionDef): "explicit" | "selector" {
  return comp.participants.selector ? "selector" : "explicit";
}

function selectorFromComp(comp: CompetitionDef): SelectorSpec {
  return comp.participants.selector ?? emptySelector();
}

export function CompetitionForm({
  editing,
  editingIndex,
  isBusy,
  teams,
  confederations,
  projectDir,
  onBack,
  onSave,
  updateField,
  commitField,
  onAssetError,
}: CompetitionFormProps) {
  const { t } = useTranslation();
  const [idAutoMode, setIdAutoMode] = useState(editingIndex === null && !editing.id);
  const {
    dataUrl: logoDataUrl,
    pick: pickLogo,
    clear: clearLogo,
  } = useAssetPicker({
    relPath: editing.logo,
    projectDir,
    entityKind: "competition",
    entityId: () => editing.id || `unnamed-competition-${Date.now()}`,
    commit: (relPath) => (commitField ?? updateField)("logo", relPath),
    onError: onAssetError,
  });

  const [participantMode, setParticipantMode] = useState<"explicit" | "selector">(
    detectParticipantMode(editing),
  );
  const [explicitTeams, setExplicitTeams] = useState<string[]>(editing.participants.explicit ?? []);
  const [selector, setSelector] = useState<SelectorSpec>(selectorFromComp(editing));

  const teamCountries = useMemo(
    () => [...new Set(teams?.map((t) => t.country).filter(Boolean) ?? [])].sort(),
    [teams],
  );
  const teamsWithIds = teams?.filter((t) => t.id) ?? [];

  function handleNameChange(v: string) {
    updateField("name", v);
    if (idAutoMode) updateField("id", toSlug(v));
  }

  function switchMode(mode: "explicit" | "selector") {
    setParticipantMode(mode);
    updateField("participants", buildParticipantSpec(mode, explicitTeams.join("\n"), selector));
  }

  function addExplicitTeam(teamId: string) {
    if (!teamId || explicitTeams.includes(teamId)) return;
    const updated = [...explicitTeams, teamId];
    setExplicitTeams(updated);
    updateField("participants", { explicit: updated });
  }

  function removeExplicitTeam(teamId: string) {
    const updated = explicitTeams.filter((t) => t !== teamId);
    setExplicitTeams(updated);
    updateField("participants", { explicit: updated });
  }

  function updateSelector(patch: Partial<SelectorSpec>) {
    const updated = { ...selector, ...patch };
    setSelector(updated);
    if (participantMode === "selector") {
      updateField("participants", { selector: updated });
    }
  }

  const selectorNeedsCount = selector.kind === "topByReputation";
  const selectorNeedsCountry = selector.kind === "allInCountry";
  const selectorNeedsRegion = selector.kind === "allInRegion";
  const selectorNeedsSource = selector.kind === "championsOf";

  const competitionTypeLabels: Record<string, string> = {
    League: t("teamSelect.kinds.League"),
    Cup: t("teamSelect.kinds.Cup"),
    ContinentalClub: t("teamSelect.kinds.ContinentalClub"),
    InternationalClub: t("teamSelect.kinds.InternationalClub"),
    InternationalNation: t("teamSelect.kinds.InternationalNation"),
    FriendlyCup: t("teamSelect.kinds.FriendlyCup"),
  };

  const competitionScopeLabels: Record<string, string> = {
    Domestic: t("teamSelect.scopes.Domestic"),
    Regional: t("teamSelect.scopes.Regional"),
    Continental: t("teamSelect.scopes.Continental"),
    International: t("teamSelect.scopes.International"),
  };

  const competitionFormatLabels: Record<string, string> = {
    LeagueTable: t("worldEditor.competitionFormats.LeagueTable"),
    Knockout: t("worldEditor.competitionFormats.Knockout"),
    GroupAndKnockout: t("worldEditor.competitionFormats.GroupAndKnockout"),
  };

  const selectorKindLabels: Record<string, string> = {
    topByReputation: t("worldEditor.selectorKinds.topByReputation"),
    allInCountry: t("worldEditor.selectorKinds.allInCountry"),
    allInRegion: t("worldEditor.selectorKinds.allInRegion"),
    championsOf: t("worldEditor.selectorKinds.championsOf"),
  };

  return (
    <div className="flex gap-6 items-start">
      <div className="flex-1 min-w-0">
        <EntityFormShell
          title={
            editingIndex === null
              ? t("worldEditor.addCompetition")
              : t("worldEditor.editCompetition")
          }
          onBack={onBack}
          onSave={onSave}
          isBusy={isBusy}
          saveDisabled={!editing.id || !editing.name}
          saveLabel={t("worldEditor.saveCompetition")}
        >
          <LabeledInput
            label={t("worldEditor.competitionId")}
            value={editing.id}
            onChange={(v) => {
              setIdAutoMode(false);
              updateField("id", v);
            }}
            placeholder="premier-league"
            help={t("worldEditor.help.competitionId")}
          />
          <LabeledInput
            label={t("worldEditor.competitionName")}
            value={editing.name}
            onChange={handleNameChange}
            placeholder="Premier League"
          />

          <LabeledSelect
            label={t("worldEditor.competitionType")}
            value={editing.type}
            options={COMPETITION_TYPES}
            optionLabels={competitionTypeLabels}
            onChange={(v) => updateField("type", v as CompetitionDef["type"])}
            help={t("worldEditor.help.competitionType")}
          />

          <LabeledSelect
            label={t("worldEditor.competitionScope")}
            value={editing.scope}
            options={COMPETITION_SCOPES}
            optionLabels={competitionScopeLabels}
            onChange={(v) => updateField("scope", v as CompetitionDef["scope"])}
            help={t("worldEditor.help.competitionScope")}
          />

          <LabeledSelect
            label={t("worldEditor.competitionFormat")}
            value={editing.format.kind}
            options={COMPETITION_FORMATS}
            optionLabels={competitionFormatLabels}
            onChange={(v) =>
              updateField("format", {
                ...editing.format,
                kind: v as CompetitionDef["format"]["kind"],
              })
            }
            help={t("worldEditor.help.competitionFormat")}
          />

          <LabeledInput
            label={t("worldEditor.competitionPriority")}
            value={editing.priority.toString()}
            type="number"
            onChange={(v) => updateField("priority", parseInt(v, 10) || 0)}
            help={t("worldEditor.help.competitionPriority")}
          />

          {/* Country ID — show team countries if available */}
          <div className="flex flex-col gap-1">
            <label className={labelClass}>{t("worldEditor.competitionCountryId")}</label>
            {teamCountries.length > 0 ? (
              <Select
                value={editing.countryId ?? ""}
                onChange={(e) => updateField("countryId", e.target.value || undefined)}
                fullWidth
              >
                <option value="">—</option>
                {teamCountries.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </Select>
            ) : (
              <input
                type="text"
                value={editing.countryId ?? ""}
                onChange={(e) => updateField("countryId", e.target.value || undefined)}
                placeholder="ENG"
                className={inputClass}
              />
            )}
          </div>

          <div className="flex flex-col gap-1">
            <label className={labelClass}>{t("worldEditor.competitionRegionId")}</label>
            {confederations && confederations.length > 0 ? (
              <Select
                value={editing.regionId ?? ""}
                onChange={(e) => updateField("regionId", e.target.value || undefined)}
                fullWidth
              >
                <option value="">—</option>
                {confederations.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name} ({c.id})
                  </option>
                ))}
              </Select>
            ) : (
              <input
                type="text"
                value={editing.regionId ?? ""}
                onChange={(e) => updateField("regionId", e.target.value || undefined)}
                placeholder="europe"
                className={inputClass}
              />
            )}
          </div>

          <div className="grid grid-cols-2 gap-3">
            <LabeledInput
              label={t("worldEditor.competitionSeasonMonth")}
              value={editing.seasonStartMonth?.toString() ?? ""}
              type="number"
              onChange={(v) => updateField("seasonStartMonth", v ? parseInt(v, 10) : undefined)}
            />
            <LabeledInput
              label={t("worldEditor.competitionSeasonDay")}
              value={editing.seasonStartDay?.toString() ?? ""}
              type="number"
              onChange={(v) => updateField("seasonStartDay", v ? parseInt(v, 10) : undefined)}
            />
          </div>

          {/* Participant mode toggle */}
          <div className="flex flex-col gap-1">
            <p className={labelClass}>{t("worldEditor.competitionParticipantsMode")}</p>
            <div className="flex gap-2">
              {(["explicit", "selector"] as const).map((mode) => (
                <button
                  key={mode}
                  type="button"
                  onClick={() => switchMode(mode)}
                  className={`flex-1 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all border ${
                    participantMode === mode
                      ? "bg-primary-500 text-white border-primary-500"
                      : "bg-white dark:bg-navy-700 text-gray-600 dark:text-gray-300 border-gray-200 dark:border-navy-600"
                  }`}
                >
                  {mode === "explicit"
                    ? t("worldEditor.competitionExplicit")
                    : t("worldEditor.competitionSelector")}
                </button>
              ))}
            </div>
            <p className="text-[10px] text-gray-400 dark:text-gray-500 leading-relaxed mt-0.5">
              {participantMode === "explicit"
                ? t("worldEditor.help.participantsExplicit")
                : t("worldEditor.help.participantsSelector")}
            </p>
          </div>

          {participantMode === "explicit" && (
            <div className="flex flex-col gap-2">
              <label className={labelClass}>{t("worldEditor.competitionExplicitTeams")}</label>

              {/* Team cards */}
              {explicitTeams.length > 0 && (
                <div className="flex flex-col gap-1.5">
                  {explicitTeams.map((teamId) => {
                    const team = teamsWithIds.find((t) => t.id === teamId);
                    return (
                      <div
                        key={teamId}
                        className="flex items-center justify-between px-3 py-2 rounded-lg bg-gray-50 dark:bg-navy-800 border border-gray-200 dark:border-navy-600"
                      >
                        <div className="flex items-center gap-2 min-w-0">
                          <span className="text-sm font-medium text-gray-900 dark:text-white truncate">
                            {team?.name ?? teamId}
                          </span>
                          {team?.country && (
                            <span className="text-[10px] px-1.5 py-0.5 rounded bg-gray-200 dark:bg-navy-700 text-gray-600 dark:text-gray-400 flex-shrink-0">
                              {team.country}
                            </span>
                          )}
                        </div>
                        <button
                          type="button"
                          onClick={() => removeExplicitTeam(teamId)}
                          className="ml-2 flex-shrink-0 text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition-colors"
                          aria-label={`Remove ${team?.name ?? teamId}`}
                        >
                          <X className="w-4 h-4" />
                        </button>
                      </div>
                    );
                  })}
                </div>
              )}

              {/* Add team picker */}
              {teamsWithIds.length > 0 ? (
                <TeamAddPicker teams={teamsWithIds} taken={explicitTeams} onAdd={addExplicitTeam} />
              ) : (
                <p className="text-xs text-gray-400 dark:text-gray-500 italic">
                  {t("worldEditor.noClubSelected")}
                </p>
              )}
            </div>
          )}

          {participantMode === "selector" && (
            <div className="flex flex-col gap-3">
              <LabeledSelect
                label={t("worldEditor.competitionSelectorKind")}
                value={selector.kind}
                options={SELECTOR_KINDS}
                optionLabels={selectorKindLabels}
                onChange={(v) => updateSelector({ kind: v as SelectorKind })}
                help={t("worldEditor.help.selectorKind")}
              />
              {selectorNeedsCountry && (
                <div className="flex flex-col gap-1">
                  <label className={labelClass}>
                    {t("worldEditor.competitionSelectorCountry")}
                  </label>
                  {teamCountries.length > 0 ? (
                    <Select
                      value={selector.country ?? ""}
                      onChange={(e) => updateSelector({ country: e.target.value || undefined })}
                      fullWidth
                    >
                      <option value="">—</option>
                      {teamCountries.map((c) => (
                        <option key={c} value={c}>
                          {c}
                        </option>
                      ))}
                    </Select>
                  ) : (
                    <input
                      type="text"
                      value={selector.country ?? ""}
                      onChange={(e) => updateSelector({ country: e.target.value || undefined })}
                      placeholder="ENG"
                      className={inputClass}
                    />
                  )}
                </div>
              )}
              {selectorNeedsRegion && (
                <div className="flex flex-col gap-1">
                  <label className={labelClass}>{t("worldEditor.competitionSelectorRegion")}</label>
                  {confederations && confederations.length > 0 ? (
                    <Select
                      value={selector.region ?? ""}
                      onChange={(e) => updateSelector({ region: e.target.value || undefined })}
                      fullWidth
                    >
                      <option value="">—</option>
                      {confederations.map((c) => (
                        <option key={c.id} value={c.id}>
                          {c.name} ({c.id})
                        </option>
                      ))}
                    </Select>
                  ) : (
                    <input
                      type="text"
                      value={selector.region ?? ""}
                      onChange={(e) => updateSelector({ region: e.target.value || undefined })}
                      placeholder="europe"
                      className={inputClass}
                    />
                  )}
                </div>
              )}
              {(selectorNeedsCount || selectorNeedsCountry || selectorNeedsRegion) && (
                <LabeledInput
                  label={t("worldEditor.competitionSelectorCount")}
                  value={selector.count?.toString() ?? ""}
                  type="number"
                  onChange={(v) => updateSelector({ count: v ? parseInt(v, 10) : undefined })}
                />
              )}
              {selectorNeedsSource && (
                <LabeledInput
                  label={t("worldEditor.competitionSelectorSource")}
                  value={selector.sourceCompetition ?? ""}
                  onChange={(v) => updateSelector({ sourceCompetition: v || undefined })}
                  placeholder="premier-league"
                  help={t("worldEditor.help.selectorSource")}
                />
              )}
            </div>
          )}

          {projectDir && (
            <div className="flex flex-col gap-1">
              <label className={labelClass}>{t("worldEditor.competitionLogo")}</label>
              <div className="flex items-center gap-3">
                {logoDataUrl ? (
                  <img
                    src={logoDataUrl}
                    alt=""
                    className="w-12 h-12 rounded-lg object-contain border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 flex-shrink-0"
                  />
                ) : (
                  <div className="w-12 h-12 rounded-lg border border-dashed border-gray-300 dark:border-navy-600 bg-gray-50 dark:bg-navy-700 flex items-center justify-center flex-shrink-0">
                    <ImagePlus className="w-5 h-5 text-gray-300 dark:text-navy-500" />
                  </div>
                )}
                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={() => {
                      void pickLogo();
                    }}
                    className="px-3 py-1.5 text-xs font-heading font-bold uppercase tracking-wide rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-navy-600 transition"
                  >
                    {t("worldEditor.chooseLogo")}
                  </button>
                  {editing.logo && (
                    <button
                      type="button"
                      onClick={() => {
                        clearLogo();
                      }}
                      className="px-2 py-1.5 text-xs rounded-lg border border-gray-200 dark:border-navy-600 text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition"
                    >
                      <X className="w-3.5 h-3.5" />
                    </button>
                  )}
                </div>
              </div>
            </div>
          )}
        </EntityFormShell>
      </div>
      <div className="w-52 flex-shrink-0 sticky top-0">
        <CompetitionPreviewCard competition={editing} logoDataUrl={logoDataUrl} />
      </div>
    </div>
  );
}
