import { useState } from "react";
import { useTranslation } from "react-i18next";
import { LabeledInput, LabeledSelect } from "./primitives";
import { EntityFormShell } from "./shared";
import { POSITIONS, emptyAttributes } from "./helpers";
import type { PlayerDef } from "./types";

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
  onBack: () => void;
  onSave: () => void;
  updateField: <K extends keyof PlayerDef>(key: K, value: PlayerDef[K]) => void;
}

export function PlayerForm({
  editing,
  editingIndex,
  isBusy,
  onBack,
  onSave,
  updateField,
}: PlayerFormProps) {
  const { t } = useTranslation();
  const [useAttributes, setUseAttributes] = useState(editing.attributes !== null);

  const labelClass =
    "text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400";

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

  const attrs = editing.attributes ?? emptyAttributes();

  return (
    <EntityFormShell
      title={editingIndex === null ? t("packageEditor.addPlayer") : t("packageEditor.editPlayer")}
      onBack={onBack}
      onSave={onSave}
      isBusy={isBusy}
      saveDisabled={!editing.id}
      saveLabel={t("packageEditor.savePlayer")}
    >
      <LabeledInput
        label={t("packageEditor.playerId")}
        value={editing.id}
        onChange={(v) => updateField("id", v)}
        placeholder="player-001"
      />
      <LabeledInput
        label={t("packageEditor.playerFirstName")}
        value={editing.firstName}
        onChange={(v) => updateField("firstName", v)}
      />
      <LabeledInput
        label={t("packageEditor.playerLastName")}
        value={editing.lastName}
        onChange={(v) => updateField("lastName", v)}
      />
      <LabeledInput
        label={t("packageEditor.playerName")}
        value={editing.name}
        onChange={(v) => updateField("name", v)}
        placeholder="Match display name"
      />
      <LabeledInput
        label={t("packageEditor.playerClub")}
        value={editing.club}
        onChange={(v) => updateField("club", v)}
        placeholder="team-id"
      />
      <LabeledInput
        label={t("packageEditor.playerNationality")}
        value={editing.nationality}
        onChange={(v) => updateField("nationality", v)}
        placeholder="ENG"
      />
      <LabeledSelect
        label={t("packageEditor.playerPosition")}
        value={editing.position}
        options={POSITIONS}
        onChange={(v) => updateField("position", v as PlayerDef["position"])}
      />
      <LabeledInput
        label={t("packageEditor.playerDateOfBirth")}
        value={editing.dateOfBirth ?? ""}
        onChange={(v) => updateField("dateOfBirth", v || null)}
        placeholder="2000-01-01"
      />
      <LabeledInput
        label={t("packageEditor.playerAge")}
        value={editing.age?.toString() ?? ""}
        type="number"
        onChange={(v) => updateField("age", v === "" ? null : parseInt(v, 10) || null)}
      />

      <div className="flex items-center gap-2 py-1">
        <input
          id="use-attributes"
          type="checkbox"
          checked={useAttributes}
          onChange={(e) => toggleAttributes(e.target.checked)}
          className="w-4 h-4 rounded border-gray-300 dark:border-navy-600 text-primary-500 focus:ring-primary-400"
        />
        <label htmlFor="use-attributes" className={labelClass}>
          {t("packageEditor.playerUseAttributes")}
        </label>
      </div>

      {!useAttributes && (
        <LabeledInput
          label={t("packageEditor.playerOverall")}
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
    </EntityFormShell>
  );
}
