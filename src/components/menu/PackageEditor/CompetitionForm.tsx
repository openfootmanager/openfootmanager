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
    <EntityFormShell
      title={editingIndex === null ? t("worldEditor.addCompetition") : t("worldEditor.editCompetition")}
      onBack={onBack}
      onSave={onSave}
      isBusy={isBusy}
      saveDisabled={!editing.id || !editing.name}
      saveLabel={t("worldEditor.saveCompetition")}
    >
      <LabeledInput
        label={t("worldEditor.competitionId")}
        value={editing.id}
        onChange={(v) => updateField("id", v)}
        placeholder="premier-league"
        help={t("worldEditor.help.competitionId")}
      />
      <LabeledInput
        label={t("worldEditor.competitionName")}
        value={editing.name}
        onChange={(v) => updateField("name", v)}
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

      <LabeledInput
        label={t("worldEditor.competitionCountryId")}
        value={editing.countryId ?? ""}
        onChange={(v) => updateField("countryId", v || undefined)}
        placeholder="ENG"
      />

      <LabeledInput
        label={t("worldEditor.competitionRegionId")}
        value={editing.regionId ?? ""}
        onChange={(v) => updateField("regionId", v || undefined)}
        placeholder="europe"
      />

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
        <p className="text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
          {t("worldEditor.competitionParticipantsMode")}
        </p>
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
        <div className="flex flex-col gap-1">
          <label className="text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
            {t("worldEditor.competitionExplicitTeams")}
          </label>
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
            label={t("worldEditor.competitionSelectorKind")}
            value={selector.kind}
            options={SELECTOR_KINDS}
            optionLabels={selectorKindLabels}
            onChange={(v) => updateSelector({ kind: v as SelectorKind })}
            help={t("worldEditor.help.selectorKind")}
          />
          {selectorNeedsCountry && (
            <LabeledInput
              label={t("worldEditor.competitionSelectorCountry")}
              value={selector.country ?? ""}
              onChange={(v) => updateSelector({ country: v || undefined })}
              placeholder="ENG"
            />
          )}
          {selectorNeedsRegion && (
            <LabeledInput
              label={t("worldEditor.competitionSelectorRegion")}
              value={selector.region ?? ""}
              onChange={(v) => updateSelector({ region: v || undefined })}
              placeholder="europe"
            />
          )}
          {(selectorNeedsCount || selectorNeedsCountry || selectorNeedsRegion) && (
            <LabeledInput
              label={t("worldEditor.competitionSelectorCount")}
              value={selector.count?.toString() ?? ""}
              type="number"
              onChange={(v) =>
                updateSelector({ count: v ? parseInt(v, 10) : undefined })
              }
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
    </EntityFormShell>
  );
}
