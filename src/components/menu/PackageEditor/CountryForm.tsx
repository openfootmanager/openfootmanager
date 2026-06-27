import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { LabeledInput } from "./primitives";
import { EntityFormShell } from "./shared";
import { Select } from "../../ui/Select";
import { toSlug } from "./helpers";
import type { ConfederationDef, CountryDef } from "./types";

interface CountryFormProps {
  editing: CountryDef;
  editingIndex: number | null;
  confederations: ConfederationDef[];
  isBusy: boolean;
  onBack: () => void;
  onSave: () => void;
  updateField: <K extends keyof CountryDef>(key: K, value: CountryDef[K]) => void;
}

export function CountryForm({
  editing,
  editingIndex,
  confederations,
  isBusy,
  onBack,
  onSave,
  updateField,
}: CountryFormProps) {
  const { t } = useTranslation();
  const [idAutoMode, setIdAutoMode] = useState(editingIndex === null && !editing.id);

  useEffect(() => {
    setIdAutoMode(editingIndex === null && !editing.id);
  // Reset only when the selected record changes, not as auto-ID populates editing.id
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editingIndex]);

  const labelClass =
    "text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400";
  const inputClass =
    "w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition";

  return (
    <EntityFormShell
      title={editingIndex === null ? t("worldEditor.addCountry") : t("worldEditor.editCountry")}
      onBack={onBack}
      onSave={onSave}
      isBusy={isBusy}
      saveDisabled={!editing.id || !editing.name}
      saveLabel={t("worldEditor.saveCountry")}
    >
      <LabeledInput
        label={t("worldEditor.countryId")}
        value={editing.id}
        onChange={(v) => { setIdAutoMode(false); updateField("id", v); }}
        placeholder="ENG"
      />
      <LabeledInput
        label={t("worldEditor.countryName")}
        value={editing.name}
        onChange={(v) => {
          updateField("name", v);
          if (idAutoMode) updateField("id", toSlug(v));
        }}
        placeholder="England"
      />
      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("worldEditor.countryConfederation")}</label>
        {confederations.length > 0 ? (
          <Select
            value={editing.confederation}
            onChange={(e) => updateField("confederation", e.target.value)}
            fullWidth
          >
            <option value="">—</option>
            {confederations.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name || c.id}
              </option>
            ))}
          </Select>
        ) : (
          <input
            type="text"
            value={editing.confederation}
            onChange={(e) => updateField("confederation", e.target.value)}
            placeholder="europe"
            className={inputClass}
          />
        )}
      </div>
    </EntityFormShell>
  );
}
