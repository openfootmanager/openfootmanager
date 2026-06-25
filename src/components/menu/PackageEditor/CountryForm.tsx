import { useTranslation } from "react-i18next";
import { LabeledInput } from "./primitives";
import { EntityFormShell } from "./shared";
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

  const labelClass =
    "text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400";
  const inputClass =
    "w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition";

  return (
    <EntityFormShell
      title={editingIndex === null ? t("packageEditor.addCountry") : t("packageEditor.editCountry")}
      onBack={onBack}
      onSave={onSave}
      isBusy={isBusy}
      saveDisabled={!editing.id || !editing.name}
      saveLabel={t("packageEditor.saveCountry")}
    >
      <LabeledInput
        label={t("packageEditor.countryId")}
        value={editing.id}
        onChange={(v) => updateField("id", v)}
        placeholder="ENG"
      />
      <LabeledInput
        label={t("packageEditor.countryName")}
        value={editing.name}
        onChange={(v) => updateField("name", v)}
        placeholder="England"
      />
      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("packageEditor.countryConfederation")}</label>
        {confederations.length > 0 ? (
          <select
            value={editing.confederation}
            onChange={(e) => updateField("confederation", e.target.value)}
            className={inputClass}
          >
            <option value="">—</option>
            {confederations.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name || c.id}
              </option>
            ))}
          </select>
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
