import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { LabeledInput, LabeledSelect, labelClass } from "./primitives";
import { EntityFormShell } from "./shared";
import { CountryCombobox } from "../../ui/CountryCombobox";
import { Checkbox } from "../../ui/Checkbox";
import { STAFF_ROLES, COACHING_SPECIALIZATIONS, toSlug } from "./helpers";
import type { StaffDef, TeamDef } from "./types";

const STAFF_ATTR_KEYS = ["coaching", "judgingAbility", "judgingPotential", "physiotherapy"] as const;
type StaffAttrKey = typeof STAFF_ATTR_KEYS[number];

interface StaffFormProps {
  editing: StaffDef;
  editingIndex: number | null;
  isBusy: boolean;
  teams?: TeamDef[];
  onBack: () => void;
  onSave: () => void;
  updateField: <K extends keyof StaffDef>(key: K, value: StaffDef[K]) => void;
}

export function StaffForm({
  editing,
  editingIndex,
  isBusy,
  teams,
  onBack,
  onSave,
  updateField,
}: StaffFormProps) {
  const { t } = useTranslation();
  const [useAttributes, setUseAttributes] = useState(editing.attributes !== null);
  const [idAutoMode, setIdAutoMode] = useState(editingIndex === null && !editing.id);

  useEffect(() => {
    setUseAttributes(editing.attributes !== null);
    setIdAutoMode(editingIndex === null && !editing.id);
  // Reset only when the selected record changes, not as auto-ID populates editing.id
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editingIndex]);

  function handleNameChange(field: "firstName" | "lastName", value: string) {
    updateField(field, value);
    if (idAutoMode) {
      const first = field === "firstName" ? value : editing.firstName;
      const last = field === "lastName" ? value : editing.lastName;
      updateField("id", toSlug(`${first}-${last}`) || "");
    }
  }

  function handleToggleAttributes(checked: boolean) {
    setUseAttributes(checked);
    if (checked && !editing.attributes) {
      updateField("attributes", { coaching: 50, judgingAbility: 50, judgingPotential: 50, physiotherapy: 50 });
    } else if (!checked) {
      updateField("attributes", null);
    }
  }

  function handleAttrChange(key: StaffAttrKey, raw: string) {
    const v = Math.max(1, Math.min(99, parseInt(raw, 10) || 1));
    updateField("attributes", { ...(editing.attributes ?? { coaching: 50, judgingAbility: 50, judgingPotential: 50, physiotherapy: 50 }), [key]: v });
  }

  const teamOptions = teams?.map((t) => t.id) ?? [];
  const teamLabels: Record<string, string> = {};
  teams?.forEach((t) => { teamLabels[t.id] = t.name; });

  const roleLabels: Record<string, string> = {};
  STAFF_ROLES.forEach((r) => {
    roleLabels[r] = t(`worldEditor.staffRole.${r}`, { defaultValue: r });
  });

  const specLabels: Record<string, string> = {};
  COACHING_SPECIALIZATIONS.forEach((s) => {
    specLabels[s] = t(`worldEditor.staffSpecialization.${s}`, { defaultValue: s });
  });

  return (
    <EntityFormShell
      title={editingIndex === null ? t("worldEditor.addStaff") : t("worldEditor.editStaff")}
      saveLabel={t("worldEditor.saveStaff")}
      isBusy={isBusy}
      onBack={onBack}
      onSave={onSave}
    >
      {/* ID */}
      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("worldEditor.staffId")}</label>
        <div className="flex gap-2 items-center">
          <input
            className="flex-1 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition"
            value={editing.id}
            onChange={(e) => { setIdAutoMode(false); updateField("id", e.target.value); }}
            placeholder="e.g. alex-ferguson"
          />
        </div>
      </div>

      {/* Name */}
      <div className="grid grid-cols-2 gap-3">
        <LabeledInput
          label={t("worldEditor.staffFirstName")}
          value={editing.firstName}
          onChange={(v) => handleNameChange("firstName", v)}
          placeholder={t("worldEditor.staffFirstName")}
        />
        <LabeledInput
          label={t("worldEditor.staffLastName")}
          value={editing.lastName}
          onChange={(v) => handleNameChange("lastName", v)}
          placeholder={t("worldEditor.staffLastName")}
        />
      </div>

      {/* Role + Specialization */}
      <div className="grid grid-cols-2 gap-3">
        <LabeledSelect
          label={t("worldEditor.staffRole.label")}
          value={editing.role}
          options={[...STAFF_ROLES]}
          optionLabels={roleLabels}
          onChange={(v) => updateField("role", v as StaffDef["role"])}
        />
        <LabeledSelect
          label={t("worldEditor.staffSpecialization.label")}
          value={editing.specialization ?? ""}
          options={["", ...COACHING_SPECIALIZATIONS]}
          optionLabels={{
            "": t("worldEditor.staffSpecialization.none", { defaultValue: "None" }),
            ...specLabels,
          }}
          onChange={(v) => updateField("specialization", v || null)}
        />
      </div>

      {/* Club + Nationality */}
      <div className="grid grid-cols-2 gap-3">
        <LabeledSelect
          label={t("worldEditor.staffClub")}
          value={editing.club}
          options={["", ...teamOptions]}
          optionLabels={{ "": t("worldEditor.noClubSelected"), ...teamLabels }}
          onChange={(v) => updateField("club", v)}
        />
        <div className="flex flex-col gap-1">
          <label className={labelClass}>{t("worldEditor.staffNationality")}</label>
          <CountryCombobox
            label={t("worldEditor.staffNationality")}
            value={editing.nationality}
            onChange={(v) => updateField("nationality", v)}
          />
        </div>
      </div>

      {/* Date of Birth */}
      <LabeledInput
        label={t("worldEditor.staffDateOfBirth")}
        type="date"
        value={editing.dateOfBirth ?? ""}
        onChange={(v) => updateField("dateOfBirth", v || null)}
      />

      {/* Attributes */}
      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-2">
          <Checkbox
            checked={useAttributes}
            onChange={(e) => handleToggleAttributes(e.target.checked)}
          />
          <span className={labelClass}>{t("worldEditor.staffUseAttributes")}</span>
        </div>
        {useAttributes && editing.attributes != null && (() => {
          const attrs = editing.attributes!;
          return (
            <div className="grid grid-cols-2 gap-3">
              {STAFF_ATTR_KEYS.map((key) => (
                <div key={key} className="flex flex-col gap-1">
                  <label className={labelClass}>
                    {t(`worldEditor.staffAttr.${key}`, { defaultValue: key })}
                  </label>
                  <input
                    type="number"
                    min={1}
                    max={99}
                    className="w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition"
                    value={attrs[key]}
                    onChange={(e) => handleAttrChange(key, e.target.value)}
                  />
                </div>
              ))}
            </div>
          );
        })()}
      </div>
    </EntityFormShell>
  );
}
