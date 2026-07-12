import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ImagePlus, Plus, X } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useAssetDataUrl, evictAssetDataUrl } from "../../../hooks/useAssetDataUrl";
import { LabeledInput, LabeledSelect, labelClass } from "./primitives";
import { EntityFormShell } from "./shared";
import { DatePicker } from "../../ui/DatePicker";
import { Checkbox } from "../../ui/Checkbox";
import { CountryCombobox } from "../../ui/CountryCombobox";
import { TeamCombobox } from "../../ui/TeamCombobox";
import { Select } from "../../ui/Select";
import { POSITIONS, PLAYER_ATTR_GROUPS, emptyAttributes, toSlug, type PlayerAttrKey } from "./helpers";
import type { CareerEntryDef, Footedness, PlayerDef, Position, TeamDef } from "./types";

const FOOT_OPTIONS: Footedness[] = ["Right", "Left", "Both"];
import { PlayerPreviewCard } from "./PlayerPreviewCard";

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
  const [photoRefresh, setPhotoRefresh] = useState(0);
  const photoDataUrl = useAssetDataUrl(editing.photo, projectDir, photoRefresh);

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
      // The path is reused for an entity, so drop any cached data URL before
      // pointing at the freshly written file.
      evictAssetDataUrl(projectDir, relPath);
      setPhotoRefresh((k) => k + 1); // refresh even if the path is unchanged
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

  function updateAttr(key: PlayerAttrKey, value: number) {
    updateField("attributes", { ...(editing.attributes ?? emptyAttributes()), [key]: value });
  }

  function handleNameChange(v: string) {
    updateField("name", v);
    if (idAutoMode) updateField("id", toSlug(v));
  }

  const attrs = editing.attributes ?? emptyAttributes();
  const teamsWithIds = teams?.filter((t) => t.id) ?? [];
  const positionLabels = Object.fromEntries(POSITIONS.map((p) => [p, t(`common.positions.${p}`)])) as Record<string, string>;

  const altPositions = editing.alternatePositions ?? [];
  const career = editing.career ?? [];

  // Optional integer field: empty clears it (engine generates/infers on import).
  function parseOptInt(raw: string, min: number, max: number): number | null {
    if (raw === "") return null;
    const n = parseInt(raw, 10);
    return Number.isNaN(n) ? null : Math.min(max, Math.max(min, n));
  }

  function toggleAltPosition(pos: Position) {
    updateField(
      "alternatePositions",
      altPositions.includes(pos)
        ? altPositions.filter((p) => p !== pos)
        : [...altPositions, pos],
    );
  }

  function addCareerRow() {
    updateField("career", [
      ...career,
      { season: new Date().getFullYear(), teamId: "", teamName: "", appearances: 0, goals: 0, assists: 0 },
    ]);
  }
  function updateCareerRow(index: number, patch: Partial<CareerEntryDef>) {
    updateField("career", career.map((row, i) => (i === index ? { ...row, ...patch } : row)));
  }
  function removeCareerRow(index: number) {
    updateField("career", career.filter((_, i) => i !== index));
  }

  return (
    <div className="flex gap-6 items-start">
    <div className="flex-1 min-w-0">
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
        placeholder={t("worldEditor.playerDisplayNamePlaceholder")}
      />

      {/* Photo */}
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
                  onClick={() => { updateField("photo", null); }}
                  className="px-2 py-1.5 text-xs rounded-lg border border-gray-200 dark:border-navy-600 text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              )}
            </div>
          </div>
        </div>
      )}

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

      <div className="grid grid-cols-2 gap-3">
        <LabeledSelect
          label={t("worldEditor.playerPosition")}
          value={editing.position}
          options={POSITIONS}
          optionLabels={positionLabels}
          onChange={(v) => updateField("position", v as PlayerDef["position"])}
        />
        <LabeledSelect
          label={t("worldEditor.playerFoot")}
          value={editing.footedness ?? "Right"}
          options={FOOT_OPTIONS}
          optionLabels={{
            Right: t("common.footedness.Right"),
            Left: t("common.footedness.Left"),
            Both: t("common.footedness.Both"),
          }}
          onChange={(v) => updateField("footedness", v as Footedness)}
        />
      </div>
      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("worldEditor.playerDateOfBirth")}</label>
        <DatePicker
          value={editing.dateOfBirth ?? ""}
          onChange={(v) => updateField("dateOfBirth", v || null)}
        />
      </div>

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
          {PLAYER_ATTR_GROUPS.map(({ groupKey, keys }) => (
            <div key={groupKey}>
              <p className={`${labelClass} mb-1`}>{t(`common.attrGroups.${groupKey}`)}</p>
              <div className="grid grid-cols-2 gap-2">
                {keys.map((key) => (
                  <div key={key} className="flex flex-col gap-0.5">
                    <label className="text-[10px] font-heading uppercase tracking-wider text-gray-400 dark:text-gray-500">
                      {t(`common.attributes.${key}`)}
                    </label>
                    <div className="flex items-center gap-1.5">
                      <input
                        type="range"
                        min={1}
                        max={99}
                        value={attrs[key as keyof typeof attrs]}
                        onChange={(e) => updateAttr(key as PlayerAttrKey, parseInt(e.target.value, 10))}
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

      {/* Contract & status — all optional; blank means the engine fills it in. */}
      <div className="grid grid-cols-2 gap-3">
        <div className="flex flex-col gap-1">
          <label className={labelClass}>{t("common.contract")}</label>
          <DatePicker
            value={editing.contractEnd ?? ""}
            onChange={(v) => updateField("contractEnd", v || null)}
          />
        </div>
        <LabeledInput
          label={t("common.wage")}
          type="number"
          value={editing.wage?.toString() ?? ""}
          onChange={(v) => updateField("wage", v === "" ? null : Math.max(0, parseInt(v, 10) || 0))}
        />
        <LabeledInput
          label={t("common.value")}
          type="number"
          value={editing.value?.toString() ?? ""}
          onChange={(v) => updateField("value", v === "" ? null : Math.max(0, parseInt(v, 10) || 0))}
        />
        <LabeledInput
          label={t("common.weakFoot")}
          type="number"
          value={editing.weakFoot?.toString() ?? ""}
          onChange={(v) => updateField("weakFoot", parseOptInt(v, 1, 5))}
        />
        <LabeledInput
          label={t("common.condition")}
          type="number"
          value={editing.condition?.toString() ?? ""}
          onChange={(v) => updateField("condition", parseOptInt(v, 0, 100))}
        />
        <LabeledInput
          label={t("common.morale")}
          type="number"
          value={editing.morale?.toString() ?? ""}
          onChange={(v) => updateField("morale", parseOptInt(v, 0, 100))}
        />
      </div>

      {/* Alternate positions */}
      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("worldEditor.playerAlternatePositions")}</label>
        <div className="flex flex-wrap gap-1.5">
          {POSITIONS.map((pos) => {
            const active = altPositions.includes(pos as Position);
            return (
              <button
                key={pos}
                type="button"
                onClick={() => toggleAltPosition(pos as Position)}
                className={`px-2 py-1 rounded-md text-xs border transition ${active ? "bg-primary-500 border-primary-500 text-white" : "border-gray-200 dark:border-navy-600 text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-navy-600"}`}
              >
                {positionLabels[pos]}
              </button>
            );
          })}
        </div>
      </div>

      {/* Career history */}
      <div className="flex flex-col gap-2">
        <label className={labelClass}>{t("worldEditor.playerCareerHistory")}</label>
        {career.map((row, i) => (
          <div key={i} className="flex flex-col gap-1.5 rounded-lg border border-gray-200 dark:border-navy-600 p-2">
            <div className="flex items-center gap-2">
              <div className="flex-1 min-w-0">
                <TeamCombobox
                  value={row.teamId}
                  teams={teamsWithIds}
                  placeholder={t("worldEditor.selectTeam")}
                  onChange={(id) => {
                    const team = teamsWithIds.find((tm) => tm.id === id);
                    updateCareerRow(i, { teamId: id, teamName: team?.name || id });
                  }}
                />
              </div>
              <button
                type="button"
                onClick={() => removeCareerRow(i)}
                aria-label={t("menu.delete")}
                className="p-1.5 text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition flex-shrink-0"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="grid grid-cols-4 gap-2">
              <LabeledInput
                label={t("playerProfile.season")}
                type="number"
                value={row.season?.toString() ?? ""}
                onChange={(v) => updateCareerRow(i, { season: parseInt(v, 10) || 0 })}
              />
              <LabeledInput
                label={t("playerProfile.apps")}
                type="number"
                value={row.appearances?.toString() ?? ""}
                onChange={(v) => updateCareerRow(i, { appearances: Math.max(0, parseInt(v, 10) || 0) })}
              />
              <LabeledInput
                label={t("playerProfile.goals")}
                type="number"
                value={row.goals?.toString() ?? ""}
                onChange={(v) => updateCareerRow(i, { goals: Math.max(0, parseInt(v, 10) || 0) })}
              />
              <LabeledInput
                label={t("playerProfile.assists")}
                type="number"
                value={row.assists?.toString() ?? ""}
                onChange={(v) => updateCareerRow(i, { assists: Math.max(0, parseInt(v, 10) || 0) })}
              />
            </div>
          </div>
        ))}
        <button
          type="button"
          onClick={addCareerRow}
          className="self-start flex items-center gap-1 px-2 py-1 text-xs font-heading font-bold uppercase tracking-wide rounded-lg border border-gray-200 dark:border-navy-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-navy-600 transition"
        >
          <Plus className="w-3.5 h-3.5" /> {t("worldEditor.addCareerEntry")}
        </button>
      </div>
    </EntityFormShell>
    </div>
    <div className="w-64 flex-shrink-0 sticky top-0">
      <PlayerPreviewCard editing={editing} photoDataUrl={photoDataUrl} teams={teams} />
    </div>
    </div>
  );
}
