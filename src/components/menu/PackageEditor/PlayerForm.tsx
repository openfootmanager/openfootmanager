import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ImagePlus, X } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { LabeledInput, LabeledSelect, labelClass } from "./primitives";
import { EntityFormShell } from "./shared";
import { Checkbox } from "../../ui/Checkbox";
import { CountryCombobox } from "../../ui/CountryCombobox";
import { Select } from "../../ui/Select";
import { POSITIONS, emptyAttributes, toSlug } from "./helpers";
import type { PlayerDef, TeamDef } from "./types";

const ATTR_GROUPS = [
  { label: "Physical", keys: ["pace", "stamina", "strength", "agility"] },
  { label: "Technical", keys: ["passing", "shooting", "tackling", "dribbling", "defending"] },
  { label: "Mental", keys: ["positioning", "vision", "decisions", "composure", "aggression", "teamwork", "leadership"] },
  { label: "Goalkeeper", keys: ["handling", "reflexes", "aerial"] },
] as const;

type AttrKey = typeof ATTR_GROUPS[number]["keys"][number];

interface PlayerFormProps {
  editing: PlayerDef;
  editingIndex: number | null;
  isBusy: boolean;
  teams?: TeamDef[];
  projectDir?: string;
  onBack: () => void;
  onSave: () => void;
  updateField: <K extends keyof PlayerDef>(key: K, value: PlayerDef[K]) => void;
}

export function PlayerForm({
  editing,
  editingIndex,
  isBusy,
  teams,
  projectDir,
  onBack,
  onSave,
  updateField,
}: PlayerFormProps) {
  const { t } = useTranslation();
  const [useAttributes, setUseAttributes] = useState(editing.attributes !== null);
  const [idAutoMode, setIdAutoMode] = useState(editingIndex === null && !editing.id);
  const [photoDataUrl, setPhotoDataUrl] = useState<string | null>(null);

  useEffect(() => {
    if (!editing.photo || !projectDir) { setPhotoDataUrl(null); return; }
    let cancelled = false;
    invoke<string>("read_file_as_data_url", { path: `${projectDir}/${editing.photo}`, baseDir: projectDir })
      .then((url) => { if (!cancelled) setPhotoDataUrl(url); })
      .catch(() => { if (!cancelled) setPhotoDataUrl(null); });
    return () => { cancelled = true; };
  }, [editing.photo, projectDir]);

  async function handlePickPhoto() {
    if (!projectDir) return;
    const selected = await open({
      multiple: false,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    try {
      const relPath = await invoke<string>("copy_package_asset", {
        dir: projectDir,
        entityId: editing.id || `unnamed-player-${Date.now()}`,
        srcPath: selected,
      });
      updateField("photo", relPath);
    } catch { /* ignore */ }
  }

  const inputClass =
    "w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition";

  function toggleAttributes(checked: boolean) {
    setUseAttributes(checked);
    if (checked) {
      updateField("overall", null);
      if (!editing.attributes) {
        updateField("attributes", emptyAttributes());
      }
    } else {
      updateField("attributes", null);
    }
  }

  function updateAttr(key: AttrKey, value: number) {
    updateField("attributes", { ...(editing.attributes ?? emptyAttributes()), [key]: value });
  }

  function handleNameChange(v: string) {
    updateField("name", v);
    if (idAutoMode) updateField("id", toSlug(v));
  }

  const attrs = editing.attributes ?? emptyAttributes();
  const teamsWithIds = teams?.filter((t) => t.id) ?? [];
  const positionLabels = Object.fromEntries(POSITIONS.map((p) => [p, t(`common.positions.${p}`)])) as Record<string, string>;

  return (
    <EntityFormShell
      title={editingIndex === null ? t("worldEditor.addPlayer") : t("worldEditor.editPlayer")}
      onBack={onBack}
      onSave={onSave}
      isBusy={isBusy}
      saveDisabled={!editing.id}
      saveLabel={t("worldEditor.savePlayer")}
    >
      <LabeledInput
        label={t("worldEditor.playerId")}
        value={editing.id}
        onChange={(v) => {
          setIdAutoMode(false);
          updateField("id", v);
        }}
        placeholder="player-001"
      />
      <LabeledInput
        label={t("worldEditor.playerFirstName")}
        value={editing.firstName}
        onChange={(v) => updateField("firstName", v)}
      />
      <LabeledInput
        label={t("worldEditor.playerLastName")}
        value={editing.lastName}
        onChange={(v) => updateField("lastName", v)}
      />
      <LabeledInput
        label={t("worldEditor.playerName")}
        value={editing.name}
        onChange={handleNameChange}
        placeholder="Match display name"
      />

      {/* Club picker */}
      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("worldEditor.playerClub")}</label>
        {teamsWithIds.length > 0 ? (
          <Select
            value={editing.club}
            onChange={(e) => updateField("club", e.target.value)}
            fullWidth
          >
            <option value="">{t("worldEditor.noClubSelected")}</option>
            {teamsWithIds.map((team) => (
              <option key={team.id} value={team.id}>
                {team.name || team.id}
              </option>
            ))}
          </Select>
        ) : (
          <input
            type="text"
            value={editing.club}
            onChange={(e) => updateField("club", e.target.value)}
            placeholder="team-id"
            className={inputClass}
          />
        )}
      </div>

      <CountryCombobox
        label={t("worldEditor.playerNationality")}
        value={editing.nationality}
        onChange={(v) => updateField("nationality", v)}
      />

      <LabeledSelect
        label={t("worldEditor.playerPosition")}
        value={editing.position}
        options={POSITIONS}
        optionLabels={positionLabels}
        onChange={(v) => updateField("position", v as PlayerDef["position"])}
      />
      <LabeledInput
        label={t("worldEditor.playerDateOfBirth")}
        value={editing.dateOfBirth ?? ""}
        onChange={(v) => updateField("dateOfBirth", v || null)}
        placeholder="2000-01-01"
      />

      <div className="flex items-center gap-2 py-1">
        <Checkbox
          id="use-attributes"
          checked={useAttributes}
          onChange={(e) => toggleAttributes(e.target.checked)}
          aria-label={t("worldEditor.playerUseAttributes")}
        />
        <label htmlFor="use-attributes" className={labelClass}>
          {t("worldEditor.playerUseAttributes")}
        </label>
      </div>

      {!useAttributes && (
        <LabeledInput
          label={t("worldEditor.playerOverall")}
          value={editing.overall?.toString() ?? ""}
          type="number"
          onChange={(v) => updateField("overall", v === "" ? null : Math.min(99, Math.max(1, parseInt(v, 10) || 1)))}
        />
      )}

      {useAttributes && (
        <div className="flex flex-col gap-3">
          {ATTR_GROUPS.map(({ label, keys }) => (
            <div key={label}>
              <p className={`${labelClass} mb-1`}>{label}</p>
              <div className="grid grid-cols-2 gap-2">
                {keys.map((key) => (
                  <div key={key} className="flex flex-col gap-0.5">
                    <label className="text-[9px] font-heading uppercase tracking-wider text-gray-400 dark:text-gray-500">
                      {key}
                    </label>
                    <div className="flex items-center gap-1.5">
                      <input
                        type="range"
                        min={1}
                        max={99}
                        value={attrs[key as keyof typeof attrs]}
                        onChange={(e) => updateAttr(key as AttrKey, parseInt(e.target.value, 10))}
                        className="flex-1 accent-primary-500"
                      />
                      <span className="text-xs font-mono text-gray-600 dark:text-gray-300 w-5 text-right">
                        {attrs[key as keyof typeof attrs]}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      {projectDir && (
        <div className="flex flex-col gap-1">
          <label className={labelClass}>{t("worldEditor.playerPhoto")}</label>
          <div className="flex items-center gap-3">
            {photoDataUrl ? (
              <img src={photoDataUrl} alt="" className="w-12 h-12 rounded-full object-cover border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 flex-shrink-0" />
            ) : (
              <div className="w-12 h-12 rounded-full border border-dashed border-gray-300 dark:border-navy-600 bg-gray-50 dark:bg-navy-700 flex items-center justify-center flex-shrink-0">
                <ImagePlus className="w-5 h-5 text-gray-300 dark:text-navy-500" />
              </div>
            )}
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => { void handlePickPhoto(); }}
                className="px-3 py-1.5 text-xs font-heading font-bold uppercase tracking-wide rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-navy-600 transition"
              >
                {t("worldEditor.chooseLogo")}
              </button>
              {editing.photo && (
                <button
                  type="button"
                  onClick={() => { updateField("photo", null); setPhotoDataUrl(null); }}
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
  );
}
