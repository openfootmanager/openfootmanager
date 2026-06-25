import { useTranslation } from "react-i18next";
import { ArrowLeft, CheckCircle, Loader2 } from "lucide-react";
import { LabeledInput, LabeledSelect } from "./primitives";
import { PLAY_STYLES, makeRange, parseRangeBound } from "./helpers";
import type { TeamDef } from "./types";

interface TeamFormProps {
  editingTeam: TeamDef;
  editingTeamIndex: number | null;
  isBusy: boolean;
  onBack: () => void;
  onSave: () => void;
  updateField: <K extends keyof TeamDef>(key: K, value: TeamDef[K]) => void;
}

export function TeamForm({ editingTeam, editingTeamIndex, isBusy, onBack, onSave, updateField }: TeamFormProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2 mb-2">
        <button
          onClick={onBack}
          className="text-gray-400 hover:text-gray-700 dark:hover:text-white transition-colors p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-navy-600"
        >
          <ArrowLeft className="w-5 h-5" />
        </button>
        <h2 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
          {editingTeamIndex === null
            ? t("packageEditor.addTeam")
            : t("packageEditor.editTeam")}
        </h2>
      </div>

      <div className="flex flex-col gap-3">
        <LabeledInput
          label={t("packageEditor.teamName")}
          value={editingTeam.name}
          onChange={(v) => updateField("name", v)}
        />
        <LabeledInput
          label={t("packageEditor.teamShortName")}
          value={editingTeam.shortName}
          onChange={(v) => updateField("shortName", v)}
        />
        <LabeledInput
          label={t("packageEditor.teamCity")}
          value={editingTeam.city}
          onChange={(v) => updateField("city", v)}
        />
        <LabeledInput
          label={t("packageEditor.teamCountry")}
          value={editingTeam.country}
          onChange={(v) => updateField("country", v)}
          placeholder="ENG"
        />
        <LabeledSelect
          label={t("packageEditor.teamPlayStyle")}
          value={editingTeam.playStyle}
          options={PLAY_STYLES}
          onChange={(v) => updateField("playStyle", v)}
        />
        <LabeledInput
          label={t("packageEditor.teamStadium")}
          value={editingTeam.stadiumName}
          onChange={(v) => updateField("stadiumName", v)}
        />

        <div className="flex gap-3">
          <div className="flex flex-col gap-1 flex-1">
            <label className="text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
              {t("packageEditor.teamPrimaryColor")}
            </label>
            <div className="flex items-center gap-2">
              <div
                className="w-7 h-7 rounded border border-gray-200 dark:border-navy-600 flex-shrink-0"
                style={{ background: editingTeam.colors.primary }}
              />
              <input
                type="text"
                value={editingTeam.colors.primary}
                onChange={(e) =>
                  updateField("colors", { ...editingTeam.colors, primary: e.target.value })
                }
                className="flex-1 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition"
                placeholder="#cc0000"
              />
            </div>
          </div>
          <div className="flex flex-col gap-1 flex-1">
            <label className="text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
              {t("packageEditor.teamSecondaryColor")}
            </label>
            <div className="flex items-center gap-2">
              <div
                className="w-7 h-7 rounded border border-gray-200 dark:border-navy-600 flex-shrink-0"
                style={{ background: editingTeam.colors.secondary }}
              />
              <input
                type="text"
                value={editingTeam.colors.secondary}
                onChange={(e) =>
                  updateField("colors", { ...editingTeam.colors, secondary: e.target.value })
                }
                className="flex-1 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition"
                placeholder="#ffffff"
              />
            </div>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <LabeledInput
            label={t("packageEditor.teamRepMin")}
            value={editingTeam.reputationRange?.[0]?.toString() ?? ""}
            type="number"
            onChange={(v) =>
              updateField(
                "reputationRange",
                makeRange(parseRangeBound(v), editingTeam.reputationRange?.[1] ?? null),
              )
            }
          />
          <LabeledInput
            label={t("packageEditor.teamRepMax")}
            value={editingTeam.reputationRange?.[1]?.toString() ?? ""}
            type="number"
            onChange={(v) =>
              updateField(
                "reputationRange",
                makeRange(editingTeam.reputationRange?.[0] ?? null, parseRangeBound(v)),
              )
            }
          />
          <LabeledInput
            label={t("packageEditor.teamFinMin")}
            value={editingTeam.financeRange?.[0]?.toString() ?? ""}
            type="number"
            onChange={(v) =>
              updateField(
                "financeRange",
                makeRange(parseRangeBound(v), editingTeam.financeRange?.[1] ?? null),
              )
            }
          />
          <LabeledInput
            label={t("packageEditor.teamFinMax")}
            value={editingTeam.financeRange?.[1]?.toString() ?? ""}
            type="number"
            onChange={(v) =>
              updateField(
                "financeRange",
                makeRange(editingTeam.financeRange?.[0] ?? null, parseRangeBound(v)),
              )
            }
          />
        </div>
      </div>

      <button
        onClick={onSave}
        disabled={isBusy || !editingTeam.name || !editingTeam.city || !editingTeam.country}
        className="w-full py-3 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white rounded-xl font-heading font-bold uppercase tracking-wide transition-all disabled:opacity-60 disabled:cursor-not-allowed flex items-center justify-center gap-2"
      >
        {isBusy ? (
          <Loader2 className="w-4 h-4 animate-spin" />
        ) : (
          <CheckCircle className="w-4 h-4" />
        )}
        {t("packageEditor.saveTeam")}
      </button>
    </div>
  );
}
