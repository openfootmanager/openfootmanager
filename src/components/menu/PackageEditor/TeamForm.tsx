import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, CheckCircle, ImagePlus, Loader2, X } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { LabeledInput, LabeledSelect, labelClass } from "./primitives";
import { CountryCombobox } from "../../ui/CountryCombobox";
import { PLAY_STYLES, makeRange, parseRangeBound, toSlug } from "./helpers";
import type { TeamDef } from "./types";
import { TeamPreviewCard } from "./TeamPreviewCard";

interface TeamFormProps {
  editingTeam: TeamDef;
  editingTeamIndex: number | null;
  isBusy: boolean;
  projectDir?: string;
  onBack: () => void;
  onSave: () => void;
  updateField: <K extends keyof TeamDef>(key: K, value: TeamDef[K]) => void;
}

export function TeamForm({ editingTeam, editingTeamIndex, isBusy, projectDir, onBack, onSave, updateField }: TeamFormProps) {
  const { t } = useTranslation();
  const [idAutoMode, setIdAutoMode] = useState(editingTeamIndex === null && !editingTeam.id);
  const [logoDataUrl, setLogoDataUrl] = useState<string | null>(null);

  useEffect(() => {
    if (!editingTeam.logo || !projectDir) { setLogoDataUrl(null); return; }
    let cancelled = false;
    invoke<string>("read_file_as_data_url", { path: `${projectDir}/${editingTeam.logo}`, baseDir: projectDir })
      .then((url) => { if (!cancelled) setLogoDataUrl(url); })
      .catch(() => { if (!cancelled) setLogoDataUrl(null); });
    return () => { cancelled = true; };
  }, [editingTeam.logo, projectDir]);

  async function handlePickLogo() {
    if (!projectDir) return;
    const selected = await open({
      multiple: false,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    try {
      const relPath = await invoke<string>("copy_package_asset", {
        dir: projectDir,
        entityId: editingTeam.id || `unnamed-team-${Date.now()}`,
        srcPath: selected,
      });
      updateField("logo", relPath);
    } catch { /* ignore */ }
  }

  function handleNameChange(v: string) {
    updateField("name", v);
    if (idAutoMode) updateField("id", toSlug(v));
  }

  return (
    <div className="flex gap-6 items-start">
    <div className="flex-1 min-w-0 flex flex-col gap-4">
      <div className="flex items-center gap-2 mb-2">
        <button
          onClick={onBack}
          className="text-gray-400 hover:text-gray-700 dark:hover:text-white transition-colors p-1 rounded-lg hover:bg-gray-100 dark:hover:bg-navy-600"
        >
          <ArrowLeft className="w-5 h-5" />
        </button>
        <h2 className="text-xl font-heading font-bold uppercase tracking-wide text-gray-900 dark:text-white">
          {editingTeamIndex === null
            ? t("worldEditor.addTeam")
            : t("worldEditor.editTeam")}
        </h2>
      </div>

      <div className="flex flex-col gap-3">
        <LabeledInput
          label={t("worldEditor.teamId")}
          value={editingTeam.id}
          onChange={(v) => {
            setIdAutoMode(false);
            updateField("id", v);
          }}
          placeholder="man-utd"
          help={t("worldEditor.help.teamId")}
        />
        <LabeledInput
          label={t("worldEditor.teamName")}
          value={editingTeam.name}
          onChange={handleNameChange}
        />
        <LabeledInput
          label={t("worldEditor.teamShortName")}
          value={editingTeam.shortName}
          onChange={(v) => updateField("shortName", v)}
        />
        <LabeledInput
          label={t("worldEditor.teamCity")}
          value={editingTeam.city}
          onChange={(v) => updateField("city", v)}
        />
        <CountryCombobox
          label={t("worldEditor.teamCountry")}
          value={editingTeam.country}
          onChange={(v) => updateField("country", v)}
        />
        <LabeledSelect
          label={t("worldEditor.teamPlayStyle")}
          value={editingTeam.playStyle}
          options={PLAY_STYLES}
          onChange={(v) => updateField("playStyle", v)}
        />
        <LabeledInput
          label={t("worldEditor.teamStadium")}
          value={editingTeam.stadiumName}
          onChange={(v) => updateField("stadiumName", v)}
        />

        <div className="flex gap-3">
          <div className="flex flex-col gap-1 flex-1">
            <label className="text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
              {t("worldEditor.teamPrimaryColor")}
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
              {t("worldEditor.teamSecondaryColor")}
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

        <div className="flex flex-col gap-2">
          <div className="grid grid-cols-2 gap-3">
            <LabeledInput
              label={t("worldEditor.teamRepMin")}
              value={editingTeam.reputationRange?.[0]?.toString() ?? ""}
              type="number"
              help={t("worldEditor.help.teamReputationRange")}
              onChange={(v) =>
                updateField(
                  "reputationRange",
                  makeRange(parseRangeBound(v), editingTeam.reputationRange?.[1] ?? null),
                )
              }
            />
            <LabeledInput
              label={t("worldEditor.teamRepMax")}
              value={editingTeam.reputationRange?.[1]?.toString() ?? ""}
              type="number"
              onChange={(v) =>
                updateField(
                  "reputationRange",
                  makeRange(editingTeam.reputationRange?.[0] ?? null, parseRangeBound(v)),
                )
              }
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <LabeledInput
              label={t("worldEditor.teamFinMin")}
              value={editingTeam.financeRange?.[0]?.toString() ?? ""}
              type="number"
              help={t("worldEditor.help.teamFinanceRange")}
              onChange={(v) =>
                updateField(
                  "financeRange",
                  makeRange(parseRangeBound(v), editingTeam.financeRange?.[1] ?? null),
                )
              }
            />
            <LabeledInput
              label={t("worldEditor.teamFinMax")}
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
      </div>

      {projectDir && (
        <div className="flex flex-col gap-1">
          <label className={labelClass}>{t("worldEditor.teamLogo")}</label>
          <div className="flex items-center gap-3">
            {logoDataUrl ? (
              <img src={logoDataUrl} alt="" className="w-12 h-12 rounded-lg object-contain border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 flex-shrink-0" />
            ) : (
              <div className="w-12 h-12 rounded-lg border border-dashed border-gray-300 dark:border-navy-600 bg-gray-50 dark:bg-navy-700 flex items-center justify-center flex-shrink-0">
                <ImagePlus className="w-5 h-5 text-gray-300 dark:text-navy-500" />
              </div>
            )}
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => { void handlePickLogo(); }}
                className="px-3 py-1.5 text-xs font-heading font-bold uppercase tracking-wide rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-navy-600 transition"
              >
                {t("worldEditor.chooseLogo")}
              </button>
              {editingTeam.logo && (
                <button
                  type="button"
                  onClick={() => { updateField("logo", null); setLogoDataUrl(null); }}
                  className="px-2 py-1.5 text-xs rounded-lg border border-gray-200 dark:border-navy-600 text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              )}
            </div>
          </div>
        </div>
      )}

      <button
        onClick={onSave}
        disabled={isBusy || !editingTeam.id || !editingTeam.name || !editingTeam.city || !editingTeam.country}
        className="w-full py-3 bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-600 hover:to-primary-700 text-white rounded-xl font-heading font-bold uppercase tracking-wide transition-all disabled:opacity-60 disabled:cursor-not-allowed flex items-center justify-center gap-2"
      >
        {isBusy ? (
          <Loader2 className="w-4 h-4 animate-spin" />
        ) : (
          <CheckCircle className="w-4 h-4" />
        )}
        {t("worldEditor.saveTeam")}
      </button>
    </div>
    <div className="w-52 flex-shrink-0 sticky top-0">
      <TeamPreviewCard team={editingTeam} logoDataUrl={logoDataUrl} />
    </div>
    </div>
  );
}
