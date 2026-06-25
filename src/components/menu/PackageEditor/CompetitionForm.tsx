import { useState } from "react";
import { useTranslation } from "react-i18next";
import { LabeledInput, LabeledSelect } from "./primitives";
import { EntityFormShell } from "./shared";
import {
  COMPETITION_FORMATS,
  COMPETITION_SCOPES,
  COMPETITION_TYPES,
  SELECTOR_KINDS,
  buildParticipantSpec,
  poolToText,
  parsePoolText,
} from "./helpers";
import type { CompetitionDef, SelectorKind, SelectorSpec } from "./types";

interface CompetitionFormProps {
  editing: CompetitionDef;
  editingIndex: number | null;
  isBusy: boolean;
  onBack: () => void;
  onSave: () => void;
  updateField: <K extends keyof CompetitionDef>(key: K, value: CompetitionDef[K]) => void;
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
  onBack,
  onSave,
  updateField,
}: CompetitionFormProps) {
  const { t } = useTranslation();

  const [participantMode, setParticipantMode] = useState<"explicit" | "selector">(
    detectParticipantMode(editing),
  );
  const [explicitText, setExplicitText] = useState(
    poolToText(editing.participants.explicit ?? []),
  );
  const [selector, setSelector] = useState<SelectorSpec>(selectorFromComp(editing));

  const labelClass =
    "text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400";
  const inputClass =
    "w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition";
  const textareaClass =
    "w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm font-mono text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition resize-none";

  function switchMode(mode: "explicit" | "selector") {
    setParticipantMode(mode);
    updateField("participants", buildParticipantSpec(mode, explicitText, selector));
  }

  function onExplicitChange(text: string) {
    setExplicitText(text);
    if (participantMode === "explicit") {
      updateField("participants", { explicit: parsePoolText(text) });
    }
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

  return (
    <EntityFormShell
      title={editingIndex === null ? t("packageEditor.addCompetition") : t("packageEditor.editCompetition")}
      onBack={onBack}
      onSave={onSave}
      isBusy={isBusy}
      saveDisabled={!editing.id || !editing.name}
      saveLabel={t("packageEditor.saveCompetition")}
    >
      <LabeledInput
        label={t("packageEditor.competitionId")}
        value={editing.id}
        onChange={(v) => updateField("id", v)}
        placeholder="premier-league"
      />
      <LabeledInput
        label={t("packageEditor.competitionName")}
        value={editing.name}
        onChange={(v) => updateField("name", v)}
        placeholder="Premier League"
      />

      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("packageEditor.competitionType")}</label>
        <select
          value={editing.type}
          onChange={(e) => updateField("type", e.target.value as CompetitionDef["type"])}
          className={inputClass}
        >
          {COMPETITION_TYPES.map((ct) => (
            <option key={ct} value={ct}>
              {t(`teamSelect.kinds.${ct}`)}
            </option>
          ))}
        </select>
      </div>

      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("packageEditor.competitionScope")}</label>
        <select
          value={editing.scope}
          onChange={(e) => updateField("scope", e.target.value as CompetitionDef["scope"])}
          className={inputClass}
        >
          {COMPETITION_SCOPES.map((cs) => (
            <option key={cs} value={cs}>
              {t(`teamSelect.scopes.${cs}`)}
            </option>
          ))}
        </select>
      </div>

      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("packageEditor.competitionFormat")}</label>
        <select
          value={editing.format.kind}
          onChange={(e) =>
            updateField("format", {
              ...editing.format,
              kind: e.target.value as CompetitionDef["format"]["kind"],
            })
          }
          className={inputClass}
        >
          {COMPETITION_FORMATS.map((cf) => (
            <option key={cf} value={cf}>
              {t(`packageEditor.competitionFormats.${cf}`)}
            </option>
          ))}
        </select>
      </div>

      <LabeledInput
        label={t("packageEditor.competitionPriority")}
        value={editing.priority.toString()}
        type="number"
        onChange={(v) => updateField("priority", parseInt(v, 10) || 0)}
      />

      <LabeledInput
        label={t("packageEditor.competitionCountryId")}
        value={editing.countryId ?? ""}
        onChange={(v) => updateField("countryId", v || undefined)}
        placeholder="ENG"
      />

      <LabeledInput
        label={t("packageEditor.competitionRegionId")}
        value={editing.regionId ?? ""}
        onChange={(v) => updateField("regionId", v || undefined)}
        placeholder="europe"
      />

      <div className="grid grid-cols-2 gap-3">
        <LabeledInput
          label={t("packageEditor.competitionSeasonMonth")}
          value={editing.seasonStartMonth?.toString() ?? ""}
          type="number"
          onChange={(v) => updateField("seasonStartMonth", v ? parseInt(v, 10) : undefined)}
        />
        <LabeledInput
          label={t("packageEditor.competitionSeasonDay")}
          value={editing.seasonStartDay?.toString() ?? ""}
          type="number"
          onChange={(v) => updateField("seasonStartDay", v ? parseInt(v, 10) : undefined)}
        />
      </div>

      {/* Participant mode toggle */}
      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("packageEditor.competitionParticipantsMode")}</label>
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
                ? t("packageEditor.competitionExplicit")
                : t("packageEditor.competitionSelector")}
            </button>
          ))}
        </div>
      </div>

      {participantMode === "explicit" && (
        <div className="flex flex-col gap-1">
          <label className={labelClass}>{t("packageEditor.competitionExplicitTeams")}</label>
          <textarea
            rows={5}
            value={explicitText}
            onChange={(e) => onExplicitChange(e.target.value)}
            className={textareaClass}
            placeholder={"team-a\nteam-b\nteam-c"}
          />
        </div>
      )}

      {participantMode === "selector" && (
        <div className="flex flex-col gap-3">
          <LabeledSelect
            label={t("packageEditor.competitionSelectorKind")}
            value={selector.kind}
            options={SELECTOR_KINDS}
            onChange={(v) => updateSelector({ kind: v as SelectorKind })}
          />
          {selectorNeedsCountry && (
            <LabeledInput
              label={t("packageEditor.competitionSelectorCountry")}
              value={selector.country ?? ""}
              onChange={(v) => updateSelector({ country: v || undefined })}
              placeholder="ENG"
            />
          )}
          {selectorNeedsRegion && (
            <LabeledInput
              label={t("packageEditor.competitionSelectorRegion")}
              value={selector.region ?? ""}
              onChange={(v) => updateSelector({ region: v || undefined })}
              placeholder="europe"
            />
          )}
          {(selectorNeedsCount || selectorNeedsCountry || selectorNeedsRegion) && (
            <LabeledInput
              label={t("packageEditor.competitionSelectorCount")}
              value={selector.count?.toString() ?? ""}
              type="number"
              onChange={(v) =>
                updateSelector({ count: v ? parseInt(v, 10) : undefined })
              }
            />
          )}
          {selectorNeedsSource && (
            <LabeledInput
              label={t("packageEditor.competitionSelectorSource")}
              value={selector.sourceCompetition ?? ""}
              onChange={(v) => updateSelector({ sourceCompetition: v || undefined })}
              placeholder="premier-league"
            />
          )}
        </div>
      )}
    </EntityFormShell>
  );
}
