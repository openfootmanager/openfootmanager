import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, CheckCircle, ImagePlus, Loader2, X } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { LabeledInput, LabeledSelect, labelClass } from "./primitives";
import { useAssetDataUrl, evictAssetDataUrl } from "../../../hooks/useAssetDataUrl";
import { CountryCombobox } from "../../ui/CountryCombobox";
import JerseyIcon from "../../ui/JerseyIcon";
import { PLAY_STYLES, makeRange, parseRangeBound, toSlug } from "./helpers";
import type { KitPattern, TeamDef } from "./types";
import { TeamPreviewCard } from "./TeamPreviewCard";

const KIT_PATTERNS: KitPattern[] = ["Solid", "Stripes", "Hoops", "HalfAndHalf", "Diagonal"];

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
  const [logoRefresh, setLogoRefresh] = useState(0);
  const logoDataUrl = useAssetDataUrl(editingTeam.logo, projectDir, logoRefresh);
  const [repMin, setRepMin] = useState<string>(editingTeam.reputationRange?.[0]?.toString() ?? "");
  const [repMax, setRepMax] = useState<string>(editingTeam.reputationRange?.[1]?.toString() ?? "");
  const [finMin, setFinMin] = useState<string>(editingTeam.financeRange?.[0]?.toString() ?? "");
  const [finMax, setFinMax] = useState<string>(editingTeam.financeRange?.[1]?.toString() ?? "");

  useEffect(() => {
    setRepMin(editingTeam.reputationRange?.[0]?.toString() ?? "");
    setRepMax(editingTeam.reputationRange?.[1]?.toString() ?? "");
    setFinMin(editingTeam.financeRange?.[0]?.toString() ?? "");
    setFinMax(editingTeam.financeRange?.[1]?.toString() ?? "");
  // Sync only when the selected entity changes, not on every field edit
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editingTeamIndex]);

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
      evictAssetDataUrl(projectDir, relPath);
      setLogoRefresh((k) => k + 1); // refresh even if the path is unchanged
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
                    onClick={() => { updateField("logo", null); }}
                    className="px-2 py-1.5 text-xs rounded-lg border border-gray-200 dark:border-navy-600 text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition"
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                )}
              </div>
            </div>
          </div>
        )}

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
              <input
                type="color"
                value={/^#[0-9a-fA-F]{6}$/.test(editingTeam.colors.primary) ? editingTeam.colors.primary : "#000000"}
                onChange={(e) =>
                  updateField("colors", { ...editingTeam.colors, primary: e.target.value })
                }
                className="w-9 h-9 rounded-lg border border-gray-200 dark:border-navy-600 cursor-pointer p-0.5 bg-white dark:bg-navy-700 flex-shrink-0"
              />
              <input
                type="text"
                value={editingTeam.colors.primary}
                onChange={(e) =>
                  updateField("colors", { ...editingTeam.colors, primary: e.target.value })
                }
                className="flex-1 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition font-mono"
                placeholder="#cc0000"
              />
            </div>
          </div>
          <div className="flex flex-col gap-1 flex-1">
            <label className="text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400">
              {t("worldEditor.teamSecondaryColor")}
            </label>
            <div className="flex items-center gap-2">
              <input
                type="color"
                value={/^#[0-9a-fA-F]{6}$/.test(editingTeam.colors.secondary) ? editingTeam.colors.secondary : "#ffffff"}
                onChange={(e) =>
                  updateField("colors", { ...editingTeam.colors, secondary: e.target.value })
                }
                className="w-9 h-9 rounded-lg border border-gray-200 dark:border-navy-600 cursor-pointer p-0.5 bg-white dark:bg-navy-700 flex-shrink-0"
              />
              <input
                type="text"
                value={editingTeam.colors.secondary}
                onChange={(e) =>
                  updateField("colors", { ...editingTeam.colors, secondary: e.target.value })
                }
                className="flex-1 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition font-mono"
                placeholder="#ffffff"
              />
            </div>
          </div>
        </div>

        {/* Kit pattern selector */}
        <div className="flex flex-col gap-1.5">
          <label className={labelClass}>{t("worldEditor.teamKitPattern")}</label>
          <div className="flex gap-2 flex-wrap">
            {KIT_PATTERNS.map((pattern) => {
              const isSelected = (editingTeam.kitPattern ?? "Solid") === pattern;
              return (
                <button
                  key={pattern}
                  type="button"
                  onClick={() => updateField("kitPattern", pattern)}
                  title={pattern}
                  className={`flex flex-col items-center gap-1 p-2 rounded-xl border transition-all ${
                    isSelected
                      ? "border-primary-400 dark:border-primary-500 bg-primary-50 dark:bg-primary-500/10"
                      : "border-gray-200 dark:border-navy-600 hover:border-gray-300 dark:hover:border-navy-500"
                  }`}
                >
                  <JerseyIcon
                    primaryColor={editingTeam.colors.primary || "#cc0000"}
                    secondaryColor={editingTeam.colors.secondary || "#ffffff"}
                    pattern={pattern}
                    size="sm"
                  />
                  <span className="text-[9px] font-heading font-bold uppercase tracking-wide text-gray-500 dark:text-gray-400">
                    {pattern === "HalfAndHalf" ? "½+½" : pattern}
                  </span>
                </button>
              );
            })}
          </div>
        </div>

        <div className="flex flex-col gap-2">
          <div className="grid grid-cols-2 gap-3">
            <LabeledInput
              label={t("worldEditor.teamRepMin")}
              value={repMin}
              type="number"
              help={t("worldEditor.help.teamReputationRange")}
              onChange={(v) => {
                setRepMin(v);
                updateField("reputationRange", makeRange(parseRangeBound(v), parseRangeBound(repMax)));
              }}
            />
            <LabeledInput
              label={t("worldEditor.teamRepMax")}
              value={repMax}
              type="number"
              onChange={(v) => {
                setRepMax(v);
                updateField("reputationRange", makeRange(parseRangeBound(repMin), parseRangeBound(v)));
              }}
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <LabeledInput
              label={t("worldEditor.teamFinMin")}
              value={finMin}
              type="number"
              help={t("worldEditor.help.teamFinanceRange")}
              onChange={(v) => {
                setFinMin(v);
                updateField("financeRange", makeRange(parseRangeBound(v), parseRangeBound(finMax)));
              }}
            />
            <LabeledInput
              label={t("worldEditor.teamFinMax")}
              value={finMax}
              type="number"
              onChange={(v) => {
                setFinMax(v);
                updateField("financeRange", makeRange(parseRangeBound(finMin), parseRangeBound(v)));
              }}
            />
          </div>
        </div>
      </div>

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
